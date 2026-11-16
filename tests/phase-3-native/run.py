#!/usr/bin/env python3
"""Exercise read-only window tracking on owned surfaces in virtual KWin 6.8.3."""
from contextlib import ExitStack
import json
import os
import re
from pathlib import Path
from queue import Empty, Queue
import signal
import stat
import subprocess
import sys
import tempfile
import threading
import time
import uuid
from xml.sax.saxutils import escape

import dbus

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
DAEMON = "org.freedesktop.DBus"
PROPERTIES = DAEMON + ".Properties"
KWIN = "org.kde.KWin"
ACTIVITIES = "org.kde.ActivityManager"
VERSION = "6.8.3"
SOCKET = "depthos-phase-3"
sys.path.insert(0, str(ROOT / "tests/integration/poc02e"))
from process_inventory import members


def require(condition, reason):
    if not condition:
        raise RuntimeError(reason)


def call(bus, service, path, interface, method, signature="", args=()):
    return bus.call_blocking(service, path, interface, method, signature, args, timeout=6)


def bus_call(bus, method, signature="", args=()):
    return call(bus, DAEMON, "/org/freedesktop/DBus", DAEMON, method, signature, args)


def host_snapshot(bus):
    """Read-only bus requests and local file reads. No host Wayland connection."""
    owner = bus_call(bus, "GetNameOwner", "s", (KWIN,))
    activity_owner = bus_call(bus, "GetNameOwner", "s", (ACTIVITIES,))
    desktop_properties = call(bus, owner, "/VirtualDesktopManager", PROPERTIES, "GetAll", "s",
                              (KWIN + ".VirtualDesktopManager",))
    effects = call(bus, owner, "/Effects", PROPERTIES, "GetAll", "s", ("org.kde.kwin.Effects",))
    activity = call(bus, activity_owner, "/ActivityManager/Activities",
                    ACTIVITIES + ".Activities", "CurrentActivity")
    config = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    data = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share"))
    return {
        "bus": bus_call(bus, "GetId"),
        "owner": owner,
        "desktopProperties": desktop_properties,
        "effects": effects,
        "activeOutput": call(bus, owner, "/KWin", "org.kde.KWin", "activeOutputName"),
        "activity": activity,
        "configContents": {name: (config / name).read_bytes() if (config / name).exists() else None
                           for name in ("kwinrc", "kglobalshortcutsrc", "kactivitymanagerdrc",
                                        "kscreenlockerrc", "plasma-workspace/env/depthos.sh")},
        "autostartContents": {path.name: path.read_bytes() if path.is_file() else None
                             for path in (config / "autostart").glob("*")},
        "extensionPaths": sorted(str(path.relative_to(data))
                                 for kind in ("effects", "scripts")
                                 for path in (data / "kwin" / kind).glob("*"))
    }


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)


def wait_for(predicate, label):
    deadline = time.monotonic() + 10
    tick = threading.Event()
    while time.monotonic() < deadline:
        result = predicate()
        if result:
            return result
        tick.wait(0.05)
    raise TimeoutError(label)


def launch(run, expected_host_bus_id):
    env = {"PATH": "/usr/bin", "LANG": "C.UTF-8", "HOME": str(run / "home"),
           "XDG_RUNTIME_DIR": str(run / "runtime"), "XDG_CONFIG_HOME": str(run / "config"),
           "XDG_DATA_HOME": str(run / "data"), "XDG_CACHE_HOME": str(run / "cache"),
           "XDG_STATE_HOME": str(run / "state"), "XDG_CONFIG_DIRS": str(run / "config"),
           "XDG_DATA_DIRS": "/usr/share", "XDG_SESSION_TYPE": "wayland",
           "XDG_CURRENT_DESKTOP": "KDE", "DBUS_SYSTEM_BUS_ADDRESS": "unix:path=/dev/null",
           "QT_NO_XDG_DESKTOP_PORTAL": "1", "QT_FORCE_STDERR_LOGGING": "1",
           "QT_LOGGING_RULES": "kwin_scripting.debug=true", "PYTHONDONTWRITEBYTECODE": "1"}
    for name in ("home", "runtime", "config", "data", "cache", "state"):
        (run / name).mkdir(mode=0o700)
    config = run / "bus.xml"
    config.write_text(f"""<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
"http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig><type>session</type>
<listen>unix:path={escape(str(run / "runtime/bus"))}</listen>
<standard_session_servicedirs/>
<policy context="default">
  <allow send_destination="*"/><allow receive_sender="*"/><allow own="*"/>
</policy>
</busconfig>
""")
    desktops = "\n".join(f"Id_{index}={uuid.uuid4()}\nName_{index}=Fixture {index}"
                         for index in (1, 2))
    (run / "config/kwinrc").write_text(f"[Desktops]\nNumber=2\nRows=1\n{desktops}\n[Plugins]\nkwin4_effect_shapecornersEnabled=false\n")
    command = ["dbus-run-session", f"--config-file={config}", "--", sys.executable,
               __file__, "--worker", str(run), expected_host_bus_id]
    process = subprocess.Popen(command, env=env, start_new_session=True)
    try:
        rc = process.wait(timeout=75)
    finally:
        # Always drain the disposable group, including D-Bus activation children.
        for sig in (signal.SIGTERM, signal.SIGKILL):
            if any(entry["state"] != "Z" for entry in members(process.pid)):
                try:
                    os.killpg(process.pid, sig)
                except ProcessLookupError:
                    pass
        process.wait(timeout=5)
        wait_for(lambda: not any(entry["state"] != "Z" for entry in members(process.pid)),
                 "disposable process group cleanup")
    require(rc == 0, f"Private worker exit {rc}")


