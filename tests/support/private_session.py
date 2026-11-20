"""Contain the private KDE regression processes. Linux qualification only."""
import ctypes
import errno
import json
import os
from pathlib import Path
import platform
import re
import signal
import shutil
import socket
import stat
import subprocess
import sys
import time
import xml.sax.saxutils

DIRECTORIES = {
    "HOME": "home", "XDG_CONFIG_HOME": "config", "XDG_DATA_HOME": "data",
    "XDG_CACHE_HOME": "cache", "XDG_STATE_HOME": "state", "XDG_RUNTIME_DIR": "runtime",
    "XDG_CONFIG_DIRS": "config-system", "TMPDIR": "tmp",
}
SOCKET = "wayland-d"
DAEMON = "org.freedesktop.DBus"
KWIN = "org.kde.KWin"
LIBC = ctypes.CDLL(None, use_errno=True)
# Linux generic syscall numbers are shared by the supported architectures.
CREATE_RULESET, ADD_RULE, RESTRICT_SELF = 444, 445, 446
RESOLVE_UNIX = 1 << 16
WRITE_FILE = 1 << 1
FS_MUTATION = WRITE_FILE | sum(1 << bit for bit in range(4, 15))
LANDLOCK_SCOPES = 3  # Abstract UNIX sockets and signals to other domains.


class IsolationError(RuntimeError):
    pass


def check(condition, message):
    if not condition:
        raise IsolationError(message)


def syscall(result, label):
    if result < 0:
        code = ctypes.get_errno()
        raise IsolationError(f"{label}: {os.strerror(code)}")
    return result


def filesystem_boundary(root):
    check(platform.machine() in ("x86_64", "aarch64", "riscv64"), "Unsupported Landlock syscall ABI")
    abi = syscall(LIBC.syscall(CREATE_RULESET, None, 0, 1), "Landlock ABI query")
    # Older kernels may lack pathname socket mediation. Require both rule creation
    # and a live external-socket denial probe before starting a compositor.
    class PathRule(ctypes.Structure):
        _layout_ = "ms"
        _pack_ = 1
        _fields_ = [("rights", ctypes.c_uint64), ("parent", ctypes.c_int)]

    rights = FS_MUTATION | RESOLVE_UNIX
    ruleset = (ctypes.c_uint64 * 3)(rights, 0, LANDLOCK_SCOPES)
    fd = syscall(LIBC.syscall(CREATE_RULESET, ctypes.byref(ruleset), ctypes.sizeof(ruleset), 0),
                 "Landlock filesystem, pathname IPC and signal rules")
    try:
        devices = [Path("/dev/null")]
        devices.extend(path for path in Path("/dev/dri").glob("renderD[0-9]*")
                       if re.fullmatch(r"renderD[0-9]+", path.name))
        for path, access in [(root, FS_MUTATION), *((node, WRITE_FILE) for node in devices)]:
            check(path == root or stat.S_ISCHR(path.lstat().st_mode), "Unexpected test device type")
            path_fd = os.open(path, os.O_PATH | os.O_NOFOLLOW | os.O_CLOEXEC)
            try:
                rule = PathRule(access, path_fd)
                syscall(LIBC.syscall(ADD_RULE, fd, 1, ctypes.byref(rule), 0), "Landlock path rule")
            finally:
                os.close(path_fd)
        syscall(LIBC.prctl(38, 1, 0, 0, 0), "no_new_privs")
        syscall(LIBC.syscall(RESTRICT_SELF, fd, 0), "Landlock restrict self")
    finally:
        os.close(fd)
    return abi


def private_directory(path):
    info = path.lstat()
    check(path.is_absolute() and path.resolve() == path, "Private directory traverses a symlink")
    check(stat.S_ISDIR(info.st_mode) and info.st_uid == os.getuid() and
          stat.S_IMODE(info.st_mode) == 0o700, "Private directory ownership or mode is invalid")


def socket_identity(path):
    info = path.lstat()
    check(path.resolve() == path and stat.S_ISSOCK(info.st_mode) and info.st_uid == os.getuid(),
          "Invalid private socket type, owner, or path")
    return [info.st_dev, info.st_ino]


def start_ticks(pid):
    return int(Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19])


