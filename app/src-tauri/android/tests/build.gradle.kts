plugins {
    id("com.android.library") version "9.3.1"
}
android {
    namespace = "com.miaoyww.nons.media.check"
    compileSdk = 37
    defaultConfig {
        minSdk = 24
        testInstrumentationRunner = "com.miaoyww.nons.MediaSessionCheck"
    }
}
dependencies {
    androidTestImplementation("androidx.media3:media3-session:1.11.1")
}
