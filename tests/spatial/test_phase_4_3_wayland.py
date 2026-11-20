#!/usr/bin/env python3
"""Production launcher and real observations under a disposable KWin Wayland session."""
import argparse
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "artifacts/phase-4-3"
LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
OBSERVER = ROOT / "experiments/kwin-observer/contents/code/main.js"


def run_isolated(results, restart):
    import dbus
    from PyQt6 import QtQml
    from PyQt6.QtCore import QTimer
    from PyQt6.QtGui import QGuiApplication
    from PyQt6.QtQuick import QQuickWindow

    assert Path(os.environ["XDG_RUNTIME_DIR"]).resolve() == results / "runtime"
    assert Path(os.environ["HOME"]).resolve() == results / "home"
    bus = dbus.SessionBus()
    assert set(bus.list_names()) - {"org.freedesktop.DBus", bus.get_unique_name()} == set()
    daemon = dbus.Interface(bus.get_object("org.freedesktop.DBus", "/org/freedesktop/DBus"),
                            "org.freedesktop.DBus")
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

    def load_script(path, name):
        number = int(bus.call_blocking("org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting",
                                       "loadScript", "ss", (str(path), name), timeout=5))
        assert number >= 0
        script = dbus.Interface(bus.get_object("org.kde.KWin", f"/Scripting/Script{number}"),
                                "org.kde.kwin.Script")
        script.run(timeout=5)
        return name

    def unload(name):
        return dbus.Interface(bus.get_object("org.kde.KWin", "/Scripting"),
                              "org.kde.kwin.Scripting").unloadScript(name)

    def move_fixture(target, pid, native_desktops, protection=None):
        # This script can only target the Konsole process spawned inside this
        # run. It is never installed and never connects to the host bus.
        assert int(daemon.GetConnectionUnixProcessID("org.kde.KWin")) == kwin.pid
        params = json.dumps({"pid": pid, "desktops": native_desktops, "protection": protection})
        target.write_text("""
const p = PARAMS;
const found = workspace.stackingOrder.filter(w => w.pid === p.pid &&
                                                    w.desktopFileName === "org.kde.konsole");
if (found.length !== 1) throw new Error("Disposable fixture missing");
if (p.protection === null)
    found[0].desktops = workspace.desktops.filter(d => p.desktops.includes(d.id));
else
    found[0].excludeFromCapture = p.protection;
""".replace("PARAMS", params))
        unload(load_script(target, target.stem))

    def compositor(label):
        kwin = process(label, "kwin_wayland", "--virtual", "--no-lockscreen", "--width", "1600",
                       "--height", "1000", "--socket", os.environ["WAYLAND_DISPLAY"])
        wait(lambda: bus.name_has_owner("org.kde.KWin") and
             (results / "runtime" / os.environ["WAYLAND_DISPLAY"]).exists())
        assert int(daemon.GetConnectionUnixProcessID("org.kde.KWin")) == kwin.pid
        return kwin

    try:
        ids = [str(uuid.uuid4()), str(uuid.uuid4())]
        (results / "config/kwinrc").write_text(
            f"[Desktops]\nNumber=2\nId_1={ids[0]}\nId_2={ids[1]}\n")
        kwin = compositor("kwin")
        properties = dbus.Interface(bus.get_object("org.kde.KWin", "/VirtualDesktopManager"),
                                    "org.freedesktop.DBus.Properties")
        native = [str(row[1]) for row in properties.Get("org.kde.KWin.VirtualDesktopManager", "desktops")]
        assert len(native) == 2
        workspace, desktops = provision()
        for source, target in zip(native, desktops):
            cli("association", "set", "native-desktop", source, "--workspace", workspace, "--desktop", target)
        controller = process("controller", str(ROOT / "target/debug/depthos"), "--db",
                             str(results / "workspace.sqlite"), "controller", "run")
        wait(lambda: bus.name_has_owner("org.depthos.Controller") and
             bus.name_has_owner("org.depthos.StateInspector"))
        observer = load_script(OBSERVER, "phase-4-3-observer")
        marker = process("konsole", "konsole", "--separate", "--hide-menubar", "--hide-tabbar",
                         "-e", "sleep", "110")

        if restart:
            endpoint = dbus.Interface(bus.get_object("org.depthos.Controller", "/org/depthos/Controller"),
                                      "org.depthos.Controller1")

            def shelf():
                return json.loads(str(endpoint.GetDesktopShelves(dbus.UInt32(0), "")))

            def observed():
                return next((row for row in shelf()["data"]["items"]
                             if row["applicationId"] == "org.kde.konsole"), None)

            wait(observed)
            before = observed()
            assert before["desktopId"] == desktops[0]
            kwin.terminate()
            kwin.wait(timeout=5)
            wait(lambda: not bus.name_has_owner("org.kde.KWin") and not shelf()["available"])
            assert shelf()["data"]["items"] == []
            marker.terminate()
            marker.wait(timeout=5)
            kwin = compositor("kwin-restarted")
            load_script(OBSERVER, "phase-4-3-compositor-restart")
            wait(lambda: shelf()["available"])
            assert not observed()
            process("konsole-after-compositor-restart", "konsole", "--separate", "-e", "sleep", "60")
            wait(observed)
            after = observed()
            assert after["desktopId"] == before["desktopId"] and after["id"] != before["id"]
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
            evidence.append({"check": name, "detail": detail})
            (results / "results.json").write_text(json.dumps(evidence, indent=2) + "\n")
            print("PASS", name, detail if detail is not None else "", flush=True)

        def steps(window):
            backend = window.property("backend")
            scene = window.property("scene")
            window.frameSwapped.connect(lambda: swaps.append(time.monotonic()))

            def models():
                rows = backend.property("windows")
                return rows.toVariant() if hasattr(rows, "toVariant") else rows

            def fixture():
                return next((row for row in models() if row.get("appId") == "org.kde.konsole"), None)

            def capture(name):
                assert window.grabWindow().save(str(ARTIFACTS / "screenshots" / (name + ".png")))

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
            load_script(OBSERVER, "phase-4-3-observer-restart")
            yield lambda: backend.property("shelfFreshness") == "live"
            assert not fixture()
            check("observer replacement excludes removed windows")
            replacement = process("konsole-replacement", "konsole", "--separate", "-e", "sleep", "90")
            yield lambda: fixture()
            assert fixture()["id"] != opaque_id
            check("new native window is assigned a new opaque identity")

            # Transition to the Desktop hosting the replacement, then destroy it.
            backend.selectDesktop(fixture()["desktopId"])
            replacement_id = fixture()["id"]
            yield lambda: card(replacement_id)
            scene.setProperty("reducedMotion", False)
            scene.returnToParent()
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

            controller.terminate()
            yield lambda: backend.property("shelfFreshness") == "unavailable" and not models()
            capture("observer-unavailable")
            check("controller owner loss reports unavailable shelves")

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
                drive(sequence, next_check, time.monotonic() + 12)
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
    args = parser.parse_args()
    os.umask(0o077)
    if args.inside:
        run_isolated(args.inside, args.compositor_restart)
        return
    (ARTIFACTS / "screenshots").mkdir(parents=True, exist_ok=True, mode=0o700)
    results = Path(tempfile.mkdtemp(dir=ARTIFACTS, prefix="wayland-"))
    env = {"PATH": os.environ["PATH"], "LANG": "C.UTF-8", "QT_QPA_PLATFORM": "wayland",
           "WAYLAND_DISPLAY": "depthos-phase-43", "QT_NO_XDG_DESKTOP_PORTAL": "1",
           "XDG_SESSION_TYPE": "wayland", "KWIN_XKB_DEFAULT_KEYMAP": "1"}
    for key, name in (("HOME", "home"), ("XDG_RUNTIME_DIR", "runtime"), ("XDG_CONFIG_HOME", "config"),
                      ("XDG_DATA_HOME", "data"), ("XDG_CACHE_HOME", "cache"), ("XDG_STATE_HOME", "state")):
        path = results / name
        path.mkdir(mode=0o700)
        env[key] = str(path)
    print(results, flush=True)
    command = ["dbus-run-session", "--", sys.executable, __file__, "--inside", str(results)]
    if args.compositor_restart:
        command.append("--compositor-restart")
    with subprocess.Popen(command,
                          env=env, start_new_session=True) as session:
        try:
            return session.wait(timeout=110)
        finally:
            try:
                os.killpg(session.pid, signal.SIGTERM)
            except ProcessLookupError:
                pass


if __name__ == "__main__":
    raise SystemExit(main())
