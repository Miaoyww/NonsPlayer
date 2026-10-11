"""Extract verified upstream wheels into .local; never execute an installer."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import urllib.request
import zipfile

VERSION = "1.28.7"
ROOT = Path(__file__).resolve().parents[1] / ".local"
DEST = ROOT / "gstreamer"
PACKAGES = (
    "gstreamer_ext_runtime", "gstreamer_libs", "gstreamer_plugins",
    "gstreamer_plugins_libs", "gstreamer_plugins_restricted", "gstreamer_plugins_gpl",
    "gstreamer_plugins_gpl_restricted", "gstreamer_cli",
)


def bootstrap():
    if os.name != "nt":
        raise SystemExit("This bootstrap is for Windows x64; use your platform's GStreamer SDK.")
    ROOT.mkdir(exist_ok=True)
    DEST.mkdir(exist_ok=True)
    for package in PACKAGES:
        with urllib.request.urlopen(f"https://pypi.org/pypi/{package}/{VERSION}/json", timeout=30) as response:
            metadata = json.load(response)
        wheel = next(u for u in metadata["urls"] if "cp39-abi3-win_amd64" in u["filename"])
        archive = ROOT / wheel["filename"]
        if not archive.exists():
            print(f"Downloading {package} ({wheel['size'] // 1024 // 1024} MiB)", flush=True)
            urllib.request.urlretrieve(wheel["url"], archive)
        with archive.open("rb") as source:
            if hashlib.file_digest(source, "sha256").hexdigest() != wheel["digests"]["sha256"]:
                raise RuntimeError(f"Checksum mismatch: {archive.name}; remove this archive and retry.")
        with zipfile.ZipFile(archive) as source:
            marker = f".data/purelib/{package}/"
            for entry in source.infolist():
                if marker not in entry.filename or entry.is_dir():
                    continue
                relative = entry.filename.split(marker, 1)[1]
                target = (DEST / relative).resolve()
                if not target.is_relative_to(DEST.resolve()):
                    raise RuntimeError("Unsafe archive path")
                target.parent.mkdir(parents=True, exist_ok=True)
                with source.open(entry) as incoming, target.open("wb") as outgoing:
                    import shutil
                    shutil.copyfileobj(incoming, outgoing)
            for entry in source.infolist():
                if ".dist-info/licenses/" in entry.filename and not entry.is_dir():
                    relative = entry.filename.split(".dist-info/licenses/", 1)[1]
                    target = (DEST / "licenses" / package / relative).resolve()
                    if not target.is_relative_to(DEST.resolve()):
                        raise RuntimeError("Unsafe license path")
                    target.parent.mkdir(parents=True, exist_ok=True)
                    target.write_bytes(source.read(entry))
    generate_import_libraries()
    (DEST / "VERSION").write_text(VERSION, encoding="utf-8")
    size = sum(p.stat().st_size for p in DEST.rglob("*") if p.is_file())
    print(f"Development runtime ready: {DEST} ({size / 1024 / 1024:.1f} MiB). This is not the release bundle.", flush=True)


def generate_import_libraries():
    # Rust sys crates have checked-in bindings. MSVC import libraries can be
    # generated with Microsoft's tools from the upstream runtime's DLL exports.
    vswhere = Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"
    installation = subprocess.check_output([str(vswhere), "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], text=True).strip()
    tools = sorted((Path(installation) / "VC/Tools/MSVC").glob("*/bin/Hostx64/x64"))[-1]
    libdir = DEST / "lib"
    libdir.mkdir(exist_ok=True)
    for name in ("glib-2.0-0", "gobject-2.0-0", "gio-2.0-0", "gstreamer-1.0-0", "gstbase-1.0-0", "gstcontroller-1.0-0"):
        dll = DEST / "bin" / f"{name}.dll"
        exports = subprocess.check_output([str(tools / "dumpbin.exe"), "/exports", str(dll)], text=True)
        names = re.findall(r"^\s+\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(\S+)", exports, re.MULTILINE)
        if not names:
            raise RuntimeError(f"No DLL exports found: {dll}")
        definition = libdir / f"{name}.def"
        definition.write_text(f"LIBRARY {name}.dll\nEXPORTS\n" + "\n".join(names) + "\n", encoding="utf-8")
        subprocess.run([str(tools / "lib.exe"), "/nologo", "/machine:x64", f"/def:{definition}", f"/out:{libdir / (name + '.lib')}"], check=True, stdout=subprocess.DEVNULL)


if __name__ == "__main__":
    bootstrap()
