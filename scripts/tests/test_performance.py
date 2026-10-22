import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("nons_performance_report", Path(__file__).resolve().parents[1] / "performance" / "report.py")
report = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(report)


class PerformanceReportTests(unittest.TestCase):
    def test_nearest_rank_and_empty_samples(self):
        self.assertEqual(report.stats([9, 1, 3, 2])["p50"], 2)
        self.assertEqual(report.stats([9, 1, 3, 2])["p95"], 9)
        self.assertIsNone(report.stats([])["p50"])

    def test_process_tree_totals_cpu_and_queue_latencies(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            meta = {"recordedAt": "2026-10-22", "cpu": ["fixture"], "os": "fixture", "osVersion": "1", "ramBytes": 8 * 2**30, "sha256": "fixture", "commit": "fixture", "dirty": False, "logicalProcessors": 4}
            (root / "metadata.json").write_text(json.dumps(meta))
            run = root / "run-01"; run.mkdir()
            (run / "launch.json").write_text('{"launchEpochMs": 1000}')
            events = [{"event": "first_screen", "epochMs": 1500, "value": {}}, {"event": "play_ready", "value": {"count": 1, "firstOutputBufferMs": 40}}, {"event": "play_ready", "value": {"count": 1000, "frontendReadyMs": 80}}, {"event": "phase_start", "epochMs": 2000, "value": {"phase": "background"}}, {"event": "phase_end", "epochMs": 5000, "value": {"phase": "background"}}, {"event": "finished", "value": {"success": True}}]
            (run / "events.jsonl").write_text("\n".join(map(json.dumps, events)))
            samples = [{"epochMs": epoch, "processes": [{"pid": pid, "cpuSeconds": cpu, "privateBytes": 2**20, "workingSetBytes": 2**21} for pid in [1, 2]]} for epoch, cpu in [(2000, 1), (3000, 1.5), (4000, 2)]]
            (run / "samples.jsonl").write_text("\n".join(map(json.dumps, samples)))
            result = report.summarize(root)
            self.assertEqual(result["firstScreenMs"]["p50"], 500)
            self.assertEqual(result["warmLocalOutputBufferMs"]["p50"], 40)
            self.assertEqual(result["queueReadyMs"]["1000"]["p50"], 80)
            data = result["phases"]["background"]
            self.assertEqual(data["privateMiB"]["p50"], 2)
            self.assertEqual(data["workingSetMiB"]["p50"], 4)
            self.assertEqual(data["cpuOneCorePercent"]["p50"], 100)
            self.assertEqual(data["cpuMachinePercent"]["p50"], 25)

    def test_incomplete_runs_are_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "metadata.json").write_text('{}')
            run = root / "run-01"; run.mkdir()
            (run / "events.jsonl").write_text('')
            with self.assertRaisesRegex(ValueError, "Incomplete"):
                report.summarize(root)

    def test_comparison_checks_environment_and_computes_deltas(self):
        with tempfile.TemporaryDirectory() as folder:
            before, after = Path(folder) / "before", Path(folder) / "after"
            metadata = {key: "same" for key in ["osVersion", "cpu", "logicalProcessors", "ramBytes", "gpu", "soundDevices", "powerScheme", "gstreamerVersion", "sampleSeconds", "sampleIntervalMs", "audioRuns", "runs", "workload"]}
            environment = {"viewport": [1200, 800], "dpr": 1, "userAgent": "fixture"}
            for root, value in [(before, 100), (after, 120)]:
                run = root / "run-01"; run.mkdir(parents=True)
                (root / "metadata.json").write_text(json.dumps(metadata))
                (run / "events.jsonl").write_text(json.dumps({"event": "first_screen", "value": environment}))
                stat = {"n": 5, "p50": value, "p95": value, "max": value}
                (root / "summary.json").write_text(json.dumps({"firstScreenMs": stat, "warmLocalOutputBufferMs": stat, "queueReadyMs": {}, "phases": {}}))
            report.compare(after, before)
            self.assertIn("+20.00%", (after / "comparison.md").read_text(encoding="utf-8"))
            metadata["sampleSeconds"] = "changed"
            (after / "metadata.json").write_text(json.dumps(metadata))
            with self.assertRaisesRegex(ValueError, "different sampleSeconds"):
                report.compare(after, before)
            metadata["sampleSeconds"] = "same"
            (after / "metadata.json").write_text(json.dumps(metadata))
            environment["dpr"] = 2
            (after / "run-01" / "events.jsonl").write_text(json.dumps({"event": "first_screen", "value": environment}))
            with self.assertRaisesRegex(ValueError, "different dpr"):
                report.compare(after, before)
