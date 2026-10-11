"""Verify private DLL loading, plugin discovery and audio decoding without SDK paths."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import wave

ROOT = Path(__file__).resolve().parents[1]

def smoke(payload, skip_audio_output=False):
    payload = payload.resolve()
    manifest = json.loads((payload / "runtime/manifest.json").read_text(encoding="utf-8"))
    for relative in manifest["files"]:
        if not (payload / relative).is_file(): raise RuntimeError(f"Missing bundled file: {relative}")
    env = {key: value for key, value in os.environ.items() if not key.upper().startswith("GST_")}
    system_root = os.environ["SystemRoot"]
    env["PATH"] = os.pathsep.join([str(payload), str(Path(system_root) / "System32"), system_root])
    env["GST_PLUGIN_SYSTEM_PATH_1_0"] = ""
    env["GST_PLUGIN_PATH_1_0"] = str(payload / "runtime/gstreamer-1.0")
    env["GST_PLUGIN_SCANNER_1_0"] = str(payload / "runtime/gst-plugin-scanner.exe")
    with tempfile.TemporaryDirectory(prefix="nons-audio-smoke-") as directory:
        temporary = Path(directory).resolve()
        env["GST_REGISTRY_1_0"] = str(temporary / "registry.bin")
        for name in ["gst-inspect-1.0.exe", "gst-launch-1.0.exe"]:
            shutil.copy2(ROOT / ".local/gstreamer/bin" / name, temporary / name)
        def run(name, args):
            result = subprocess.run([str(temporary / name), *args], env=env, cwd=temporary, capture_output=True, text=True, timeout=30)
            if result.returncode: raise RuntimeError(result.stdout + result.stderr)
            return result.stdout
        factories = ["playbin3", "autoaudiosink", "wasapi2sink", "audioconvert", "audioresample", "audioamplify", "audiobuffersplit", "flacdec", "mpg123audiodec", "opusdec", "vorbisdec", "wavparse", "wavpackdec", "avdec_aac", "avdec_ape", "qtdemux"]
        for factory in factories: run("gst-inspect-1.0.exe", [factory])
        sample = temporary / "silence.wav"
        with wave.open(str(sample), "wb") as audio:
            audio.setnchannels(2); audio.setsampwidth(2); audio.setframerate(44100); audio.writeframes(bytes(44100 * 4))
        run("gst-launch-1.0.exe", ["-q", "filesrc", f"location={sample.as_posix()}", "!", "decodebin", "!", "audioconvert", "!", "audioresample", "!", "fakesink"])
        # Generate compressed fixtures with the development encoder set, then decode
        # them using only the staged playback runtime. Encoders are not shipped.
        development = dict(env)
        sdk = ROOT / ".local/gstreamer"
        development["PATH"] = os.pathsep.join([str(sdk / "bin"), env["PATH"]])
        development["GST_PLUGIN_PATH_1_0"] = str(sdk / "lib/gstreamer-1.0")
        development["GST_PLUGIN_SCANNER_1_0"] = str(sdk / "libexec/gstreamer-1.0/gst-plugin-scanner.exe")
        development["GST_REGISTRY_1_0"] = str(temporary / "development-registry.bin")
        encoders = {"flac": ["flacenc"], "mp3": ["lamemp3enc"], "m4a": ["avenc_aac", "!", "aacparse", "!", "mp4mux"], "ogg": ["vorbisenc", "!", "oggmux"], "opus": ["opusenc", "!", "oggmux"]}
        for extension, encoder in encoders.items():
            compressed = temporary / f"silence.{extension}"
            command = [str(temporary / "gst-launch-1.0.exe"), "-q", "filesrc", f"location={sample.as_posix()}", "!", "wavparse", "!", "audioconvert", "!", "audioresample", "!", *encoder, "!", "filesink", f"location={compressed.as_posix()}"]
            encoded = subprocess.run(command, env=development, cwd=temporary, capture_output=True, text=True, timeout=30)
            if encoded.returncode: raise RuntimeError(f"{extension} fixture encoding failed: " + encoded.stdout + encoded.stderr)
            run("gst-launch-1.0.exe", ["-q", "filesrc", f"location={compressed.as_posix()}", "!", "decodebin", "!", "audioconvert", "!", "audioresample", "!", "fakesink"])
        # Exercise the real output device too, using silent PCM.
        if not skip_audio_output:
            run("gst-launch-1.0.exe", ["-q", "filesrc", f"location={sample.as_posix()}", "!", "decodebin", "!", "audioconvert", "!", "audioresample", "!", "autoaudiosink"])
    output_status = "audio output skipped" if skip_audio_output else "silent audio output"
    print(f"PASS: {len(manifest['files'])} files, {len(factories)} factories, WAV/FLAC/MP3/AAC/Vorbis/Opus decoding, {output_status}, SDK-free PATH")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(); parser.add_argument("payload", type=Path)
    parser.add_argument("--skip-audio-output", action="store_true", help="Skip physical audio output on CI runners without an audio device")
    args = parser.parse_args()
    smoke(args.payload, args.skip_audio_output)
