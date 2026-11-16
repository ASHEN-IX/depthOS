#!/usr/bin/env python3
"""New immutable overview package on an isolated virtual KWin, never host writes."""
import os,sys,json,time,subprocess,tempfile,datetime,uuid,shutil,signal,hashlib,importlib.util,threading
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
if '--worker' not in sys.argv:
 sys.path.insert(0,str(ROOT/'tests/integration/poc02e'))
 from process_inventory import Inventory,members
 spec=importlib.util.spec_from_file_location('baseline',ROOT/'tests/integration/live-acceptance.py');a=importlib.util.module_from_spec(spec);spec.loader.exec_module(a)
 def settled_host():
  previous=a.state()
  for _ in range(10):
   time.sleep(.3);current=a.state()
   if current==previous:return current
   previous=current
  raise RuntimeError('Host state did not settle; no host restoration is attempted')
 before=settled_host()
 run=ROOT/'artifacts/p25'/('run-'+datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:6]);run.mkdir(parents=True,mode=0o700)
 for n in ['home','runtime','config','data','cache','state']:(run/n).mkdir(mode=0o700)
 (run/'host-before.json').write_text(json.dumps(before,indent=2))
 subprocess.run(['cargo','build','--locked'],cwd=ROOT,check=True)
 flags=subprocess.check_output(['pkg-config','--cflags','--libs','libei-1.0'],text=True).split();subprocess.run(['gcc','-O2',str(ROOT/'tests/integration/poc02e/native-input.c'),'-o',str(run/'native-input'),*flags],check=True)
 env={'PATH':os.environ['PATH'],'LANG':'C.UTF-8','PYTHONDONTWRITEBYTECODE':'1','HOME':str(run/'home'),'XDG_RUNTIME_DIR':str(run/'runtime'),'XDG_CONFIG_HOME':str(run/'config'),'XDG_DATA_HOME':str(run/'data'),'XDG_CACHE_HOME':str(run/'cache'),'XDG_STATE_HOME':str(run/'state'),'XDG_DATA_DIRS':'/usr/local/share:/usr/share','XDG_SESSION_TYPE':'wayland','XDG_CURRENT_DESKTOP':'KDE','KWIN_WAYLAND_NO_PERMISSION_CHECKS':'1','KWIN_SCREENSHOT_NO_PERMISSION_CHECKS':'1','QT_FORCE_STDERR_LOGGING':'1','QT_LOGGING_TO_CONSOLE':'1','KWIN_XKB_DEFAULT_KEYMAP':'1'}
 with (run/'worker.log').open('w') as log:
  p=subprocess.Popen(['dbus-run-session','--',sys.executable,__file__,'--worker',str(run)],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True);inventory=Inventory(p.pid,run/'processes.json');inventory.start()
  print(run,flush=True)
  try:p.wait(timeout=120)
  except subprocess.TimeoutExpired:p.kill();p.wait()
  finally:
   try:os.killpg(p.pid,signal.SIGTERM)
   except ProcessLookupError:pass
   time.sleep(.3)
   if any(r['state']!='Z' for r in members(p.pid)):
    try:os.killpg(p.pid,signal.SIGKILL)
    except ProcessLookupError:pass
    time.sleep(.3)
   inventory.close()
 after=settled_host();remaining=[r for r in members(p.pid) if r['state']!='Z'];(run/'host-after.json').write_text(json.dumps(after,indent=2));(run/'cleanup.json').write_text(json.dumps({'hostEqual':before==after,'changedKeys':[k for k in before if before[k]!=after.get(k)],'remaining':remaining,'exit':p.returncode},indent=2))
 print((run/'worker.log').read_text()[-6000:]);print('CLEANUP',json.loads((run/'cleanup.json').read_text()))
 assert p.returncode==0 and before==after and not remaining
 raise SystemExit()
import dbus
from PyQt6.QtGui import QImage
run=Path(sys.argv[-1]).resolve();assert Path(os.environ['HOME']).resolve()==run/'home';assert Path(os.environ['XDG_RUNTIME_DIR']).resolve()==run/'runtime'
bus=dbus.SessionBus();assert [str(n) for n in bus.list_names() if not str(n).startswith(':')]==['org.freedesktop.DBus']
checks=[];children=[];kwin=None;controller=None;input_process=None;observer_name='';package='';probe=''
def check(name,value,detail=None):
 checks.append({'name':name,'pass':bool(value),'detail':detail});(run/'assertions.json').write_text(json.dumps(checks,indent=2));assert value,name
 print('PASS',name,flush=True)
