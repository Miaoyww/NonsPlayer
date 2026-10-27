package com.miaoyww.nons

import android.os.Handler
import android.os.Looper
import android.os.Bundle
import android.os.SystemClock
import android.net.Uri
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.HeartRating
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.SimpleBasePlayer
import androidx.media3.common.util.UnstableApi
import app.tauri.annotation.InvokeArg
import app.tauri.plugin.Channel
import app.tauri.plugin.JSObject
import com.google.common.util.concurrent.Futures
import com.google.common.util.concurrent.ListenableFuture
import com.google.common.util.concurrent.SettableFuture

@InvokeArg
class PlaybackArgs {
    var sequence = 0L
    var key = ""
    var title = ""
    var artist = ""
    var album = ""
    var cover = ""
    var status = "stopped"
    var positionMs = 0L
    var durationMs = 0L
    var revision = 0L
    var canNext = false
    var canPrevious = false
    var shuffle = false
    var error: String? = null
    val active: Boolean get() = status in listOf("playing", "loading", "buffering")
}

/** No decoder or audio output: every mutation is handled by the Rust playback actor. */
@UnstableApi
class NativeMediaPlayer(private val commands: Channel, cacheDirectory: java.io.File) : SimpleBasePlayer(Looper.getMainLooper()) {
    private var snapshot = PlaybackArgs()
    private var currentState = State.Builder().build()
    private var artwork: ByteArray? = null
    private var favorite = MediaFavoriteArgs()
    private var lyrics = MediaLyricsArgs()
    private var lyricIndex = -1
    private var positionUpdatedAt = SystemClock.elapsedRealtime()
    private val pending = mutableMapOf<Long, SettableFuture<Void>>()
    private val handler = Handler(Looper.getMainLooper())
    private val artworkFiles = ArtworkFileLoader(cacheDirectory, { work -> handler.post(work) }, { file ->
        try {
            android.system.OsConstants.S_ISLNK(android.system.Os.lstat(file.path).st_mode)
        } catch (error: android.system.ErrnoException) { throw java.io.IOException("Artwork path unavailable", error) }
    })
    private val lyricTick = object : Runnable {
        override fun run() {
            if (updateLyricIndex()) rebuild()
            if (snapshot.status == "playing" && lyrics.lines.isNotEmpty()) handler.postDelayed(this, 250)
        }
    }

    fun update(value: PlaybackArgs) {
        if (value.sequence < snapshot.sequence) return
        if (snapshot.key != value.key || snapshot.cover != value.cover) {
            artwork = null
            artworkFiles.invalidate()
        }
        snapshot = value
        positionUpdatedAt = SystemClock.elapsedRealtime()
        if (favorite.key != value.key) favorite = MediaFavoriteArgs()
        if (lyrics.key != value.key) { lyrics = MediaLyricsArgs(); lyricIndex = -1 }
        handler.removeCallbacks(lyricTick)
        updateLyricIndex()
        rebuild()
        if (value.status == "playing" && lyrics.lines.isNotEmpty()) handler.postDelayed(lyricTick, 250)
    }

    fun lyrics(value: MediaLyricsArgs) {
        if (value.key != snapshot.key || value.sequence < lyrics.sequence) return
        lyrics = value
        handler.removeCallbacks(lyricTick)
        updateLyricIndex()
        rebuild()
        if (snapshot.status == "playing" && value.lines.isNotEmpty()) handler.postDelayed(lyricTick, 250)
    }
    fun debugStatus(): JSObject = JSObject().put("lineCount", lyrics.lines.size)
        .put("lineIndex", lyricIndex).put("positionMs", snapshot.positionMs)
        .put("currentLyricLength", lyrics.lines.getOrNull(lyricIndex)?.text?.length ?: 0)

    private fun updateLyricIndex(): Boolean {
        val position = snapshot.positionMs + if (snapshot.status == "playing") SystemClock.elapsedRealtime() - positionUpdatedAt else 0
        var low = 0; var high = lyrics.lines.size
        while (low < high) {
            val middle = (low + high) / 2
            if (lyrics.lines[middle].startTime <= position) low = middle + 1 else high = middle
        }
        val found = low - 1
        // Keep the current line during the gap before the next line, like the in-app player.
        val next = if (found >= 0 && (snapshot.durationMs <= 0 || position < snapshot.durationMs)) found else -1
        if (next == lyricIndex) return false
        lyricIndex = next
        return true
    }

