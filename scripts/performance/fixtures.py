"""Deterministic 5-minute silent PCM and local lyrics; no network or copyrighted media."""
import argparse
from pathlib import Path
import wave


def create(root: Path):
    root.mkdir(parents=True, exist_ok=True)
    with wave.open(str(root / "fixture.wav"), "wb") as audio:
        audio.setparams((2, 2, 48000, 0, "NONE", "not compressed"))
        for _ in range(300):
            audio.writeframesraw(bytes(48000 * 4))
    lines = [f"[{i // 60:02}:{i % 60:02}.00]Baseline lyric line {i:03}" for i in range(300)]
    (root / "fixture.lrc").write_text("\n".join(lines), encoding="utf-8")


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    create(parser.parse_args().root)
