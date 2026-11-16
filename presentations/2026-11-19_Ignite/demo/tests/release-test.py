#!/usr/bin/env python3
"""Unit checks for release guards; these do not qualify the live backend."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("release", Path(__file__).resolve().parents[1] / "ops/release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseGuards(unittest.TestCase):
    def test_scoped_container_set_rejects_unrelated_or_missing_services(self):
        names = [f"ignite-observability-service-{i}" for i in range(10)]
        with patch.object(release, "command", return_value="\n".join(names)):
            self.assertEqual(release.containers(), names)
        for invalid in [names[:-1], [*names[:-1], "another-project-postgres-1"]]:
            with patch.object(release, "command", return_value="\n".join(invalid)):
                with self.assertRaises(AssertionError):
                    release.containers()

    def test_previous_failure_evidence_is_preserved(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(release, "OUT", Path(temporary)):
            release.save("cycles.json", {"passed": False, "error": "unit-test failure"})
            release.preserve("cycles.json")
            release.save("cycles.json", {"passed": True})
            previous = list(Path(temporary).glob("cycles-*.json"))
            self.assertEqual(len(previous), 1)
            self.assertFalse(json.loads(previous[0].read_text())["passed"])
            self.assertTrue(json.loads((Path(temporary) / "cycles.json").read_text())["passed"])

    def test_stamp_refuses_short_or_failed_soak_before_collecting_artifacts(self):
        with tempfile.TemporaryDirectory() as temporary:
            demo = Path(temporary)
            native = demo / ".native"
            native.mkdir()
            out = native / "evidence"
            out.mkdir()
            gates = {
                "cycles": {"passed": True, "cycles": [{}] * 5},
                "soak": {"passed": True, "duration_seconds": 3599.99, "errors": []},
                "offline-browser": {"passed": True}, "offline-images": {"passed": True},
                "recordings": {"clips": [{}] * 4},
            }
            with patch.object(release, "DEMO", demo), patch.object(release, "OUT", out):
                for passed, duration in [(True, 3599.99), (False, 3601)]:
                    gates["soak"].update(passed=passed, duration_seconds=duration)
                    for name, value in gates.items():
                        release.save(f"{name}.json", value)
                    with self.assertRaises(AssertionError):
                        release.stamp()
                    self.assertFalse((native / "release/manifest.json").exists())


if __name__ == "__main__":
    unittest.main()
