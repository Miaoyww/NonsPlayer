"""Verify offline frontend assets and ARM64 native runtime in a trial APK."""

import argparse
from pathlib import Path
import re
import struct
import zipfile


FRONTEND_ASSET = re.compile(rb"assets/[A-Za-z0-9_.-]+\.(?:css|js|mjs)")


def verify_frontend_asset_keys(binary):
    if b"index.html" not in binary:
        raise RuntimeError("APK does not contain the embedded frontend entrypoint")
    if not FRONTEND_ASSET.search(binary):
        raise RuntimeError("APK does not contain the embedded frontend asset keys")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("apk", type=Path)
    args = parser.parse_args()
    with zipfile.ZipFile(args.apk) as package:
        corrupt = package.testzip()
        if corrupt:
            raise RuntimeError(f"Corrupt APK entry: {corrupt}")
        names = set(package.namelist())
        if "lib/arm64-v8a/libnons_lib.so" not in names:
            raise RuntimeError("Missing native library: libnons_lib.so")
        for name in sorted(names):
            if not name.startswith("lib/") or not name.endswith(".so"):
                continue
            if name not in names:
                raise RuntimeError(f"Missing native library: {name}")
            binary = package.read(name)
            header = binary[:20]
            if header[:6] != b"\x7fELF\x02\x01" or struct.unpack_from("<H", header, 18)[0] != 183:
                raise RuntimeError(f"Not an ARM64 ELF library: {name}")
            offset = struct.unpack_from("<Q", binary, 32)[0]
            size, count = struct.unpack_from("<HH", binary, 54)
            for index in range(count):
                program = offset + index * size
                if struct.unpack_from("<I", binary, program)[0] == 1:
                    alignment = struct.unpack_from("<Q", binary, program + 48)[0]
                    if alignment < 16384:
                        raise RuntimeError(f"Native library is not 16 KB aligned: {name}")
        # Tauri embeds the bundled frontend in its Rust library, rather than
        # placing index.html in the APK assets directory.
        frontend = package.read("lib/arm64-v8a/libnons_lib.so")
        verify_frontend_asset_keys(frontend)
        unexpected = [name for name in names if name.startswith("lib/") and not name.startswith("lib/arm64-v8a/")]
        if unexpected:
            raise RuntimeError("Unexpected native architecture")
    print(f"Verified ARM64 runtime and offline frontend: {args.apk}")


if __name__ == "__main__":
    main()