def work(run, expected_host_bus_id):
    runtime = run / "runtime"
    require(run.parent == Path("/tmp") and run.name.startswith("depthos-p3-") and
            run.resolve() == run, "Unrecognized private run directory")
    roots = {"HOME": "home", "XDG_RUNTIME_DIR": "runtime", "XDG_CONFIG_HOME": "config",
             "XDG_DATA_HOME": "data", "XDG_CACHE_HOME": "cache", "XDG_STATE_HOME": "state",
             "XDG_CONFIG_DIRS": "config"}
    for variable, name in roots.items():
        path = run / name
        require(os.environ.get(variable) == str(path), f"Private {variable} required")
        directory = path.stat(follow_symlinks=False)
        require(stat.S_ISDIR(directory.st_mode) and directory.st_uid == os.getuid()
                and stat.S_IMODE(directory.st_mode) == 0o700, f"Unsafe private directory {name}")
    address = re.escape(f"unix:path={runtime / 'bus'}") + r"(?:,guid=[0-9a-f]{32})?"
    require(re.fullmatch(address, os.environ.get("DBUS_SESSION_BUS_ADDRESS", "")),
            "Private D-Bus socket required")
    require(os.environ.get("DBUS_SYSTEM_BUS_ADDRESS") == "unix:path=/dev/null",
            "System D-Bus must be unreachable")
    require(all(name not in os.environ for name in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "DISPLAY")),
            "Unexpected inherited display")
    bus = dbus.SessionBus()
    require(bus_call(bus, "GetId") != expected_host_bus_id, "Private D-Bus identity required")
    owners = {str(name) for name in bus_call(bus, "ListNames") if not name.startswith(":")}
    require(owners == {DAEMON}, "Private D-Bus must start empty")

    local = subprocess.check_output(["kwin_wayland", "--version"], text=True).strip()
    require(local == f"kwin {VERSION}", "KWin version has not been qualified")
    records = Queue()

    with ExitStack() as stack:
        kwin = subprocess.Popen(["kwin_wayland", "--virtual", "--no-lockscreen",
                                 "--no-global-shortcuts", "--socket", SOCKET, "--width", "800",
                                 "--height", "600"], text=True, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT)
        stack.callback(kwin.stdout.close)
        stack.callback(stop, kwin)

        def collect():
            with (run / "kwin.log").open("w") as log:
                for line in kwin.stdout:
                    log.write(line)
                    if "DEPTHOS_AUTHORITY " in line:
                        records.put(json.loads(line.split("DEPTHOS_AUTHORITY ", 1)[1]))
            records.put({"event": "compositor-exited"})

        reader = threading.Thread(target=collect, daemon=True)
        reader.start()

        def expect(event):
            deadline = time.monotonic() + 12
            for _ in range(20):
                try:
                    record = records.get(timeout=max(0, deadline - time.monotonic()))
                except Empty as error:
                    raise TimeoutError(f"Waiting for private KWin event {event}") from error
                print(json.dumps(record), flush=True)
                if record["event"] == event:
                    return record
                require(record["event"] != "compositor-exited", "Private compositor exited")
            raise RuntimeError(f"Missing event {event}")

        wait_for(lambda: bus_call(bus, "NameHasOwner", "s", (KWIN,)), "Private KWin owner")
        owner = bus_call(bus, "GetNameOwner", "s", (KWIN,))
        require(bus_call(bus, "GetConnectionUnixProcessID", "s", (owner,)) == kwin.pid,
                "Private KWin PID mismatch")
        socket = runtime / SOCKET
        require(stat.S_ISSOCK(socket.stat().st_mode), "Private Wayland socket missing")
        support = call(bus, owner, "/KWin", "org.kde.KWin", "supportInformation")
        require(f"KWin version: {VERSION}" in support.splitlines(), "Running KWin version mismatch")
        desktop_count = call(bus, owner, "/VirtualDesktopManager", PROPERTIES, "Get", "ss",
                             (KWIN + ".VirtualDesktopManager", "count"))
        require(desktop_count == 2, "Expected two private desktops")
        env = {"WAYLAND_DISPLAY": SOCKET, "QT_QPA_PLATFORM": "wayland"}
        bus_call(bus, "UpdateActivationEnvironment", "a{ss}", (env,))
        path = "/ActivityManager/Activities"
        interface = ACTIVITIES + ".Activities"
        # This process has already verified the isolated bus and compositor.
        activities = call(bus, ACTIVITIES, path, interface, "ListActivities")
        if not activities:
            activity = call(bus, ACTIVITIES, path, interface, "AddActivity", "s",
                            ("DepthOS private fixture",))
            call(bus, ACTIVITIES, path, interface, "SetCurrentActivity", "s", (activity,))
        require(len(call(bus, ACTIVITIES, path, interface, "ListActivities")) == 1,
                "Expected one private Activity")
        print(json.dumps({"isolatedKWin": VERSION, "desktops": 2, "activities": 1,
                          "authority": "deny-only"}), flush=True)

        apps = []
        for index in range(2):
            output = stack.enter_context((run / f"app{index}.log").open("w"))
            process = subprocess.Popen([sys.executable, str(HERE / "fixture.py")],
                                       env={**os.environ, **env}, stdin=subprocess.PIPE,
                                       stdout=output, stderr=subprocess.STDOUT, text=True)
            stack.callback(process.stdin.close)
            stack.callback(stop, process)
            apps.append(process)
        script = run / "authority-fixture.js"
        script.write_text((ROOT / "experiments/kwin-authority/authority.js").read_text() +
                          f"\nconst fixturePids = {[app.pid for app in apps]};\n" +
                          (HERE / "probe.js").read_text())
        identifier = "depthos-phase-3-private-authority"
        handle = call(bus, owner, "/Scripting", "org.kde.kwin.Scripting", "loadScript", "ss",
                      (str(script), identifier))
        require(handle >= 0, "Private fixture script load failed")
        call(bus, owner, f"/Scripting/Script{handle}", "org.kde.kwin.Script", "run")
        expect("ready")

        def check_capture():
            record = expect("captured")
            require(all(record[key] is True for key in ("liveRevalidated", "foreignProposalRefused",
                        "pendingConflictRefused", "immutable", "distinctDesktops")),
                    "Private live snapshot validation failed")
            return record

        def assert_invalidated(reason):
            record = expect("invalidated")
            require(record["reason"] == reason and record["staleRefused"] is True,
                    f"Expected live proposal invalidation due to {reason}")

        def show(process):
            process.stdin.write("show\n")
            process.stdin.flush()

        show(apps[0])
        first = check_capture()
        show(apps[1])
        assert_invalidated("focus")
        second = check_capture()
        require(second["generation"] > first["generation"], "Native object generation reused")
        stop(apps[1])
        assert_invalidated("window-removed")
        expect("fixture-removed")
        stop(apps[0])
        expect("fixture-removed")
        call(bus, owner, "/Scripting", "org.kde.kwin.Scripting", "unloadScript", "s", (identifier,))

        # Exercise the passive view as a client of this compositor. QtTest uses
        # local view events and cannot supply a compositor approval event.
        subprocess.run(
            ["/usr/lib/qt6/bin/qmltestrunner", "-input", str(ROOT / "tests/phase-3")],
            env={**os.environ, **env, "QT_QUICK_BACKEND": "software"},
            check=True, timeout=20,
        )
        stop(kwin)
        reader.join(timeout=3)
        require(not reader.is_alive(), "Private compositor log reader did not stop")
    require(not socket.exists(), "Private Wayland socket remained after KWin teardown")


