package com.miaoyww.nons

import android.app.Instrumentation
import android.app.Activity
import android.content.ComponentName
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import androidx.media3.common.HeartRating
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.CommandButton
import androidx.media3.session.MediaController
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import androidx.media3.session.SessionToken
import java.util.concurrent.FutureTask
import java.util.concurrent.TimeUnit

/** Device check against an existing playable queue; restores shuffle/like and ends paused. */
@UnstableApi
class MediaSessionCheck : Instrumentation() {
    private var coverOnly = false
    private fun <T> main(block: () -> T): T {
        val task = FutureTask(block)
        Handler(Looper.getMainLooper()).post(task)
        return task.get(5, TimeUnit.SECONDS)
    }
    private fun waitFor(message: String, condition: () -> Boolean) {
        repeat(120) { if (main(condition)) return; Thread.sleep(250) }
        error(message)
    }
    override fun onCreate(arguments: Bundle?) { super.onCreate(arguments); coverOnly = arguments?.getString("coverOnly") == "true"; start() }
    override fun onStart() {
        var controller: MediaController? = null
        var originalShuffle: Boolean? = null
        var originalLike: Boolean? = null
        var favoriteChanged = false
        val report = Bundle()
        var code = Activity.RESULT_OK
        try {
            uiAutomation.adoptShellPermissionIdentity("android.permission.MEDIA_CONTENT_CONTROL")
            val sessions = targetContext.getSystemService(android.media.session.MediaSessionManager::class.java).getActiveSessions(null)
            for (reference in sessions.filter { it.packageName == "com.netease.cloudmusic" }) {
                val data = reference.metadata
                sendStatus(0, Bundle().apply {
                    putString("reference", reference.packageName)
                    putString("referenceKeys", data?.keySet()?.sorted()?.joinToString(","))
                    for (key in listOf("lyric", "currentLyric", "translationLyric", "romanLyric", "lyricInfo", "lyrics", "android.media.metadata.LYRIC"))
                        putInt("referenceLength:$key", data?.getString(key)?.length ?: -1)
                    val events = "vivomusicmix.media.metadata.support_event"
                    putString("supportEventsText", data?.getString(events))
                    putLong("supportEventsNumber", data?.getLong(events) ?: 0)
                })
            }
            val platform = sessions.firstOrNull { it.packageName == "com.miaoyww.nons" }
            check(platform != null) { "Start Nons playback before running this check" }
            val metadata = platform.metadata
            sendStatus(0, Bundle().apply {
                putString("stage", "platform metadata")
                putBoolean("bitmap", metadata?.getBitmap(android.media.MediaMetadata.METADATA_KEY_ALBUM_ART) != null)
                putInt("lyricLength", metadata?.getString("lyric")?.length ?: -1)
                putInt("currentLyricLength", metadata?.getString("currentLyric")?.length ?: -1)
                putString("metadataKeys", metadata?.keySet()?.sorted()?.joinToString(","))
            })
            sendStatus(0, Bundle().apply { putString("stage", "connect") })
            val future = main { MediaController.Builder(targetContext,
                SessionToken(targetContext, ComponentName("com.miaoyww.nons", "com.miaoyww.nons.PlaybackService"))).buildAsync() }
            val media = future.get(15, TimeUnit.SECONDS)
            controller = media
            waitFor("Prepare a playable queue before running this check") { media.currentMediaItem != null }
            sendStatus(0, Bundle().apply { putString("stage", "metadata") })
            if (coverOnly) {
                waitFor("Cover missing from Media3 metadata") { media.mediaMetadata.artworkData?.isNotEmpty() == true }
                report.putString("result", "PASS: cover bytes present in MediaSession metadata")
                main { media.release() }
                controller = null
                finish(Activity.RESULT_OK, report)
                return
            }
            main { media.play() }
            waitFor("Native play failed") { media.isPlaying }
            main { media.pause() }
            waitFor("Native pause failed") { !media.playWhenReady }
            val position = main { media.currentPosition }
            val duration = main { media.duration }
            check(duration > 10000) { "A track longer than 10 seconds is required" }
            main { media.seekTo(5000) }
            waitFor("Native seek failed") { kotlin.math.abs(media.currentPosition - 5000) < 500 }
            main { media.seekTo(position.coerceIn(0, duration)) }
            originalShuffle = main { media.shuffleModeEnabled }
            fun custom(action: String) {
                val result = main { media.sendCustomCommand(SessionCommand(action, Bundle.EMPTY), Bundle.EMPTY) }
                    .get(25, TimeUnit.SECONDS)
                check(result.resultCode == SessionResult.RESULT_SUCCESS) { "$action failed: ${result.resultCode}" }
            }
            custom("nons.shuffle")
            waitFor("Shuffle did not synchronize") { media.shuffleModeEnabled != originalShuffle }
            custom("nons.shuffle")
            waitFor("Shuffle did not restore") { media.shuffleModeEnabled == originalShuffle }
            fun heart(): CommandButton? = media.mediaButtonPreferences.firstOrNull {
                it.sessionCommand?.customAction == "nons.favorite"
            }
            waitFor("Custom buttons are missing") { heart() != null }
            // Account/source support is optional. The disabled action must reject unauthorized writes.
            Thread.sleep(3000)
            if (main { heart()?.isEnabled == true }) {
                originalLike = main { (media.mediaMetadata.userRating as? HeartRating)?.isHeart }
                check(originalLike != null) { "Favorite metadata is missing" }
                custom("nons.favorite")
                favoriteChanged = true
                waitFor("Favorite state did not synchronize") {
                    heart()?.isEnabled == true && (media.mediaMetadata.userRating as? HeartRating)?.isHeart != originalLike
                }
                custom("nons.favorite")
                favoriteChanged = false
                waitFor("Favorite state did not restore") {
                    heart()?.isEnabled == true && (media.mediaMetadata.userRating as? HeartRating)?.isHeart == originalLike
                }
                report.putString("favorite", "PASS (toggle and restore)")
            } else {
                val result = main { media.sendCustomCommand(SessionCommand("nons.favorite", Bundle.EMPTY), Bundle.EMPTY) }
                    .get(25, TimeUnit.SECONDS)
                check(result.resultCode != SessionResult.RESULT_SUCCESS) { "Disabled favorite accepted a write" }
                report.putString("favorite", "PASS (unsupported/logged-out action rejected)")
            }
            report.putString("result", "PASS: Media3 native controller play/pause/seek/shuffle/favorite")
        } catch (error: Throwable) {
            report.putString("result", "FAIL: ${error.message}")
            code = Activity.RESULT_CANCELED
        } finally {
            controller?.let { media ->
                if (favoriteChanged && originalLike != null) runCatching {
                    main { media.sendCustomCommand(SessionCommand("nons.favorite", Bundle.EMPTY), Bundle.EMPTY) }
                        .get(25, TimeUnit.SECONDS)
                }
                originalShuffle?.let { original -> if (main { media.shuffleModeEnabled } != original) runCatching {
                    main { media.sendCustomCommand(SessionCommand("nons.shuffle", Bundle.EMPTY), Bundle.EMPTY) }
                        .get(10, TimeUnit.SECONDS)
                } }
                main { media.pause(); media.release() }
            }
            uiAutomation.dropShellPermissionIdentity()
        }
        finish(code, report)
    }
}
