#!/usr/bin/env python3
"""Unified CLI contract tests; every D-Bus mutation is on dbus-run-session."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
ROOT=Path(__file__).resolve().parents[2]
if '--inner' not in sys.argv:
    build=subprocess.check_output(['cargo','build','--locked','--quiet','-p','depthos-core','--bin','depthos','--message-format=json'],cwd=ROOT,text=True)
    exe=next(json.loads(line)['executable'] for line in build.splitlines() if json.loads(line).get('executable'))
    subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--inner',exe],check=True,cwd=ROOT)
    sys.exit(0)
import dbus
import dbus.service
bus=dbus.SessionBus()
assert sorted(str(n) for n in bus.list_names() if not str(n).startswith(':'))==['org.freedesktop.DBus']
exe=sys.argv[-1]
SERVICE='org.depthos.StateInspector'
def wait_owned():
    deadline=time.monotonic()+5
    while not bus.name_has_owner(SERVICE) and time.monotonic()<deadline:time.sleep(.01)
    assert bus.name_has_owner(SERVICE)
def run(args,**kwargs):
    return subprocess.run([exe,*args],capture_output=True,text=True,timeout=8,**kwargs)
def publish():
    wait_owned()
    proxy=dbus.Interface(bus.get_object(SERVICE,'/org/depthos/StateInspector'),SERVICE+'1')
    hello=json.loads(str(proxy.Hello('test-session',timeout=3)))
    frame=json.loads((ROOT/'tests/observer-adapter/trace.json').read_text())['frames'][0]
    packet={'schemaVersion':1,'sessionId':'test-session','consumerId':hello['consumerId'],'frames':[frame],
            'position':{k:frame[k] for k in ('sequence','revision')}}
    assert not json.loads(str(proxy.Push(json.dumps(packet),timeout=3)))['needSnapshot']
with tempfile.TemporaryDirectory(prefix='depthos-unified-cli-') as directory:
    db=Path(directory)/'never-opened.db'
    common=['--db',str(db),'system']
    # Unavailable endpoint must not block storage-free diagnostics or touch --db.
    env=dict(os.environ,DBUS_SESSION_BUS_ADDRESS='unix:path=/nonexistent/depthos-integration')
    for command in ['health','inspect','snapshot']:
        result=run([*common,command,'--timeout-seconds','1'],env=env)
        assert result.returncode==0,result.stderr
        assert json.loads(result.stdout)['health']=='Disconnected'
        assert not db.exists()
    result=run([*common,'health','--timeout-seconds','1'])
    assert result.returncode==0 and json.loads(result.stdout)['snapshotAvailable'] is False
    assert not bus.name_has_owner(SERVICE) and not db.exists()
    # The real, unchanged Python inspector holds its name throughout collision.
    python=subprocess.Popen([sys.executable,str(ROOT/'tools/state-inspector/main.py'),'--snapshot'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=dict(os.environ,PYTHONDONTWRITEBYTECODE='1'))
    try:
        wait_owned();owner=bus.get_name_owner(SERVICE)
        result=run([*common,'snapshot','--timeout-seconds','1'])
        assert result.returncode==1 and 'Another receiver owns' in result.stderr
        assert bus.get_name_owner(SERVICE)==owner and python.poll() is None
    finally:
        python.terminate();python.communicate(timeout=5)
    assert not bus.name_has_owner(SERVICE)
    kwin=dbus.service.BusName('org.kde.KWin',bus,do_not_queue=True)
    for command in ['snapshot','health','inspect']:
        process=subprocess.Popen([exe,*common,command,'--timeout-seconds','1'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        try:
            publish();out,err=process.communicate(timeout=5)
            assert process.returncode==0,err
            records=[];remaining=out.strip()
            while remaining:
                record,end=json.JSONDecoder().raw_decode(remaining);records.append(record);remaining=remaining[end:].lstrip()
            if command=='snapshot':assert records[0]['state']['windows'][0]['title']=='<redacted>'
            else:assert any(r['health']=='Synchronized' for r in records)
            assert 'Sanitized editor' not in out+err
            assert not bus.name_has_owner(SERVICE) and not db.exists()
        finally:
            if process.poll() is None:process.terminate();process.communicate(timeout=5)
    process=subprocess.Popen([exe,*common,'inspect','--timeout-seconds','30'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    try:
        wait_owned();process.terminate();out,err=process.communicate(timeout=5)
        assert process.returncode==0 and not bus.name_has_owner(SERVICE)
    finally:
        if process.poll() is None:process.kill();process.communicate(timeout=5)
print('PASS unified CLI: unavailable bus/KWin, real Python collision preserved, redaction, verified health, timeout/SIGTERM cleanup, zero SQLite creation')
