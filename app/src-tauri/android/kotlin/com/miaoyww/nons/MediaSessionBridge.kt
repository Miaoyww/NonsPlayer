package com.miaoyww.nons

import android.content.Context
import android.content.Intent
import androidx.core.content.ContextCompat
import androidx.media3.common.util.UnstableApi
import app.tauri.plugin.Channel

/** Process-scoped bridge, without an Activity or WebView reference. Main-thread only. */
@UnstableApi
object MediaSessionBridge {
    var commands: Channel? = null
    var latest = PlaybackArgs()
    var favorite = MediaFavoriteArgs()
    var lyrics = MediaLyricsArgs()
    var service: PlaybackService? = null
    private var starting = false
    private var artworkKey = ""
    private var artworkCover = ""
    private var artworkData = ""

    fun update(context: Context, value: PlaybackArgs) {
        if (value.sequence < latest.sequence) return
        if (value.key != latest.key || value.cover != latest.cover) artworkData = ""
        if (value.key != latest.key) favorite = MediaFavoriteArgs()
        if (value.key != latest.key) lyrics = MediaLyricsArgs()
        latest = value
        service?.update(value)
        if (value.active && service == null && !starting && commands != null) {
            starting = true
            try {
                ContextCompat.startForegroundService(context, Intent(context, PlaybackService::class.java).setAction(PlaybackService.SYNC))
            } catch (error: Exception) { starting = false; throw error }
        }
    }
    fun artwork(key: String, cover: String, data: String) {
        if (key != latest.key || cover != latest.cover || data.length > 6 * 1024 * 1024) return
        artworkKey = key; artworkCover = cover; artworkData = data
        service?.player?.artwork(key, cover, data)
    }
    fun attached(value: PlaybackService) {
        service = value; starting = false
        if (artworkData.isNotEmpty()) value.player?.artwork(artworkKey, artworkCover, artworkData)
        value.favorite(favorite)
        value.player?.lyrics(lyrics)
    }
    fun favorite(value: MediaFavoriteArgs) {
        if (value.key != latest.key || value.sequence < favorite.sequence) return
        favorite = value
        service?.favorite(value)
    }
    fun lyrics(value: MediaLyricsArgs) {
        if (value.key != latest.key || value.sequence < lyrics.sequence) return
        lyrics = value
        service?.player?.lyrics(value)
    }
    fun detached(value: PlaybackService) { if (service === value) service = null; starting = false }
    fun shutdown(context: Context) {
        latest = PlaybackArgs()
        favorite = MediaFavoriteArgs()
        lyrics = MediaLyricsArgs()
        service?.update(latest)
        commands = null
        artworkData = ""
        context.stopService(Intent(context, PlaybackService::class.java))
        starting = false
    }
}
