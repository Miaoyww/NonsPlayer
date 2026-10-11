"""Stage the Windows audio runtime, using MSVC dumpbin for DLL dependency closure."""
import argparse
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PLUGINS = (
    "coreelements", "playback", "typefindfunctions", "audioconvert", "audioresample",
    "volume", "autodetect", "wasapi", "wasapi2", "audioparsers", "id3demux",
    "apetag", "isomp4", "matroska", "ogg", "flac", "mpg123", "opus", "vorbis",
    "wavparse", "wavpack", "aiff", "libav", "audiotestsrc",
)
CORE = ("glib-2.0-0.dll", "gobject-2.0-0.dll", "gio-2.0-0.dll", "gstreamer-1.0-0.dll", "gstbase-1.0-0.dll", "gstcontroller-1.0-0.dll")

def dumpbin_tool():
    vswhere = Path(os.environ.get("ProgramFiles(x86)", "C:/Program Files (x86)")) / "Microsoft Visual Studio/Installer/vswhere.exe"
    installation = subprocess.check_output([str(vswhere), "-latest", "-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"], text=True).strip()
    return sorted((Path(installation) / "VC/Tools/MSVC").glob("*/bin/Hostx64/x64/dumpbin.exe"))[-1]

def dependencies(path, tool):
    output = subprocess.check_output([str(tool), "/dependents", str(path)], text=True)
    return sorted(set(re.findall(r"^\s+([\w.+-]+\.dll)\s*$", output, re.MULTILINE | re.IGNORECASE)))

def dependency_closure(seeds, libraries, read_dependencies, system_exists):
    pending = list(seeds)
    found = {}
    while pending:
        path = pending.pop()
        name = path.name.lower()
        if name in found:
            continue
        if not path.is_file():
            raise RuntimeError(f"Missing runtime file: {path}")
        found[name] = path
        for dependency in read_dependencies(path):
            dep = dependency.lower()
            if dep in libraries:
                pending.append(libraries[dep])
            elif not dep.startswith(("api-ms-", "ext-ms-")) and not system_exists(dependency):
                raise RuntimeError(f"Missing dependency of {path.name}: {dependency}")
    return list(found.values())

def stage(destination, gst=ROOT / ".local/gstreamer"):
    if os.name != "nt":
        raise RuntimeError("Audio runtime staging supports Windows x64 only.")
    if not (gst / "VERSION").is_file():
        raise RuntimeError("Run python scripts/bootstrap-gstreamer.py before building.")
    tool = dumpbin_tool()
    libraries = {path.name.lower(): path for path in (gst / "bin").glob("*.dll")}
    seeds = [gst / "bin" / name for name in CORE]
    plugins = [gst / "lib/gstreamer-1.0" / f"gst{name}.dll" for name in PLUGINS]
    scanner = gst / "libexec/gstreamer-1.0/gst-plugin-scanner.exe"
    files = dependency_closure(seeds + plugins + [scanner], libraries, lambda path: dependencies(path, tool), lambda name: (Path(os.environ["SystemRoot"]) / "System32" / name).is_file())
    # The stage is a dedicated, generated directory; never remove a caller-supplied path.
    stage_root = (ROOT / ".local/gstreamer-bundle").resolve()
    if not stage_root.is_relative_to((ROOT / ".local").resolve()):
        raise RuntimeError("Unsafe runtime staging directory")
    if stage_root.exists():
        shutil.rmtree(stage_root)
    payload = stage_root / "root"
    payload.mkdir(parents=True)
    for path in files:
        relative = Path("runtime/gstreamer-1.0") / path.name if path in plugins else Path("runtime/gst-plugin-scanner.exe") if path == scanner else Path(path.name)
        target = payload / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(path, target)
    # Preserve upstream licenses from the verified wheels used by bootstrap.
    for wheel in (ROOT / ".local").glob("gstreamer*-win_amd64.whl"):
        with zipfile.ZipFile(wheel) as archive:
            for entry in archive.infolist():
                if ".dist-info/licenses/" not in entry.filename or entry.is_dir():
                    continue
                package, relative = entry.filename.split(".dist-info/licenses/", 1)
                target = payload / "runtime/licenses" / package / relative
                if not target.resolve().is_relative_to(payload.resolve()):
                    raise RuntimeError("Unsafe license path")
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.read(entry))
    manifest = {"version": (gst / "VERSION").read_text().strip(), "plugins": list(PLUGINS), "files": [str(path.relative_to(payload)).replace("\\", "/") for path in sorted(payload.rglob("*")) if path.is_file()]}
    (payload / "runtime/manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    destination = destination.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    shutil.copytree(payload, destination, dirs_exist_ok=True)
    size = sum(path.stat().st_size for path in payload.rglob("*") if path.is_file())
    print(f"Staged audio runtime: {len(files)} native files, {len(PLUGINS)} plugins, {size / 1024**2:.1f} MiB", flush=True)

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--destination", type=Path, required=True)
    stage(parser.parse_args().destination)
