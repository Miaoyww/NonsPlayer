import base64
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("update_manifest", Path(__file__).resolve().parents[1] / "update-manifest.py")
updater = importlib.util.module_from_spec(spec)
spec.loader.exec_module(updater)


class UpdaterManifestTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.assets = Path(self.temporary.name)
        self.installer = self.assets / "Nons_1.4.0_x64-setup.exe"
        self.installer.write_bytes(b"installer")
        self.signature = self.installer.with_name(self.installer.name + ".sig")
        self.signature.write_text(base64.b64encode(b"untrusted comment: fixture\nsignature").decode())

    def test_mirror_and_github_use_identical_signed_artifact(self):
        mirror = updater.manifest("v1.4.0", self.assets, "更新说明", "https://download.miaoyww.top/nons/releases/v1.4.0", "2026-10-16T00:00:00Z")
        github = updater.manifest("v1.4.0", self.assets, "更新说明", "https://github.com/Miaoyww/Nons/releases/download/v1.4.0", mirror["pub_date"])
        self.assertEqual(mirror["version"], "1.4.0")
        self.assertEqual(mirror["notes"], "更新说明")
        self.assertEqual(mirror["pub_date"], github["pub_date"])
        m = mirror["platforms"]["windows-x86_64"]
        g = github["platforms"]["windows-x86_64"]
        self.assertEqual(m["signature"], g["signature"])
        self.assertTrue(m["url"].startswith("https://download.miaoyww.top/nons/releases/v1.4.0/"))
        self.assertTrue(g["url"].startswith("https://github.com/Miaoyww/Nons/releases/download/v1.4.0/"))

    def test_missing_signatures_stale_versions_and_insecure_urls_fail(self):
        for tag, url in [("v1.4.1", "https://example.com"), ("v1.4.0", "http://example.com"), ("../v1.4.0", "https://example.com")]:
            with self.assertRaises(ValueError):
                updater.manifest(tag, self.assets, "", url)
        self.signature.unlink()
        with self.assertRaises(FileNotFoundError):
            updater.manifest("v1.4.0", self.assets, "", "https://example.com")

    def test_corrupt_signature_fails(self):
        self.signature.write_text("invalid signature")
        with self.assertRaises(ValueError):
            updater.manifest("v1.4.0", self.assets, "", "https://example.com")
