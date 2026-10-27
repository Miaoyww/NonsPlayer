package com.miaoyww.nons

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import android.webkit.WebView
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import app.tauri.plugin.Channel
import androidx.media3.common.util.UnstableApi
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

@InvokeArg
class CredentialArgs {
    var service: String = ""
    var key: String = ""
    var operation: String = ""
    var value: String? = null
}

@InvokeArg
class WindowLayoutArgs {
    var immersive: Boolean = false
    var dark: Boolean = false
}

@InvokeArg
class MediaInitializeArgs { lateinit var commands: Channel }
@InvokeArg
class MediaCommandResultArgs { var id = 0L; var snapshot: PlaybackArgs? = null; var error: String? = null }
@InvokeArg
class MediaArtworkArgs { var key = ""; var cover = ""; var path = "" }
@InvokeArg
class MediaFavoriteArgs { var key = ""; var sequence = 0L; var liked = false; var available = false }
@InvokeArg
class MediaLyricLine { var startTime = 0L; var endTime = 0L; var text = ""; var translation = ""; var romanization = "" }
@InvokeArg
class MediaLyricsArgs { var key = ""; var sequence = 0L; var lines: List<MediaLyricLine> = emptyList() }

/** Keys stay in Android Keystore; only AES-GCM ciphertext is persisted. */
@TauriPlugin
@UnstableApi
class NonsAndroidPlugin(private val activity: Activity) : Plugin(activity) {
    private val alias = "NonsPlayer.credentials.v1"
    private var windowLayout: AndroidWindowLayout? = null

    override fun load(webView: WebView) {
        activity.runOnUiThread { windowLayout = AndroidWindowLayout(activity, webView) }
    }

    @Command
    fun mediaInitialize(invoke: Invoke) {
        val args = invoke.parseArgs(MediaInitializeArgs::class.java)
        activity.runOnUiThread { MediaSessionBridge.commands = args.commands; invoke.resolve() }
    }

    @Command
    fun mediaCommandResult(invoke: Invoke) {
        val args = invoke.parseArgs(MediaCommandResultArgs::class.java)
        activity.runOnUiThread {
            MediaSessionBridge.service?.player?.complete(args.id, args.snapshot, args.error)
            invoke.resolve()
        }
    }

    @Command
    fun mediaArtwork(invoke: Invoke) {
        val args = invoke.parseArgs(MediaArtworkArgs::class.java)
        activity.runOnUiThread {
            MediaSessionBridge.artwork(args.key, args.cover, args.path)
            invoke.resolve()
        }
    }

    @Command
    fun mediaFavorite(invoke: Invoke) {
        val args = invoke.parseArgs(MediaFavoriteArgs::class.java)
        activity.runOnUiThread { MediaSessionBridge.favorite(args); invoke.resolve() }
    }

    @Command
    fun mediaLyrics(invoke: Invoke) {
        val args = invoke.parseArgs(MediaLyricsArgs::class.java)
        activity.runOnUiThread { MediaSessionBridge.lyrics(args); invoke.resolve() }
    }
    @Command
    fun mediaStatus(invoke: Invoke) {
        activity.runOnUiThread {
            invoke.resolve(MediaSessionBridge.service?.debugStatus() ?: JSObject().put("active", false))
        }
    }

    @Command
    fun mediaShutdown(invoke: Invoke) {
        activity.runOnUiThread { MediaSessionBridge.shutdown(activity.applicationContext); invoke.resolve() }
    }

    @Command
    fun playback(invoke: Invoke) {
        val args = invoke.parseArgs(PlaybackArgs::class.java)
        activity.runOnUiThread {
            try {
                MediaSessionBridge.update(activity.applicationContext, args)
                invoke.resolve()
            } catch (_: Exception) {
                invoke.reject("Android 后台播放服务启动失败，请返回应用后重试")
            }
        }
    }

    @Command
    fun windowLayout(invoke: Invoke) {
        val args = invoke.parseArgs(WindowLayoutArgs::class.java)
        activity.runOnUiThread {
            val layout = windowLayout
            if (layout == null) invoke.reject("Android 窗口尚未就绪")
            else {
                layout.update(args.immersive, args.dark)
                invoke.resolve()
            }
        }
    }

    private fun encryptionKey(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(alias, null) as? SecretKey)?.let { return it }
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
        generator.init(
            KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setKeySize(256)
                .build()
        )
        return generator.generateKey()
    }

    @Command
    @Synchronized
    fun credentials(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(CredentialArgs::class.java)
            require(args.service.isNotEmpty() && args.key.isNotEmpty() && args.key.length <= 1024)
            val identity = "${args.service}\u0000${args.key}".toByteArray(Charsets.UTF_8)
            val slot = MessageDigest.getInstance("SHA-256").digest(identity)
                .joinToString("") { "%02x".format(it) }
            val preferences = activity.getSharedPreferences("nons-encrypted-credentials", Context.MODE_PRIVATE)
            val result = JSObject()
            when (args.operation) {
                "read" -> {
                    val stored = preferences.getString(slot, null)
                    if (stored != null) {
                        val bytes = Base64.decode(stored, Base64.NO_WRAP)
                        require(bytes.size >= 28 && bytes.size <= 262144)
                        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                        cipher.init(Cipher.DECRYPT_MODE, encryptionKey(), GCMParameterSpec(128, bytes.copyOfRange(0, 12)))
                        cipher.updateAAD(identity)
                        result.put("value", Base64.encodeToString(cipher.doFinal(bytes.copyOfRange(12, bytes.size)), Base64.NO_WRAP))
                    }
                }
                "write" -> {
                    val value = requireNotNull(args.value)
                    require(value.length <= 262144)
                    val bytes = Base64.decode(value, Base64.NO_WRAP)
                    require(bytes.size <= 196608)
                    val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                    cipher.init(Cipher.ENCRYPT_MODE, encryptionKey())
                    cipher.updateAAD(identity)
                    val stored = Base64.encodeToString(cipher.iv + cipher.doFinal(bytes), Base64.NO_WRAP)
                    check(preferences.edit().putString(slot, stored).commit())
                }
                "delete" -> check(preferences.edit().remove(slot).commit())
                else -> error("Unknown credential operation")
            }
            invoke.resolve(result)
        } catch (_: Exception) {
            // Do not put credential values, keys, or platform error details in logs.
            invoke.reject("Android 凭据存储操作失败")
        }
    }
}
