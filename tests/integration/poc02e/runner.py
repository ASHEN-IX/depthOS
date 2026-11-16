"""Isolated native EIS acceptance. Run ONLY through launch.py (private bus/home).
The only QML addition reads state/geometry; all interactions use real EIS events.
"""
import os,sys,time,json,uuid,hashlib,shutil,subprocess,threading,traceback,select,re
from pathlib import Path
import dbus
from PyQt6.QtGui import QImage
R=Path(__file__).resolve().parents[3];run=Path(sys.argv[1]).resolve()
assert Path(os.environ['HOME']).resolve()==run/'home'
assert Path(os.environ['XDG_RUNTIME_DIR']).resolve()==run/'runtime'
assert 'DBUS_SESSION_BUS_ADDRESS' in os.environ
bus=dbus.SessionBus();checks=[];children=[];helper=None;kwin=None;recorder=None;observer=None;frames=[];failed=False;started=time.monotonic();actions=[]
def write(name,obj): (run/name).write_text(json.dumps(obj,indent=2))
def track(proc,role):
 m=json.loads((run/'manifest.json').read_text());m.setdefault('processes',[]).append({'pid':proc.pid,'startTicks':Path(f'/proc/{proc.pid}/stat').read_text().rsplit(')',1)[1].split()[19],'role':role});write('manifest.json',m)
def log(name,obj):
 with (run/name).open('a') as f:f.write(json.dumps(obj)+'\n')
def invoke(name):
 dbus.Interface(bus.get_object('org.kde.kglobalaccel','/component/kwin'),'org.kde.kglobalaccel.Component').invokeShortcut(name)
