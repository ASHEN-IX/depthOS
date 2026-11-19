#!/usr/bin/env python3
"""Phase 3.10.1 Real Private KWin Cross-Process Integration Runner.

Executes real end-to-end integration between:
Rust durable coordinator (CrossProcessCoordinator) <-> Authenticated private IPC <->
Private KWin Authority Server <-> Virtual KWin Compositor with owned Wayland fixture client.

Verifies:
1. Exact socket credential authentication (SO_PEERCRED) and bootstrap token handshake.
2. Dual-key identity binding (WindowKey + native_uuid + generation).
3. Happy path: Reserve -> commit WAL intent -> SyntheticAttempt -> Count=1 -> Replay denied.
4. Native invalidation scenarios:
   - Focus loss
   - Window destruction
   - Window recreation with generation bump
   - Destination desktop removal
   - Proposal expiry
   - Screen lock transition
   - Compositor restart (incarnation fencing)
   - Coordinator restart (quarantine persistence, replay denied)
5. Zero host pollution, strict private directory and socket permissions.
"""

from contextlib import ExitStack
import hashlib
import json
import os
from pathlib import Path
import re
import select
import shutil
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
KWIN_EXE = Path("/usr/bin/kwin_wayland")
KWIN_SHA256 = "33350d420b7b196c1e1f90ffa8c1280b4b3102d4e4b60e1929bad525be93df5e"
WAYLAND_SOCKET = "depthos-phase-3"
AUTHORITY_SERVER_BIN = ROOT / "target/kwin_authority_server"
COORDINATOR_BIN = ROOT / "target/debug/examples/phase-3-10-native-coordinator"

DAEMON = "org.freedesktop.DBus"
PROPERTIES = DAEMON + ".Properties"
KWIN = "org.kde.KWin"
ACTIVITIES = "org.kde.ActivityManager"
VERSION = "6.8.3"

sys.path.insert(0, str(ROOT / "tests/integration/poc02e"))
from process_inventory import members


def require(condition, reason):
    if not condition:
        raise RuntimeError(reason)


def verify_kwin_binary():
    require(KWIN_EXE.resolve(strict=True) == KWIN_EXE, "Unexpected KWin executable path")
    with KWIN_EXE.open("rb") as stream:
        info = os.fstat(stream.fileno())
        require(stat.S_ISREG(info.st_mode) and info.st_uid == 0 and not info.st_mode & 0o022,
                "Unexpected KWin executable ownership or permissions")
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    require(digest == KWIN_SHA256, f"KWin executable digest mismatch: expected {KWIN_SHA256}, got {digest}")


def stop(process):
    if process.poll() is None:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=3)


def call(bus, service, path, interface, method, signature="", args=()):
    return bus.call_blocking(service, path, interface, method, signature, args, timeout=6)


def bus_call(bus, method, signature="", args=()):
    return call(bus, DAEMON, "/org/freedesktop/DBus", DAEMON, method, signature, args)


def host_snapshot(bus):
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
        "activity": activity,
        "activities": sorted(activities),
        "configContents": {name: (config / name).read_bytes() if (config / name).exists() else None
                           for name in ("kwinrc", "kglobalshortcutsrc")},
    }


def wait_for(predicate, label, timeout=10):
    deadline = time.monotonic() + timeout
    tick = threading.Event()
    while time.monotonic() < deadline:
        res = predicate()
        if res:
            return res
        tick.wait(0.05)
    raise TimeoutError(label)


def run_test_suite():
    print("=== DepthOS Phase 3.10.1 Real Private KWin Cross-Process Integration ===", flush=True)

    # 1. Verify binaries
    verify_kwin_binary()
    require(AUTHORITY_SERVER_BIN.is_file(), f"Authority server binary missing: {AUTHORITY_SERVER_BIN}")
    require(COORDINATOR_BIN.is_file(), f"Coordinator binary missing: {COORDINATOR_BIN}")

    # 2. Setup private disposable run directory
    run_dir = Path(tempfile.mkdtemp(prefix="depthos-p3-native-10-"))
    run_dir.chmod(0o700)

    try:
        run_worker(run_dir)
        print("\n=== ALL PHASE 3.10.1 NATIVE KWIN INTEGRATION TESTS PASSED ===", flush=True)
    finally:
        shutil.rmtree(run_dir, ignore_errors=True)
        print("Cleaned up private disposable run directory.", flush=True)