def environment(root, wayland):
    env = {key: str(root / suffix) for key, suffix in DIRECTORIES.items()}
    env.update(PATH="/usr/bin:/bin", LANG="C.UTF-8", XDG_DATA_DIRS="/usr/share",
               DBUS_SYSTEM_BUS_ADDRESS=f"unix:path={root}/runtime/no-system-bus",
               QT_NO_XDG_DESKTOP_PORTAL="1", QT_FORCE_STDERR_LOGGING="1",
               PYTHONDONTWRITEBYTECODE="1",
               QT_QPA_PLATFORM="wayland" if wayland else "offscreen")
    if wayland:
        env.update(WAYLAND_DISPLAY=SOCKET, XDG_SESSION_TYPE="wayland",
                   KWIN_XKB_DEFAULT_KEYMAP="1", LIBGL_ALWAYS_SOFTWARE="1")
    else:
        env["QT_QUICK_BACKEND"] = "software"
    return env


def mark(root, name, **details):
    """Write bounded, explicitly selected evidence for the outer monitor."""
    path = root / "stages.jsonl"
    entry = {"time_ns": time.time_ns(), "pid": os.getpid(), "stage": name, **details}
    with path.open("a") as log:
        log.write(json.dumps(entry) + "\n")
    if os.environ.get("DEPTHOS_OBSERVE") == "1":
        end = time.monotonic() + 3
        while time.monotonic() < end:
            try:
                if int((root / "stage-ack").read_text()) >= entry["time_ns"]:
                    return
            except (FileNotFoundError, ValueError):
                pass
            time.sleep(0.005)
        raise IsolationError("Host monitor did not acknowledge the scenario checkpoint")


def guard(root):
    """Validate the inherited session endpoints before importing Qt or touching KDE."""
    private_directory(root)
    for suffix in DIRECTORIES.values():
        private_directory(root / suffix)
    expected = environment(root, os.environ.get("QT_QPA_PLATFORM") == "wayland")
    for key, value in expected.items():
        check(os.environ.get(key) == value, f"Invalid private environment value for {key}")
    unexpected = set(os.environ) - set(expected) - {
        "DBUS_SESSION_BUS_ADDRESS", "DEPTHOS_OBSERVE",
    }
    check(not unexpected, "Unexpected inherited environment variables")
    bus_path = root / "runtime/bus"
    check(os.environ.get("DBUS_SESSION_BUS_ADDRESS") == f"unix:path={bus_path}",
          "Private D-Bus address required")
    manifest = json.loads((root / "session.json").read_text())
    check(socket_identity(bus_path) == manifest["bus_socket"], "Private D-Bus inode changed")
    check(start_ticks(manifest["bus_pid"]) == manifest["bus_start"], "Private D-Bus process changed")
    check(LIBC.prctl(39, 0, 0, 0, 0) == 1, "Private session requires no_new_privs")
    import dbus

    bus = dbus.bus.BusConnection(f"unix:path={bus_path}")
    try:
        bus_id = str(bus.call_blocking(DAEMON, "/org/freedesktop/DBus", DAEMON, "GetId", "", ()))
        check(re.fullmatch(r"[0-9a-f]{32}", bus_id) and bus_id == manifest["bus_id"],
              "Private D-Bus daemon identity changed")
        pid = int(bus.call_blocking(DAEMON, "/org/freedesktop/DBus", DAEMON,
                                    "GetConnectionUnixProcessID", "s", (DAEMON,)))
        check(pid == manifest["bus_pid"], "The connected D-Bus daemon is not the owned process")
        return bus
    except BaseException:
        bus.close()
        raise


class PrivateKWin:
    def __init__(self, root, bus, process):
        self.root, self.bus, self.process = root, bus, process
        self.socket = root / "runtime" / SOCKET
        self.start = start_ticks(process.pid)
        self.inode = socket_identity(self.socket)
        self.owner = str(bus.get_name_owner(KWIN))
        self.verify()

    def verify(self):
        check(self.process.poll() is None and start_ticks(self.process.pid) == self.start,
              "Private compositor has exited")
        check(str(self.bus.get_name_owner(KWIN)) == self.owner,
              "Private compositor D-Bus owner changed")
        pid = int(self.bus.call_blocking(DAEMON, "/org/freedesktop/DBus", DAEMON,
                                         "GetConnectionUnixProcessID", "s", (self.owner,)))
        check(pid == self.process.pid and socket_identity(self.socket) == self.inode,
              "KWin PID or private Wayland socket changed")

    def get_object(self, path):
        self.verify()
        return self.bus.get_object(self.owner, path)

    def call(self, path, interface, method, signature, args):
        self.verify()
        return self.bus.call_blocking(self.owner, path, interface, method, signature, args, timeout=5)

    def report(self):
        self.verify()
        return {"kwin_pid": self.process.pid, "kwin_start": self.start,
                "kwin_owner": self.owner, "wayland_socket": str(self.socket),
                "wayland_inode": self.inode}