def wait(fn,seconds=10):
 deadline=time.monotonic()+seconds
 while time.monotonic()<deadline:
  try:
   value=fn()
   if value:return value
  except (dbus.DBusException,ValueError,FileNotFoundError):pass
  time.sleep(.1)
 raise TimeoutError('isolated condition')
def effects(method,*args):return getattr(dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.kde.kwin.Effects'),method)(*args)
def action(name):dbus.Interface(bus.get_object('org.kde.kglobalaccel','/component/kwin'),'org.kde.kglobalaccel.Component').invokeShortcut(name)
def script(path,name):
 number=int(bus.call_blocking('org.kde.KWin','/Scripting','org.kde.kwin.Scripting','loadScript','ss',(str(path),name),timeout=5));assert number>=0;dbus.Interface(bus.get_object('org.kde.KWin',f'/Scripting/Script{number}'),'org.kde.kwin.Script').run();return name
def unload_script(name):dbus.Interface(bus.get_object('org.kde.KWin','/Scripting'),'org.kde.kwin.Scripting').unloadScript(name)
def snap():
 size=(run/'kwin.log').stat().st_size;action(probe)
 for _ in range(40):
  time.sleep(.025)
  text=(run/'kwin.log').read_bytes()[size:].decode(errors='replace')
  for line in text.splitlines():
   if 'DEPTHOS25 ' in line:
    result=json.loads(line.split('DEPTHOS25 ',1)[1]);(run/'latest-ui.json').write_text(json.dumps(result,indent=2));return result
 raise TimeoutError('native probe')
def ui_wait(predicate,seconds=10):
 def read():
  value=snap();return value if predicate(value) else None
 return wait(read,seconds)
def native_names(s):return [i.get('text') for i in s['items'] if i.get('text')]
def cli(*args):return json.loads(subprocess.check_output([str(ROOT/'target/debug/depthos'),'--db',str(run/'workspaces.db'),*args],text=True))
def start_controller():
 p=subprocess.Popen([str(ROOT/'target/debug/depthos'),'--db',str(run/'workspaces.db'),'controller','run'],stdout=(run/('controller-'+str(len(children))+'.log')).open('w'),stderr=subprocess.STDOUT);children.append(p);wait(lambda:bus.name_has_owner('org.depthos.Controller'));return p
def screenshot():
 r,w=os.pipe();data=[]
 def drain():
  while True:
   b=os.read(r,65536)
   if not b:break
   data.append(b)
  os.close(r)
 thread=threading.Thread(target=drain);thread.start()
 try:m=dbus.Interface(bus.get_object('org.kde.KWin','/org/kde/KWin/ScreenShot2'),'org.kde.KWin.ScreenShot2').CaptureWorkspace(dbus.Dictionary({},signature='sv'),dbus.types.UnixFd(w),signature='a{sv}h',timeout=5)
 finally:os.close(w)
 thread.join(5);assert not thread.is_alive();assert QImage(b''.join(data),int(m['width']),int(m['height']),int(m['stride']),QImage.Format(int(m['format']))).copy().save(str(run/'native-hierarchy.png'))
try:
 # A newly content-addressed package in a disposable data root: no host install.
 digest=hashlib.sha256(b''.join(p.read_bytes() for p in sorted((ROOT/'experiments/kwin-overview').rglob('*')) if p.is_file())).hexdigest();package='depthos-spatial-test-'+digest[:12];probe=package+'-probe';dest=run/'package-source'/package;shutil.copytree(ROOT/'experiments/kwin-overview',dest)
 meta=json.loads((dest/'metadata.json').read_text());meta['KPlugin']['Id']=package;(dest/'metadata.json').write_text(json.dumps(meta))
 probe_code='''
    property var acceptanceViews: []
    ShortcutHandler { name: "PROBE"; text: "DepthOS private spatial read probe"; sequence: ""; onActivated: {
        let items=[];
        function walk(item,root) {
            if (!item) return;
            const n=item.objectName || "";
            if (n.startsWith("logical-") || n === "native-window-card" || n === "native-window-grid" || n === "window-state") {
                const p=item.mapToItem(root,0,0);items.push({name:n,text:typeof item.text === "string" ? item.text : "",visible:item.visible,x:p.x,y:p.y,w:item.width,h:item.height,previewReason:item.previewReason, count:item.count});
            }
            for (let child of item.children || []) walk(child,root);
        }
        for (let v of effect.acceptanceViews) walk(v,v);
        console.info("DEPTHOS25 "+JSON.stringify({nativeDesktop:Workspace.currentDesktop.id,nativeActivity:Workspace.currentActivity,nativeFocus:Workspace.activeWindow ? String(Workspace.activeWindow.internalId):"",visible:effect.visible,reveal:effect.reveal,outputs:effect.constructedOutputs,backend:effect.logicalBackend ? {state:effect.logicalBackend.state,health:effect.logicalBackend.health,rows:effect.logicalBackend.rows,context:effect.logicalBackend.associations,selected:effect.logicalBackend.selectedWorkspaceId,lastCall:effect.logicalBackend.lastCall,lastError:effect.logicalBackend.lastError}:null,items:items}));
    }}
'''.replace('PROBE',probe)
 p=dest/'contents/ui/main.qml';source=p.read_text().replace('    delegate: FocusScope {',probe_code+'\n    delegate: FocusScope {').replace('effect.constructedOutputs++;','effect.constructedOutputs++;effect.acceptanceViews.push(outputRoot);');p.write_text(source)
 subprocess.run(['kpackagetool6','--type','KWin/Effect','--packageroot',str(run/'data/kwin/effects'),'--install',str(dest)],check=True)
 kwin=subprocess.Popen(['kwin_wayland','--virtual','--no-lockscreen','--width','1920','--height','1080','--socket','d25'],stdout=(run/'kwin.log').open('w'),stderr=subprocess.STDOUT);children.append(kwin);wait(lambda:bus.name_has_owner('org.kde.KWin'))
 daemon=dbus.Interface(bus.get_object('org.freedesktop.DBus','/org/freedesktop/DBus'),'org.freedesktop.DBus');check('private-compositor-identity',int(daemon.GetConnectionUnixProcessID('org.kde.KWin'))==kwin.pid)
 support=str(dbus.Interface(bus.get_object('org.kde.KWin','/KWin'),'org.kde.KWin').supportInformation());check('isolated-qualified-version','KWin version: 6.8.3' in support);(run/'support.txt').write_text(support)
 os.environ.update(WAYLAND_DISPLAY='d25',QT_QPA_PLATFORM='wayland',QT_NO_XDG_DESKTOP_PORTAL='1');daemon.UpdateActivationEnvironment(dbus.Dictionary({k:os.environ[k] for k in ['WAYLAND_DISPLAY','QT_QPA_PLATFORM','QT_NO_XDG_DESKTOP_PORTAL']},signature='ss'))
 activity=dbus.Interface(bus.get_object('org.kde.ActivityManager','/ActivityManager/Activities'),'org.kde.ActivityManager.Activities');activities=activity.ListActivities()
 if not activities:
  aid=activity.AddActivity('Private spatial acceptance');activity.SetCurrentActivity(aid)
 time.sleep(.5)
 check('effect-load',effects('loadEffect',package));time.sleep(.5)
 app=subprocess.Popen(['konsole','--separate','--hide-menubar','--hide-tabbar','-e','sleep','90'],stdout=(run/'app.log').open('w'),stderr=subprocess.STDOUT);children.append(app);time.sleep(1)
 action('depthos-spatial-overview-toggle');s=ui_wait(lambda s:s['outputs']==1);check('native-ui-backend-unavailable',s['backend'] is not None and s['backend']['state'] in ('Disconnected','Backend unavailable'))
 check('native-windows-still-rendered',any(i['name']=='native-window-card' for i in s['items']))
 # Read the actual logical SQLite hierarchy through the trusted existing CLI.
 a=cli('world','create','Research')['id'];b=cli('world','create','Operations')['id'];wa=cli('workspace','create','--world',a,'--name','Engine')['id'];wb=cli('workspace','create','--world',a,'--name','Renderer')['id'];wc=cli('workspace','create','--world',b,'--name','Lab')['id']
 da=cli('desktop','create','--workspace',wa,'--name','Code')['id'];db=cli('desktop','create','--workspace',wa,'--name','Review')['id'];cli('desktop','create','--workspace',wc,'--name','Notes')
 controller=start_controller();s=ui_wait(lambda s:s.get('reveal')==1 and s['backend'] and s['backend']['state']=='Degraded' and len(s['backend']['rows'])==8);check('real-hierarchy-through-rust',len(s['backend']['rows'])==8)
 check('actual-world-labels',{'Research','Operations'}.issubset(set(native_names(s))))
 # Genuine private EIS pointer clicks select only logical presentation context.
 fdresult=dbus.Interface(bus.get_object('org.kde.KWin','/org/kde/KWin/EIS/RemoteDesktop'),'org.kde.KWin.EIS.RemoteDesktop').connectToEIS(dbus.Int32(63));fd=fdresult[0].take();input_process=subprocess.Popen([str(run/'native-input'),str(fd)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(run/'input.log').open('w'),text=True,bufsize=1,pass_fds=[fd]);os.close(fd);assert input_process.stdout.readline().strip()=='READY';children.append(input_process)
 def command(text):input_process.stdin.write(text+'\n');input_process.stdin.flush();assert input_process.stdout.readline().strip()=='OK'
 def click(row):command('move %.2f %.2f'%(row['x']+row['w']/2,row['y']+row['h']/2));time.sleep(.1);command('button 272 1');time.sleep(.05);command('button 272 0');time.sleep(.25)
 time.sleep(.4);s=snap();native_before=(s['nativeDesktop'],s['nativeActivity'],s['nativeFocus']);world_row=next(i for i in s['items'] if i['name']=='logical-row-world');click(world_row);s=ui_wait(lambda s:any(i['name']=='logical-row-workspace' for i in s['items']));check('native-logical-world-navigation',any(i['name']=='logical-row-workspace' for i in s['items']))
 time.sleep(.3);s=snap();workspace_row=next(i for i in s['items'] if i['name']=='logical-row-workspace');click(workspace_row);s=ui_wait(lambda s:s['backend']['selected']);check('native-workspace-selection',bool(s['backend']['selected']));check('logical-selection-no-native-mutation',native_before==(s['nativeDesktop'],s['nativeActivity'],s['nativeFocus']));screenshot()
 t=time.monotonic();subprocess.run([str(ROOT/'target/debug/depthos'),'--db',str(run/'workspaces.db'),'workspace','rename',wa,'Engine Updated'],check=True);s=ui_wait(lambda s:any(r['name']=='Engine Updated' for r in s['backend']['rows']));latency=(time.monotonic()-t)*1000;check('external-cli-invalidation',any(r['name']=='Engine Updated' for r in s['backend']['rows']),latency)
 subprocess.run([str(ROOT/'target/debug/depthos'),'--db',str(run/'workspaces.db'),'desktop','delete',da],check=True);subprocess.run([str(ROOT/'target/debug/depthos'),'--db',str(run/'workspaces.db'),'desktop','delete',db],check=True);subprocess.run([str(ROOT/'target/debug/depthos'),'--db',str(run/'workspaces.db'),'workspace','delete',wa],check=True);s=ui_wait(lambda s:len(s['backend']['rows'])==5 and not s['backend']['selected']);check('native-selected-workspace-deletion',not s['backend']['selected'])
 observer_name=script(ROOT/'experiments/kwin-observer/contents/code/main.js',package+'-observer');s=ui_wait(lambda s:s['backend']['health']=='Synchronized' and s['backend']['context'],seconds=15);check('real-native-observer-associations',all(r['status']=='unknown' for r in s['backend']['context']))
 check('native-card-association-badge',any(i['name']=='window-state' and i['text'].startswith('unknown') for i in s['items']))
 check('native-thumbnails-preserved',any(i['name']=='native-window-card' and i.get('previewReason')=='' for i in s['items']))
 unload_script(observer_name);observer_name='';s=ui_wait(lambda s:s['backend']['health']=='Recovering',seconds=9);check('observer-outage-keeps-hierarchy',len(s['backend']['rows'])==5 and not s['backend']['context'])
 controller.terminate();controller.wait(timeout=8);s=ui_wait(lambda s:s['backend']['state']=='Disconnected');check('controller-exit-native-survives',any(i['name']=='native-window-card' for i in s['items']))
 controller=start_controller();s=ui_wait(lambda s:len(s['backend']['rows'])==5);check('controller-reconnect',len(s['backend']['rows'])==5)
 observer_name=script(ROOT/'experiments/kwin-observer/contents/code/main.js',package+'-observer2');s=ui_wait(lambda s:s['backend']['context'],seconds=15);check('restart-no-ephemeral-restoration',all(r['status']=='unknown' for r in s['backend']['context']))
 app.terminate();app.wait(timeout=5);s=ui_wait(lambda s:not s['backend']['context']);check('native-window-disappearance-clears-association',not s['backend']['context'])
 command('key 1 1');command('key 1 0');wait(lambda:not snap()['visible']);check('native-escape-releases-overview',package not in list(dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.freedesktop.DBus.Properties').Get('org.kde.kwin.Effects','activeEffects')))
 (run/'performance.json').write_text(json.dumps({'externalHierarchyEditToNativeUiMs':latency,'scope':'private KWin 6.8.3, native QML, actual Rust controller'},indent=2))
finally:
 if observer_name:
  try:unload_script(observer_name)
  except Exception:pass
 if package and kwin and kwin.poll() is None:
  try:effects('unloadEffect',package)
  except Exception:pass
 for p in reversed(children):
  if p.poll() is None:
   p.terminate()
   try:p.wait(timeout=5)
   except subprocess.TimeoutExpired:p.kill();p.wait()
 shutil.rmtree(run/'data/kwin/effects'/package,ignore_errors=True)
