import importlib.util
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
import zipfile


spec = importlib.util.spec_from_file_location(
    "verify_android_apk", Path(__file__).resolve().parents[1] / "verify-android-apk.py"
)
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class AndroidApkVerificationTests(unittest.TestCase):
    def apk_check(self, assets):
        binary = bytearray(120)
        binary[:6] = b"\x7fELF\x02\x01"
        struct.pack_into("<H", binary, 18, 183)
        struct.pack_into("<Q", binary, 32, 64)
        struct.pack_into("<HH", binary, 54, 56, 1)
        struct.pack_into("<I", binary, 64, 1)
        struct.pack_into("<Q", binary, 112, 16384)
        binary.extend(assets)
        with tempfile.TemporaryDirectory() as directory:
            apk = Path(directory) / "test.apk"
            with zipfile.ZipFile(apk, "w") as package:
                package.writestr("lib/arm64-v8a/libnons_lib.so", binary)
            return subprocess.run(
                [sys.executable, str(Path(verify.__file__)), str(apk)],
                capture_output=True, text=True,
            )

    def test_apk_cli_accepts_current_main_entry_and_rejects_missing_assets(self):
        result = self.apk_check(b"index.html assets/main-ABC123.js")
        self.assertEqual(result.returncode, 0, result.stderr)
        result = self.apk_check(b"index.html")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("embedded frontend asset keys", result.stderr)

    def test_frontend_accepts_hashed_entry_chunk_with_a_non_index_name(self):
        frontend = b"asset index.html assets/main-ABC123.js assets/main-ABC123.css"

        verify.verify_frontend_asset_keys(frontend)

    def test_frontend_requires_index_and_a_javascript_or_css_asset(self):
        with self.assertRaisesRegex(RuntimeError, "embedded frontend entrypoint"):
            verify.verify_frontend_asset_keys(b"assets/main-ABC123.js")
        with self.assertRaisesRegex(RuntimeError, "embedded frontend asset"):
            verify.verify_frontend_asset_keys(b"index.html assets/icon.png")


if __name__ == "__main__":
    unittest.main()