def observer_package(root, action, source=None, install_root=None):
    # The harness must explicitly authorize one exact package root.
    check(install_root == root / "data/kwin/scripts", "Explicit private observer installation root required")
    private_directory(root / "data")
    check(install_root.resolve() == install_root, "Observer installation root traverses a symlink")
    package_id = "depthos-system-observer"
    target = install_root / package_id
    check(target.resolve() == target, "Observer package path traverses a symlink")
    check(action in ("install", "remove"), "Invalid observer package operation")
    check(action != "install" or (source is not None and source.is_absolute() and source.is_dir()),
          "Explicit observer source required")
    args = ("--install", str(source)) if action == "install" else ("--remove", package_id)
    subprocess.run(["/usr/bin/kpackagetool6", "--type", "KWin/Script",
                    "--packageroot", str(install_root), *args], check=True, timeout=10)
    check(target.is_dir() if action == "install" else not target.exists(), "Observer package state disagrees")
    return target / "contents/code/main.js"


def boundary_probe(root):
    sentinel = root.parent / (root.name + ".outside")
    try:
        with (sentinel / "marker").open("a"):
            pass
    except PermissionError:
        pass
    else:
        raise IsolationError("Outside-root write was not denied")
    address = str(sentinel / "socket")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        try:
            peer.connect(address)
        except PermissionError:
            pass
        else:
            raise IsolationError("Outside-domain pathname socket was not denied")
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as peer:
        try:
            peer.connect("\0" + root.name)
        except OSError as error:
            check(error.errno in (errno.EPERM, errno.EACCES), "Abstract socket denial was not enforced")
        else:
            raise IsolationError("Outside-domain abstract socket was not denied")


def child_pids():
    """Subreaper descendants, including processes which made a new session."""
    table = {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            state, parent, *_ = (entry / "stat").read_text().rsplit(")", 1)[1].split()
            if state != "Z":
                table.setdefault(int(parent), []).append(int(entry.name))
        except FileNotFoundError:
            continue
        except PermissionError:
            continue
    pending = [os.getpid()]
    result = []
    while pending:
        found = table.get(pending.pop(), [])
        pending.extend(found)
        result.extend(found)
    return result


def cleanup(children):
    # pidfds avoid PID-reuse races. Recheck ancestry after opening the handle.
    for sig in (signal.SIGTERM, signal.SIGKILL):
        for pid in reversed(child_pids()):
            try:
                fd = os.pidfd_open(pid)
                try:
                    if pid in child_pids():
                        signal.pidfd_send_signal(fd, sig)
                finally:
                    os.close(fd)
            except ProcessLookupError:
                continue
        end = time.monotonic() + 2
        while time.monotonic() < end:
            for child in children:
                child.poll()
            while True:
                try:
                    if os.waitpid(-1, os.WNOHANG)[0] == 0:
                        break
                except ChildProcessError:
                    break
            if not child_pids():
                return
            time.sleep(0.025)
    check(not child_pids(), "Owned process cleanup incomplete")


def supervise(root, argv):
    os.umask(0o077)
    private_directory(root)
    for suffix in DIRECTORIES.values():
        private_directory(root / suffix)
    for name in ("bus", SOCKET, SOCKET + ".lock"):
        check(not (root / "runtime" / name).is_symlink() and
              not (root / "runtime" / name).exists(), "Stale session socket or lock")
    expected = environment(root, os.environ.get("QT_QPA_PLATFORM") == "wayland")
    check(os.environ == expected | ({"DEPTHOS_OBSERVE": "1"} if os.environ.get("DEPTHOS_OBSERVE") == "1" else {}),
          "Unexpected launch environment")
    abi = filesystem_boundary(root)
    boundary_probe(root)
    syscall(LIBC.prctl(36, 1, 0, 0, 0), "Child subreaper")
    child_env = os.environ.copy()
    children = []

    def interrupted(_signum, _frame):
        raise InterruptedError("Private session interrupted")

    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGINT, interrupted)
    try:
        mark(root, "session:before", landlock_abi=abi)
        path = root / "runtime/bus"
        config = root / "bus.xml"
        config.write_text(
            '<busconfig><type>session</type><listen>unix:path=' +
            xml.sax.saxutils.escape(str(path)) + '</listen><auth>EXTERNAL</auth>'
            '<policy context="default"><allow send_destination="*"/><allow receive_sender="*"/>'
            '<allow own="*"/></policy></busconfig>')
        daemon = subprocess.Popen(["/usr/bin/dbus-daemon", "--nofork", "--print-address=1",
                                   f"--config-file={config}"], stdout=subprocess.PIPE, text=True)
        children.append(daemon)
        import select
        check(select.select([daemon.stdout], [], [], 5)[0], "Private D-Bus startup timed out")
        address = daemon.stdout.readline().strip()
        check(address.split(",guid=", 1)[0] == f"unix:path={path}", "Unexpected private D-Bus address")
        child_env["DBUS_SESSION_BUS_ADDRESS"] = f"unix:path={path}"
        import dbus

        bus = dbus.bus.BusConnection(child_env["DBUS_SESSION_BUS_ADDRESS"])
        try:
            bus_id = str(bus.call_blocking(DAEMON, "/org/freedesktop/DBus", DAEMON, "GetId", "", ()))
        finally:
            bus.close()
        manifest = {"bus_id": bus_id, "bus_pid": daemon.pid,
                    "bus_start": start_ticks(daemon.pid), "bus_socket": socket_identity(path)}
        (root / "session.json").write_text(json.dumps(manifest) + "\n")
        mark(root, "session:during", **manifest, address=child_env["DBUS_SESSION_BUS_ADDRESS"],
             paths={key: child_env[key] for key in DIRECTORIES},
             qt_platform=child_env["QT_QPA_PLATFORM"])
        worker = subprocess.Popen(argv, env=child_env, cwd=root)
        children.append(worker)
        return worker.wait(timeout=110)
    except InterruptedError:
        mark(root, "interruption:requested")
        return 130
    finally:
        signal.signal(signal.SIGTERM, signal.SIG_IGN)
        signal.signal(signal.SIGINT, signal.SIG_IGN)
        try:
            mark(root, "cleanup:before", owned_pids=child_pids())
        finally:
            cleanup(children)
            package = root / "data/kwin/scripts/depthos-system-observer"
            if package.is_dir() and package.resolve() == package:
                shutil.rmtree(package)
            for name in ("bus", SOCKET, SOCKET + ".lock"):
                (root / "runtime" / name).unlink(missing_ok=True)
            mark(root, "cleanup:after", owned_pids=child_pids())


