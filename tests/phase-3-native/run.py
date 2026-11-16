#!/usr/bin/env python3
"""Exercise read-only window tracking on owned surfaces in virtual KWin 6.8.3."""
from contextlib import ExitStack
import json
import os
import re
from pathlib import Path
from queue import Empty, Queue
import select
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
ADAPTER = "phase3-native-preview"
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
    activities = call(bus, activity_owner, "/ActivityManager/Activities",
                      ACTIVITIES + ".Activities", "ListActivities")
    config = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    data = Path(os.environ.get("XDG_DATA_HOME", Path.home() / ".local/share"))
    return {
        "bus": bus_call(bus, "GetId"),
        "owner": owner,
        "compositorPid": bus_call(bus, "GetConnectionUnixProcessID", "s", (owner,)),
        "desktopProperties": desktop_properties,
        "effects": effects,
        "activeOutput": call(bus, owner, "/KWin", "org.kde.KWin", "activeOutputName"),
        "activity": activity,
        "activityOwner": activity_owner,
        "activities": sorted(activities),
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


def launch(run, expected_host_bus_id, adapter):
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
               __file__, "--worker", str(run), expected_host_bus_id, str(adapter)]
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


def work(run, expected_host_bus_id, adapter):
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
    bus_id = bus_call(bus, "GetId")
    require(bus_id != expected_host_bus_id, "Private D-Bus identity required")
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

        def expect(event, **expected_fields):
            try:
                record = records.get(timeout=12)
            except Empty as error:
                raise TimeoutError(f"Waiting for private KWin event {event}") from error
            print(json.dumps({k: v for k, v in record.items() if k != "preview"}), flush=True)
            require(record["event"] == event and all(
                record.get(field) == value for field, value in expected_fields.items()
            ), f"Unexpected private KWin event while waiting for {event}")
            return record

        wait_for(lambda: bus_call(bus, "NameHasOwner", "s", (KWIN,)), "Private KWin owner")
        owner = bus_call(bus, "GetNameOwner", "s", (KWIN,))
        require(bus_call(bus, "GetConnectionUnixProcessID", "s", (owner,)) == kwin.pid,
                "Private KWin PID mismatch")
        socket = runtime / SOCKET
        require(stat.S_ISSOCK(socket.stat().st_mode), "Private Wayland socket missing")
        support = call(bus, owner, "/KWin", "org.kde.KWin", "supportInformation")
        require(f"KWin version: {VERSION}" in support.splitlines(), "Running KWin version mismatch")
        desktops_path = "/VirtualDesktopManager"
        desktops_interface = KWIN + ".VirtualDesktopManager"
        desktop_initial = call(bus, owner, desktops_path, PROPERTIES, "GetAll", "s",
                               (desktops_interface,))
        require(desktop_initial["count"] == 2, "Expected two private desktops")
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
        for index in range(5):
            output = stack.enter_context((run / f"app{index}.log").open("w"))
            process = subprocess.Popen([sys.executable, str(HERE / "fixture.py")],
                                       env={**os.environ, **env}, stdin=subprocess.PIPE,
                                       stdout=subprocess.PIPE, stderr=output, text=True)
            stack.callback(process.stdin.close)
            stack.callback(process.stdout.close)
            stack.callback(stop, process)
            apps.append(process)
        script = run / "authority-fixture.js"
        incarnation = str(uuid.uuid4())
        script.write_text((ROOT / "experiments/kwin-authority/authority.js").read_text() +
                          f"\nconst compositorIncarnation = {json.dumps(incarnation)};" +
                          f"\nconst fixturePids = {[app.pid for app in apps]};\n" +
                          (HERE / "probe.js").read_text())
        identifier = "depthos-phase-3-private-authority"
        handle = call(bus, owner, "/Scripting", "org.kde.kwin.Scripting", "loadScript", "ss",
                      (str(script), identifier))
        require(handle >= 0, "Private fixture script load failed")
        call(bus, owner, f"/Scripting/Script{handle}", "org.kde.kwin.Script", "run")
        expect("ready")

        def check_capture(process):
            record = expect("captured", pid=process.pid)
            require(all(record[key] is True for key in ("liveRevalidated", "foreignProposalRefused",
                        "pendingConflictRefused", "immutable", "distinctDesktops")),
                    "Private live snapshot validation failed")
            require(record["incarnation"] == incarnation and
                    record["runtimeId"] == f"window-{record['generation']}",
                    "Missing fixture-local identity")
            return record

        def assert_invalidated(reason, capture):
            record = expect("invalidated")
            require(record["reason"] == reason and record["staleRefused"] is True and
                    record["generation"] == capture["generation"],
                    f"Expected live proposal invalidation due to {reason}")

        def show(process):
            process.stdin.write("show\n")
            process.stdin.flush()

        def view_command(process, command, expected):
            process.stdin.write(json.dumps(command) + "\n")
            process.stdin.flush()
            require(select.select([process.stdout], [], [], 6)[0], "Private fixture view timeout")
            result = json.loads(process.stdout.readline())
            require(result["event"] == expected, f"Expected private fixture view {expected}")
            return result

        def present(process, capture):
            payload = {"busId": str(uuid.UUID(str(bus_id))), "uniqueOwner": str(owner),
                       "uid": os.getuid(), "preview": capture["preview"]}
            denial = subprocess.run([adapter], input=json.dumps(payload), text=True,
                                    capture_output=True, check=True, timeout=3)
            projected = json.loads(denial.stdout)
            require(projected == {"authorization": "UnverifiedInteraction",
                                  "preview": capture["preview"]},
                    "Rust preview attachment changed placement or did not refuse")
            result = view_command(process, {"preview": projected["preview"]}, "presented")
            expected = [f"From {capture['preview']['sourceDesktop']}",
                        f"To {capture['preview']['destinationDesktop']}",
                        f"Activity {capture['preview']['activity']} stays the same"]
            require(result["labels"] == expected, "View labels diverged from the native snapshot")
            print("Private native preview presented after Rust refusal", flush=True)

        show(apps[0])
        first = check_capture(apps[0])
        show(apps[1])
        assert_invalidated("focus", first)
        second = check_capture(apps[1])
        require(second["generation"] > first["generation"], "Native object generation reused")
        stop(apps[1])
        assert_invalidated("window-removed", second)
        expect("fixture-removed", pid=apps[1].pid)
        stop(apps[0])
        expect("fixture-removed", pid=apps[0].pid)

        show(apps[2])
        third = check_capture(apps[2])
        require(third["generation"] > second["generation"], "Native object generation reused")
        present(apps[2], third)
        view_command(apps[2], {"dismiss": True}, "cancelled")
        assert_invalidated("window-removed", third)
        expect("fixture-removed", pid=apps[2].pid)
        require(apps[2].wait(timeout=3) == 0, "Cancelled fixture did not close cleanly")

        show(apps[3])
        fourth = check_capture(apps[3])
        present(apps[3], fourth)
        assert_invalidated("expired", fourth)
        require(view_command(apps[3], {"invalidate": True}, "hidden")["empty"],
                "Expired native preview remained visible")
        stop(apps[3])
        expect("fixture-removed", pid=apps[3].pid)

        show(apps[4])
        fifth = check_capture(apps[4])
        present(apps[4], fifth)
        desktop_state = call(bus, owner, desktops_path, PROPERTIES, "GetAll", "s",
                             (desktops_interface,))
        require(desktop_state["current"] == desktop_initial["current"],
                "Fixture current desktop changed before topology exercise")
        unused = [row[1] for row in desktop_state["desktops"] if row[1] != desktop_state["current"]]
        require(len(unused) == 1, "Expected one unused private desktop")
        # Remove only the unused fixture destination. The sole owned window remains
        # on the private current desktop. No window placement or focus setter is used.
        call(bus, owner, desktops_path, desktops_interface, "removeDesktop", "s", (unused[0],))
        assert_invalidated("topology", fifth)
        require(view_command(apps[4], {"invalidate": True}, "hidden")["empty"],
                "Stale topology preview remained visible")
        desktop_final = call(bus, owner, desktops_path, PROPERTIES, "GetAll", "s",
                             (desktops_interface,))
        require(desktop_final["count"] == 1 and desktop_final["current"] == desktop_state["current"],
                "Unexpected post-test fixture desktop state")
        stop(apps[4])
        expect("fixture-removed", pid=apps[4].pid)
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
    subprocess.run(["cargo", "build", "--locked", "--offline", "--example", ADAPTER],
                   cwd=ROOT, check=True)
    target_dir = Path(os.environ.get("CARGO_TARGET_DIR", str(ROOT / "target")))
    adapter = (ROOT / target_dir / "debug" / "examples" / ADAPTER).resolve()
    host_bus = dbus.SessionBus()
    before = host_snapshot(host_bus)
    with tempfile.TemporaryDirectory(prefix="depthos-p3-", dir="/tmp") as directory:
        run = Path(directory)
        try:
            launch(run, str(before["bus"]), adapter)
        except Exception:
            for output in sorted(run.glob("*.log")):
                print(f"\n{output.name}\n{output.read_text()[-3500:]}", file=sys.stderr)
            raise
        finally:
            after = host_snapshot(host_bus)
            changed = sorted(key for key, value in before.items() if after[key] != value)
            for field in ("desktopProperties", "effects"):
                if field in changed:
                    previous, current = before[field], after[field]
                    changed.extend(f"{field}.{key}" for key in sorted(set(previous) | set(current))
                                   if previous.get(key) != current.get(key))
            require(not changed, f"Host-state snapshot changed: {changed}")
            print("Host-state snapshot matched", flush=True)
    require(not run.exists(), "Private runtime and XDG roots remained after cleanup")
    print("Private fixture processes and temporary resources removed", flush=True)


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "--worker":
        work(Path(sys.argv[2]), sys.argv[3], Path(sys.argv[4]))
    elif len(sys.argv) == 1:
        main()
    else:
        raise SystemExit("Usage: run.py")
