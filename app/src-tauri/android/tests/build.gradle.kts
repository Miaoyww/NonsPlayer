plugins {
    id("com.android.library") version "9.3.1"
}
val prepareArtworkLoader = tasks.register<Copy>("prepareArtworkLoader") {
    from("../kotlin/com/miaoyww/nons/ArtworkFileLoader.kt")
    into(layout.buildDirectory.dir("generated/artwork"))
}
android {
    namespace = "com.miaoyww.nons.media.check"
    compileSdk = 37
    defaultConfig {
        minSdk = 24
        testInstrumentationRunner = "com.miaoyww.nons.MediaSessionCheck"
    }
}
extensions.configure<com.android.build.api.dsl.LibraryExtension> {
    sourceSets.getByName("main").kotlin.directories.add(file("build/generated/artwork").path)
}
tasks.named("preBuild") { dependsOn(prepareArtworkLoader) }
dependencies {
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.media3:media3-session:1.11.1")
}
