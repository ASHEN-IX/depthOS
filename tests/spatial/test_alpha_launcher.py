#!/usr/bin/env python3
"""Exercise the production launcher with disposable SQLite and session-bus state."""
import argparse
import hashlib
import importlib.machinery
import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import traceback

ROOT = Path(__file__).resolve().parents[2]
DEPTHOS = ROOT / "target/debug/depthos"
LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
CONTROLLER = "org.depthos.Controller"


def check(*args):
    subprocess.run([str(DEPTHOS), *args], check=True, stdout=subprocess.DEVNULL, timeout=6)


def await_owner():
    import dbus

    bus = dbus.SessionBus(private=True)
    try:
        for _ in range(100):
            if bus.name_has_owner(CONTROLLER):
                return
            time.sleep(0.05)
        raise AssertionError("Private controller failed to acquire its name")
    finally:
        bus.close()


def stop(proc):
    if proc and proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=4)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait(timeout=4)


def run_client(mode, folder, screenshot):
    from PyQt6 import QtQml
    from PyQt6.QtCore import QTimer
    from PyQt6.QtGui import QGuiApplication
    from PyQt6.QtQuick import QQuickWindow

    loader = importlib.machinery.SourceFileLoader("depthos_launcher", str(LAUNCHER))
    spec = importlib.util.spec_from_loader(loader.name, loader)
    launcher = importlib.util.module_from_spec(spec)
    loader.exec_module(launcher)
    db = folder / (mode + ".sqlite")
    controller = None
    error = []

    def start_controller():
        return subprocess.Popen(
            [str(DEPTHOS), "--db", str(db), "controller", "run"],
            stdout=subprocess.DEVNULL,
        )

    def worlds(backend, name):
        data = backend.property("worlds")
        rows = data if isinstance(data, list) else data.toVariant()
        return next((row["id"] for row in rows if row["name"] == name), None)

    def steps(window):
        nonlocal controller
        backend = window.property("backend")
        scene = window.property("scene")
        if mode == "onboarding":
            assert not db.exists()
            assert backend.property("onboardingState") == "offer"
            backend.requestCreateWorkspace()
            assert backend.property("onboardingState") == "success"
            assert backend.property("sourceKind") == "sqlite"
            assert db.exists() and worlds(backend, "Default World")
            return

        yield lambda: backend.property("state") == "Connected" and worlds(backend, "Default World")
        assert not backend.property("observerLive")
        assert backend.property("observationHealth") != "Live"
        original = hashlib.sha256(db.read_bytes()).hexdigest()
        if screenshot:
            yield window.isExposed
            assert window.grabWindow().save(str(screenshot))
        assert original == hashlib.sha256(db.read_bytes()).hexdigest()

        check("--db", str(db), "world", "create", "Launcher fixture")
        yield lambda: worlds(backend, "Launcher fixture")
        selected = worlds(backend, "Launcher fixture")
        backend.selectWorld(selected)
        check("--db", str(db), "world", "create", "Second launcher fixture")
        yield lambda: worlds(backend, "Second launcher fixture")
        assert backend.property("selectedWorldId") == selected

        stop(controller)
        yield lambda: backend.property("state") == "Unavailable"
        assert not backend.property("observerLive")
        assert backend.property("selectedWorldId") == selected
        assert scene.property("currentLevel") == 1
        controller = start_controller()
        yield lambda: backend.property("state") == "Connected"
        assert backend.property("selectedWorldId") == selected

    def drive(sequence, predicate=None, deadline=0):
        try:
            if predicate is not None and not predicate():
                assert time.monotonic() < deadline, "Expected launcher state did not arrive"
                QTimer.singleShot(25, lambda: drive(sequence, predicate, deadline))
                return
            check_ready = next(sequence)
            drive(sequence, check_ready, time.monotonic() + 6)
        except StopIteration:
            print(f"PASS production launcher {mode}", flush=True)
            QGuiApplication.exit(0)
        except Exception:
            error.append(traceback.format_exc())
            QGuiApplication.exit(1)

    class InstrumentedEngine(QtQml.QQmlApplicationEngine):
        def load(self, url):
            super().load(url)
            if self.rootObjects():
                window = self.rootObjects()[0]
                assert isinstance(window, QQuickWindow)
                QTimer.singleShot(0, lambda: drive(steps(window)))

    QtQml.QQmlApplicationEngine = InstrumentedEngine
    try:
        sys.argv = [str(LAUNCHER)]
        if mode == "onboarding":
            sys.argv.extend(["--db", str(db)])
        else:
            check("--db", str(db), "init", "--default-hierarchy")
            controller = start_controller()
            await_owner()
        launcher.main()
    finally:
        stop(controller)
        if error:
            print(error[0], file=sys.stderr)


