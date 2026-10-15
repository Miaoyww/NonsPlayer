import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import urllib.error
import unittest
from unittest.mock import MagicMock, patch

spec = importlib.util.spec_from_file_location("bootstrap_android", Path(__file__).resolve().parents[1] / "bootstrap-android.py")
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class AndroidBootstrapTests(unittest.TestCase):
    def run_gstreamer_bootstrap(self, responses):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        base = Path(temporary.name)
        sdk = base / "sdk"
        manager = sdk / "cmdline-tools/latest/bin/sdkmanager.bat"
        manager.parent.mkdir(parents=True)
        manager.touch()
        response = MagicMock()
        response.__enter__.return_value.read.return_value = ("a" * 64 + "  sdk.tar.xz\n").encode()
        sequence = []
        for item in responses:
            if isinstance(item, bytes):
                custom = MagicMock()
                custom.__enter__.return_value.read.return_value = item
                sequence.append(custom)
            else:
                sequence.append(response if item == "ok" else item)
        with patch.object(bootstrap, "BASE", base), patch.object(bootstrap, "SDK", sdk), \
                patch("sys.argv", ["bootstrap-android.py", "--download-only"]), \
                patch.object(bootstrap.urllib.request, "urlopen", side_effect=sequence) as request, \
                patch.object(bootstrap.time, "sleep") as sleep, \
                patch.object(bootstrap, "download") as download, \
                patch.object(bootstrap.tarfile, "open") as archive:
            archive.return_value.__enter__.return_value.getmembers.return_value = []
            try:
                bootstrap.main()
            finally:
                self.requests = request.call_count
                self.waits = sleep.call_count
                self.downloads = download.call_args_list

    def test_checksum_503_is_retried_before_downloading_sdk(self):
        error = urllib.error.HTTPError("https://example.test/checksum", 503, "Unavailable", {}, None)
        self.run_gstreamer_bootstrap([error, "ok"])
        self.assertEqual(self.requests, 2)
        self.assertEqual(self.waits, 1)
        self.assertEqual(self.downloads[0].args[2], "a" * 64)

    def test_checksum_network_timeout_is_retried(self):
        self.run_gstreamer_bootstrap([TimeoutError("timeout"), "ok"])
        self.assertEqual(self.requests, 2)

    def test_checksum_connection_failure_is_retried(self):
        self.run_gstreamer_bootstrap([urllib.error.URLError("connection closed"), "ok"])
        self.assertEqual(self.requests, 2)

    def test_checksum_retries_are_bounded_and_do_not_download_unverified_sdk(self):
        with self.assertRaises(urllib.error.HTTPError):
            self.run_gstreamer_bootstrap([
                urllib.error.HTTPError("https://example.test/checksum", 503, "Unavailable", {}, None)
                for _ in range(4)
            ])
        self.assertEqual(self.requests, 4)
        self.assertEqual(self.waits, 3)
        self.assertEqual(self.downloads, [])

    def test_checksum_permanent_http_error_is_not_retried(self):
        with self.assertRaises(urllib.error.HTTPError):
            self.run_gstreamer_bootstrap([
                urllib.error.HTTPError("https://example.test/checksum", 404, "Not found", {}, None)
            ])
        self.assertEqual(self.requests, 1)
        self.assertEqual(self.waits, 0)
        self.assertEqual(self.downloads, [])

    def test_malformed_checksum_does_not_download_sdk(self):
        for content in [b"", b"upstream error", b"a" * 63, b"a" * 64 + b" " * 4096]:
            with self.subTest(content_length=len(content)):
                with self.assertRaises(ValueError):
                    self.run_gstreamer_bootstrap([content])
                self.assertEqual(self.requests, 1)
                self.assertEqual(self.downloads, [])

    @patch.dict(os.environ, {"JAVA_HOME": "test-jdk"})
    @patch.object(bootstrap.subprocess, "run")
    def test_ci_supplies_license_answers_before_installing_packages(self, run):
        bootstrap.install_sdk(Path("sdkmanager.bat"), accept_licenses=True)
        licenses, packages = run.call_args_list
        self.assertIn("--licenses", licenses.args[0])
        self.assertEqual(set(licenses.kwargs["input"].splitlines()), {"y"})
        self.assertTrue(licenses.kwargs["text"])
        self.assertTrue(licenses.kwargs["check"])
        self.assertIn(f"ndk;{bootstrap.NDK}", packages.args[0])
        self.assertNotIn("input", packages.kwargs)

    @patch.dict(os.environ, {"JAVA_HOME": "test-jdk"})
    @patch.object(bootstrap.subprocess, "run")
    def test_local_bootstrap_keeps_interactive_license_acceptance(self, run):
        bootstrap.install_sdk(Path("sdkmanager.bat"))
        self.assertNotIn("input", run.call_args_list[0].kwargs)

    @patch.dict(os.environ, {"JAVA_HOME": "test-jdk"})
    @patch.object(bootstrap.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "sdkmanager"))
    def test_failed_license_step_does_not_install_packages(self, run):
        with self.assertRaises(subprocess.CalledProcessError):
            bootstrap.install_sdk(Path("sdkmanager.bat"), accept_licenses=True)
        self.assertEqual(run.call_count, 1)


if __name__ == "__main__":
    unittest.main()
