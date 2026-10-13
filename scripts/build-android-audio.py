"""Build the official GStreamer Android glue with the selected audio plugins."""

import hashlib
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / ".local" / "android"
SOURCE = ROOT / "app" / "src-tauri" / "android" / "audio"


def main():
    ndk = Path(os.environ["NDK_HOME"])
    gst = BASE / "gstreamer"
    if not (gst / "arm64" / "lib" / "libgstreamer-1.0.a").is_file():
        raise RuntimeError("Run python scripts/bootstrap-android.py first")
    output = BASE / "audio"
    output.mkdir(parents=True, exist_ok=True)
    signature = hashlib.sha256(
        (SOURCE / "Android.mk").read_bytes()
        + (SOURCE / "Application.mk").read_bytes()
        + (ndk / "source.properties").read_bytes()
        + (gst / "arm64" / "lib" / "pkgconfig" / "gstreamer-1.0.pc").read_bytes()
        + Path(__file__).read_bytes()
    ).hexdigest()
    marker = output / "signature"
    if marker.exists() and marker.read_text() == signature and (output / "libs" / "arm64-v8a" / "libgstreamer_android.so").is_file():
        return
    java = BASE / "java"
    assets = BASE / "assets"
    java.mkdir(parents=True, exist_ok=True)
    assets.mkdir(parents=True, exist_ok=True)
    env = dict(os.environ)
    command = [
        str(ndk / "ndk-build.cmd"),
        "-j4",
        f"NDK_PROJECT_PATH={output.as_posix()}",
        f"APP_BUILD_SCRIPT={(SOURCE / 'Android.mk').as_posix()}",
        f"NDK_APPLICATION_MK={(SOURCE / 'Application.mk').as_posix()}",
        f"GSTREAMER_ROOT_ANDROID={gst.as_posix()}",
        f"NONS_ANDROID_JAVA={java.as_posix()}",
        f"NONS_ANDROID_ASSETS={assets.as_posix()}",
    ]
    subprocess.run(command, cwd=output, env=env, check=True)
    # The SDK's make rules can install a stale intermediate during incremental
    # parallel builds. Publish the library that was actually linked this run.
    library = output / "libs" / "arm64-v8a" / "libgstreamer_android.so"
    shutil.copyfile(output / "gst-android-build" / "arm64-v8a" / "libgstreamer_android.so", library)
    subprocess.run([str(ndk / "toolchains" / "llvm" / "prebuilt" / "windows-x86_64" / "bin" / "llvm-strip.exe"), "--strip-unneeded", str(library)], check=True)
    marker.write_text(signature)


if __name__ == "__main__":
    main()
