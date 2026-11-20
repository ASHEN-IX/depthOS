#!/usr/bin/env python3
"""Measure the production controller and standalone QML shelves with synthetic observations."""
import copy
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile
from threading import Thread
import time
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target/debug/depthos"
LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
sys.path.insert(0, str(ROOT / "tests/support"))
from private_session import guard, launch

ARTIFACTS = ROOT / "artifacts/phase-4-3-1"


def memory(pid):
    status = Path(f"/proc/{pid}/status").read_text().splitlines()
    rollup = Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines()
    return {line.split(":")[0]: int(line.split()[1]) for line in status + rollup
            if line.startswith(("VmHWM:", "Rss:", "Pss:"))}


def cpu(pid):
    stats = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
    return (int(stats[11]) + int(stats[12])) / os.sysconf("SC_CLK_TCK")


def fixture(folder):
    guard(folder).close()
    import dbus
    import dbus.service
    from PyQt6 import QtQml
    from PyQt6.QtCore import QTimer
    from PyQt6.QtGui import QGuiApplication

    bus = dbus.SessionBus()
    assert set(bus.list_names()) - {"org.freedesktop.DBus", bus.get_unique_name()} == set()
    db_path = folder / "workspace.sqlite"

    def cli(*args):
        out = subprocess.check_output([EXE, "--db", db_path, *args], text=True)
        return json.loads(out) if out else None

    native = str(uuid.uuid4())
    world = cli("world", "create", "Measurement")["id"]
    workspace = cli("workspace", "create", "--world", world, "--name", "Projection")["id"]
    desktop = cli("desktop", "create", "--workspace", workspace, "--name", "Shelf")["id"]
    cli("association", "set", "native-desktop", native, "--workspace", workspace, "--desktop", desktop)
    controller = subprocess.Popen([EXE, "--db", db_path, "controller", "run"],
                                  stdout=subprocess.DEVNULL)
    try:
        for _ in range(200):
            if all(bus.name_has_owner(name) for name in ("org.depthos.Controller", "org.depthos.StateInspector")):
                break
            time.sleep(0.02)
        else:
            raise RuntimeError("Controller startup")
        publisher = dbus.service.BusName("org.kde.KWin", bus, do_not_queue=True)
        observer = dbus.Interface(bus.get_object("org.depthos.StateInspector", "/org/depthos/StateInspector"),
                                  "org.depthos.StateInspector1")
        endpoint = dbus.Interface(bus.get_object("org.depthos.Controller", "/org/depthos/Controller"),
                                  "org.depthos.Controller1")
        session = "test-session"
        hello = json.loads(str(observer.Hello(session)))
        sequence = 0
        metrics = {"snapshotMs": {}, "focusMs": [], "focusSendToUiMs": [], "burstRounds": []}
        frame = json.loads((ROOT / "tests/observer-adapter/trace.json").read_text())["frames"][0]
        template = frame["state"]["windows"][0]
        windows = [dict(template, runtimeId=f"window-{i}", nativeId=str(uuid.uuid4()),
                        applicationId=f"org.depthos.fixture{i}", desktopIds=[native], onAllDesktops=False)
                   for i in range(101)]
        frame["state"]["desktops"] = [{"id": native, "nativeId": native, "name": "Native"}]
        frame["state"]["activeDesktopId"] = native
        frame["state"]["activeDesktopByScreen"] = None
        frame["state"]["activeWindowId"] = None

        def push(events, gap=False):
            nonlocal sequence
            frames = []
            if gap:
                sequence += 1
            for event in events:
                sequence += 1
                frames.append(dict(event, schemaVersion=1, sessionId=session, sequence=sequence,
                                   revision=sequence))
            message = dict(schemaVersion=1, sessionId=session, consumerId=hello["consumerId"],
                           frames=frames, position=dict(sequence=sequence, revision=sequence))
            receipt_start = time.monotonic()
            reply = json.loads(str(observer.Push(json.dumps(message), timeout=4)))
            assert bool(reply["needSnapshot"]) == gap, reply
            return receipt_start, time.monotonic()

        def snapshot(count):
            state = dict(frame["state"], windows=windows[:count],
                         applicationWindowIds=[w["runtimeId"] for w in windows[:count]])
            return push([dict(frame, state=state)])

        loader = importlib.machinery.SourceFileLoader("phase_4_3_burst_launcher", str(LAUNCHER))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        launcher = importlib.util.module_from_spec(spec)
        loader.exec_module(launcher)
        failures = []
        timer_intervals = []
        queue_high_water = 0

        def steps(window):
            nonlocal queue_high_water
            backend, scene = window.property("backend"), window.property("scene")
            yield lambda: backend.property("state") == "Connected"
            scene.setProperty("currentLevel", 3)
            engine = QtQml.qmlEngine(scene)
            reader = engine.controller_reader

            def rows():
                nonlocal queue_high_water
                queue_high_water = max(queue_high_water, int(reader.busy) + int(bool(reader.pending)))
                data = backend.property("windows")
                return data.toVariant() if hasattr(data, "toVariant") else data

            def elapsed(start):
                return round((time.monotonic() - start) * 1000, 2)

            for count in (1, 10, 50, 100):
                started, received = snapshot(count)
                payload = json.loads(str(endpoint.GetDesktopShelves(dbus.UInt32(0), "")))
                assert payload["data"]["lastValidatedSequence"] == str(sequence)
                projection_read = elapsed(received)
                yield lambda: len(rows()) == count
                model = rows()
                assert len({row["id"] for row in model}) == count
                assert all(row["desktopId"] == desktop and row["titleRedacted"] for row in model)
                metrics["snapshotMs"][count] = {"publisherRoundTrip": round((received-started)*1000, 2),
                                                "firstPageAfterAck": projection_read,
                                                "uiAfterAck": elapsed(received)}

            first = next(row for row in rows() if row["appId"] == "org.depthos.fixture0")
            card_name = "windowCard-" + first["id"]
            yield lambda: scene.findNamed(card_name)
            card = scene.findNamed(card_name)
            for i in range(32):
                focus = windows[i % 2]
                sent, received = push([{"type": "window.focus.changed", "activeWindowId": focus["runtimeId"]}])
                yield lambda: any(row["appId"] == focus["applicationId"] and row["focused"] for row in rows())
                metrics["focusMs"].append(elapsed(received))
                metrics["focusSendToUiMs"].append(elapsed(sent))
                assert scene.findNamed(card_name) == card

            old_ids = {row["id"] for row in rows()}
            event = {"type": "window.focus.changed", "activeWindowId": None}
            push([event], gap=True)
            yield lambda: backend.property("shelfFreshness") == "stale" and not rows()
            snapshot(100)
            yield lambda: backend.property("shelfFreshness") == "live" and len(rows()) == 100
            assert old_ids.isdisjoint(row["id"] for row in rows())
            metrics["gapRecoveryReplacedIds"] = len(old_ids)

            def run_burst():
                try:
                    window = windows[-1]
                    add = dict(type="window.added", windowId=window["runtimeId"], window=window)
                    remove = dict(type="window.removed", windowId=window["runtimeId"])
                    for _ in range(25):
                        push([add, remove] * 10)
                except Exception:
                    failures.append(traceback.format_exc())

            for _ in range(3):
                start = time.monotonic()
                worker = Thread(target=run_burst)
                worker.start()
                yield lambda: not worker.is_alive()
                worker.join()
                assert not failures, failures
                yield lambda: len(rows()) == 100
                settled_at = time.monotonic() + 0.35
                yield lambda: time.monotonic() >= settled_at
                assert all(row["appId"] != "org.depthos.fixture100" for row in rows())
                metrics["burstRounds"].append({"events": 500, "settledMs": elapsed(start),
                                                "uiKiB": memory(os.getpid()),
                                                "controllerKiB": memory(controller.pid)})

            start_cpu = (cpu(os.getpid()), cpu(controller.pid))
            start_idle = time.monotonic()
            for _ in range(5):
                push([])
                next_tick = time.monotonic() + 1
                yield lambda: time.monotonic() >= next_tick
            idle_time = time.monotonic() - start_idle
            metrics["idleCpuPercentOneCore"] = {
                "ui": round(100 * (cpu(os.getpid()) - start_cpu[0]) / idle_time, 3),
                "controller": round(100 * (cpu(controller.pid) - start_cpu[1]) / idle_time, 3),
            }
            metrics["observedClientOutstandingPlusPendingMax"] = queue_high_water
            metrics["focusP95Ms"] = round(statistics.quantiles(metrics["focusMs"], n=20, method="inclusive")[18], 2)
            metrics["focusSendToUiP95Ms"] = round(statistics.quantiles(
                metrics["focusSendToUiMs"], n=20, method="inclusive")[18], 2)
            metrics["maxTimerGapMs"] = round(max(timer_intervals) * 1000, 2)
            (folder / "synthetic-burst.json").write_text(json.dumps(metrics, indent=2) + "\n")
            print("Synthetic focus p95:", metrics["focusP95Ms"], "ms", flush=True)
            assert metrics["focusSendToUiP95Ms"] < 250

        def drive(sequence, predicate=None, deadline=0):
            try:
                if predicate is not None and not predicate():
                    assert time.monotonic() < deadline, "Synthetic shelf update timed out"
                    start = time.monotonic()

                    def resume():
                        timer_intervals.append(time.monotonic() - start)
                        drive(sequence, predicate, deadline)

                    QTimer.singleShot(6, resume)
                    return
                check = next(sequence)
                drive(sequence, check, time.monotonic() + 7)
            except StopIteration:
                QGuiApplication.exit(0)
            except Exception:
                failures.append(traceback.format_exc())
                QGuiApplication.exit(1)

        class InstrumentedEngine(QtQml.QQmlApplicationEngine):
            def load(self, url):
                super().load(url)
                window = self.rootObjects()[0]
                QTimer.singleShot(0, lambda: drive(steps(window)))

        QtQml.QQmlApplicationEngine = InstrumentedEngine
        sys.argv = [str(LAUNCHER), "--reduced-motion", "--window-size", "1366x768"]
        try:
            launcher.main()
        finally:
            for failure in failures:
                print(failure, file=sys.stderr)
            assert publisher.get_name() == "org.kde.KWin"
    finally:
        controller.terminate()
        controller.wait(timeout=5)


def main():
    os.umask(0o077)
    if len(sys.argv) == 2:
        fixture(Path(sys.argv[1]))
        return
    ARTIFACTS.mkdir(parents=True, exist_ok=True, mode=0o700)
    folder = Path(tempfile.mkdtemp(dir=ARTIFACTS, prefix="b-"))
    print(folder, flush=True)
    raise SystemExit(launch(folder, [sys.executable, __file__, str(folder)], wayland=False))


if __name__ == "__main__":
    main()
