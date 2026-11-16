#!/usr/bin/env python3
"""Approved live-test receiver: production reducer/auth, title-free event evidence.

Does not load/unload any compositor code or operate on windows. SIGUSR1 asks
the observer for a new baseline through its existing acknowledgment protocol.
Output is restricted to an explicitly supplied private artifacts directory.
"""
import argparse
from collections import Counter, deque
import json
import os
from pathlib import Path
import signal
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/state-inspector"))
from main import Inspector, SERVICE, DBusGMainLoop, GLib, dbus
from state import Replica, redacted


class EvidenceReplica(Replica):
    def __init__(self):
        super().__init__()
        self.counts = Counter()
        self.events = deque(maxlen=512)
        self.sessions = set()
        self.snapshots = 0
        self.removal_consistent = True

    def reset(self):
        Replica.__init__(self)

    def apply(self, frame):
        previous = (self.session_id, self.sequence)
        super().apply(frame)
        if self.need_snapshot or previous == (self.session_id, self.sequence):
            return
        self.sessions.add(self.session_id)
        kind = frame["type"]
        self.counts[kind] += 1
        if kind == "snapshot":
            self.snapshots += 1
        if kind == "window.removed":
            self.removal_consistent &= not any(w["runtimeId"] == frame["windowId"] for w in self.state["windows"])
        entry = {key: frame[key] for key in ("type", "sequence", "revision", "windowId", "changedFields", "activeWindowId", "activeDesktopId", "reason") if key in frame}
        window = frame.get("window")
        if isinstance(window, dict):
            entry["window"] = {key: window[key] for key in ("runtimeId", "nativeId", "applicationId", "desktopIds", "classification", "activityIds", "onAllDesktops", "minimized", "fullscreen", "excludeFromCapture", "screenId", "geometry") if key in window}
        self.events.append(entry)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    directory = args.output.resolve()
    if not directory.is_relative_to(ROOT / "artifacts") or not directory.is_dir() or directory.is_symlink():
        parser.error("output must be an existing private repository artifacts directory")
    if directory.stat().st_mode & 0o077:
        parser.error("output directory must have mode 0700")
    DBusGMainLoop(set_as_default=True)
    bus, loop = dbus.SessionBus(), GLib.MainLoop()
    name = dbus.service.BusName(SERVICE, bus, do_not_queue=True, allow_replacement=False)
    receiver = Inspector(bus, name, argparse.Namespace(snapshot=False, include_titles=False), loop)
    evidence = EvidenceReplica()
    receiver.replica = evidence
    (directory / "receiver.pid").write_text(str(os.getpid()) + "\n")
    last_summary = None

    def save():
        nonlocal last_summary
        summary = {"sessions": sorted(evidence.sessions), "snapshots": evidence.snapshots,
            "eventCounts": dict(evidence.counts), "lastEvent": evidence.last_event,
            "sequence": evidence.sequence, "revision": evidence.revision,
            "needSnapshot": evidence.need_snapshot, "errors": receiver.errors,
            "removalConsistent": evidence.removal_consistent,
            "packetAgeSeconds": round(time.monotonic() - receiver.last_packet, 2) if receiver.last_packet else None,
            "windows": len(evidence.state["windows"]) if evidence.state else None,
            "applications": len(evidence.state["applicationWindowIds"]) if evidence.state else None}
        (directory / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
        (directory / "events.json").write_text(json.dumps(list(evidence.events), indent=2) + "\n")
        if evidence.state is not None:
            snapshot = {"schemaVersion": 1, "sessionId": evidence.session_id, "sequence": evidence.sequence,
                "revision": evidence.revision, "state": redacted(evidence.state)}
            if not (directory / "initial-snapshot.json").exists():
                (directory / "initial-snapshot.json").write_text(json.dumps(snapshot, indent=2) + "\n")
            (directory / "latest-snapshot.json").write_text(json.dumps(snapshot, indent=2) + "\n")
        marker = (evidence.sequence, receiver.errors)
        if marker != last_summary:
            print(json.dumps(summary), flush=True)
            last_summary = marker
        return True

    def resync():
        evidence.need_snapshot = True
        print("Fresh snapshot requested (existing acknowledgment protocol).", flush=True)
        return True

    for sig in (signal.SIGINT, signal.SIGTERM):
        GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, sig, lambda: (loop.quit(), False)[1])
    GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, signal.SIGUSR1, resync)
    GLib.timeout_add(500, save)
    print("Receiver ready; no title text is printed or saved.", flush=True)
    try:
        loop.run()
    finally:
        save()
        receiver.remove_from_connection()
        bus.release_name(SERVICE)


if __name__ == "__main__":
    main()
