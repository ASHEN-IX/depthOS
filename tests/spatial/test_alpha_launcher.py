#!/usr/bin/env python3
"""Exercise the production launcher with disposable SQLite and session-bus state."""
import argparse
import hashlib
import importlib.machinery
import importlib.util
import os
import shutil
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import traceback

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/support"))
from private_session import SOCKET, PrivateKWin, guard, launch

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
            capture_at = time.monotonic() + 0.5
            yield lambda: window.isExposed() and time.monotonic() >= capture_at
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
    root = args.inside
    bus = guard(root)
    compositor = None
    try:
        if args.wayland:
            compositor = subprocess.Popen(
                ["/usr/bin/kwin_wayland", "--virtual", "--no-lockscreen", "--no-global-shortcuts",
                 "--width", "1600", "--height", "1000", "--socket", SOCKET],
                stdout=sys.stderr,
            )
            for _ in range(150):
                if bus.name_has_owner("org.kde.KWin") and (root / "runtime" / SOCKET).is_socket():
                    break
                assert compositor.poll() is None, "Private KWin did not start"
                time.sleep(0.05)
            else:
                raise AssertionError("Private KWin did not expose its Wayland socket")
            private = PrivateKWin(root, bus, compositor)
        for mode in ("onboarding", "controller"):
            if compositor:
                private.verify()
            command = [sys.executable, __file__, "--inside", str(root), "--client", mode]
            if args.screenshot and mode == "controller":
                command.extend(["--screenshot", str(root / "launcher.png")])
            subprocess.run(command, check=True, timeout=35)
    finally:
        bus.close()
        stop(compositor)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--wayland", action="store_true")
    parser.add_argument("--screenshot", type=Path, help="Optional own-window capture of the controller state")
    parser.add_argument("--inside", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--client", choices=["onboarding", "controller"], help=argparse.SUPPRESS)
    args = parser.parse_args()
    if args.client:
        if not args.inside:
            parser.error("Internal client requires an owned session root")
        guard(args.inside).close()
        run_client(args.client, args.inside / "runtime", args.screenshot)
        return
    if args.inside:
        inside(args)
        return

    os.umask(0o077)
    destination = args.screenshot.absolute() if args.screenshot else None
    if destination and (destination.resolve() != destination or
                        not destination.is_relative_to(ROOT / "artifacts")):
        parser.error("Save regression screenshots under the repository's artifacts directory")
    with tempfile.TemporaryDirectory(prefix="depthos-launcher-") as name:
        root = Path(name)
        command = [sys.executable, __file__, "--inside", str(root)]
        if args.wayland:
            command.append("--wayland")
        if destination:
            command.extend(["--screenshot", str(root / "launcher.png")])
        if launch(root, command, wayland=args.wayland):
            raise RuntimeError("Private launcher regression failed")
        if destination:
            destination.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
            with destination.open("xb") as output, (root / "launcher.png").open("rb") as source:
                shutil.copyfileobj(source, output)


if __name__ == "__main__":
    main()
