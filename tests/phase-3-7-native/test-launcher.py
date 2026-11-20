#!/usr/bin/env python3
"""Deterministic native transcript and private artifact boundary checks."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("native_fixture", ROOT / "tests/phase-3-native/run.py")
fixture = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(fixture)


class NativeTranscript(unittest.TestCase):
    def test_terminal_transitions(self):
        fixture.verify_native_lifecycle([
            "ready deny-only", "pending native-match prompt=absent lock=unknown",
            "retired focus", "pending native-match prompt=absent lock=unknown", "retired shutdown"])
        fixture.verify_native_lifecycle([
            "pending native-match prompt=synthetic-fixture lock=fixture-unlocked",
            "synthetic first=counted replay=denied count=1"])

    def test_rejects_dropped_or_duplicate_terminal_events(self):
        invalid = [
            ["pending native-match x", "pending native-match y", "retired focus"],
            ["pending native-match x"],
            ["retired shutdown"],
            ["pending native-match x", "retired focus", "retired focus"],
            ["synthetic first=counted replay=denied count=1"],
            ["pending native-match x", "synthetic first=counted replay=counted count=2"],
        ]
        for trace in invalid:
            with self.subTest(trace=trace), self.assertRaises(RuntimeError):
                fixture.verify_native_lifecycle(trace)

    def test_rejects_executable_outside_private_artifacts(self):
        with self.assertRaises(RuntimeError):
            fixture.experiment_file(Path("/usr/bin/false"), "0" * 64, "kwin_wayland")
        with tempfile.TemporaryDirectory(prefix="depthos-launcher-test-") as name:
            executable = Path(name) / "kwin_wayland"
            executable.write_bytes(b"fixture")
            with self.assertRaises(RuntimeError):
                fixture.experiment_file(executable, fixture.hash_file(executable), "kwin_wayland")


if __name__ == "__main__":
    unittest.main()
