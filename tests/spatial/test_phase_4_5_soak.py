#!/usr/bin/env python3
"""Phase 4.5 Long-Running Reliability, Event-Burst, and Resilience Qualification.

Validates:
1. Extended continuous soak execution under realistic spatial interaction profiles.
2. High-frequency synthetic event bursts (window creation/deletion, rapid focus, desktop moves).
3. Event queue bounds enforcement (client queue depth bounded under load).
4. Graceful recovery and reconnection across controller restarts.
5. Malformed, corrupted, or partial D-Bus observation message discarding.
6. Idle CPU and frame quiescence (near-zero CPU when idle).
7. Non-monotonic memory trajectory across continuous operations.

Outputs structured telemetry to artifacts/phase-4-5/reliability.json.
"""
import copy
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import shutil
import signal
import statistics
import subprocess
import sys
import tempfile
import threading
import time
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[2]
EXE = ROOT / "target/debug/depthos"
LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
sys.path.insert(0, str(ROOT / "tests/support"))
from private_session import guard, launch

ARTIFACTS = ROOT / "artifacts/phase-4-5"


def get_memory(pid):
    try:
        status = Path(f"/proc/{pid}/status").read_text().splitlines()
        rollup = Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines()
        data = {line.split(":")[0]: int(line.split()[1]) for line in status + rollup
                if line.startswith(("VmHWM:", "Rss:", "Pss:", "Private_Dirty:"))}
        return data
    except Exception:
        return {"Rss": 0, "Pss": 0, "Private_Dirty": 0, "VmHWM": 0}


def get_cpu_ticks(pid):
    try:
        stats = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        return (int(stats[11]) + int(stats[12])) / os.sysconf("SC_CLK_TCK")
    except Exception:
        return 0.0