def inside(args):
    folder = Path(os.environ["XDG_RUNTIME_DIR"])
    compositor = None
    try:
        if args.wayland:
            import dbus

            compositor = subprocess.Popen(
                ["kwin_wayland", "--virtual", "--no-lockscreen", "--width", "1600",
                 "--height", "1000", "--socket", "depthos-launcher-test"],
                stdout=sys.stderr,
            )
            bus = dbus.SessionBus()
            for _ in range(150):
                if bus.name_has_owner("org.kde.KWin") and (folder / os.environ["WAYLAND_DISPLAY"]).exists():
                    break
                assert compositor.poll() is None, "Private KWin did not start"
                time.sleep(0.05)
            else:
                raise AssertionError("Private KWin did not expose its Wayland socket")
        for mode in ("onboarding", "controller"):
            command = [sys.executable, __file__, "--client", mode]
            if args.screenshot and mode == "controller":
                command.extend(["--screenshot", str(args.screenshot)])
            subprocess.run(command, check=True, timeout=35)
    finally:
        stop(compositor)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wayland", action="store_true")
    parser.add_argument("--screenshot", type=Path, help="Optional own-window capture of the controller state")
    parser.add_argument("--inside", action="store_true", help=argparse.SUPPRESS)
    parser.add_argument("--client", choices=["onboarding", "controller"], help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.client or args.inside:
        private_runtime = os.environ.get("DEPTHOS_LAUNCHER_TEST")
        if not private_runtime or private_runtime != os.environ.get("XDG_RUNTIME_DIR"):
            parser.error("Run without internal test flags to create a private session")
        if args.client:
            run_client(args.client, Path(os.environ["XDG_RUNTIME_DIR"]), args.screenshot)
        else:
            inside(args)
        return

    os.umask(0o077)
    with tempfile.TemporaryDirectory(prefix="depthos-launcher-") as name:
        folder = Path(name)
        env = {
            "PATH": os.environ["PATH"],
            "LANG": "C.UTF-8",
            "HOME": str(folder / "home"),
            "XDG_RUNTIME_DIR": str(folder / "run"),
            "XDG_CONFIG_HOME": str(folder / "config"),
            "XDG_DATA_HOME": str(folder / "data"),
            "XDG_CACHE_HOME": str(folder / "cache"),
            "XDG_STATE_HOME": str(folder / "state"),
            "QT_NO_XDG_DESKTOP_PORTAL": "1",
        }
        for key in ("HOME", "XDG_RUNTIME_DIR", "XDG_CONFIG_HOME", "XDG_DATA_HOME",
                    "XDG_CACHE_HOME", "XDG_STATE_HOME"):
            Path(env[key]).mkdir(mode=0o700)
        env["DEPTHOS_LAUNCHER_TEST"] = env["XDG_RUNTIME_DIR"]
        if args.wayland:
            env.update(QT_QPA_PLATFORM="wayland", WAYLAND_DISPLAY="depthos-launcher-test",
                       XDG_SESSION_TYPE="wayland")
        else:
            env.update(QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software")
        command = ["dbus-run-session", "--", sys.executable, __file__, "--inside", *sys.argv[1:]]
        subprocess.run(command, env=env, check=True, timeout=85)


if __name__ == "__main__":
    main()