def main():
    os.umask(0o077)
    host_bus = dbus.SessionBus()
    before = host_snapshot(host_bus)
    with tempfile.TemporaryDirectory(prefix="depthos-p3-", dir="/tmp") as directory:
        run = Path(directory)
        try:
            launch(run, str(before["bus"]))
        except Exception:
            for output in sorted(run.glob("*.log")):
                print(f"\n{output.name}\n{output.read_text()[-3500:]}", file=sys.stderr)
            raise
        finally:
            after = host_snapshot(host_bus)
            changed = sorted(key for key, value in before.items() if after[key] != value)
            if "desktopProperties" in changed:
                before_desktops, after_desktops = (snapshot["desktopProperties"]
                                                   for snapshot in (before, after))
                changed.extend(f"desktopProperties.{key}" for key in
                               sorted(set(before_desktops) | set(after_desktops))
                               if before_desktops.get(key) != after_desktops.get(key))
            require(not changed, f"Host-state snapshot changed: {changed}")
            print("Host-state snapshot matched", flush=True)
    require(not run.exists(), "Private runtime and XDG roots remained after cleanup")
    print("Private fixture processes and temporary resources removed", flush=True)


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--worker":
        work(Path(sys.argv[2]), sys.argv[3])
    elif len(sys.argv) == 1:
        main()
    else:
        raise SystemExit("Usage: run.py")
