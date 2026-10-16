"""Generate a Tauri updater feed from collected, signed Windows installers."""
import argparse
import base64
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import re
from urllib.parse import quote


def manifest(tag, assets, notes, base_url, pub_date=None):
    if not re.fullmatch(r"v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", tag):
        raise ValueError("Expected stable release tag v<major>.<minor>.<patch>")
    if not base_url.startswith("https://"):
        raise ValueError("Updater assets require HTTPS")
    installers = sorted(assets.glob(f"Nons_{tag[1:]}_*-setup.exe"))
    if len(installers) != 1:
        raise ValueError("Expected one NSIS installer for the release version")
    installer = installers[0]
    signature = installer.with_name(installer.name + ".sig").read_text(encoding="utf-8").strip()
    decoded = base64.b64decode(signature, validate=True)
    if not decoded.startswith(b"untrusted comment:"):
        raise ValueError("Invalid Tauri updater signature")
    return {
        "version": tag[1:],
        "notes": notes,
        "pub_date": pub_date or datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "platforms": {"windows-x86_64": {
            "url": f"{base_url.rstrip('/')}/{quote(installer.name)}",
            "signature": signature,
        }},
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    parser.add_argument("assets", type=Path)
    parser.add_argument("notes", type=Path)
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--checksum", action="store_true")
    args = parser.parse_args()
    output = args.output or args.assets / "latest.json"
    # Both feeds retain identical versions, notes, signatures and publication dates.
    existing = args.assets / "latest.json"
    pub_date = json.loads(existing.read_text(encoding="utf-8"))["pub_date"] if existing.exists() else None
    value = manifest(args.tag, args.assets, args.notes.read_text(encoding="utf-8"), args.base_url, pub_date)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    if args.checksum:
        checksum = args.assets / "SHA256SUMS.txt"
        lines = [line for line in checksum.read_text(encoding="utf-8").splitlines() if not line.endswith(f"  {output.name}")]
        lines.append(f"{hashlib.sha256(output.read_bytes()).hexdigest()}  {output.name}")
        checksum.write_text("\n".join(lines) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
