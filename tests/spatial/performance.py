#!/usr/bin/env python3
"""Actual Rust/D-Bus/production QML timing. Synthetic observations; no compositor."""
import argparse,os,sys,subprocess,tempfile,json,time,uuid,queue,threading,copy,statistics
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
if '--inner' not in sys.argv:
 subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--inner',*sys.argv[1:]],check=True);raise SystemExit()
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--inner', action='store_true', help=argparse.SUPPRESS)
parser.add_argument('--workspaces', type=int, default=100)
parser.add_argument('--windows', type=int, default=1000)
parser.add_argument('--lifecycle', action='store_true', help='also time external edits, a sequence gap and restart')
parser.add_argument('--output', type=Path)
args = parser.parse_args()
if not 0 <= args.workspaces <= 1365 or not 31 <= args.windows <= 1000:
 parser.error('expected 0–1365 workspaces and 31–1000 windows')
if args.lifecycle and not args.workspaces:
 parser.error('--lifecycle requires a workspace')
hierarchy_rows = (1 + 3*args.workspaces) if args.workspaces else 0
import dbus,dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib
DBusGMainLoop(set_as_default=True);bus=dbus.SessionBus();assert [str(n) for n in bus.list_names() if not str(n).startswith(':')]==['org.freedesktop.DBus']
ctx=GLib.MainContext.default()
def pump():
 while ctx.pending():ctx.iteration(False)
def wait(fn,seconds=12):
 end=time.monotonic()+seconds
 while time.monotonic()<end:
  pump();v=fn()
  if v:return v
  time.sleep(.005)
 raise TimeoutError('private performance fixture')
def cpu(pid):
 a=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split();return (int(a[11])+int(a[12]))/os.sysconf('SC_CLK_TCK')
