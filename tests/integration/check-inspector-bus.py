#!/usr/bin/env python3
"""Run ONLY under dbus-run-session; fake KWin sender, never the real compositor."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import dbus
import dbus.service

ROOT = Path(__file__).resolve().parents[2]
assert os.environ.get("DEPTHOS_PRIVATE_BUS_TEST") == "1", "Use the documented isolated-bus command"
bus = dbus.SessionBus()
well_known = [str(n) for n in bus.list_names() if not str(n).startswith(":")]
assert well_known == ["org.freedesktop.DBus"], "Refusing a populated/real desktop bus"
name = dbus.service.BusName("org.kde.KWin", bus, do_not_queue=True)
process = subprocess.Popen([sys.executable, str(ROOT / "tools/state-inspector/main.py"), "--snapshot"],
                           stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
try:
    deadline = time.monotonic() + 5
    while not bus.name_has_owner("org.depthos.StateInspector") and time.monotonic() < deadline:
        time.sleep(.02)
    proxy = bus.get_object("org.depthos.StateInspector", "/org/depthos/StateInspector")
    endpoint = dbus.Interface(proxy, "org.depthos.StateInspector1")
    other = dbus.bus.BusConnection(os.environ["DBUS_SESSION_BUS_ADDRESS"])
    unauthorized = dbus.Interface(other.get_object("org.depthos.StateInspector", "/org/depthos/StateInspector"), "org.depthos.StateInspector1")
    try:
        unauthorized.Hello("forbidden", timeout=3)
        raise AssertionError("unauthorized sender accepted")
    except dbus.exceptions.DBusException as error:
        assert str(error.get_dbus_name()).endswith(".AccessDenied")
    hello = json.loads(str(endpoint.Hello("test-session", timeout=3)))
    frame = json.loads(subprocess.check_output(["node", "-e",
        "const {fixture,window}=require('./tests/observer/fixtures.cjs');console.log(JSON.stringify(fixture([window({caption:'SENSITIVE TEST TITLE'})]).initial));"],
        cwd=ROOT, text=True))
    packet = {"schemaVersion": 1, "sessionId": "test-session", "consumerId": hello["consumerId"],
        "frames": [frame], "position": {"sequence": frame["sequence"], "revision": frame["revision"]}}
    reply = json.loads(str(endpoint.Push(json.dumps(packet), timeout=3)))
    assert not reply["needSnapshot"]
    output, errors = process.communicate(timeout=5)
    assert process.returncode == 0, errors
    snapshot = json.loads(output)
    assert snapshot["state"]["windows"][0]["title"] == "<redacted>"
    assert "SENSITIVE TEST TITLE" not in output
    print("Isolated D-Bus receiver: authorized delivery, wrong-sender rejection, redacted snapshot, clean exit PASS")
finally:
    if process.poll() is None:
        process.terminate()
        process.communicate(timeout=5)
