"""Install a project-local Android command-line SDK and ARM64 GStreamer SDK.

The JDK is supplied by JAVA_HOME. No Android Studio or machine-wide settings
are installed. SDK license acceptance is handled interactively by sdkmanager.
"""

import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import tarfile
import time
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
BASE = ROOT / ".local" / "android"
SDK = BASE / "sdk"
TOOLS = "commandlinetools-win-15859902_latest.zip"
TOOLS_SHA = "90ae805d20434428bffcb699c290860f19bb5f66a67e6b330067e3de801fb04a"
GST_VERSION = "1.28.7"
GST_ARCHIVE = f"gstreamer-1.0-android-universal-{GST_VERSION}.tar.xz"
GST_URL = f"https://gstreamer.freedesktop.org/data/pkg/android/{GST_VERSION}/{GST_ARCHIVE}"
NDK = "27.2.12479018"


def download(url, destination, digest):
    if destination.is_file():
        with destination.open("rb") as source:
            if hashlib.file_digest(source, "sha256").hexdigest() == digest:
                return
    partial = destination.with_suffix(destination.suffix + ".partial")
    print(f"Downloading {destination.name}", flush=True)
    for attempt in range(128):
        offset = partial.stat().st_size if partial.exists() else 0
        request = urllib.request.Request(url, headers={"Range": f"bytes={offset}-{offset + 32 * 1024 * 1024 - 1}"})
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                append = response.status == 206
                count = offset if append else 0
                expected = count + int(response.headers["Content-Length"])
                total = int(response.headers.get("Content-Range", f"/{expected}").split("/")[-1])
                next_progress = count + 64 * 1024 * 1024
                with partial.open("ab" if append else "wb") as output:
                    while chunk := response.read(1024 * 1024):
                        output.write(chunk)
                        count += len(chunk)
                        if count >= next_progress:
                            print(f"  {count // 1024 // 1024} MiB", flush=True)
                            next_progress += 64 * 1024 * 1024
            print(f"  {count // 1024 // 1024}/{total // 1024 // 1024} MiB", flush=True)
            if count == total:
                break
        except (OSError, urllib.error.URLError) as error:
            print(f"Retrying download: {error}", flush=True)
        time.sleep(2)
    else:
        raise RuntimeError(f"Incomplete download: {destination.name}")
    with partial.open("rb") as source:
        if hashlib.file_digest(source, "sha256").hexdigest() != digest:
            raise RuntimeError(f"SHA256 mismatch: {destination.name}")
    partial.replace(destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--download-only", action="store_true")
    options = parser.parse_args()
    BASE.mkdir(parents=True, exist_ok=True)
    manager = SDK / "cmdline-tools" / "latest" / "bin" / "sdkmanager.bat"
    if not manager.is_file():
        archive = BASE / TOOLS
        download(f"https://dl.google.com/android/repository/{TOOLS}", archive, TOOLS_SHA)
        with zipfile.ZipFile(archive) as package:
            package.extractall(BASE / "tools-staging")
        manager.parent.parent.parent.mkdir(parents=True, exist_ok=True)
        (BASE / "tools-staging" / "cmdline-tools").rename(manager.parent.parent)
    gst = BASE / "gstreamer"
    if not (gst / "arm64" / "lib" / "libgstreamer-1.0.a").is_file():
        digest = urllib.request.urlopen(GST_URL + ".sha256sum", timeout=60).read().decode().split()[0]
        archive = BASE / GST_ARCHIVE
        download(GST_URL, archive, digest)
        gst.mkdir(parents=True, exist_ok=True)
        print("Extracting ARM64 GStreamer SDK", flush=True)
        with tarfile.open(archive) as package:
            members = [member for member in package.getmembers() if member.name.lstrip("./").startswith("arm64/")]
            package.extractall(gst, members=members, filter="data")
    if not options.download_only:
        if not os.environ.get("JAVA_HOME"):
            raise RuntimeError("Set JAVA_HOME to a JDK before running sdkmanager")
        subprocess.run([str(manager), f"--sdk_root={SDK}", "--licenses"], check=True)
        subprocess.run([str(manager), f"--sdk_root={SDK}", "platform-tools", "platforms;android-37.0", "build-tools;37.0.0", f"ndk;{NDK}"], check=True)
    print(f"ANDROID_HOME={SDK}\nNDK_HOME={SDK / 'ndk' / NDK}", flush=True)


if __name__ == "__main__":
    main()
