package com.miaoyww.nons

import android.app.PendingIntent
import android.content.Intent
import android.os.PowerManager
import android.os.Bundle
import androidx.media3.common.Rating
import androidx.media3.common.HeartRating
import androidx.media3.session.CommandButton
import androidx.media3.session.SessionCommand
import androidx.media3.session.SessionResult
import androidx.media3.session.SessionError
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture
import com.google.common.util.concurrent.MoreExecutors
import androidx.media3.common.util.UnstableApi
import androidx.media3.session.DefaultMediaNotificationProvider
import androidx.media3.session.MediaSession
import androidx.media3.session.MediaSessionService
import androidx.media3.session.CacheBitmapLoader
import androidx.media3.datasource.DataSourceBitmapLoader

/** Media3 owns the session/notification; the Rust actor coordinates WebAudio playback. */
@UnstableApi
class PlaybackService : MediaSessionService() {
    var player: NativeMediaPlayer? = null
        private set
    private var session: MediaSession? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var buttonState: List<Boolean>? = null
    private val shuffleCommand = SessionCommand("nons.shuffle", Bundle.EMPTY)
    private val favoriteCommand = SessionCommand("nons.favorite", Bundle.EMPTY)

    override fun onCreate() {
        super.onCreate()
        val commands = MediaSessionBridge.commands
        if (commands == null) { stopSelf(); return }
        val nativePlayer = NativeMediaPlayer(commands, cacheDir)
        player = nativePlayer
        nativePlayer.update(MediaSessionBridge.latest)
        val open = PendingIntent.getActivity(this, 0, Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
        session = MediaSession.Builder(this, nativePlayer).setSessionActivity(open)
            .setBitmapLoader(CacheBitmapLoader(DataSourceBitmapLoader.Builder(this).setMaximumOutputDimension(512).build()))
            .setCallback(object : MediaSession.Callback {
                override fun onConnectAsync(session: MediaSession, controller: MediaSession.ControllerInfo): ListenableFuture<MediaSession.ConnectionResult> {
                    val result = MediaSession.ConnectionResult.AcceptedResultBuilder(session, controller)
                    if (controller.isTrusted) result.setAvailableSessionCommands(
                        MediaSession.ConnectionResult.DEFAULT_SESSION_COMMANDS.buildUpon()
                            .add(shuffleCommand).add(favoriteCommand).build())
                    return Futures.immediateFuture(result.build())
                }

                override fun onCustomCommand(session: MediaSession, controller: MediaSession.ControllerInfo,
                    command: SessionCommand, args: Bundle): ListenableFuture<SessionResult> {
                    val action = when (command.customAction) {
                        shuffleCommand.customAction -> "shuffle"
                        favoriteCommand.customAction -> "favorite"
                        else -> return Futures.immediateFuture(SessionResult(SessionError.ERROR_NOT_SUPPORTED))
                    }
                    return commandResult(nativePlayer.custom(action))
                }
                override fun onSetRating(session: MediaSession, controller: MediaSession.ControllerInfo,
                    rating: Rating): ListenableFuture<SessionResult> {
                    if (rating !is HeartRating || !rating.isRated)
                        return Futures.immediateFuture(SessionResult(SessionError.ERROR_BAD_VALUE))
                    return commandResult(nativePlayer.setHeart(rating.isHeart))
                }
                override fun onSetRating(session: MediaSession, controller: MediaSession.ControllerInfo,
                    mediaId: String, rating: Rating): ListenableFuture<SessionResult> {
                    if (mediaId != nativePlayer.currentMediaItem?.mediaId)
                        return Futures.immediateFuture(SessionResult(SessionError.ERROR_BAD_VALUE))
                    return onSetRating(session, controller, rating)
                }
            })
            .setMediaButtonPreferences(buttons())
            .build()
        addSession(requireNotNull(session))
        val notifications = DefaultMediaNotificationProvider.Builder(this)
            .setNotificationId(1).setChannelId("nons-playback").build()
        notifications.setSmallIcon(R.drawable.ic_playback_notification)
        setMediaNotificationProvider(notifications)
        setForegroundServiceTimeoutMs(0)
        wakeLock = getSystemService(PowerManager::class.java)
            .newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "Nons:playback").apply { setReferenceCounted(false) }
        MediaSessionBridge.attached(this)
        update(MediaSessionBridge.latest)
        triggerNotificationUpdate()
    }

