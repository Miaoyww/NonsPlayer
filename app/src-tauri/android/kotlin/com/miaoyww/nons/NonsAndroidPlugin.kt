package com.miaoyww.nons

import android.app.Activity
import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
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

/** Keys stay in Android Keystore; only AES-GCM ciphertext is persisted. */
@TauriPlugin
class NonsAndroidPlugin(private val activity: Activity) : Plugin(activity) {
    private val alias = "NonsPlayer.credentials.v1"

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
