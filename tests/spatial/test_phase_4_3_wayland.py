#!/usr/bin/env python3
"""Production launcher and real observations under a disposable KWin Wayland session."""
import argparse
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/support"))
from private_session import SOCKET, PrivateKWin, guard, launch, mark, observer_package
from host_isolation_monitor import HostMonitor

ARTIFACTS = ROOT / "artifacts/phase-4-3-1"
LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
OBSERVER = ROOT / "experiments/kwin-observer/contents/code/main.js"
FIXTURE_ID = "org.depthos.phase431"
FIXTURE = """
import sys
from PyQt6.QtWidgets import QApplication, QLabel
app = QApplication([])
app.setDesktopFileName(sys.argv[1])
window = QLabel("Disposable DepthOS fixture")
window.setWindowTitle("Disposable DepthOS fixture")
window.resize(360, 240)
window.show()
app.exec()
"""


def run_isolated(results, restart):
    bus = guard(results)
    import dbus
    from PyQt6 import QtQml
    from PyQt6.QtCore import QTimer
    from PyQt6.QtGui import QGuiApplication
    from PyQt6.QtQuick import QQuickWindow

    assert set(bus.list_names()) - {"org.freedesktop.DBus", bus.get_unique_name()} == set()
    install_root = results / "data/kwin/scripts"
    installed = None
    private = None
    children = []

    def process(name, *command):
        with (results / (name + ".log")).open("w") as log:
            child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        children.append(child)
        return child

    def wait(predicate):
        for _ in range(200):
            if predicate():
                return
            time.sleep(0.05)
        raise TimeoutError("Private KWin or controller startup")

    def cli(*args):
        return subprocess.check_output([str(ROOT / "target/debug/depthos"), "--db",
                                        str(results / "workspace.sqlite"), *args], text=True)

    def provision():
        cli("init")
        world = json.loads(cli("world", "create", "Phase 4.3 Research"))["id"]
        workspace = json.loads(cli("workspace", "create", "--world", world,
                                   "--name", "Private Wayland"))["id"]
        desks = [json.loads(cli("desktop", "create", "--workspace", workspace,
                                "--name", label))["id"] for label in ("Development", "Review")]
        return workspace, desks

    def checkpoint(stage, **details):
        identity = private.report() if private is not None and private.process.poll() is None else {}
        mark(results, stage, **identity, **details)

    def package(action):
        nonlocal installed
        checkpoint(f"package-{action}:before", package_root=str(install_root))
        path = observer_package(results, action, OBSERVER.parents[2], install_root)
        installed = path if action == "install" else None
        checkpoint(f"package-{action}:after", package_root=str(install_root))

    def load_script(path, name):
        assert path.resolve().is_relative_to(results) and path.is_file()
        checkpoint("script-load:before", script=name)
        number = int(private.call("/Scripting", "org.kde.kwin.Scripting",
                                  "loadScript", "ss", (str(path), name)))
        assert number >= 0
        private.call(f"/Scripting/Script{number}", "org.kde.kwin.Script", "run", "", ())
        checkpoint("script-load:after", script=name)
        return name

    def unload(name):
        checkpoint("script-unload:before", script=name)
        removed = private.call("/Scripting", "org.kde.kwin.Scripting", "unloadScript", "s", (name,))
        checkpoint("script-unload:after", script=name)
        return removed

    def move_fixture(target, pid, native_desktops, protection=None):
        # This script can only target the Qt fixture process spawned inside this
        # run. It is never installed and never connects to the host bus.
        private.verify()
        params = json.dumps({"pid": pid, "desktops": native_desktops, "protection": protection,
                             "application": FIXTURE_ID})
        target.write_text("""
const p = PARAMS;
const found = workspace.stackingOrder.filter(w => w.pid === p.pid &&
                                                    w.desktopFileName === p.application);
if (found.length !== 1) throw new Error("Disposable fixture missing");
if (p.protection === null)
    found[0].desktops = workspace.desktops.filter(d => p.desktops.includes(d.id));
else
    found[0].excludeFromCapture = p.protection;
""".replace("PARAMS", params))
        checkpoint("fixture-change:before", fixture_pid=pid, script=target.stem)
        unload(load_script(target, target.stem))
        checkpoint("fixture-change:after", fixture_pid=pid, script=target.stem)

    def compositor(label):
        nonlocal private
        private = None
        checkpoint(f"{label}:before")
        socket = results / "runtime" / SOCKET
        assert not socket.exists() and not socket.is_symlink()
        kwin = process(label, "/usr/bin/kwin_wayland", "--virtual", "--no-lockscreen",
                       "--no-global-shortcuts", "--width", "1600",
                       "--height", "1000", "--socket", SOCKET)
        wait(lambda: kwin.poll() is not None or (bus.name_has_owner("org.kde.KWin") and socket.is_socket()))
        assert kwin.poll() is None, "Private KWin failed to start"
        private = PrivateKWin(results, bus, kwin)
        checkpoint(f"{label}:after")
        return kwin

    try:
        ids = [str(uuid.uuid4()), str(uuid.uuid4())]
        (results / "config/kwinrc").write_text(
            f"[Desktops]\nNumber=2\nId_1={ids[0]}\nId_2={ids[1]}\n"
            "[Plugins]\nkwin4_effect_shapecornersEnabled=false\n")
        kwin = compositor("kwin")
        properties = dbus.Interface(private.get_object("/VirtualDesktopManager"),
                                    "org.freedesktop.DBus.Properties")
        native = [str(row[1]) for row in properties.Get("org.kde.KWin.VirtualDesktopManager", "desktops")]
        assert native == ids, "KWin did not read the seeded private desktop configuration"
        checkpoint("configuration:verified", desktop_count=len(native))
        workspace, desktops = provision()
        for source, target in zip(native, desktops):
            cli("association", "set", "native-desktop", source, "--workspace", workspace, "--desktop", target)
        def start_controller(label):
            child = process(label, str(ROOT / "target/debug/depthos"), "--db",
                            str(results / "workspace.sqlite"), "controller", "run")
            wait(lambda: bus.name_has_owner("org.depthos.Controller") and
                 bus.name_has_owner("org.depthos.StateInspector"))
            return child

        controller = start_controller("controller")
        package("install")
        observer = load_script(installed, "depthos-system-observer-session")
        endpoint = dbus.Interface(bus.get_object("org.depthos.Controller", "/org/depthos/Controller"),
                                  "org.depthos.Controller1")

        def shelf():
            return json.loads(str(endpoint.GetDesktopShelves(dbus.UInt32(0), "")))

        def observed():
            return next((row for row in shelf()["data"]["items"]
                         if row["applicationId"] == FIXTURE_ID), None)

        checkpoint("window-create:before")
        marker = process("window-fixture", sys.executable, "-c", FIXTURE, FIXTURE_ID)

        checkpoint("window-create:during", fixture_pid=marker.pid)
        wait(lambda: observed() and observed()["focused"])
        checkpoint("window-focus:after", fixture_pid=marker.pid)
        if restart:
            before = observed()
            assert before["desktopId"] == desktops[0]
            checkpoint("kwin-restart:before")
            kwin.terminate()
            kwin.wait(timeout=5)
            wait(lambda: not bus.name_has_owner("org.kde.KWin") and not shelf()["available"])
            assert shelf()["data"]["items"] == []
            marker.terminate()
            marker.wait(timeout=5)
            kwin = compositor("kwin-restarted")
            observer = load_script(installed, "depthos-system-observer-session")
            wait(lambda: shelf()["available"])
            assert not observed()
            process("window-after-compositor-restart", sys.executable, "-c", FIXTURE, FIXTURE_ID)
            wait(observed)
            after = observed()
            assert after["desktopId"] == before["desktopId"] and after["id"] != before["id"]
            checkpoint("kwin-restart:after")
            assert unload(observer)
            package("remove")
            print("PASS private KWin owner restart rebuilt shelves and retained durable rules", flush=True)
            return

        loader = importlib.machinery.SourceFileLoader("phase_4_3_launcher", str(LAUNCHER))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        launcher = importlib.util.module_from_spec(spec)
        loader.exec_module(launcher)
        errors = []
        evidence = []
        swaps = []

        def check(name, detail=None):
            checkpoint("case:observed", case=name, detail=detail)
            evidence.append({"check": name, "detail": detail})
            (results / "results.json").write_text(json.dumps(evidence, indent=2) + "\n")
            print("PASS", name, detail if detail is not None else "", flush=True)

        def steps(window):
            nonlocal controller
            backend = window.property("backend")
            scene = window.property("scene")
            window.frameSwapped.connect(lambda: swaps.append(time.monotonic()))

            def models():
                rows = backend.property("windows")
                return rows.toVariant() if hasattr(rows, "toVariant") else rows

            def fixture():
                return next((row for row in models() if row.get("appId") == FIXTURE_ID), None)

            def capture(name):
                assert window.grabWindow().save(str(results / "screenshots" / (name + ".png")))

            def card(identifier):
                return scene.findNamed("windowCard-" + identifier)

            yield lambda: backend.property("shelfFreshness") == "live" and fixture()
            row = fixture()
            opaque_id = row["id"]
            print("Mapped fixture reached the Qt model", flush=True)
            assert row["assignment"] == "observed" and row["desktopId"] == desktops[0]
            assert "title" not in row and row["titleRedacted"]
            scene.setProperty("currentLevel", 3)
            scene.syncIndices()
            yield lambda: card(opaque_id)
            capture("live-logical-desktop")
            check("authenticated real window appears on the mapped Desktop")
            backend.selectDesktop(desktops[1])
            started = time.monotonic()
            move_fixture(results / "move-owned-fixture.js", marker.pid, [native[1]])
            yield lambda: fixture() and fixture()["desktopId"] == desktops[1] and card(opaque_id)
            check("private KDE membership change preserves opaque identity", round((time.monotonic()-started)*1000, 2))
            capture("moved-logical-desktop")

            # A temporary all-desktops window may not acquire observed assignment.
            move_fixture(results / "sticky-owned-fixture.js", marker.pid, [])
            yield lambda: not fixture() and not card(opaque_id)
            check("all-native-desktops fixture appears on no Logical Desktop")

            move_fixture(results / "restore-owned-fixture.js", marker.pid, [native[1]])
            yield lambda: fixture() and card(opaque_id)
            move_fixture(results / "protect-owned-fixture.js", marker.pid, [], True)
            yield lambda: not fixture() and not card(opaque_id)
            check("protected real window is withheld from the UI")
            move_fixture(results / "unprotect-owned-fixture.js", marker.pid, [], False)
            yield lambda: fixture() and card(opaque_id)

            assert unload(observer)
            yield lambda: backend.property("shelfFreshness") != "live" and not models()
            capture("observer-stale")
            check("publisher loss clears real cards and preserves hierarchy")
            marker.terminate()
            marker.wait(timeout=5)
            package("remove")
            package("install")
            replacement_observer = load_script(installed, "depthos-system-observer-session")
            yield lambda: backend.property("shelfFreshness") == "live"
            assert not fixture()
            check("observer replacement excludes removed windows")
            replacement = process("window-replacement", sys.executable, "-c", FIXTURE, FIXTURE_ID)
            yield lambda: fixture()
            assert fixture()["id"] != opaque_id
            check("new native window is assigned a new opaque identity")

            # Transition to the Desktop hosting the replacement, then destroy it.
            backend.selectDesktop(fixture()["desktopId"])
            replacement_id = fixture()["id"]
            yield lambda: card(replacement_id)
            scene.setProperty("reducedMotion", False)
            scene.returnToParent()
            checkpoint("window-destroy:before", fixture_pid=replacement.pid)
            replacement.terminate()
            scene.enterDeeper()
            yield lambda: not fixture() and not card(replacement_id)
            check("destroyed native window drops its delegate")

            idle_started = time.monotonic() + 0.5
            yield lambda: time.monotonic() >= idle_started + 5
            check("settled idle swaps", sum(t >= idle_started for t in swaps))
            for child, name in [(controller, "controller"), (None, "ui")]:
                pid = child.pid if child else os.getpid()
                totals = Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines()
                totals += Path(f"/proc/{pid}/status").read_text().splitlines()
                check(name + " settled memory KiB", {line.split(":")[0]: int(line.split()[1])
                      for line in totals if line.startswith(("Rss:", "Pss:", "VmHWM:"))})

            checkpoint("controller-reconnect:before", controller_pid=controller.pid)
            controller.terminate()
            yield lambda: backend.property("shelfFreshness") == "unavailable" and not models()
            capture("observer-unavailable")
            check("controller owner loss reports unavailable shelves")
            controller.wait(timeout=5)
            controller = start_controller("controller-restarted")
            # KWin errors can suppress the observer callback. Its existing
            # transport gives an outstanding call 30 seconds before retrying.
            yield (lambda: backend.property("shelfFreshness") == "live", 45)
            assert not fixture()
            checkpoint("controller-reconnect:after", controller_pid=controller.pid)
            check("controller restart reconnected the observer")
            assert unload(replacement_observer)
            package("remove")

        def drive(sequence, predicate=None, deadline=0):
            try:
                if predicate is not None and not predicate():
                    if time.monotonic() >= deadline:
                        endpoint = dbus.Interface(bus.get_object("org.depthos.Controller", "/org/depthos/Controller"),
                                                  "org.depthos.Controller1")
                        print("Shelf at timeout", endpoint.GetDesktopShelves(dbus.UInt32(0), ""), file=sys.stderr)
                        probe = results / "native-probe.js"
                        probe.write_text('console.warn("phase-43 probe " + JSON.stringify(workspace.stackingOrder.map(w => ({app:w.desktopFileName, all:w.onAllDesktops, ds:w.desktops.map(d => d.id), excluded:w.excludeFromCapture, skip:w.skipTaskbar}))));')
                        unload(load_script(probe, probe.stem))
                        raise TimeoutError("Expected private shelf state did not arrive")
                    QTimer.singleShot(15, lambda: drive(sequence, predicate, deadline))
                    return
                next_check = next(sequence)
                predicate, budget = next_check if isinstance(next_check, tuple) else (next_check, 12)
                drive(sequence, predicate, time.monotonic() + budget)
            except StopIteration:
                QGuiApplication.exit(0)
            except Exception:
                errors.append(traceback.format_exc())
                QGuiApplication.exit(1)

        class InstrumentedEngine(QtQml.QQmlApplicationEngine):
            def load(self, url):
                super().load(url)
                window = self.rootObjects()[0]
                assert isinstance(window, QQuickWindow)
                QTimer.singleShot(0, lambda: drive(steps(window)))

        QtQml.QQmlApplicationEngine = InstrumentedEngine
        (results / "screenshots").mkdir(mode=0o700)
        private.verify()
        sys.argv = [str(LAUNCHER), "--window-size", "1366x768", "--reduced-motion"]
        try:
            launcher.main()
        finally:
            for error in errors:
                print(error, file=sys.stderr)
    finally:
        for child in reversed(children):
            if child.poll() is None:
                child.terminate()
        for child in reversed(children):
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inside", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--compositor-restart", action="store_true")
    parser.add_argument("--interrupt", action="store_true")
    args = parser.parse_args()
    os.umask(0o077)
    if args.inside:
        run_isolated(args.inside, args.compositor_restart)
        return
    ARTIFACTS.mkdir(parents=True, exist_ok=True, mode=0o700)
    results = Path(tempfile.mkdtemp(dir=ARTIFACTS, prefix="w-"))
    print(results, flush=True)
    command = [sys.executable, __file__, "--inside", str(results)]
    if args.compositor_restart:
        command.append("--compositor-restart")
    with HostMonitor(results / "host.jsonl") as monitor:
        status = launch(results, command, wayland=True, monitor=monitor,
                        interrupt_at="script-load:after" if args.interrupt else None)
    monitor.require_clean()
    print(f"Host monitor clean across {monitor.checkpoints} checkpoints", flush=True)
    assert status == (130 if args.interrupt else 0), f"Private worker exited with {status}"
    stages = [json.loads(line) for line in (results / "stages.jsonl").read_text().splitlines()]
    assert stages[-1]["stage"] == "cleanup:after" and not stages[-1]["owned_pids"]
    assert not (results / "data/kwin/scripts/depthos-system-observer").exists()
    assert not (results / "runtime" / SOCKET).exists()
    if args.interrupt:
        assert any(row["stage"] == "interruption:requested" for row in stages)
        print("PASS interruption after script load left no owned processes or package", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