def effects(method,*args):return getattr(dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.kde.kwin.Effects'),method)(*args)
def check(name,value,detail=None):
 checks.append({'id':name,'pass':bool(value),'detail':detail});write('assertions.json',checks)
 if not value:raise AssertionError(name)
def snap():
 before=(run/'scoped-kwin.log').stat().st_size;invoke(probe)
 for _ in range(30):
  time.sleep(.02)
  text=(run/'scoped-kwin.log').read_bytes()[before:].decode(errors='replace')
  for line in text.splitlines():
   if 'DEPTHOS02E_STATE ' in line:
    s=json.loads(line.split('DEPTHOS02E_STATE ',1)[1]);s['monotonic']=time.monotonic();s['effectActive']=package in list(dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.freedesktop.DBus.Properties').Get('org.kde.kwin.Effects','activeEffects'));log('native-state.jsonl',s);return s
 raise TimeoutError('read probe did not answer')
def capture(label):
 # One ScreenShot2 producer. Earlier partial-image assessments were not
 # reproduced by independent decoding; no rendering defect is assumed.
 requested=time.monotonic();until=requested+3
 while time.monotonic()<until:
  time.sleep(.05)
  records=(run/'capture-timestamps.jsonl').read_text().splitlines()
  if not records:continue
  try:f=json.loads(records[-1])
  except json.JSONDecodeError:continue
  if f['requestMonotonic']<requested:continue
  source=Path(f['path']);img=QImage(str(source))
  if img.isNull():continue
  # Reject an entirely black gate frame; retain EVERY source frame in the video.
  nonblack=any(img.pixelColor(x,y).red()+img.pixelColor(x,y).green()+img.pixelColor(x,y).blue()>12 for x in range(0,img.width(),24) for y in range(0,img.height(),24))
  if not nonblack:continue
  path=run/'screenshots'/('%04d-%s.png'%(len(frames),label));shutil.copyfile(source,path);frames.append((path,f['requestMonotonic']))
  log('gate-captures.jsonl',{'gate':label,'source':str(source.relative_to(run)),'screenshot':str(path.relative_to(run)),'requestMonotonic':f['requestMonotonic'],'gateCaptureRequested':requested,'nonblack':True})
  return str(path.relative_to(run))
 raise RuntimeError('No fresh nonblack recorder frame for '+label)
def command(cmd):
 assert helper and helper.poll() is None
 helper.stdin.write(cmd+'\n');helper.stdin.flush()
 ready,_,_=select.select([helper.stdout],[],[],3);assert ready,'input acknowledgement timeout'
 assert helper.stdout.readline().strip()=='OK'
def key(code):command('key %d 1'%code);command('key %d 0'%code)
def click(item):
 command('move %.2f %.2f'%(item['x']+item['w']/2,item['y']+item['h']/2));time.sleep(.08);command('button 272 1');command('button 272 0')
def action(name,target,fn,expect,predicate):
 t=time.monotonic();assert kwin.poll() is None;assert str(bus.get_name_owner('org.kde.KWin'))==owner;fn();time.sleep(.4);s=snap();ok=bool(predicate(s));shot=capture(name)
 record={'id':name,'inputMethod':'libei/EIS native Wayland' if not name.startswith('K01') else 'registered KGlobalAccel action','target':target,'timestamp':time.time(),'monotonicStart':t,'expected':expect,'observed':{k:s[k] for k in ['visible','effectActive','outputs','level','selected','active','motionOff','desktop','activity']},'screenshot':shot,'verdict':'PASS' if ok else 'FAIL'}
 actions.append(record);log('actions.jsonl',record);check(name,ok);return s
def item(s,name=None,action=None,index=0):
 candidates=[i for i in s['items'] if i['visible'] and ((name and i['name']==name) or (action and i.get('action')==action))]
 assert len(candidates)>index,(name,action,candidates)
 return candidates[index]
def invariant(s):return (s['active'],[(w['uuid'],w['desktops'],w['activities'],w['geometry']) for w in s['windows']])
try:
 # Private desktop model, no copy of host preferences.
 ids=[str(uuid.uuid4()) for _ in range(4)]
 (run/'config/kwinrc').write_text('[Desktops]\nNumber=4\nRows=1\n'+''.join(f'Id_{i+1}={x}\nName_{i+1}=Acceptance {i+1}\n' for i,x in enumerate(ids))+'\n[Plugins]\nblurEnabled=false\n')
 production=R/'experiments/kwin-overview'
 probe_template=Path(__file__).with_name('probe.qml.inc').read_text()
 package_inputs={str(p.relative_to(production)):hashlib.sha256(p.read_bytes()).hexdigest() for p in production.rglob('*') if p.is_file()}
 digest=hashlib.sha256(json.dumps([package_inputs,probe_template],sort_keys=True).encode()).hexdigest()
 package='depthos-02e-'+digest[:12];probe=package+'-read';dest=run/'package-source'/package
 shutil.copytree(production,dest)
 meta=json.loads((dest/'metadata.json').read_text());meta['KPlugin']['Id']=package;(dest/'metadata.json').write_text(json.dumps(meta))
 p=dest/'contents/ui/main.qml';s=p.read_text();inc=probe_template.replace('DEPTHOS_PROBE_ID',probe)
 s=s.replace('    delegate: FocusScope {',inc+'\n    delegate: FocusScope {').replace('effect.constructedOutputs++;','effect.constructedOutputs++; effect.acceptanceViews.push(outputRoot);').replace('Component.onDestruction: effect.constructedOutputs = Math.max(0, effect.constructedOutputs - 1)','Component.onDestruction: { effect.constructedOutputs = Math.max(0, effect.constructedOutputs - 1); effect.acceptanceViews = effect.acceptanceViews.filter(v => v !== outputRoot); }')
 p.write_text(s)
 subprocess.run(['kpackagetool6','--type','KWin/Effect','--packageroot',str(run/'data/kwin/effects'),'--install',str(dest)],check=True)
 poc1='depthos-02e-contention-'+run.name[-6:];poc1src=run/'package-source'/poc1
 shutil.copytree(R/'experiments/kwin-effect',poc1src)
 pm=json.loads((poc1src/'metadata.json').read_text());pm['KPlugin']['Id']=poc1;(poc1src/'metadata.json').write_text(json.dumps(pm))
 pq=poc1src/'contents/ui/main.qml';pq.write_text(pq.read_text().replace('depthos-spatial-poc-toggle',poc1+'-toggle').replace('Meta+Ctrl+Shift+F11',''))
 subprocess.run(['kpackagetool6','--type','KWin/Effect','--packageroot',str(run/'data/kwin/effects'),'--install',str(poc1src)],check=True)
 kwinlog=(run/'scoped-kwin.log').open('w')
 kwin=subprocess.Popen(['kwin_wayland','--virtual','--no-lockscreen','--width','1920','--height','1080','--socket','d02e'],stdout=kwinlog,stderr=subprocess.STDOUT)
 for _ in range(150):
  if kwin.poll() is not None:raise RuntimeError('virtual KWin exited')
  if bus.name_has_owner('org.kde.KWin'):break
  time.sleep(.1)
 owner=str(bus.get_name_owner('org.kde.KWin'));host=json.loads((run/'baseline.json').read_text());assert owner
 # Bus is created by launch.py, endpoint process must be our child, never host.
 db=dbus.Interface(bus.get_object('org.freedesktop.DBus','/org/freedesktop/DBus'),'org.freedesktop.DBus')
 assert int(db.GetConnectionUnixProcessID(owner))==kwin.pid
 busid=str(db.GetId());assert busid!=host['busId'] and kwin.pid!=host['kwinPid']
 installed=subprocess.check_output(['kwin_wayland','--version'],text=True).strip().split()[-1]
 support=str(dbus.Interface(bus.get_object('org.kde.KWin','/KWin'),'org.kde.KWin').supportInformation())
 running=re.search(r'^KWin version: (.+)$',support,re.MULTILINE).group(1).strip()
 assert installed==running and (not os.environ.get('DEPTHOS_ISOLATED_EXPECTED_VERSION') or running==os.environ['DEPTHOS_ISOLATED_EXPECTED_VERSION'])
 write('version-verification.json',{'installed':installed,'running':running,'privateBusId':busid,'privateOwner':owner,'privatePid':kwin.pid,'hostPid':host['kwinPid'],'versionMatched':True,'privateOwnerPidMatched':True})
 write('manifest.json',{'scope':'isolated virtual KWin','installedVersion':installed,'runningVersion':running,'busId':busid,'kwinOwner':owner,'kwinPid':kwin.pid,'runtime':os.environ['XDG_RUNTIME_DIR'],'input':'libei 1.6 / private EIS authorized only inside this disposable compositor','hostInput':False,'packages':[package,poc1],'busAddress':os.environ['DBUS_SESSION_BUS_ADDRESS'],'processStartTicks':Path(f'/proc/{kwin.pid}/stat').read_text().rsplit(')',1)[1].split()[19],'probeSha256':hashlib.sha256(Path(__file__).with_name('probe.qml.inc').read_bytes()).hexdigest(),'productionFiles':{str(p.relative_to(R)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (R/'experiments/kwin-overview').rglob('*') if p.is_file()},'sourceSha256':hashlib.sha256((R/'experiments/kwin-overview/contents/ui/main.qml').read_bytes()).hexdigest(),'gitHead':subprocess.run(['git','rev-parse','HEAD'],cwd=R,capture_output=True,text=True).stdout.strip() or 'unborn/untracked workspace','watchdog':'independent launch.py process, 240s bounded child process group'})
 track(kwin,'compositor')
 os.environ.update(WAYLAND_DISPLAY='d02e',QT_QPA_PLATFORM='wayland',QT_NO_XDG_DESKTOP_PORTAL='1')
 db.UpdateActivationEnvironment(dbus.Dictionary({k:os.environ[k] for k in ['WAYLAND_DISPLAY','QT_QPA_PLATFORM','QT_NO_XDG_DESKTOP_PORTAL']},signature='ss'))
 # Start private Activity manager through its public D-Bus activation.
 try:
  act=dbus.Interface(bus.get_object('org.kde.ActivityManager','/ActivityManager/Activities'),'org.kde.ActivityManager.Activities')
  existing=act.ListActivities()
  if not existing:
   activity=act.AddActivity('DepthOS isolated acceptance');act.SetCurrentActivity(activity)
 except dbus.DBusException as e:log('limitations.jsonl',{'activity':str(e)})
 # Marker receives only a fixed string. Other lines are discarded, never executed.
 marker=run/'marker.py';marker.write_text('import sys,json,time\nfrom pathlib import Path\nprint("DEPTHOS ISOLATED INPUT RECEIVER — not a shell",flush=True)\nfor line in sys.stdin:\n if line.rstrip("\\r\\n")=="depthos-02e-ok":\n  Path(sys.argv[1]).write_text(json.dumps({"matched":True,"time":time.time()}));print("INPUT VERIFIED",flush=True)\n')
 for n in range(2):
  f=(run/f'app-{n}.log').open('w')
  p=subprocess.Popen(['konsole','--separate','--hide-menubar','--hide-tabbar','-e','python3',str(marker),str(run/f'ack-{n}.json')],stdout=f,stderr=subprocess.STDOUT);children.append(p);track(p,'test-window')
 time.sleep(2)
 if os.environ.get('DEPTHOS_ISOLATED_QUALIFICATION'):
  sys.path.insert(0,str(R/'tests/integration/poc02g'));from native_observer import Observer
  observer=Observer(bus,run,R,track);observer.start();check('G-observer-initial-two',len(observer.windows())==2)
 (run/'support.txt').write_text(str(dbus.Interface(bus.get_object('org.kde.KWin','/KWin'),'org.kde.KWin').supportInformation()))
 log('debug.jsonl',{'effectSupported':bool(effects('isEffectSupported',package))})
 assert effects('loadEffect',package)
 time.sleep(.3)
 fdresult=dbus.Interface(bus.get_object('org.kde.KWin','/org/kde/KWin/EIS/RemoteDesktop'),'org.kde.KWin.EIS.RemoteDesktop').connectToEIS(dbus.Int32(63));fd=fdresult[0].take()
 helper=subprocess.Popen([str(run/'native-input'),str(fd)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(run/'input.log').open('w'),text=True,bufsize=1,pass_fds=[fd]);os.close(fd)
 assert helper.stdout.readline().strip()=='READY'
 track(helper,'input')
 write('input-process.json',{'pid':helper.pid,'startTicks':Path(f'/proc/{helper.pid}/stat').read_text().rsplit(')',1)[1].split()[19]})
 recorder=subprocess.Popen(['python3',str(Path(__file__).with_name('record.py')),str(run)],stdout=(run/'recorder.log').open('w'),stderr=subprocess.STDOUT)
 track(recorder,'capture')
 for _ in range(60):
  if (run/'recording-ready').exists():break
  if recorder.poll() is not None:raise RuntimeError('recorder failed')
  time.sleep(.1)
 toggle=lambda:invoke('depthos-spatial-overview-toggle')
 s=action('K01','overview action',toggle,'visible Z1 with output',lambda s:s['visible'] and s['effectActive'] and s['outputs']==1 and s['level']==1)
 check('two-owned-windows',len(s['windows'])==2 and all(w['pid'] in [p.pid for p in children] for w in s['windows']),s['windows'])
 base=invariant(s);grid=lambda s:item(s,'native-window-grid')
 check('K01-grid',grid(s)['count']==2)
 for n,code,idx in [('right',106,1),('left',105,0),('down',108,1),('up',103,0)]:
  s=action('K02-'+n,'window grid',lambda c=code:key(c),'selection index '+str(idx),lambda s,i=idx:grid(s)['index']==i and invariant(s)==base)
 for name,code in [('enter',28),('space',57)]:
  s=action('K03-'+name,'selected window card',lambda c=code:key(c),'local selection only',lambda s:bool(s['selected']) and invariant(s)==base and s['level']==1)
 for name,code,level in [('minus',12,2),('backspace',14,3),('equals',13,2),('plus',13,1)]:
  def send(c=code,n=name):
   if n=='plus':command('key 42 1')
   key(c)
   if n=='plus':command('key 42 0')
  s=action('K04-'+name,'depth navigation',send,'Z'+str(level),lambda s,l=level:s['level']==l and invariant(s)==base)
 check('K04-real-desktops',len(s['desktops'])==4 and set(s['desktops'])==set(ids))
 check('K04-real-activity',len(s['activities'])==1 and s['activity'] in s['activities'])
 # Native keyboard context selection, then return by native keys.
 key(12);time.sleep(.25);key(106)
 s=action('K04-desktop-enter','desktop 2 via Right/Enter',lambda:key(28),'inspect desktop 2 only',lambda s:s['desktop']==ids[1] and s['level']==1 and invariant(s)==base)
 key(12);key(105)
 s=action('K04-desktop-space','desktop 1 via Left/Space',lambda:key(57),'inspect desktop 1 only',lambda s:s['desktop']==ids[0] and s['level']==1 and invariant(s)==base)
 key(12);key(14);time.sleep(.2)
 s=action('K04-activity-space','real Activity via Space',lambda:key(57),'inspect same Activity at Z2',lambda s:s['level']==2 and s['activity'] in s['activities'] and invariant(s)==base)
 key(13);time.sleep(.25);s=snap()
 # Mouse targets obtained from the live scene after all transforms.
 cards=lambda s:[i for i in s['items'] if i['name']=='native-window-card' and i['visible']]
 for n in range(2):
  c=cards(s)[n]
  s=action('M-hover-'+str(n),'owned card geometry',lambda c=c:command('move %.2f %.2f'%(c['x']+c['w']/2,c['y']+c['h']/2)),'hover delivered',lambda s,n=n:cards(s)[n]['hovered'])
  s=action('M-card-'+str(n),'owned card geometry',lambda c=c:click(c),'clicked UUID selected, native focus preserved',lambda s,c=c:s['selected']==c['uuid'] and invariant(s)==base)
 s=action('M-zoom-out','− Zoom out',lambda:click(item(s,action='− Zoom out')),'Z2',lambda s:s['level']==2)
 c=[i for i in s['items'] if i.get('contextIndex')==1 and i['visible']][0]
 target=s['desktops'][1]
 s=action('M-desktop','desktop card 2',lambda:click(c),'inspect desktop without switching',lambda s:s['level']==1 and s['desktop']==target and invariant(s)==base)
 s=action('M-z2','zoom out',lambda:click(item(s,action='− Zoom out')),'Z2',lambda s:s['level']==2)
 s=action('M-z3','zoom out',lambda:click(item(s,action='− Zoom out')),'Z3',lambda s:s['level']==3)
 c=[i for i in s['items'] if i.get('contextIndex')==0 and i['visible']][0]
 s=action('M-activity','actual Activity card',lambda:click(c),'inspect Activity Z2',lambda s:s['level']==2 and s['activity'] in s['activities'] and invariant(s)==base)
 s=action('M-zoom-in','+ Zoom in',lambda:click(item(s,action='+ Zoom in')),'Z1',lambda s:s['level']==1)
 s=action('M-motion','Reduce motion',lambda:click(item(s,action='Reduce motion')),'motion disabled',lambda s:s['motionOff'])
 s=action('M-close','close button',lambda:click(item(s,action='Close · Esc')),'overlay closed',lambda s:not s['visible'] and s['outputs']==0 and not s['effectActive'])
 for level in [1,2,3]:
  toggle();time.sleep(.35)
  for _ in range(level-1):key(12)
  time.sleep(.25)
  s=action('K05-escape-Z'+str(level),'Escape',lambda:key(1),'closed with no delegate',lambda s:not s['visible'] and s['outputs']==0 and not s['effectActive'])
 # Two reopen/cancel passes, native rapid keys.
 for rep in range(2):
  toggle();time.sleep(.3)
  for c in [106,105,108,103,12,14,13,13]:key(c)
  s=action('rapid-cancel-'+str(rep),'native navigation then Escape',lambda:key(1),'closed',lambda s:not s['visible'])
 toggle();time.sleep(.3);s=snap()
 s=action('M-backdrop','known empty bottom-right scene backdrop',lambda:click({'x':item(s,'depthos-overview-scene')['x']+item(s,'depthos-overview-scene')['w']-12,'y':item(s,'depthos-overview-scene')['y']+item(s,'depthos-overview-scene')['h']-12,'w':2,'h':2}),'closed',lambda s:not s['visible'])
 # Native focus recovery by pointer, not Workspace.activeWindow assignment.
 w=next(w for w in s['windows'] if w['uuid']==s['active']);g=w['geometry'];targetIndex=next(i for i,p in enumerate(children) if p.pid==w['pid'])
 def deliver_marker():
  command('move %d %d'%(g['x']+g['w']//2,g['y']+g['h']//2));command('button 272 1');command('button 272 0');time.sleep(.3)
  ev={'d':32,'e':18,'p':25,'t':20,'h':35,'o':24,'s':31,'-':12,'0':11,'2':3,'k':37}
  for ch in 'depthos-02e-ok':key(ev[ch])
  key(28);time.sleep(.5)
 s=action('K06-marker','owned non-shell marker receiver',deliver_marker,'exact marker acknowledged by clicked owned terminal',lambda s:(run/f'ack-{targetIndex}.json').exists() and s['active']==w['uuid'] and not s['effectActive'])
 # Contention through registered actions, observed independently via activeEffects.
 assert effects('loadEffect',poc1);invoke(poc1+'-toggle');time.sleep(.4)
 check('contention-other-active',poc1 in list(dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.freedesktop.DBus.Properties').Get('org.kde.kwin.Effects','activeEffects')))
 toggle();time.sleep(1.2);refused=snap()
 check('contention-bounded-refusal',not refused['visible'] and not refused['requested'] and not refused['effectActive'] and refused['outputs']==0)
 check('contention-other-preserved',poc1 in list(dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.freedesktop.DBus.Properties').Get('org.kde.kwin.Effects','activeEffects')))
 capture('contention-refused');key(1);time.sleep(.4)
 effects('unloadEffect',poc1)
 s=action('K01-after-contention','registered overview action',toggle,'explicit later activation succeeds',lambda s:s['visible'] and s['effectActive'] and s['outputs']==1)
 s=action('K05-final','native Escape',lambda:key(1),'inactive',lambda s:not s['visible'] and not s['effectActive'])
 # Settled quiet samples: recorder pauses while there are no interactions.
 s=action('K01-profile','registered overview action',toggle,'active for quiet profiling',lambda s:s['effectActive'])
 quiet=[]
 def quiet_sample(label):
  (run/'pause-recording').touch();time.sleep(.5)
  def sample():
   stat=Path(f'/proc/{kwin.pid}/stat').read_text().rsplit(')',1)[1].split();rss=next(x for x in Path(f'/proc/{kwin.pid}/status').read_text().splitlines() if x.startswith('VmRSS:'))
   return {'monotonic':time.monotonic(),'cpuTicks':int(stat[11])+int(stat[12]),'rssKiB':int(rss.split()[1])}
  first=sample();time.sleep(3);last=sample();quiet.append({'label':label,'first':first,'last':last,'cpuPercentOneCore':100*(last['cpuTicks']-first['cpuTicks'])/os.sysconf('SC_CLK_TCK')/(last['monotonic']-first['monotonic']),'capturePaused':True,'noInputDuringSample':True})
  (run/'pause-recording').unlink();time.sleep(.3)
 quiet_sample('settled previews enabled, two windows')
 s=action('M-previews-off','Disable previews',lambda:click(item(s,action='Disable previews')),'metadata fallback',lambda s:all(c['previewReason']=='Previews disabled' for c in cards(s)))
 quiet_sample('settled previews disabled, two windows')
 s=action('K05-profile','native Escape',lambda:key(1),'inactive after profile',lambda s:not s['effectActive'])
 quiet_sample('overlay closed')
 write('quiet-performance.json',{'scope':'isolated compositor, no screenshot requests during each 3s sample; not a host benchmark','samples':quiet})
 if observer:
  from native_observer import qualify
  qualify(observer,globals())
 write('performance.json',{'elapsedSeconds':time.monotonic()-started,'captureFrames':len(frames),'capturePurpose':'gate screenshots; recording is sampled visual evidence, not presentation timing','framePacing':'PENDING: no validated display presentation timestamp collector','renderer':'See version-matched support.txt; no adapter vendor assumed'})
except Exception as e:
 failed=True
 log('errors.jsonl',{'error':str(e),'traceback':traceback.format_exc()});print(traceback.format_exc(),flush=True)
finally:
 # Independent launcher bounds the entire child process group even if this path stalls.
 cleanup={'inputStopped':True,'effectsUnloaded':True,'appsStopped':True,'compositorStopped':True}
 if helper:
  try:helper.stdin.write('quit\n');helper.stdin.flush();helper.wait(timeout=3)
  except Exception:helper.terminate();helper.wait(timeout=3)
  cleanup['inputStopped']=helper.poll() is not None
 if recorder:
  (run/'stop-recording').touch();recorder.wait(timeout=12)
  if (run/'continuous-frames.json').exists():
   videoFrames=json.loads((run/'continuous-frames.json').read_text());frames=[(Path(f['path']),f['requestMonotonic']) for f in videoFrames]
 if observer:
  observer.close();cleanup['observerUnloaded']=True
 if 'poc1' in globals():
  try:effects('unloadEffect',poc1)
  except Exception:pass
 if 'package' in globals():
  try:effects('unloadEffect',package);cleanup['effectsUnloaded']=package not in list(dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.freedesktop.DBus.Properties').Get('org.kde.kwin.Effects','loadedEffects'))
  except Exception:cleanup['effectsUnloaded']=False
 for p in children:
  if p.poll() is None:p.terminate()
  try:p.wait(timeout=5)
  except subprocess.TimeoutExpired:p.kill();p.wait()
 cleanup['appsStopped']=all(p.poll() is not None for p in children)
 if 'dest' in globals() and dest.exists():shutil.rmtree(dest)
 if 'package' in globals():shutil.rmtree(run/'data/kwin/effects'/package,ignore_errors=True)
 if 'poc1' in globals():shutil.rmtree(run/'data/kwin/effects'/poc1,ignore_errors=True)
 cleanup['packagesRemoved']=not any((run/'data/kwin/effects').glob('depthos*'))
 if kwin and kwin.poll() is None:kwin.terminate();kwin.wait(timeout=10)
 cleanup['compositorStopped']=kwin is None or kwin.poll() is not None
 write('cleanup.json',cleanup);write('assertions.json',checks)
 try:
  # Encode the time-aligned gate captures; never mislabel these as full-rate presentation.
  if frames:
   concat=run/'frames.ffconcat';concat.write_text('ffconcat version 1.0\n'+''.join("file '"+str(p)+"'\nduration "+str(max(.1,frames[i+1][1]-t) if i+1<len(frames) else 1)+'\n' for i,(p,t) in enumerate(frames)))
   v=subprocess.run(['ffmpeg','-v','error','-y','-safe','0','-i',str(concat),'-fps_mode','vfr','-c:v','libvpx-vp9','-deadline','realtime','-cpu-used','8','-b:v','1M',str(run/'recording.webm')],capture_output=True,text=True,timeout=30)
   write('recording.json',{'kind':'continuous sampled ScreenShot2 capture, target 5 Hz; not display presentation timing','quietCapturePauses':'3s measurement intervals; no input during these pauses','frames':len(frames),'success':v.returncode==0,'presentationTiming':False})
 except Exception as e:
  write('recording-error.json',{'error':str(e)});failed=True
 print(json.dumps({'checks':len(checks),'passed':sum(x['pass'] for x in checks),'cleanup':cleanup}),flush=True)

if failed:sys.exit(1)
