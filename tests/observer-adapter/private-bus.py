#!/usr/bin/env python3
"""Cross-language D-Bus/CLI contract test; always re-execs on a fresh bus."""
import json
import os
from pathlib import Path
import subprocess
import sys
import time
ROOT=Path(__file__).resolve().parents[2]
if '--inner' not in sys.argv:
    build=subprocess.check_output(['cargo','build','--quiet','--locked','--manifest-path',str(ROOT/'crates/depthos-observer/Cargo.toml'),'--bin','depthos-observer','--message-format=json'],cwd=ROOT,text=True)
    exe=next(json.loads(line)['executable'] for line in build.splitlines() if json.loads(line).get('executable'))
    subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--inner',exe],check=True,cwd=ROOT)
    sys.exit(0)
import dbus
import dbus.service
bus=dbus.SessionBus()
assert sorted(str(n) for n in bus.list_names() if not str(n).startswith(':')) == ['org.freedesktop.DBus'], 'Refusing populated bus'
name=dbus.service.BusName('org.kde.KWin',bus,do_not_queue=True)
exe=sys.argv[-1]
process=subprocess.Popen([exe,'snapshot','--redact-titles'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
try:
    deadline=time.monotonic()+5
    while not bus.name_has_owner('org.depthos.StateInspector') and time.monotonic()<deadline: time.sleep(.02)
    proxy=dbus.Interface(bus.get_object('org.depthos.StateInspector','/org/depthos/StateInspector'),'org.depthos.StateInspector1')
    hello=json.loads(str(proxy.Hello('test-session',timeout=3)))
    frame=json.loads((ROOT/'tests/observer-adapter/trace.json').read_text())['frames'][0]
    packet={'schemaVersion':1,'sessionId':'test-session','consumerId':hello['consumerId'],'frames':[frame],
            'position':{key:frame[key] for key in ('sequence','revision')}}
    assert json.loads(str(proxy.Push(json.dumps(packet),timeout=3)))['needSnapshot'] is False
    out,err=process.communicate(timeout=5)
    assert process.returncode==0,err
    snapshot=json.loads(out)
    assert snapshot['state']['windows'][0]['title']=='<redacted>'
    assert 'Sanitized editor' not in out+err
    assert not bus.name_has_owner('org.depthos.StateInspector')
finally:
    if process.poll() is None:process.terminate();process.communicate(timeout=5)
process=subprocess.Popen([exe,'health'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
try:
    deadline=time.monotonic()+5
    while not bus.name_has_owner('org.depthos.StateInspector') and time.monotonic()<deadline:time.sleep(.02)
    process.terminate();out,err=process.communicate(timeout=5)
    assert process.returncode==0
    assert not bus.name_has_owner('org.depthos.StateInspector')
    assert 'Waiting' in err and not out
finally:
    if process.poll() is None:process.kill();process.communicate(timeout=5)
print('PASS dbus-run-session: Python Hello/Push → Rust CLI, redacted export, SIGTERM/name release')
