#!/usr/bin/env python3
"""Production Plasma D-Bus client lifecycle and models on a fresh private bus."""
import os,sys,subprocess,tempfile,time,queue,threading
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
if '--inner' not in sys.argv:
 subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--inner'],check=True);raise SystemExit()
with tempfile.TemporaryDirectory(prefix='depthos-spatial-client-') as tmp:
 at=Path(tmp);db=at/'db';exe=ROOT/'target/debug/depthos'
 def cli(*args):
  import json
  raw=subprocess.check_output([str(exe),'--db',str(db),*args],text=True)
  return json.loads(raw) if raw.strip() else None
 world=cli('world','create','World One')['id'];cli('world','create','World Two')
 workspace=cli('workspace','create','--world',world,'--name','Temporary')['id']
 source=ROOT/'experiments/kwin-overview/contents/ui/ControllerClient.qml'
 panel=ROOT/'experiments/kwin-overview/contents/ui/LogicalPanel.qml'
 qml=at/'tst_client.qml';qml.write_text('''import QtQuick
import QtTest
Item { width:1920;height:1080
Loader {id: backend; source: "SOURCE"}
Loader {id: panel; width:1856;height:72;source:"PANEL";onLoaded:item.backend=Qt.binding(()=>backend.item)}
TestCase { name:"RealSpatialClient"; when:windowShown
function test_transport() {
tryVerify(()=>backend.item!==null,2000);const b=backend.item;
compare(b.state,"Disconnected");console.info("STAGE_INITIAL");
tryVerify(()=>b.rows.length===3 && b.state==="Degraded",7000);
compare(b.rows[0].name,"World One");compare(b.rows[2].name,"World Two");
tryVerify(()=>panel.item!==null,1000);
const w=b.rows.find(r=>r.kind==="world");b.choose(w);
const ws=b.rows.find(r=>r.kind==="workspace");b.choose(ws);compare(b.selectedWorkspaceId,ws.id);
console.info("STAGE_SELECTED");
tryVerify(()=>b.rows.length===2 && b.rows.some(r=>r.name==="Renamed World"),6000);
compare(b.selectedWorkspaceId,"");compare(b.selectedWorldId,w.id);console.info("STAGE_DELETED");
tryVerify(()=>b.state==="Disconnected",6000);compare(b.rows.length,0);compare(b.selectedWorldId,"");console.info("STAGE_DISCONNECTED");
tryVerify(()=>b.rows.length===2 && b.state==="Degraded",6000);compare(b.selectedWorkspaceId,"");console.info("STAGE_RECONNECTED");
// Independent local cache fencing and schema rejection; no transport mocks.
b.refresh();const ticket=b.serial;b.reset();verify(b.serial>ticket);wait(100);compare(b.state,"Disconnected");compare(b.rows.length,0);compare(b.associations.length,0);
let rejected=false;try {b.packet('{"schemaVersion":2}');}catch(e){rejected=true;}verify(rejected);
b.rows=[{kind:"world",id:"w",parentId:null,name:"World"}];b.choose(b.rows[0]);b.rows=[];b.reconcile();compare(b.selectedWorldId,"");
b.fail();compare(b.state,"Backend unavailable");compare(b.contextAvailable,false);
console.info("CLIENT_LIFECYCLE_PASS");
}
}
}'''.replace('SOURCE',source.as_uri()).replace('PANEL',panel.as_uri()))
 ui=subprocess.Popen(['/usr/lib/qt6/bin/qmltestrunner','-input',str(qml)],env=dict(os.environ,QT_QPA_PLATFORM='offscreen',QT_QUICK_BACKEND='software',QT_FORCE_STDERR_LOGGING='1'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 messages=queue.Queue();lines=[]
 def read():
  for line in ui.stdout:lines.append(line);messages.put(line)
 reader=threading.Thread(target=read);reader.start()
 def stage(marker):
  end=time.monotonic()+10
  while time.monotonic()<end:
   try:
    if marker in messages.get(timeout=.1):return
   except queue.Empty:pass
   if ui.poll() is not None:break
  raise AssertionError('missing '+marker+'\n'+''.join(lines))
 def start():return subprocess.Popen([str(exe),'--db',str(db),'controller','run'],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
 controller=None
 try:
  stage('STAGE_INITIAL');controller=start();stage('STAGE_SELECTED')
  cli('world','rename',world,'Renamed World');cli('workspace','delete',workspace);stage('STAGE_DELETED')
  controller.terminate();controller.wait(timeout=5);stage('STAGE_DISCONNECTED');controller=start();stage('STAGE_RECONNECTED')
  ui.wait(timeout=10);reader.join();assert ui.returncode==0 and 'CLIENT_LIFECYCLE_PASS' in ''.join(lines),''.join(lines)
  print('PASS actual Rust → production QML client: initial outage, hierarchy, edit/delete selection, stop/restart, cache fencing, schema mismatch, unavailable model')
 finally:
  if controller:controller.terminate();controller.wait(timeout=5)
  if ui.poll() is None:ui.terminate();ui.wait(timeout=5)
