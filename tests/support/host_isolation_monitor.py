"""Read-only, unprivileged monitor for host KDE configuration."""
import ctypes
import hashlib
import json
import os
from pathlib import Path
import stat
import struct
import time

from private_session import LIBC, IsolationError, start_ticks, syscall

FILES = ("kwinrc", "kglobalshortcutsrc", "kdeglobals")
MASKS = {
    0x0002: "modify", 0x0004: "attrib", 0x0008: "close-write",
    0x0040: "moved-from", 0x0080: "moved-to", 0x0100: "create",
    0x0200: "delete", 0x0400: "delete-self", 0x0800: "move-self",
    0x4000: "queue-overflow", 0x8000: "watch-ignored",
}
WATCH = sum(MASKS) & ~(0x4000 | 0x8000)
EVENT = struct.Struct("iIII")


def file_evidence(path):
    try:
        fd = os.open(path, os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW | os.O_NOATIME)
    except FileNotFoundError:
        return {"exists": False}
    with os.fdopen(fd, "rb") as stream:
        before = os.fstat(fd)
        if not stat.S_ISREG(before.st_mode):
            raise IsolationError("Host monitor entry is not a regular file")
        sha = hashlib.file_digest(stream, "sha256").hexdigest()
        after = path.lstat()
        metadata = ("st_size", "st_dev", "st_ino", "st_uid", "st_gid",
                    "st_mode", "st_mtime_ns", "st_ctime_ns", "st_nlink")
        info = {field.removeprefix("st_"): getattr(before, field) for field in metadata}
        stable = all(info[field.removeprefix("st_")] == getattr(after, field) for field in metadata)
        return {"exists": True, "sha256": sha, "stable_read": stable, **info}


def processes():
    result = []
    for entry in Path("/proc").iterdir():
        if not entry.name.isdecimal():
            continue
        try:
            name = (entry / "comm").read_text().strip()
            if name.startswith(("kwin_wayland", "kglobalaccel", "kactivitymanag", "plasmashell", "kded")):
                pid = int(entry.name)
                result.append({"pid": pid, "start_ticks": start_ticks(pid), "comm": name})
        except (FileNotFoundError, ProcessLookupError, PermissionError):
            continue
    return sorted(result, key=lambda process: process["pid"])


class HostMonitor:
    def __init__(self, output, directories=None):
        active = os.environ.get("XDG_CONFIG_HOME")
        candidates = [Path.home() / ".config"]
        if active and Path(active).is_absolute():
            candidates.append(Path(active))
        self.directories = sorted({path.resolve(strict=True) for path in (directories or candidates)})
        self.output = output
        self.fd = None
        self.watches = {}
        self.events = []
        self.changed_snapshots = 0
        self.checkpoints = 0

    def __enter__(self):
        self.fd = syscall(LIBC.inotify_init1(os.O_CLOEXEC | os.O_NONBLOCK), "inotify_init1")
        try:
            for directory in self.directories:
                watch = syscall(LIBC.inotify_add_watch(self.fd, os.fsencode(directory), WATCH),
                                "inotify_add_watch")
                self.watches[watch] = directory
            self.baseline = self.snapshot()
            self.process_baseline = processes()
            self.checkpoint({"stage": "host:baseline"})
            return self
        except BaseException:
            os.close(self.fd)
            self.fd = None
            raise

    def __exit__(self, *_):
        try:
            self.checkpoint({"stage": "host:final"})
        finally:
            os.close(self.fd)
            self.fd = None

    def snapshot(self):
        return {str(directory / leaf): file_evidence(directory / leaf)
                for directory in self.directories for leaf in FILES}

    def drain(self):
        records = []
        while True:
            try:
                data = os.read(self.fd, 64 * 1024)
            except BlockingIOError:
                return records
            offset = 0
            while offset < len(data):
                wd, mask, cookie, size = EVENT.unpack_from(data, offset)
                start = offset + EVENT.size
                name = os.fsdecode(data[start:start + size].split(b"\0", 1)[0])
                offset = start + size
                # Do not retain unrelated user filenames.
                if name not in FILES and not mask & (0x0400 | 0x0800 | 0x4000 | 0x8000):
                    continue
                records.append({"received_ns": time.time_ns(), "cookie": cookie,
                                "watched_dir": str(self.watches.get(wd, "")),
                                "name": name, "events": [label for bit, label in MASKS.items() if mask & bit],
                                "writer": "unknown"})

    def checkpoint(self, entry):
        events = self.drain()
        try:
            files = self.snapshot()
        except (OSError, IsolationError) as error:
            files = {"sampling_error": str(error)}
        events += self.drain()
        self.events.extend(events)
        if files != self.baseline:
            self.changed_snapshots += 1
        self.checkpoints += 1
        with self.output.open("a") as log:
            log.write(json.dumps({"observed_ns": time.time_ns(), "checkpoint": entry,
                                  "host_files": files, "filesystem_events": events,
                                  "processes": processes()}) + "\n")

    def require_clean(self):
        live = processes()
        original_kwin = [row for row in self.process_baseline if row["comm"] == "kwin_wayland"]
        if self.events or self.changed_snapshots or any(row not in live for row in original_kwin):
            raise IsolationError("Host observations changed or monitoring coverage was interrupted")