def rss(pid):return int(next(s for s in Path(f'/proc/{pid}/status').read_text().splitlines() if s.startswith('VmRSS:')).split()[1])
with tempfile.TemporaryDirectory(prefix='depthos-ui-perf-') as t:
 t=Path(t);db=t/'db';exe=ROOT/'target/debug/depthos'
 def cli(*args):
  raw=subprocess.check_output([str(exe),'--db',str(db),*args],text=True)
  return json.loads(raw) if raw.strip() else None
 world=cli('world','create','Measurement')['id'] if args.workspaces else None
 for i in range(args.workspaces):
  ws=cli('workspace','create','--world',world,'--name',f'Workspace {i}')['id']
  desktop_ids=[cli('desktop','create','--workspace',ws,'--name',name)['id'] for name in ('Code','Review')]
 controller=subprocess.Popen([str(exe),'--db',str(db),'controller','run'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL);ui=None
 try:
  wait(lambda:bus.name_has_owner('org.depthos.Controller'))
  endpoint=dbus.Interface(bus.get_object('org.depthos.Controller','/org/depthos/Controller'),'org.depthos.Controller1')
  signals=[];bus.add_signal_receiver(lambda *a:signals.append(time.monotonic()),signal_name='Invalidated',dbus_interface='org.depthos.Controller1',path='/org/depthos/Controller',bus_name='org.depthos.Controller')
  # Full consistent paged hierarchy round trip, repeated independently.
  times=[]
  for _ in range(10):
   start=time.monotonic();cursor=0;rev='';n=0
   while True:
    p=json.loads(str(endpoint.GetHierarchy(dbus.UInt32(cursor),rev,timeout=3)));n+=len(p['data']['items']);rev=p['revision'];cursor=p['data']['nextCursor']
    if cursor is None:break
   assert n==hierarchy_rows;times.append((time.monotonic()-start)*1000)
  publisher=dbus.service.BusName('org.kde.KWin',bus,do_not_queue=True)
  observer=dbus.Interface(bus.get_object('org.depthos.StateInspector','/org/depthos/StateInspector'),'org.depthos.StateInspector1')
  hello=json.loads(str(observer.Hello('test-session')));consumer=hello['consumerId']
  frame=json.loads((ROOT/'tests/observer-adapter/trace.json').read_text())['frames'][0]
  template=frame['state']['windows'][0]
  frame['state']['windows']=[dict(template,runtimeId=f'window-{i}',nativeId=str(uuid.uuid4())) for i in range(args.windows)]
  frame['state']['applicationWindowIds']=[w['runtimeId'] for w in frame['state']['windows']];frame['state']['activeWindowId']=None
  def push(frames,seq,rev,need_snapshot=False):
   p={'schemaVersion':1,'sessionId':'test-session','consumerId':consumer,'frames':frames,'position':{'sequence':seq,'revision':rev}}
   assert json.loads(str(observer.Push(json.dumps(p),timeout=5)))['needSnapshot'] == need_snapshot
  def qml():
   file=t/'tst_perf.qml';source=(ROOT/'experiments/kwin-overview/contents/ui/ControllerClient.qml').as_uri()
   lifecycle = """
    tryVerify(()=>b.item.state==="Connected" && b.item.rows.some(r=>r.name==="Benchmark renamed"),10000);
    console.info("STAGE rename");
    tryVerify(()=>b.item.state==="Connected" && b.item.rows.length===ROWS_EXPECTED-3,10000);
    console.info("STAGE delete");
    tryVerify(()=>b.item.health==="Recovering" && !b.item.contextAvailable && b.item.associations.length===0,10000);
    console.info("STAGE desynchronized");
    tryVerify(()=>b.item.contextAvailable && b.item.associations.length===WINDOWS_EXPECTED-1,10000);
    console.info("STAGE resynchronized");
    const oldInstance=b.item.instance;
    tryVerify(()=>b.item.state==="Disconnected" && b.item.rows.length===0,10000);
    console.info("STAGE disconnected");
    tryVerify(()=>b.item.state==="Degraded" && b.item.instance!==oldInstance && b.item.rows.length===ROWS_EXPECTED-3 && !b.item.contextAvailable,10000);
    console.info("STAGE restarted");
""" if args.lifecycle else ""
   source_text = '''import QtQuick
import QtTest
Item {Loader {id:b;source:"SOURCE"}
Timer {interval:10;running:true;repeat:true;property int last:-1;onTriggered:if(b.item && b.item.contextAvailable && b.item.associations.length!==last){last=b.item.associations.length;console.info("ROWS "+last);}}
TestCase {name:"MeasuredProductionClient";when:windowShown;function test_updates(){
    tryVerify(()=>b.item && b.item.rows.length===ROWS_EXPECTED && b.item.associations.length===WINDOWS_EXPECTED,12000);
    tryVerify(()=>b.item.associations.length===WINDOWS_EXPECTED-1,18000);
    LIFECYCLE
}}
}'''
   file.write_text(source_text.replace('LIFECYCLE', lifecycle).replace('SOURCE',source)
                   .replace('ROWS_EXPECTED',str(hierarchy_rows)).replace('WINDOWS_EXPECTED',str(args.windows)))
   return subprocess.Popen(['/usr/lib/qt6/bin/qmltestrunner','-input',str(file)],env=dict(os.environ,QT_QPA_PLATFORM='offscreen',QT_QUICK_BACKEND='software',QT_FORCE_STDERR_LOGGING='1'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
  ui=qml();marks={};lines=[]
  def reader():
   for line in ui.stdout:
    lines.append(line)
    if 'STAGE ' in line:marks[line.rsplit('STAGE ',1)[1].strip()]=time.monotonic()
    if 'ROWS ' in line:marks[int(line.rsplit('ROWS ',1)[1].strip())]=time.monotonic()
  thread=threading.Thread(target=reader);thread.start()
  time.sleep(.3);signals.clear();started=time.monotonic();push([frame],1,1)
  wait(lambda:args.windows in marks);ui_ms=(marks[args.windows]-started)*1000;signal_ms=(signals[0]-started)*1000
  first_cpu=cpu(controller.pid);ui_cpu=cpu(ui.pid);start=time.monotonic()
  for _ in range(3):time.sleep(1);push([],1,1);pump()
  idle_seconds=time.monotonic()-start;idle=100*(cpu(controller.pid)-first_cpu)/idle_seconds;ui_idle=100*(cpu(ui.pid)-ui_cpu)/idle_seconds
  memory={'controllerRssKiB':rss(controller.pid),'qmlClientRssKiB':rss(ui.pid)}
  # Thirty consecutive production-format geometry events: no semantic UI invalidations.
  signals.clear();start=time.monotonic()
  for i in range(30):
   window=copy.deepcopy(frame['state']['windows'][i]);window['geometry']['x']=i+1
   event={'schemaVersion':1,'sessionId':'test-session','type':'event','sequence':i+2,'revision':i+2,'event':'window-changed','data':{'window':window}}
   # Use the exact production replay event fields below.
   sample=json.loads((ROOT/'tests/observer-adapter/trace.json').read_text())['frames'][1]
   event=copy.deepcopy(sample);event.update(sequence=i+2,revision=i+2)
   # The replay fixture's event data has a full serialized window.
   event['window']=window;event['windowId']=window['runtimeId'];event['type']='window.geometry.changed';event['changedFields']=['geometry']
   push([event],i+2,i+2)
  burst_ms=(time.monotonic()-start)*1000
  time.sleep(.7);pump();geometry_invalidations=len(signals);assert geometry_invalidations==0,geometry_invalidations
  # Remove one runtime identity by complete replacement; client must discard its badge.
  frame['sequence']=32;frame['revision']=32;frame['state']['windows'].pop();frame['state']['applicationWindowIds'].pop();signals.clear();start=time.monotonic();push([frame],32,32)
  wait(lambda:args.windows-1 in marks);removal_ms=(marks[args.windows-1]-start)*1000
  lifecycle_ms={}
  if args.lifecycle:
   def timed(stage, action):
    start=time.monotonic()
    action()
    wait(lambda:stage in marks)
    lifecycle_ms[stage+'Ms']=(marks[stage]-start)*1000
   timed('rename',lambda:cli('workspace','rename',ws,'Benchmark renamed'))
   def delete_workspace():
    for desktop_id in desktop_ids:
     cli('desktop','delete',desktop_id)
    cli('workspace','delete',ws)
   timed('delete',delete_workspace)
   gap=dict(event,sequence=34,revision=34)
   timed('desynchronized',lambda:push([gap],34,34,need_snapshot=True))
   frame.update(sequence=35,revision=35)
   timed('resynchronized',lambda:push([frame],35,35))
   def stop_controller():
    controller.terminate()
    controller.wait(timeout=5)
   timed('disconnected',stop_controller)
   start=time.monotonic()
   controller=subprocess.Popen([str(exe),'--db',str(db),'controller','run'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
   wait(lambda:'restarted' in marks)
   lifecycle_ms['restartedMs']=(marks['restarted']-start)*1000
  ui.wait(timeout=8);thread.join();assert ui.returncode==0,''.join(lines)
  result={'scope':'debug Rust, real private D-Bus and Plasma QML plugin, offscreen software client; synthetic observer; no FPS claim','hierarchyRows':hierarchy_rows,'windows':args.windows,'hierarchyFetchSamplesMs':times,**lifecycle_ms,'hierarchyFetchMedianMs':statistics.median(times),'hierarchyFetchMaxMs':max(times),'initialSnapshotToQmlMs':ui_ms,'initialSnapshotToInvalidationMs':signal_ms,'nativeRemovalToQmlMs':removal_ms,'idleSeconds':idle_seconds,'controllerIdleCpuPercentOneCore':idle,'qmlIdleCpuPercentOneCore':ui_idle,**memory,'geometryOnlyInvalidations':geometry_invalidations,'burstEvents':30,'burstPublishMs':burst_ms}
  out=args.output or ROOT/'artifacts/p25/performance.json';out.parent.mkdir(parents=True,exist_ok=True);out.write_text(json.dumps(result,indent=2));print(json.dumps(result,indent=2))
 finally:
  if ui and ui.poll() is None:ui.terminate();ui.wait(timeout=5)
  controller.terminate();controller.wait(timeout=5)
