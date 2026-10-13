package com.miaoyww.nons

import android.os.Bundle
import android.os.Build
import android.view.WindowManager
import androidx.activity.enableEdgeToEdge
import org.freedesktop.gstreamer.GStreamer

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    System.loadLibrary("gstreamer_android")
    GStreamer.init(this)
    super.onCreate(savedInstanceState)
    enableEdgeToEdge()
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
      window.attributes = window.attributes.apply {
        layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
      }
    }
  }
}
