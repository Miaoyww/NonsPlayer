LOCAL_PATH := $(call my-dir)
GSTREAMER_ROOT := $(GSTREAMER_ROOT_ANDROID)/arm64
GSTREAMER_NDK_BUILD_PATH := $(GSTREAMER_ROOT)/share/gst-android/ndk-build
include $(GSTREAMER_NDK_BUILD_PATH)/plugins.mk
GSTREAMER_PLUGINS := coreelements typefindfunctions playback audioconvert audioresample volume autodetect opensles audioparsers id3demux isomp4 ogg vorbis opus flac wavparse mpg123 libav audiofx
GSTREAMER_EXTRA_DEPS := gstreamer-controller-1.0 gio-2.0
GSTREAMER_INCLUDE_FONTS := no
GSTREAMER_INCLUDE_CA_CERTIFICATES := no
# The SDK's glue linker bypasses NDK TARGET_LDFLAGS; align this shared library explicitly.
override GSTREAMER_LD := -fuse-ld=lld$(if $(filter windows,$(HOST_OS)),.exe,) -Wl,-soname,libgstreamer_android.so -Wl,-z,max-page-size=16384
GSTREAMER_JAVA_SRC_DIR := $(NONS_ANDROID_JAVA)
GSTREAMER_ASSETS_DIR := $(NONS_ANDROID_ASSETS)
include $(GSTREAMER_NDK_BUILD_PATH)/gstreamer-1.0.mk
