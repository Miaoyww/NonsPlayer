"""Validate release metadata and collect Nons Windows and Android release assets."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def validate_version(root, tag):
    if not re.fullmatch(r"v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", tag):
        raise ValueError("Release tag must be v<major>.<minor>.<patch>, for example v1.0.0")
    version = tag[1:]
    native = root / "app/src-tauri"
    cargo = tomllib.loads((native / "Cargo.toml").read_text(encoding="utf-8"))
    lock = tomllib.loads((native / "Cargo.lock").read_text(encoding="utf-8"))
    locked = next(p["version"] for p in lock["package"] if p["name"] == cargo["package"]["name"])
    versions = {
        "app/package.json": read_json(root / "app/package.json")["version"],
        "tauri.conf.json": read_json(native / "tauri.conf.json")["version"],
        "Cargo.toml": cargo["package"]["version"],
        "Cargo.lock": locked,
    }
    for name, actual in versions.items():
        if actual != version:
            raise ValueError(f"{name} has version {actual}; tag requires {version}")
    return version


def release_notes(root, tag):
    version = validate_version(root, tag)
    changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8-sig")
    sections = re.split(r"^## ", changelog, flags=re.MULTILINE)
    for section in sections[1:]:
        heading, _, body = section.partition("\n")
        if re.fullmatch(rf"\[{re.escape(version)}\](?: - \d{{4}}-\d{{2}}-\d{{2}})?\s*", heading):
            if body.strip():
                return body.strip() + "\n"
            break
    raise ValueError(f"No release notes found in CHANGELOG.md for [{version}]")


def portable_name(root, version):
    product = read_json(root / "app/src-tauri/tauri.conf.json")["productName"]
    return f"{product}_{version}_windows_x64_portable"


def build_portable(root, tag, output):
    version = validate_version(root, tag)
    native = root / "app/src-tauri"
    config = read_json(native / "tauri.conf.json")
    windows = read_json(native / "tauri.windows.conf.json")
    name = portable_name(root, version)
    executable = native / "target/release" / f"{config['mainBinaryName']}.exe"
    files = {executable.name: executable}
    # Use the same resource mappings as the Windows installers, never the entire target directory.
    resources = {**config["bundle"]["resources"], **windows["bundle"]["resources"]}
    for source, destination in resources.items():
        directory = (native / source).resolve()
        if not directory.is_dir():
            raise ValueError(f"Missing portable resource directory: {directory}")
        for path in sorted(directory.rglob("*")):
            if not path.is_file() or path.name == ".gitkeep":
                continue
            relative = (Path(destination) / path.relative_to(directory)).as_posix()
            if Path(relative).is_absolute() or ".." in Path(relative).parts or relative in files:
                raise ValueError(f"Invalid or duplicate portable resource path: {relative}")
            files[relative] = path
    if "runtime/manifest.json" not in files:
        raise ValueError("Missing portable resource: runtime/manifest.json")
    manifest = read_json(files["runtime/manifest.json"])
    required = ["runtime/gst-plugin-scanner.exe", "bundled-plugins/netease-island.zip"]
    required += [f"runtime/gstreamer-1.0/gst{plugin}.dll" for plugin in manifest["plugins"]]
    for relative in [*manifest["files"], *required]:
        if relative not in files:
            raise ValueError(f"Missing portable resource: {relative}")
    if not executable.is_file():
        raise ValueError(f"Missing portable executable: {executable}")
    output.mkdir(parents=True, exist_ok=True)
    target = output / f"{name}.zip"
    readme = (
        f"Nons {version} Windows x64 免安装版\n\n"
        f"请完整解压整个目录，再运行 {executable.name}；不要在压缩包内直接启动或单独移动 EXE。\n"
        "需要 Microsoft Edge WebView2 Runtime；缺失时请先从微软官方网站安装：\n"
        "https://developer.microsoft.com/microsoft-edge/webview2/\n\n"
        "包含私有音频运行时和随包插件，无需另行安装 GStreamer。\n"
        "设置、账号、曲库和缓存仍保存到 Windows 用户数据目录，与安装版共用；不会写入解压目录。\n"
        "音频运行时随附文件沿用安装包的资源映射，上游提供的许可文件会一并打包。\n"
    )
    with zipfile.ZipFile(target, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        for relative, source in sorted(files.items()):
            archive.write(source, f"{name}/{relative}")
        archive.writestr(f"{name}/README.txt", readme)
    print(f"Portable: {target}")
    return target


def collect_files(root, tag, output):
    version = validate_version(root, tag)
    native = root / "app/src-tauri"
    config = read_json(native / "tauri.conf.json")
    product = config["productName"]
    bundle = native / "target/release/bundle"
    sources = []
    for directory, suffix in [("msi", ".msi"), ("nsis", ".exe")]:
        matches = sorted((bundle / directory).glob(f"{product}_{version}_*{suffix}"))
        if len(matches) != 1:
            raise ValueError(f"Expected one {directory} installer for {version}, found {len(matches)}")
        sources.append((matches[0], matches[0].name))
    portable = bundle / "portable" / f"{portable_name(root, version)}.zip"
    if not portable.is_file():
        raise ValueError(f"Missing portable archive: {portable}")
    sources.append((portable, portable.name))
    plugin = native / "bundled-plugins/netease-island.zip"
    with zipfile.ZipFile(plugin) as archive:
        manifest = json.loads(archive.read("manifest.json"))
    source_manifest = read_json(root / "plugins/netease-island/manifest.json")
    if manifest != source_manifest:
        raise ValueError("Bundled plugin manifest is stale; rebuild the plugin")
    if manifest["engines"]["app"] != f"^{version.split('.')[0]}.0.0":
        raise ValueError("Bundled plugin must declare compatibility with the release major version")
    sources.append((plugin, f"netease-island-{manifest['version']}.zip"))
    write_assets(output, sources)


def collect_android_files(root, tag, output):
    version = validate_version(root, tag)
    apk = root / ".local/android" / f"Nons-{version}-android-arm64-debug.apk"
    checksum = apk.with_suffix(".apk.sha256")
    if not apk.is_file() or not checksum.is_file():
        raise ValueError(f"Missing verified Android APK or checksum: {apk}")
    with apk.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if checksum.read_text(encoding="ascii").strip() != f"{digest}  {apk.name}":
        raise ValueError("Android APK checksum does not match the verified build")
    write_assets(output, [(apk, apk.name)], "SHA256SUMS-android.txt")


def write_assets(output, sources, checksum_name="SHA256SUMS.txt"):
    if output.exists() and any(output.iterdir()):
        raise ValueError(f"Release output must be empty: {output}")
    output.mkdir(parents=True, exist_ok=True)
    checksums = []
    for source, name in sources:
        target = output / name
        shutil.copy2(source, target)
        with target.open("rb") as stream:
            digest = hashlib.file_digest(stream, "sha256").hexdigest()
        checksums.append(f"{digest}  {name}\n")
        print(f"Collect: {name}")
    (output / checksum_name).write_text("".join(checksums), encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=["notes", "portable", "files", "android-files"])
    parser.add_argument("tag")
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    if args.command == "notes":
        notes = release_notes(ROOT, args.tag)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(notes, encoding="utf-8")
        print(f"Release notes: {args.output}")
    elif args.command == "portable":
        build_portable(ROOT, args.tag, args.output)
    elif args.command == "android-files":
        collect_android_files(ROOT, args.tag, args.output)
    else:
        collect_files(ROOT, args.tag, args.output)


if __name__ == "__main__":
    main()