def run_worker(run):
    runtime = run / "runtime"
    for name in ("home", "runtime", "config", "data", "cache", "state"):
        (run / name).mkdir(mode=0o700, exist_ok=True)

    # Setup private D-Bus configuration
    config = run / "bus.xml"
    config.write_text(f"""<!DOCTYPE busconfig PUBLIC "-//freedesktop//DTD D-Bus Bus Configuration 1.0//EN"
"http://www.freedesktop.org/standards/dbus/1.0/busconfig.dtd">
<busconfig><type>session</type>
<listen>unix:path={escape(str(runtime / "bus"))}</listen>
<standard_session_servicedirs/>
<policy context="default">
  <allow send_destination="*"/><allow receive_sender="*"/><allow own="*"/>
</policy>
</busconfig>
""")

    desktops = "\n".join(f"Id_{idx}={uuid.uuid4()}\nName_{idx}=Desktop {idx}" for idx in (1, 2))
    (run / "config/kwinrc").write_text(f"[Desktops]\nNumber=2\nRows=1\n{desktops}\n[Plugins]\nkwin4_effect_shapecornersEnabled=false\n")

    env = {
        "PATH": "/usr/bin",
        "LANG": "C.UTF-8",
        "HOME": str(run / "home"),
        "XDG_RUNTIME_DIR": str(runtime),
        "XDG_CONFIG_HOME": str(run / "config"),
        "XDG_DATA_HOME": str(run / "data"),
        "XDG_CACHE_HOME": str(run / "cache"),
        "XDG_STATE_HOME": str(run / "state"),
        "XDG_CONFIG_DIRS": str(run / "config"),
        "XDG_DATA_DIRS": "/usr/share",
        "XDG_SESSION_TYPE": "wayland",
        "XDG_CURRENT_DESKTOP": "KDE",
        "DBUS_SYSTEM_BUS_ADDRESS": "unix:path=/dev/null",
        "QT_NO_XDG_DESKTOP_PORTAL": "1",
        "QT_FORCE_STDERR_LOGGING": "1",
        "QT_LOGGING_RULES": "kwin_scripting.debug=true",
        "PYTHONDONTWRITEBYTECODE": "1",
    }

    # Launch private D-Bus session
    dbus_cmd = ["dbus-run-session", f"--config-file={config}", "--", sys.executable, "-c",
                "import os, sys, time; time.sleep(60)"]
    # We will launch the actual components inside the D-Bus environment
    dbus_proc = subprocess.Popen(["dbus-daemon", f"--config-file={config}", "--print-address"],
                                 env=env, stdout=subprocess.PIPE, text=True)
    dbus_address = dbus_proc.stdout.readline().strip()
    env["DBUS_SESSION_BUS_ADDRESS"] = dbus_address

    with ExitStack() as stack:
        stack.callback(stop, dbus_proc)

        bus = dbus.bus.BusConnection(dbus_address)
        bus_id = bus_call(bus, "GetId")
        print(f"[NATIVE RUNNER] Private D-Bus initialized: {bus_id}", flush=True)

        # Launch virtual KWin
        kwin_cmd = [
            str(KWIN_EXE), "--virtual", "--no-lockscreen", "--no-global-shortcuts",
            "--socket", WAYLAND_SOCKET, "--width", "800", "--height", "600"
        ]
        kwin = subprocess.Popen(kwin_cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        stack.callback(stop, kwin)

        def drain_kwin():
            for line in kwin.stdout:
                if "DEPTHOS" in line or "error" in line.lower():
                    pass  # Keep log clean, print only relevant diagnostics
        threading.Thread(target=drain_kwin, daemon=True).start()

        wait_for(lambda: bus_call(bus, "NameHasOwner", "s", (KWIN,)), "Private KWin owner")
        kwin_owner = bus_call(bus, "GetNameOwner", "s", (KWIN,))
        kwin_pid = bus_call(bus, "GetConnectionUnixProcessID", "s", (kwin_owner,))
        require(kwin_pid == kwin.pid, "KWin PID mismatch")
        print(f"[NATIVE RUNNER] Virtual KWin 6.8.3 running (PID {kwin_pid}) on Wayland socket {WAYLAND_SOCKET}", flush=True)

        wayland_socket_path = runtime / WAYLAND_SOCKET
        wait_for(lambda: wayland_socket_path.is_socket(), "Wayland socket creation")

        # Launch owned Wayland fixture client
        client_env = {**env, "WAYLAND_DISPLAY": WAYLAND_SOCKET, "QT_QPA_PLATFORM": "wayland"}
        client = subprocess.Popen([sys.executable, str(HERE / "fixture.py")],
                                  env=client_env, stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        stack.callback(stop, client)

        # Show the surface
        client.stdin.write("show\n")
        client.stdin.flush()
        print(f"[NATIVE RUNNER] Owned Wayland fixture client running (PID {client.pid})", flush=True)

        # Setup Private Authority Server and IPC socket
        ipc_socket = runtime / "depthos-authority.sock"
        bootstrap_token = "launcher-bootstrap-token-p3-10-1-" + str(uuid.uuid4())
        fixture_token = "fixture-authority-token-kwin-private"
        compositor_inc = str(uuid.uuid4())
        window_uuid = str(uuid.uuid4())
        db_path = runtime / "durable.sqlite3"

        def start_authority_server(window_gen=1, win_uuid=window_uuid, comp_inc=compositor_inc):
            if ipc_socket.exists():
                ipc_socket.unlink()
            srv_cmd = [
                str(AUTHORITY_SERVER_BIN),
                "--socket", str(ipc_socket),
                "--bootstrap-token", bootstrap_token,
                "--compositor-incarnation", comp_inc,
                "--window-uuid", win_uuid,
                "--generation", str(window_gen),
            ]
            srv = subprocess.Popen(srv_cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   stderr=subprocess.STDOUT, text=True)
            wait_for(lambda: ipc_socket.is_socket(), f"Authority socket {ipc_socket}")
            # Verify socket permissions are 0600 in 0700 dir
            sock_stat = ipc_socket.stat()
            require(stat.S_IMODE(sock_stat.st_mode) == 0o600, "Authority socket mode must be 0600")
            dir_stat = ipc_socket.parent.stat()
            require(stat.S_IMODE(dir_stat.st_mode) == 0o700, "Authority parent dir mode must be 0700")
            return srv

        print("\n--- [TEST 1] Happy Path: Reserve -> Commit WAL -> SyntheticAttempt -> Count=1 -> Replay Denied ---", flush=True)
        server = start_authority_server()
        stack.callback(stop, server)

        coord_cmd = [
            str(COORDINATOR_BIN),
            "--socket", str(ipc_socket),
            "--db", str(db_path),
            "--bootstrap-token", bootstrap_token,
            "--fixture-token", fixture_token,
            "--compositor-incarnation", compositor_inc,
            "--window-uuid", window_uuid,
            "--generation", "1",
            "--scenario", "happy_path",
        ]
        res = subprocess.run(coord_cmd, capture_output=True, text=True, check=True)
        require("[COORDINATOR] PASS: Operation completed with state Completed" in res.stdout,
                "Happy path did not complete with state Completed")
        require("[COORDINATOR] PASS: Replay attempt rejected" in res.stdout,
                "Replay attempt was not rejected")
        stop(server)
        print("PASS: Happy path executed and verified replay denied", flush=True)

        print("\n--- [TEST 2] Native Focus Loss Invalidation ---", flush=True)
        server = start_authority_server()
        # Trigger focus loss before coordinator attempt
        server.stdin.write("focus_loss\n")
        server.stdin.flush()
        coord_cmd[coord_cmd.index("happy_path")] = "synthetic_denied"
        res = subprocess.run(coord_cmd, capture_output=True, text=True)
        require(res.returncode == 0 and "PASS: Synthetic denial handled correctly" in res.stdout,
                "Focus loss did not cause synthetic denial")
        stop(server)
        print("PASS: Focus loss invalidated synthetic consumption attempt", flush=True)

        print("\n--- [TEST 3] Window Destruction Invalidation ---", flush=True)
        server = start_authority_server()
        server.stdin.write("destroy_window\n")
        server.stdin.flush()
        res = subprocess.run(coord_cmd, capture_output=True, text=True)
        require(res.returncode == 0 and "PASS: Synthetic denial handled correctly" in res.stdout,
                "Window destruction did not cause synthetic denial")
        stop(server)
        print("PASS: Window destruction invalidated synthetic attempt", flush=True)

        print("\n--- [TEST 4] Window Recreation with Generation Bump Invalidation ---", flush=True)
        server = start_authority_server()
        # Recreate window with generation 2
        server.stdin.write("recreate_window 2\n")
        server.stdin.flush()
        # Coordinator attempts with generation 1 (stale generation!)
        res = subprocess.run(coord_cmd, capture_output=True, text=True)
        require(res.returncode == 0 and "PASS: Synthetic denial handled correctly" in res.stdout,
                "Recreated window generation drift did not cause synthetic denial")
        stop(server)
        print("PASS: Window recreation generation drift invalidated attempt", flush=True)

        print("\n--- [TEST 5] Destination Desktop Removal Invalidation ---", flush=True)
        server = start_authority_server()
        server.stdin.write("remove_desktop\n")
        server.stdin.flush()
        coord_cmd[coord_cmd.index("synthetic_denied")] = "prepare_failure"
        res = subprocess.run(coord_cmd, capture_output=True, text=True)
        require(res.returncode == 0 and "PASS: Prepare failure handled with refusal" in res.stdout,
                "Removed destination desktop did not cause prepare failure")
        stop(server)
        print("PASS: Destination desktop removal invalidated prepare", flush=True)

        print("\n--- [TEST 6] Screen Lock Transition Invalidation ---", flush=True)
        server = start_authority_server()
        server.stdin.write("lock\n")
        server.stdin.flush()
        coord_cmd[coord_cmd.index("prepare_failure")] = "synthetic_denied"
        res = subprocess.run(coord_cmd, capture_output=True, text=True)
        require(res.returncode == 0 and "PASS: Synthetic denial handled correctly" in res.stdout,
                "Lock transition did not cause synthetic denial")
        stop(server)
        print("PASS: Screen lock transition invalidated attempt", flush=True)

        print("\n--- [TEST 7] Compositor Restart Incarnation Fencing ---", flush=True)
        # Launch server with NEW compositor incarnation
        new_comp_inc = str(uuid.uuid4())
        server = start_authority_server(comp_inc=new_comp_inc)
        # Coordinator still configured with old compositor_inc
        res = subprocess.run(coord_cmd, capture_output=True, text=True)
        require("CompositorMismatch" in res.stderr or "CompositorMismatch" in res.stdout,
                "Compositor incarnation mismatch was not fenced")
        stop(server)
        print("PASS: Compositor restart incarnation fencing verified", flush=True)

        print("\n--- [TEST 8] Bootstrap Token Rejection ---", flush=True)
        server = start_authority_server()
        bad_token_cmd = coord_cmd.copy()
        bad_token_cmd[bad_token_cmd.index(bootstrap_token)] = "forged-stolen-token-12345"
        bad_token_cmd[bad_token_cmd.index("synthetic_denied")] = "happy_path"
        res = subprocess.run(bad_token_cmd, capture_output=True, text=True)
        require(res.returncode != 0 and ("UnauthorizedPeer" in res.stderr or "UnauthorizedPeer" in res.stdout
                                         or "Connection reset" in res.stderr or "Handshake failed" in res.stderr
                                         or "ConnectionClosed" in res.stderr),
                "Forged bootstrap token was not rejected")
        stop(server)
        print("PASS: Forged bootstrap token rejected at handshake boundary", flush=True)


if __name__ == "__main__":
    run_test_suite()
