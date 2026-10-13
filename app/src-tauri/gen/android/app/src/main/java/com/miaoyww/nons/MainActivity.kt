package com.miaoyww.nons

import android.os.Bundle
import org.freedesktop.gstreamer.GStreamer

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    System.loadLibrary("gstreamer_android")
    GStreamer.init(this)
    super.onCreate(savedInstanceState)
  }
}