    fun favorite(value: MediaFavoriteArgs) {
        if (value.key != snapshot.key || value.sequence < favorite.sequence) return
        favorite = value
        rebuild()
    }

    fun custom(action: String): ListenableFuture<*> {
        if (action == "favorite" && !favorite.available)
            return Futures.immediateFailedFuture<Void>(IllegalStateException("当前歌曲无法收藏"))
        return request(action)
    }
    fun setHeart(liked: Boolean): ListenableFuture<*> {
        if (!favorite.available) return Futures.immediateFailedFuture<Void>(IllegalStateException("当前歌曲无法收藏"))
        if (favorite.liked == liked) return Futures.immediateVoidFuture()
        return request("favorite", liked = liked)
    }

    fun artwork(key: String, cover: String, path: String) {
        if (key != snapshot.key || cover != snapshot.cover) return
        artworkFiles.load(path) { bytes ->
            if (key != snapshot.key || cover != snapshot.cover) return@load
            artwork = bytes
            rebuild()
        }
    }

    private fun rebuild() {
        val s = snapshot
        val hasTrack = s.key.isNotEmpty()
        val available = Player.Commands.Builder().addAll(
            Player.COMMAND_GET_CURRENT_MEDIA_ITEM, Player.COMMAND_GET_METADATA,
            Player.COMMAND_GET_TIMELINE, Player.COMMAND_GET_AUDIO_ATTRIBUTES,
            Player.COMMAND_GET_VOLUME, Player.COMMAND_RELEASE
        )
        if (hasTrack) available.addAll(Player.COMMAND_PLAY_PAUSE, Player.COMMAND_STOP, Player.COMMAND_PREPARE)
        if (hasTrack && s.durationMs > 0) available.add(Player.COMMAND_SEEK_IN_CURRENT_MEDIA_ITEM)
        if (s.canNext) available.addAll(Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM)
        if (s.canPrevious) available.addAll(Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM)
        val metadata = MediaMetadata.Builder().setTitle(s.title.ifEmpty { "Nons 音乐播放" })
            .setArtist(s.artist).setAlbumTitle(s.album).setIsPlayable(true)
            .setMediaType(MediaMetadata.MEDIA_TYPE_MUSIC)
        val extras = Bundle().apply {
            putString("hash", s.key)
            putString("songId", s.key.substringAfter(':', s.key))
            // KA-Music-VIVO documents 0x01f as transport/lyrics/progress and 0x040 as heart.
            // Advertise only implemented capabilities, without its queue/library flags.
            putLong("vivomusicmix.media.metadata.support_event", 0x01fL or if (favorite.available) 0x040L else 0L)
            // Vivo/other compatible consumers read current-line fields; these are vendor extensions.
            val line = lyrics.lines.getOrNull(lyricIndex)
            putString("lyric", line?.text ?: "")
            putString("currentLyric", line?.text ?: "")
            putString("translationLyric", line?.translation ?: "")
            putString("romanLyric", line?.romanization ?: "")
        }
        metadata.setExtras(extras)
        // Media3 ignores extras contents in equality checks. A semantic description change
        // emits EVENT_MEDIA_METADATA_CHANGED so the platform receives each new lyric line.
        val currentLine = lyrics.lines.getOrNull(lyricIndex)
        metadata.setDescription(listOfNotNull(currentLine?.text, currentLine?.translation, currentLine?.romanization)
            .filter { it.isNotEmpty() }.joinToString("\n"))
        if (favorite.available) metadata.setUserRating(HeartRating(favorite.liked))
        artwork?.let {
            metadata.setArtworkData(it, MediaMetadata.PICTURE_TYPE_FRONT_COVER)
            if (s.cover.startsWith("https://") || s.cover.startsWith("http://")) metadata.setArtworkUri(Uri.parse(s.cover))
        }
        val playlist = mutableListOf<MediaItemData>()
        // A bounded navigation window allows BasePlayer's next/previous methods to route correctly.
        if (hasTrack && s.canPrevious) playlist.add(MediaItemData.Builder("previous:${s.key}").build())
        val index = playlist.size
        if (hasTrack || s.active) playlist.add(
            MediaItemData.Builder(s.key.ifEmpty { "loading" })
                .setMediaItem(MediaItem.Builder().setMediaId(s.key).setMediaMetadata(metadata.build()).build())
                .setDurationUs(if (s.durationMs > 0) s.durationMs * 1000 else C.TIME_UNSET)
                .setIsSeekable(s.durationMs > 0).build()
        )
        if (hasTrack && s.canNext) playlist.add(MediaItemData.Builder("next:${s.key}").build())
        val state = when {
            playlist.isEmpty() -> Player.STATE_IDLE
            s.status == "loading" || s.status == "buffering" -> Player.STATE_BUFFERING
            s.status == "playing" || s.status == "paused" -> Player.STATE_READY
            else -> Player.STATE_IDLE
        }
        val builder = State.Builder().setAvailableCommands(available.build()).setPlaylist(playlist)
            .setShuffleModeEnabled(s.shuffle)
            .setCurrentMediaItemIndex(if (playlist.isEmpty()) C.INDEX_UNSET else index)
            .setPlaybackState(state).setIsLoading(state == Player.STATE_BUFFERING)
            .setPlayWhenReady(s.active, Player.PLAY_WHEN_READY_CHANGE_REASON_USER_REQUEST)
            .setContentPositionMs(s.positionMs.coerceAtLeast(0))
        if (s.status == "error") builder.setPlayerError(
            PlaybackException(s.error ?: "播放失败", null, PlaybackException.ERROR_CODE_UNSPECIFIED)
        )
        currentState = builder.build()
        invalidateState()
    }

