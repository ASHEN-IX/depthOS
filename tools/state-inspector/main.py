#!/usr/bin/env python3
"""Foreground session-bus receiver and terminal view. Never starts a KWin script.

Requires dbus-python and PyGObject/GLib (already available on the audited host).
No sockets, network, title history, D-Bus metadata signals, or persistent service.
"""
import argparse
import json
import os
import signal
import shutil
import sys
import time
import uuid

import dbus
import dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib
from state import Replica, decode, redacted, safe_terminal

SERVICE = "org.depthos.StateInspector"
PATH = "/org/depthos/StateInspector"
INTERFACE = SERVICE + "1"


class Inspector(dbus.service.Object):
    def __init__(self, bus, name, args, loop):
        super().__init__(name, PATH)
        self.bus, self.args, self.loop = bus, args, loop
        self.replica = Replica()
        self.consumer_id = str(uuid.uuid4())
        self.last_packet = 0
        self.errors = 0

    def authorize(self, sender):
        try:
            owner = self.bus.get_name_owner("org.kde.KWin")
            uid = self.bus.get_unix_user(sender)
            if sender != owner or uid != os.getuid():
                raise ValueError()
        except Exception:
            raise dbus.exceptions.DBusException("Only this user's KWin may publish state",
                                                name=INTERFACE + ".AccessDenied") from None

    @dbus.service.method(INTERFACE, in_signature="s", out_signature="s", sender_keyword="sender")
    def Hello(self, session, sender=None):
        self.authorize(sender)
        if not session or len(session) > 256:
            raise dbus.exceptions.DBusException("Invalid session", name=INTERFACE + ".InvalidPacket")
        self.replica.reset()
        return json.dumps({"schemaVersion": 1, "consumerId": self.consumer_id})

    @dbus.service.method(INTERFACE, in_signature="s", out_signature="s", sender_keyword="sender")
    def Push(self, raw, sender=None):
        self.authorize(sender)
        try:
            answer = self.replica.accept(decode(str(raw)), self.consumer_id)
            self.last_packet = time.monotonic()
        except (ValueError, TypeError, KeyError, RecursionError):
            self.errors += 1
            self.replica.need_snapshot = True
            answer = {"consumerId": self.consumer_id, "needSnapshot": True}
        if self.args.snapshot and self.replica.state is not None and not self.replica.need_snapshot:
            state = self.replica.state if self.args.include_titles else redacted(self.replica.state)
            print(json.dumps({"schemaVersion": 1, "sessionId": self.replica.session_id,
                "sequence": self.replica.sequence, "revision": self.replica.revision,
                "state": state}, ensure_ascii=True, indent=2))
            GLib.idle_add(self.loop.quit)
        return json.dumps(answer)

    def render(self):
        replica, state = self.replica, self.replica.state
        lines = ["DepthOS — POC-02A native state inspector (Ctrl+C to stop)"]
        if state is None:
            lines.append("Observer status: " + self.replica.status if self.replica.status else
                         "Waiting for an explicitly loaded DepthOS observer. No script is loaded by this tool.")
        else:
            age = time.monotonic() - self.last_packet
            health = replica.status or ("RESYNC REQUIRED" if replica.need_snapshot else ("STALE / observer unavailable" if age > 5 else "connected"))
            lines += [f"{health} | revision {replica.revision} | sequence {replica.sequence} | packet age {age:.1f}s",
                "Desktop: " + safe_terminal(state.get("activeDesktopId")) + " | Activity: " + safe_terminal(state.get("activeActivityId")),
                "Last event: " + safe_terminal(json.dumps(replica.last_event)),
                "Screens: " + safe_terminal(json.dumps(state.get("screens")), 400), ""]
            selected = set(state.get("applicationWindowIds", []))
            for window in state["windows"]:
                if not self.args.all and window["runtimeId"] not in selected:
                    continue
                mark = "*" if window["runtimeId"] == state.get("activeWindowId") else " "
                title = "<redacted>" if self.args.redact_titles else window.get("title")
                lines += [f"{mark} {safe_terminal(window['runtimeId'])} [{safe_terminal(window.get('classification'))}] {safe_terminal(title)}",
                    "  app=" + safe_terminal(window.get("applicationId")) + " class=" + safe_terminal(window.get("resourceClass")) +
                    " desktops=" + safe_terminal(json.dumps(window.get("desktopIds"))) +
                    " activities=" + safe_terminal(json.dumps(window.get("activityIds"))) +
                    f" sticky={window.get('onAllDesktops')} allActivities={window.get('onAllActivities')} minimized={window.get('minimized')} fullscreen={window.get('fullscreen')}"]
        size = shutil.get_terminal_size(fallback=(100, 30))
        rows = max(1, size.lines - 1)
        columns = size.columns
        print("\x1b[H\x1b[2J" + "\n".join(line[:columns] for line in lines[:rows]), end="", flush=True)
        return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--snapshot", action="store_true", help="Print one snapshot (titles redacted by default), then exit")
    parser.add_argument("--include-titles", action="store_true", help="Explicitly include sensitive titles in --snapshot output")
    parser.add_argument("--redact-titles", action="store_true", help="Hide titles in the terminal view")
    parser.add_argument("--all", action="store_true", help="Show all API-visible windows, including system UI")
    args = parser.parse_args()
    if args.include_titles and not args.snapshot:
        parser.error("--include-titles is only for explicit --snapshot exports")
    if not args.snapshot and not sys.stdout.isatty():
        parser.error("interactive view needs a terminal; use --snapshot for redacted JSON")
    DBusGMainLoop(set_as_default=True)
    bus, loop = dbus.SessionBus(), GLib.MainLoop()
    try:
        name = dbus.service.BusName(SERVICE, bus, do_not_queue=True, allow_replacement=False)
    except dbus.exceptions.DBusException:
        parser.exit(1, "Another inspector owns the session-bus name; stop it first.\n")
    receiver = Inspector(bus, name, args, loop)
    for sig in (signal.SIGINT, signal.SIGTERM):
        GLib.unix_signal_add(GLib.PRIORITY_DEFAULT, sig, lambda: (loop.quit(), False)[1])
    if not args.snapshot:
        print("\x1b[?1049h\x1b[?25l", end="", flush=True)
        GLib.timeout_add(250, receiver.render)
    try:
        loop.run()
    finally:
        receiver.remove_from_connection()
        bus.release_name(SERVICE)
        if not args.snapshot:
            print("\x1b[H\x1b[2J\x1b[?25h\x1b[?1049l", end="", flush=True)


if __name__ == "__main__":
    main()