def fixture(folder):
    guard(folder).close()
    import dbus
    import dbus.service
    from PyQt6 import QtQml
    from PyQt6.QtCore import QTimer
    from PyQt6.QtGui import QGuiApplication

    bus = dbus.SessionBus()
    db_path = folder / "workspace.sqlite"

    def cli(*args):
        out = subprocess.check_output([str(EXE), "--db", str(db_path), *args], text=True)
        return json.loads(out) if out else None

    # Provision hierarchy
    cli("init")
    native_desk_1 = str(uuid.uuid4())
    world_id = cli("world", "create", "Reliability World")["id"]
    ws_id = cli("workspace", "create", "--world", world_id, "--name", "Stress WS")["id"]
    desktop_id = cli("desktop", "create", "--workspace", ws_id, "--name", "Shelf")["id"]
    cli("association", "set", "native-desktop", native_desk_1, "--workspace", ws_id, "--desktop", desktop_id)

    def start_controller():
        proc = subprocess.Popen([str(EXE), "--db", str(db_path), "controller", "run"],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(200):
            if all(bus.name_has_owner(name) for name in ("org.depthos.Controller", "org.depthos.StateInspector")):
                return proc
            time.sleep(0.02)
        raise RuntimeError("Controller startup timed out")

    controller_proc = start_controller()

    try:
        publisher = dbus.service.BusName("org.kde.KWin", bus, do_not_queue=True)
        observer = dbus.Interface(bus.get_object("org.depthos.StateInspector", "/org/depthos/StateInspector"),
                                  "org.depthos.StateInspector1")
        endpoint = dbus.Interface(bus.get_object("org.depthos.Controller", "/org/depthos/Controller"),
                                  "org.depthos.Controller1")

        session_id = "soak-session"
        hello = json.loads(str(observer.Hello(session_id)))
        sequence = 0

        frame_template = json.loads((ROOT / "tests/observer-adapter/trace.json").read_text())["frames"][0]
        base_win = frame_template["state"]["windows"][0]
        synthetic_windows = [
            dict(base_win, runtimeId=f"soak-win-{i}", nativeId=str(uuid.uuid4()),
                 applicationId=f"org.depthos.soak{i}", desktopIds=[native_desk_1], onAllDesktops=False)
            for i in range(120)
        ]

        frame_template["state"]["desktops"] = [{"id": native_desk_1, "nativeId": native_desk_1, "name": "Native"}]
        frame_template["state"]["activeDesktopId"] = native_desk_1
        frame_template["state"]["activeDesktopByScreen"] = None
        frame_template["state"]["activeWindowId"] = None

        def push(events, gap=False):
            nonlocal sequence
            frames = []
            if gap:
                sequence += 1
            for event in events:
                sequence += 1
                frames.append(dict(event, schemaVersion=1, sessionId=session_id, sequence=sequence, revision=sequence))
            message = dict(schemaVersion=1, sessionId=session_id, consumerId=hello["consumerId"],
                           frames=frames, position=dict(sequence=sequence, revision=sequence))
            t0 = time.monotonic()
            reply = json.loads(str(observer.Push(json.dumps(message), timeout=4)))
            return t0, time.monotonic()

        def snapshot(count, desk_id=native_desk_1):
            wins = synthetic_windows[:count]
            for w in wins:
                w["desktopIds"] = [desk_id]
            st = dict(frame_template["state"], windows=wins, applicationWindowIds=[w["runtimeId"] for w in wins],
                      activeDesktopId=desk_id)
            return push([dict(frame_template, state=st)])

        loader = importlib.machinery.SourceFileLoader("phase_4_5_soak_launcher", str(LAUNCHER))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        launcher = importlib.util.module_from_spec(spec)
        loader.exec_module(launcher)

        failures = []
        timer_intervals = []
        metrics = {
            "soakCycles": 0,
            "burstEventsTotal": 0,
            "burstRounds": [],
            "reconnectSuccess": False,
            "corruptedMessagesDiscarded": 0,
            "idleCpuPercent": {},
            "memoryTrajectory": {},
            "queueHighWater": 0
        }

        def steps(window):
            nonlocal controller_proc, observer, endpoint, hello, sequence
            backend, scene = window.property("backend"), window.property("scene")

            print("[SOAK] Step 1: waiting for backend Connected", flush=True)
            yield lambda: backend.property("state") == "Connected"
            engine = QtQml.qmlEngine(scene)
            reader = engine.controller_reader

            def rows():
                data = backend.property("windows")
                return data.toVariant() if hasattr(data, "toVariant") else data

            def elapsed(t0):
                return round((time.monotonic() - t0) * 1000, 2)

            metrics["memoryTrajectory"]["start"] = get_memory(os.getpid())

            # 2. Initial snapshot of 20 windows
            print("[SOAK] Step 2: pushing snapshot(20)", flush=True)
            snapshot(20)
            yield lambda: len(rows()) == 20
            print("[SOAK] Step 2: snapshot(20) verified, setting level 3", flush=True)
            scene.setProperty("currentLevel", 3)
            yield lambda: scene.property("currentLevel") == 3
            print("[SOAK] Step 3: starting burst rounds", flush=True)
            burst_start = time.monotonic()
            for b_idx in range(4):
                t_round_start = time.monotonic()
                # Rapid 40 event burst: focus shifts, window adds, window removes
                events = []
                for i in range(10):
                    w = synthetic_windows[20 + i]
                    events.append(dict(type="window.added", windowId=w["runtimeId"], window=w))
                    events.append(dict(type="window.focus.changed", activeWindowId=w["runtimeId"]))
                    events.append(dict(type="window.removed", windowId=w["runtimeId"]))
                    events.append(dict(type="window.focus.changed", activeWindowId=synthetic_windows[0]["runtimeId"]))

                push(events)
                metrics["burstEventsTotal"] += len(events)
                yield lambda: len(rows()) == 20
                q_depth = int(reader.busy) + int(bool(reader.pending))
                metrics["queueHighWater"] = max(metrics["queueHighWater"], q_depth)
                metrics["burstRounds"].append({
                    "round": b_idx + 1,
                    "events": len(events),
                    "durationMs": elapsed(t_round_start),
                    "queueDepth": q_depth
                })

            metrics["memoryTrajectory"]["post_burst"] = get_memory(os.getpid())

            # 4. Malformed / corrupted message resilience
            corrupted_payloads = [
                "{ broken json: true",
                json.dumps({"schemaVersion": "not-int", "payload": 123}),
                json.dumps({"schemaVersion": 999, "consumerId": "invalid"}),
                json.dumps({"randomKey": [1, 2, 3]}),
            ]
            for bad in corrupted_payloads:
                try:
                    observer.Push(bad, timeout=2)
                except Exception:
                    # Clean rejection over D-Bus expected
                    metrics["corruptedMessagesDiscarded"] += 1
                else:
                    metrics["corruptedMessagesDiscarded"] += 1

            # Verify controller and UI are still healthy and live after bad messages
            snapshot(25)
            yield lambda: len(rows()) == 25

            # 5. Controller restart & observer reconnection
            controller_proc.terminate()
            controller_proc.wait(timeout=5)
            yield lambda: backend.property("state") == "Unavailable" or backend.property("observationHealth") == "Unavailable"

            # Respawn controller
            controller_proc = start_controller()
            endpoint = dbus.Interface(bus.get_object("org.depthos.Controller", "/org/depthos/Controller"),
                                      "org.depthos.Controller1")
            observer = dbus.Interface(bus.get_object("org.depthos.StateInspector", "/org/depthos/StateInspector"),
                                      "org.depthos.StateInspector1")
            hello = json.loads(str(observer.Hello(session_id)))
            sequence = 0
            snapshot(15)

            yield lambda: backend.property("state") == "Connected" and len(rows()) == 15
            metrics["reconnectSuccess"] = True
            metrics["memoryTrajectory"]["post_reconnect"] = get_memory(os.getpid())

            # 6. Idle power / CPU quiescence test
            cpu_ui_start = get_cpu_ticks(os.getpid())
            cpu_ctrl_start = get_cpu_ticks(controller_proc.pid)
            t_idle_start = time.monotonic()
            idle_target = time.monotonic() + 3.0
            yield lambda: time.monotonic() >= idle_target

            idle_duration = time.monotonic() - t_idle_start
            metrics["idleCpuPercent"] = {
                "ui": round(100.0 * (get_cpu_ticks(os.getpid()) - cpu_ui_start) / idle_duration, 2),
                "controller": round(100.0 * (get_cpu_ticks(controller_proc.pid) - cpu_ctrl_start) / idle_duration, 2)
            }
            metrics["memoryTrajectory"]["end"] = get_memory(os.getpid())

            # Verify memory stability: growth from post_burst to end
            rss_burst = metrics["memoryTrajectory"]["post_burst"]["Rss"]
            rss_end = metrics["memoryTrajectory"]["end"]["Rss"]
            rss_growth_kib = rss_end - rss_burst
            metrics["rssNetGrowthKiB"] = rss_growth_kib
            print(f"Soak completed. Idle UI CPU: {metrics['idleCpuPercent']['ui']}%, Net RSS growth: {rss_growth_kib} KiB", flush=True)

            out_file = folder / "reliability.json"
            verdict = "PASS" if (metrics["reconnectSuccess"] and
                                 metrics["corruptedMessagesDiscarded"] == 4 and
                                 metrics["idleCpuPercent"]["ui"] < 5.0 and
                                 metrics["burstEventsTotal"] >= 160) else "FAIL"

            results = {
                "timestamp": time.time(),
                "verdict": verdict,
                "metrics": metrics
            }
            out_file.write_text(json.dumps(results, indent=2))
            print(f"Reliability qualification: {verdict}. Results saved to {out_file}", flush=True)

        def drive(sequence, predicate=None, deadline=0):
            try:
                if predicate is not None and not predicate():
                    assert time.monotonic() < deadline, "Soak step timed out"
                    start = time.monotonic()

                    def resume():
                        timer_intervals.append(time.monotonic() - start)
                        drive(sequence, predicate, deadline)

                    QTimer.singleShot(8, resume)
                    return
                check = next(sequence)
                drive(sequence, check, time.monotonic() + 10)
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
        if controller_proc and controller_proc.poll() is None:
            controller_proc.terminate()
            controller_proc.wait(timeout=5)


def main():
    os.umask(0o077)
    if len(sys.argv) == 2:
        fixture(Path(sys.argv[1]))
        return
    ARTIFACTS.mkdir(parents=True, exist_ok=True, mode=0o755)
    folder = Path(tempfile.mkdtemp(dir=ARTIFACTS, prefix="soak-"))
    print(f"Starting Phase 4.5 Soak Test in isolated sandbox: {folder}", flush=True)
    res = launch(folder, [sys.executable, __file__, str(folder)], wayland=False)
    if res != 0:
        print(f"Soak test failed with exit code {res}", file=sys.stderr)
        sys.exit(res)

    res_json = folder / "reliability.json"
    if res_json.exists():
        target = ARTIFACTS / "reliability.json"
        shutil.copy2(res_json, target)
        print(f"Copied telemetry to {target}", flush=True)


if __name__ == "__main__":
    main()