    fun complete(id: Long, value: PlaybackArgs?, error: String?) {
        value?.let { update(it) }
        pending.remove(id)?.let { future ->
            if (error == null) future.set(null) else future.setException(IllegalStateException(error))
        }
    }

    private fun request(action: String, positionMs: Long = 0, liked: Boolean? = null): ListenableFuture<*> {
        if (pending.size >= 8) return Futures.immediateFailedFuture<Void>(IllegalStateException("媒体命令过多"))
        val id = commandSequence.incrementAndGet()
        val future = SettableFuture.create<Void>()
        pending[id] = future
        commands.send(JSObject().put("id", id).put("action", action).put("positionMs", positionMs)
            .put("key", snapshot.key).put("favoriteSequence", favorite.sequence).put("liked", liked))
        handler.postDelayed({ pending.remove(id)?.setException(IllegalStateException("媒体命令超时")) },
            if (action == "favorite") 20000 else 6000)
        return future
    }
    override fun getState(): State = currentState
    override fun handleSetPlayWhenReady(playWhenReady: Boolean): ListenableFuture<*> = request(if (playWhenReady) "play" else "pause")
    override fun handlePrepare(): ListenableFuture<*> = Futures.immediateVoidFuture()
    override fun handleStop(): ListenableFuture<*> = request("stop")
    override fun handleSeek(mediaItemIndex: Int, positionMs: Long, seekCommand: Int): ListenableFuture<*> = when (seekCommand) {
        Player.COMMAND_SEEK_TO_NEXT, Player.COMMAND_SEEK_TO_NEXT_MEDIA_ITEM -> request("next")
        Player.COMMAND_SEEK_TO_PREVIOUS, Player.COMMAND_SEEK_TO_PREVIOUS_MEDIA_ITEM -> request("previous")
        else -> request("seek", positionMs.coerceAtLeast(0).coerceAtMost(snapshot.durationMs))
    }
    override fun handleRelease(): ListenableFuture<*> {
        handler.removeCallbacksAndMessages(null)
        artworkFiles.close()
        pending.values.forEach { it.setException(IllegalStateException("媒体会话已释放")) }
        pending.clear()
        artwork = null
        return Futures.immediateVoidFuture()
    }
    companion object { private val commandSequence = java.util.concurrent.atomic.AtomicLong() }
}