def launch(root, argv, wayland, monitor=None, interrupt_at=None, prepare=None):
    os.umask(0o077)
    private_directory(root)
    for suffix in DIRECTORIES.values():
        (root / suffix).mkdir(mode=0o700)
    env = environment(root, wayland)
    if monitor:
        env["DEPTHOS_OBSERVE"] = "1"
    if prepare:
        prepare(root, env)
    sentinel = root.parent / (root.name + ".outside")
    sentinel.mkdir(mode=0o700)
    try:
        (sentinel / "marker").write_text("outside-domain test fixture\n")
        with (socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as external,
              socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as abstract):
            external.bind(str(sentinel / "socket"))
            external.listen()
            abstract.bind("\0" + root.name)
            abstract.listen()
            process = subprocess.Popen([sys.executable, __file__, str(root), *argv],
                                       env=env, cwd=root, stdin=subprocess.DEVNULL, start_new_session=True)
            try:
                return wait_supervisor(root, process, monitor, interrupt_at)
            finally:
                if process.poll() is None:
                    process.terminate()
                    wait_supervisor(root, process, monitor, None)
    finally:
        for child in sentinel.iterdir():
            child.unlink()
        sentinel.rmdir()


def wait_supervisor(root, process, monitor, interrupt_at):
    cursor = 0
    interrupted = False
    end = time.monotonic() + 125
    while process.poll() is None:
        if time.monotonic() >= end:
            process.terminate()
            raise IsolationError("Private session supervisor timed out")
        if monitor:
            stages = root / "stages.jsonl"
            if stages.exists():
                with stages.open() as log:
                    log.seek(cursor)
                    for line in log:
                        if not line.endswith("\n"):
                            break
                        entry = json.loads(line)
                        cursor += len(line)
                        monitor.checkpoint(entry)
                        (root / "stage-ack").write_text(str(entry["time_ns"]))
                        if interrupt_at == entry["stage"] and not interrupted:
                            interrupted = True
                            process.send_signal(signal.SIGTERM)
        time.sleep(0.015)
    return process.returncode


if __name__ == "__main__":
    try:
        raise SystemExit(supervise(Path(sys.argv[1]), sys.argv[2:]))
    except (IsolationError, OSError, subprocess.TimeoutExpired) as error:
        sys.exit(f"Private session refused: {error}")
