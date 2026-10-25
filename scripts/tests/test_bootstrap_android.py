import importlib.util
import os
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("bootstrap_android", Path(__file__).resolve().parents[1] / "bootstrap-android.py")
bootstrap = importlib.util.module_from_spec(spec)
spec.loader.exec_module(bootstrap)


class AndroidBootstrapTests(unittest.TestCase):
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