    fun update(value: PlaybackArgs) {
        player?.update(value)
        updateButtons()
        if (value.active) { if (wakeLock?.isHeld == false) wakeLock?.acquire() }
        else if (wakeLock?.isHeld == true) wakeLock?.release()
    }
    private fun commandResult(operation: ListenableFuture<*>): ListenableFuture<SessionResult> {
        val result = SettableFuture.create<SessionResult>()
        operation.addListener({
            try { operation.get(); result.set(SessionResult(SessionResult.RESULT_SUCCESS)) }
            catch (error: Exception) { result.set(SessionResult(SessionError(SessionError.ERROR_UNKNOWN,
                error.cause?.message ?: error.message ?: "媒体操作失败"))) }
        }, MoreExecutors.directExecutor())
        return result
    }

    fun favorite(value: MediaFavoriteArgs) {
        player?.favorite(value)
        updateButtons()
    }
    fun debugStatus(): app.tauri.plugin.JSObject {
        val result = player?.debugStatus() ?: app.tauri.plugin.JSObject()
        val metadata = session?.let { android.media.session.MediaController(this, it.platformToken).metadata }
        result.put("platformLyricLength", metadata?.getString("lyric")?.length ?: -1)
        result.put("platformCurrentLyricLength", metadata?.getString("currentLyric")?.length ?: -1)
        result.put("platformBitmap", metadata?.getBitmap(android.media.MediaMetadata.METADATA_KEY_ALBUM_ART) != null)
        return result
    }

    private fun buttons(): List<CommandButton> {
        val snapshot = MediaSessionBridge.latest
        val favorite = MediaSessionBridge.favorite
        return listOf(
            CommandButton.Builder(if (snapshot.shuffle) CommandButton.ICON_SHUFFLE_ON else CommandButton.ICON_SHUFFLE_OFF)
                .setDisplayName(if (snapshot.shuffle) "关闭随机播放" else "开启随机播放")
                .setSessionCommand(shuffleCommand).setEnabled(snapshot.key.isNotEmpty())
                .setSlots(CommandButton.SLOT_OVERFLOW).build(),
            CommandButton.Builder(if (favorite.liked) CommandButton.ICON_HEART_FILLED else CommandButton.ICON_HEART_UNFILLED)
                .setDisplayName(if (favorite.liked) "取消收藏" else "收藏")
                .setSessionCommand(favoriteCommand).setEnabled(favorite.available)
                .setSlots(CommandButton.SLOT_OVERFLOW).build()
        )
    }
    private fun updateButtons() {
        val next = listOf(MediaSessionBridge.latest.shuffle, MediaSessionBridge.favorite.liked,
            MediaSessionBridge.favorite.available, MediaSessionBridge.latest.key.isNotEmpty())
        if (next != buttonState) { session?.setMediaButtonPreferences(buttons()); buttonState = next }
    }

    override fun onGetSession(controllerInfo: MediaSession.ControllerInfo): MediaSession? = session

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (MediaSessionBridge.commands == null) { stopSelf(); return START_NOT_STICKY }
        if (intent?.action == SYNC) {
            update(MediaSessionBridge.latest)
            triggerNotificationUpdate()
            return START_NOT_STICKY
        }
        super.onStartCommand(intent, flags, startId)
        return START_NOT_STICKY
    }

    override fun onDestroy() {
        if (wakeLock?.isHeld == true) wakeLock?.release()
        wakeLock = null
        session?.release()
        session = null
        player?.release()
        player = null
        MediaSessionBridge.detached(this)
        super.onDestroy()
    }

    companion object { const val SYNC = "com.miaoyww.nons.MEDIA_SYNC" }
}
