#!/usr/bin/env python3
"""CLI process recovery + /proc measurements; always a fresh dbus-run-session."""
import hashlib
import json
import os
from pathlib import Path
import queue
import signal
import subprocess
import sys
import tempfile
import threading
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
SERVICE='org.depthos.StateInspector'
exe=sys.argv[-1]
class Child:
    def __init__(self,path,env=None):
        self.started=time.monotonic()
        self.process=subprocess.Popen([exe,'--db',str(path),'controller','run'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,env=env)
        self.statuses=queue.Queue(maxsize=32);self.records=[]
        def read():
            pending=''
            for line in self.process.stdout:
                pending+=line
                try:record=json.loads(pending)
                except json.JSONDecodeError:continue
                pending='';self.records.append(record)
                try:self.statuses.put_nowait(record)
                except queue.Full:self.statuses.get_nowait();self.statuses.put_nowait(record)
        self.reader=threading.Thread(target=read,daemon=True);self.reader.start()
    def until(self,predicate,seconds=8):
        deadline=time.monotonic()+seconds
        while time.monotonic()<deadline:
            status=self.statuses.get(timeout=max(.01,deadline-time.monotonic()))
            if predicate(status):return status
        raise AssertionError('status deadline exceeded')
    def stop(self,force=False):
        if self.process.poll() is None:
            self.process.send_signal(signal.SIGKILL if force else signal.SIGTERM)
        self.process.wait(timeout=8);self.reader.join(timeout=2)
        assert not self.reader.is_alive()
        errors=self.process.stderr.read();self.process.stdout.close();self.process.stderr.close()
        assert 'Sanitized' not in json.dumps(self.records)+errors
        return errors
    def cleanup(self):
        if self.process.poll() is None:self.process.kill();self.process.wait(timeout=5)
def owner_gone():
    deadline=time.monotonic()+3
    while bus.name_has_owner(SERVICE) and time.monotonic()<deadline:time.sleep(.01)
    assert not bus.name_has_owner(SERVICE)
def publish(frame):
    proxy=dbus.Interface(bus.get_object(SERVICE,'/org/depthos/StateInspector'),SERVICE+'1')
    hello=json.loads(str(proxy.Hello('test-session',timeout=3)))
    packet={'schemaVersion':1,'sessionId':'test-session','consumerId':hello['consumerId'],'frames':[frame],
            'position':{k:frame[k] for k in ('sequence','revision')}}
    assert not json.loads(str(proxy.Push(json.dumps(packet),timeout=5)))['needSnapshot']
    return hello['consumerId']
def cpu(pid):
    values=Path(f'/proc/{pid}/stat').read_text().split()
    return (int(values[13])+int(values[14]))/os.sysconf('SC_CLK_TCK')
def memory(pid):
    rows={line.split(':')[0]:line.split(':')[1].strip() for line in Path(f'/proc/{pid}/status').read_text().splitlines() if ':' in line}
    return {key:int(rows[key].split()[0]) for key in ('VmRSS','VmHWM')}
with tempfile.TemporaryDirectory(prefix='depthos-lifecycle-') as directory:
    directory=Path(directory);path=directory/'db'
    child=Child(path,dict(os.environ,DBUS_SESSION_BUS_ADDRESS='unix:path=/nonexistent/depthos-cli-recovery'))
    try:
        child.until(lambda h:h['reason']=='RetryExhausted');child.stop();assert child.process.returncode==0
        assert child.records[-1]['lifecycle']=='Stopped'
    finally:child.cleanup()
    # Actual Python receiver is not displaced or stopped by the controller.
    inspector=subprocess.Popen([sys.executable,'-B',str(ROOT/'tools/state-inspector/main.py'),'--snapshot'],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
    try:
        deadline=time.monotonic()+3
        while not bus.name_has_owner(SERVICE) and time.monotonic()<deadline:time.sleep(.01)
        owner=bus.get_name_owner(SERVICE)
        result=subprocess.run([exe,'--db',str(path),'controller','run','--timeout-seconds','2'],capture_output=True,text=True,timeout=5)
        assert result.returncode==1 and 'Another receiver owns' in result.stderr
        assert bus.get_name_owner(SERVICE)==owner and inspector.poll() is None
    finally:inspector.terminate();inspector.communicate(timeout=5)
    owner_gone()
    child=Child(path)
    try:
        child.until(lambda h:h['lifecycle']=='WaitingForObserver');startup=(time.monotonic()-child.started)*1000
        before=hashlib.sha256(path.read_bytes()).hexdigest();empty_memory=memory(child.process.pid)
        publisher=dbus.service.BusName('org.kde.KWin',bus,do_not_queue=True)
        frame=json.loads((ROOT/'tests/observer-adapter/trace.json').read_text())['frames'][0]
        template=frame['state']['windows'][0]
        frame['state']['windows']=[dict(template,runtimeId=f'window-{n}') for n in range(1000)]
        frame['state']['applicationWindowIds']=[f'window-{n}' for n in range(1000)]
        frame['state']['activeWindowId']=None
        start=time.monotonic();consumer=publish(frame);synced=child.until(lambda h:h['lifecycle']=='Synchronized');sync=(time.monotonic()-start)*1000
        pid=child.process.pid;ticks=cpu(pid);start=time.monotonic();time.sleep(1)
        proxy=dbus.Interface(bus.get_object(SERVICE,'/org/depthos/StateInspector'),SERVICE+'1')
        heartbeat={'schemaVersion':1,'sessionId':'test-session','consumerId':consumer,'frames':[],
                   'position':{k:frame[k] for k in ('sequence','revision')}}
        assert not json.loads(str(proxy.Push(json.dumps(heartbeat),timeout=3)))['needSnapshot']
        time.sleep(1);elapsed=time.monotonic()-start
        idle=100*(cpu(pid)-ticks)/elapsed;loaded_memory=memory(pid)
        proxy=dbus.Interface(bus.get_object(SERVICE,'/org/depthos/StateInspector'),SERVICE+'1')
        start=time.monotonic();hello=json.loads(str(proxy.Hello('test-session',timeout=3)))
        child.until(lambda h:h['lifecycle']=='Recovering')
        packet={'schemaVersion':1,'sessionId':'test-session','consumerId':hello['consumerId'],'frames':[frame],'position':{k:frame[k] for k in ('sequence','revision')}}
        assert not json.loads(str(proxy.Push(json.dumps(packet),timeout=5)))['needSnapshot']
        child.until(lambda h:h['lifecycle']=='Synchronized');recovery=(time.monotonic()-start)*1000
        assert Path(f'/proc/{pid}/task/{pid}/children').read_text().strip()==''
        child.stop();assert child.process.returncode==0 and child.records[-1]['lifecycle']=='Stopped';owner_gone()
        assert hashlib.sha256(path.read_bytes()).hexdigest()==before
        measurements={'fixture':'private-bus CLI, 1000 windows','startupToWaitingMs':startup,'initialSyncMs':sync,'idleSampleSeconds':elapsed,'idleHeartbeats':1,'idleCpuPercentOneCore':idle,'emptyMemoryKiB':empty_memory,'loadedMemoryKiB':loaded_memory,'rehelloRecoveryMs':recovery}
    finally:child.cleanup()
    # SIGKILL cannot run cleanup, but OS connection teardown releases the name.
    first_epoch=None
    for forced in (True,False,False):
        child=Child(path)
        try:
            child.until(lambda h:h['lifecycle']=='WaitingForObserver');publish(frame);status=child.until(lambda h:h['lifecycle']=='Synchronized')
            epoch=status['context']['epoch']
            assert epoch!=first_epoch;first_epoch=epoch
            child.stop(force=forced)
            assert child.process.returncode==(-signal.SIGKILL if forced else 0)
            owner_gone()
        finally:child.cleanup()
    assert hashlib.sha256(path.read_bytes()).hexdigest()==before
    doctor=subprocess.run([exe,'--db',str(path),'doctor'],capture_output=True,text=True,timeout=5)
    assert doctor.returncode==0 and json.loads(doctor.stdout)['healthy']
print('PERFORMANCE '+json.dumps(measurements,sort_keys=True))
print('PASS controller CLI: bounded bus failure, Python collision, waiting/sync/re-Hello, SIGTERM, SIGKILL/restart, repeated name release, zero child processes, redaction, database preservation/integrity')
