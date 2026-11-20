#!/usr/bin/env python3
"""Fail-closed checks against invented paths, peers, and disposable processes."""
from concurrent.futures import ThreadPoolExecutor
import json
import os
import shutil
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "artifacts/phase-4-3-1"
sys.path.insert(0, str(ROOT / "tests/support"))
from host_isolation_monitor import HostMonitor
from private_session import (SOCKET, IsolationError, PrivateKWin, guard, launch, mark,
                             observer_package, start_ticks)


def worker(root, mode):
    bus = guard(root)
    if mode == "escaped-child":
        child = subprocess.Popen([sys.executable, "-c",
                                  "import os,signal,time;"
                                  "signal.signal(signal.SIGTERM,signal.SIG_IGN);"
                                  "open('ready','w').close();time.sleep(60)"],
                                 stdin=subprocess.DEVNULL, start_new_session=True)
        deadline = time.monotonic() + 2
        while not (root / "ready").is_file() and time.monotonic() < deadline:
            time.sleep(0.01)
        assert (root / "ready").is_file()
        (root / "escaped.json").write_text(json.dumps([child.pid, start_ticks(child.pid)]))
        mark(root, "escaped-child:ready", fixture_pid=child.pid)
        raise SystemExit(42)
    if mode == "kwin-unavailable":
        exited = subprocess.Popen(["/usr/bin/false"])
        exited.wait()
        try:
            PrivateKWin(root, bus, exited)
        except (IsolationError, FileNotFoundError):
            return
        raise AssertionError("An exited compositor was accepted")

    kwin = subprocess.Popen(["/usr/bin/kwin_wayland", "--virtual", "--no-lockscreen",
                             "--no-global-shortcuts", "--socket", SOCKET, "--width", "700",
                             "--height", "500"])
    try:
        deadline = time.monotonic() + 7
        while not (bus.name_has_owner("org.kde.KWin") and (root / "runtime" / SOCKET).is_socket()):
            assert kwin.poll() is None and time.monotonic() < deadline
            time.sleep(0.025)
        identity = PrivateKWin(root, bus, kwin)
        mark(root, "concurrent:ready", **identity.report())
        time.sleep(1.5)
        identity.verify()
    finally:
        kwin.terminate()
        kwin.wait(timeout=5)


class IsolationTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="di-")
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name) / "p"
        self.root.mkdir(mode=0o700)
        self.outside = Path(self.tmp.name) / "outside"
        self.outside.mkdir(mode=0o700)
        self.addCleanup(self.retain_evidence)

    def retain_evidence(self):
        destination = ARTIFACTS / self.evidence.name / self._testMethodName
        destination.mkdir(mode=0o700)
        for directory in (self.root, self.root.with_name("q")):
            if directory.exists():
                for source in directory.glob("*.json*"):
                    shutil.copyfile(source, destination / f"{directory.name}-{source.name}")
        self.monitor.checkpoint({"stage": "fault:after", "case": self._testMethodName})

    def run(self, result=None):
        self.monitor.checkpoint({"stage": "fault:before", "case": self._testMethodName})
        return super().run(result)

    def command(self, mode):
        return [sys.executable, __file__, "--worker", str(self.root), mode]

    def refusal(self, prepare):
        # The worker would create a process marker if it reached setup.
        status = launch(self.root, self.command("escaped-child"), wayland=True, prepare=prepare)
        self.assertNotEqual(status, 0)
        self.assertFalse((self.root / "session.json").exists())
        self.assertFalse((self.root / "escaped.json").exists())

    def test_injected_dbus_address_is_rejected_before_startup(self):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
            endpoint = self.outside / "synthetic"
            peer.bind(str(endpoint))
            peer.listen()
            self.refusal(lambda _, env: env.update(DBUS_SESSION_BUS_ADDRESS=f"unix:path={endpoint}"))

    def test_injected_wayland_display_is_rejected_before_startup(self):
        self.refusal(lambda _, env: env.update(WAYLAND_DISPLAY=str(self.outside / "wayland-0")))

    def test_missing_config_directory(self):
        self.refusal(lambda root, _: (root / "config").rmdir())

    def test_runtime_ownership_mode_is_required(self):
        self.refusal(lambda root, _: (root / "runtime").chmod(0o755))

    def test_private_symlink_escape(self):
        def prepare(root, _):
            (root / "data").rmdir()
            (root / "data").symlink_to(self.outside)
        self.refusal(prepare)

    def test_stale_socket(self):
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as stale:
            self.refusal(lambda root, _: stale.bind(str(root / "runtime" / SOCKET)))

    def test_missing_private_bus(self):
        # Use the same structure as the internal worker, minus the bus.
        from private_session import DIRECTORIES, environment
        for suffix in DIRECTORIES.values():
            (self.root / suffix).mkdir(mode=0o700)
        with patch.dict(os.environ, environment(self.root, True), clear=True):
            with self.assertRaisesRegex(IsolationError, "Private D-Bus address required"):
                guard(self.root)

    def test_installer_needs_explicit_private_path(self):
        with patch("private_session.subprocess.run") as call:
            for root in (None, self.outside):
                with self.assertRaises(IsolationError):
                    observer_package(self.root, "install", ROOT / "experiments/kwin-observer", root)
            call.assert_not_called()

    def test_missing_compositor_fails_before_script_api(self):
        self.assertEqual(launch(self.root, self.command("kwin-unavailable"), wayland=True), 0)

    def test_orphaned_session_leader_is_reaped(self):
        self.assertEqual(launch(self.root, self.command("escaped-child"), wayland=False), 42)
        pid, ticks = json.loads((self.root / "escaped.json").read_text())
        try:
            self.assertNotEqual(start_ticks(pid), ticks, "Owned child survived supervisor cleanup")
        except FileNotFoundError:
            pass
        self.assertFalse((self.root / "runtime/bus").exists())
        final = json.loads((self.root / "stages.jsonl").read_text().splitlines()[-1])
        self.assertEqual((final["stage"], final["owned_pids"]), ("cleanup:after", []))

    def test_monitor_detects_replacement_even_when_content_is_restored(self):
        target = self.outside / "kwinrc"
        target.write_text("original")
        with HostMonitor(self.root / "synthetic-host.jsonl", [self.outside]) as monitor:
            replacement = self.outside / "temporary"
            replacement.write_text("alternate")
            replacement.replace(target)
            target.write_text("original")
        self.assertTrue(any("moved-to" in item["events"] for item in monitor.events))
        self.assertTrue(all(item["writer"] == "unknown" for item in monitor.events))
        with self.assertRaises(IsolationError):
            monitor.require_clean()

    def test_concurrent_kwin_instances_have_independent_owners(self):
        second = self.root.with_name("q")
        second.mkdir(mode=0o700)
        roots = (self.root, second)
        with ThreadPoolExecutor(max_workers=2) as pool:
            jobs = [pool.submit(launch, root, [sys.executable, __file__, "--worker", str(root),
                                               "concurrent"], True) for root in roots]
            self.assertEqual([job.result() for job in jobs], [0, 0])
        audits = [[json.loads(line) for line in (root / "stages.jsonl").read_text().splitlines()]
                  for root in roots]
        ready = [next(row for row in run if row["stage"] == "concurrent:ready") for run in audits]
        bus_ids = [next(row["bus_id"] for row in run if row["stage"] == "session:during") for run in audits]
        self.assertNotEqual(bus_ids[0], bus_ids[1])
        self.assertNotEqual(ready[0]["kwin_pid"], ready[1]["kwin_pid"])
        self.assertNotEqual(ready[0]["wayland_inode"], ready[1]["wayland_inode"])
        self.assertLess(max(row["time_ns"] for row in ready),
                        min(run[-1]["time_ns"] for run in audits))
        print("Concurrent KWin PIDs:", ready[0]["kwin_pid"], ready[1]["kwin_pid"], flush=True)


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--worker":
        worker(Path(sys.argv[2]), sys.argv[3])
    else:
        os.umask(0o077)
        ARTIFACTS.mkdir(parents=True, exist_ok=True, mode=0o700)
        evidence = Path(tempfile.mkdtemp(dir=ARTIFACTS, prefix="a-"))
        print(evidence, flush=True)
        with HostMonitor(evidence / "host.jsonl") as monitor:
            IsolationTests.monitor = monitor
            IsolationTests.evidence = evidence
            results = unittest.main(exit=False)
        monitor.require_clean()
        raise SystemExit(0 if results.result.wasSuccessful() else 1)
