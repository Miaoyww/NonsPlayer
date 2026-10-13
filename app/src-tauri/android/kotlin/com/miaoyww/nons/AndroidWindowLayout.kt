package com.miaoyww.nons

import android.app.Activity
import android.os.Build
import android.view.View
import android.view.WindowManager
import android.webkit.WebView
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import app.tauri.plugin.JSObject

/** The native content owns keyboard resizing; CSS owns system-bar/cutout spacing. */
class AndroidWindowLayout(private val activity: Activity, private val webView: WebView) {
    private val content = activity.findViewById<View>(android.R.id.content)
    private var immersive = false
    private var lastInsets: WindowInsetsCompat? = null
    private var lastPayload = ""

    init {
        activity.window.setSoftInputMode(
            if (Build.VERSION.SDK_INT >= 30) WindowManager.LayoutParams.SOFT_INPUT_ADJUST_NOTHING
            else WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE
        )
        ViewCompat.setOnApplyWindowInsetsListener(content) { _, insets ->
            lastInsets = insets
            val keyboard = insets.getInsets(WindowInsetsCompat.Type.ime()).bottom
            // Older Android versions resize the window for us; Android 11+ uses IME insets.
            val bottom = if (Build.VERSION.SDK_INT >= 30) keyboard else 0
            if (content.paddingBottom != bottom) content.setPadding(0, 0, 0, bottom)
            publish(insets)
            // Do not let WebView apply another safe-area inset to the same content.
            WindowInsetsCompat.CONSUMED
        }
        ViewCompat.requestApplyInsets(content)
    }

    fun update(fullscreen: Boolean, dark: Boolean) {
        immersive = fullscreen
        val controller = WindowCompat.getInsetsController(activity.window, activity.window.decorView)
        controller.isAppearanceLightStatusBars = !dark && !fullscreen
        controller.isAppearanceLightNavigationBars = !dark && !fullscreen
        controller.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        if (fullscreen) controller.hide(WindowInsetsCompat.Type.systemBars())
        else controller.show(WindowInsetsCompat.Type.systemBars())
        // Resend after frontend readiness/navigation, even if native insets did not change.
        lastPayload = ""
        (lastInsets ?: ViewCompat.getRootWindowInsets(content))?.let { publish(it) }
        ViewCompat.requestApplyInsets(content)
    }

    private fun publish(insets: WindowInsetsCompat) {
        val cutout = insets.getInsets(WindowInsetsCompat.Type.displayCutout())
        val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
        val gestures = insets.getInsets(WindowInsetsCompat.Type.mandatorySystemGestures())
        val keyboard = insets.isVisible(WindowInsetsCompat.Type.ime())
        val scale = webView.resources.displayMetrics.density
        val payload = JSObject()
        payload.put("top", maxOf(cutout.top, if (immersive) 0 else bars.top) / scale)
        payload.put("left", maxOf(cutout.left, if (immersive) 0 else bars.left) / scale)
        payload.put("right", maxOf(cutout.right, if (immersive) 0 else bars.right) / scale)
        payload.put("bottom", if (keyboard) 0f else maxOf(cutout.bottom, if (immersive) gestures.bottom else bars.bottom) / scale)
        val json = payload.toString()
        if (json == lastPayload) return
        lastPayload = json
        webView.evaluateJavascript(
            "(()=>{const i=$json;for(const k of ['top','right','bottom','left'])document.documentElement.style.setProperty('--nons-safe-area-'+k,i[k]+'px')})()",
            null
        )
    }
}
