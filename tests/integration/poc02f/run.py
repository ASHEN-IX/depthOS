"""Authorized native host test. Start only after host.py prepare + portal.py consent.
Input is sent only by portal.py after fresh native context checks; no private EIS.
"""
import os,sys,json,time,subprocess,shutil,hashlib,traceback
from pathlib import Path
from host import a,save,cleanup,ROOT
a.require_guard() # A cleaned run must never be reused.
a.compatibility_gate() # Recheck before installing/loading test effects.
root=a.directory();context=json.loads((root/'host-context.json').read_text());checks=[];seq=0;started=time.monotonic();failed=False;effects={}
def log(name,value):
 with (root/name).open('a') as f:f.write(json.dumps(value)+'\n')
def renew():a.require_guard();a.renew()
def invoke(name):a.call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','invokeShortcut',name)
def check(name,value,detail=None):
 checks.append({'id':name,'pass':bool(value),'detail':detail});save('assertions.json',checks)
 if not value:raise AssertionError(name)
def snap():
 renew();before=(root/'scoped-kwin.log').stat().st_size;invoke(effects['probe'])
 for _ in range(60):
  time.sleep(.025)
  text=(root/'scoped-kwin.log').read_bytes()[before:].decode(errors='replace')
  for line in text.splitlines():
   if 'DEPTHOS02F_STATE ' in line:
    s=json.loads(line.split('DEPTHOS02F_STATE ',1)[1]);s['effectActive']=a.active(effects['overview']);s['monotonic']=time.monotonic();log('native-state.jsonl',s);return s
 raise TimeoutError('Native read-only probe response missing')
def command(cmd):
 global seq
 renew();status=json.loads((root/'portal-status.json').read_text());assert status['status']=='AUTHORIZED_READY',status
 seq+=1;a.atomic(root/'input-request.json',{'sequence':seq,'command':cmd,'monotonic':time.monotonic()})
 def done():
  status=json.loads((root/'portal-status.json').read_text())
  if status['status']!='AUTHORIZED_READY':raise RuntimeError(str(status))
  p=root/'input-result.json'
  return p.exists() and json.loads(p.read_text()).get('sequence')==seq
 a.wait_for(done,4)
def key(code):command(f'key {code} 1');command(f'key {code} 0')
def click(i):
 command('move %.2f %.2f'%(i['x']+i['w']/2,i['y']+i['h']/2));time.sleep(.12);command('button 272 1');command('button 272 0')
def item(s,name=None,action=None,index=0):
 rows=[i for i in s['items'] if i['visible'] and ((name and i['name']==name) or (action and i.get('action')==action))];assert len(rows)>index,(name,action);return rows[index]
def invariant(s):return (s['active'],sorted([(w['uuid'],w['desktops'],w['activities'],w['geometry']) for w in s['windows']],key=lambda x:x[0]))
def capture(label):
 # Portal consumer may be unavailable/denied. Never silently fall back to private screenshot APIs.
 if not (root/'capture-timestamps.jsonl').exists():return None
 start=time.monotonic()
 while time.monotonic()-start<2:
  rows=(root/'capture-timestamps.jsonl').read_text().splitlines()
  if rows:
   try:f=json.loads(rows[-1])
   except json.JSONDecodeError:continue
   if f['monotonic']>=start:
    target=root/'screenshots'/f'{len(list((root/"screenshots").glob("*.png"))):03d}-{label}.png';shutil.copyfile(root/f['path'],target);return str(target.relative_to(root))
  time.sleep(.04)
 return None
def action(name,target,fn,expected,predicate,method='portal/libei native Wayland'):
 renew();t=time.monotonic();fn();time.sleep(.45);s=snap();ok=bool(predicate(s));shot=capture(name)
 log('actions.jsonl',{'id':name,'method':method,'target':target,'startMonotonic':t,'endMonotonic':time.monotonic(),'wallTime':time.time(),'expected':expected,'observed':{k:s[k] for k in ['visible','effectActive','outputs','level','selected','active','motionOff','desktop','activity']},'screenshot':shot,'verdict':'PASS' if ok else 'FAIL'});check(name,ok);return s
try:
 renew();status=json.loads((root/'portal-status.json').read_text());assert status['status']=='AUTHORIZED_READY',status
 # Ensure portal UI has gone; changing this setup context never moves an original app.
 a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(context['desktop'],variant_level=1));time.sleep(1.3)
 suffix=a.manifest()['run'].split('-')[-1];identifier='depthos-02f-'+suffix;poc1='depthos-02f-contention-'+suffix;probe=identifier+'-read';toggle=identifier+'-toggle'
 effects={'overview':identifier,'poc1':poc1,'probe':probe,'toggle':toggle};save('effects.json',effects);(root/'screenshots').mkdir(exist_ok=True)
 for source,target in [('kwin-overview',identifier),('kwin-effect',poc1)]:
  dest=root/target;shutil.copytree(ROOT/'experiments'/source,dest);meta=json.loads((dest/'metadata.json').read_text());meta['KPlugin']['Id']=target;meta['KPlugin']['Name']='DepthOS scoped host acceptance';a.atomic(dest/'metadata.json',meta)
  qml=dest/'contents/ui/main.qml';text=qml.read_text()
  if source=='kwin-overview':
   inc=(ROOT/'tests/integration/poc02e/probe.qml.inc').read_text().replace('DEPTHOS_PROBE_ID',probe).replace('DEPTHOS02E_STATE','DEPTHOS02F_STATE')
   inc=inc.replace('if(w.normalWindow || w.dialog)','if(!w.deleted)').replace('ws.push({uuid:', 'ws.push({background:(w.desktopWindow===true||w.dock===true),uuid:')
   text=text.replace('    delegate: FocusScope {',inc+'\n    delegate: FocusScope {').replace('effect.constructedOutputs++;','effect.constructedOutputs++; effect.acceptanceViews.push(outputRoot);').replace('Component.onDestruction: effect.constructedOutputs = Math.max(0, effect.constructedOutputs - 1)','Component.onDestruction: {effect.constructedOutputs = Math.max(0,effect.constructedOutputs-1);effect.acceptanceViews=effect.acceptanceViews.filter(v=>v!==outputRoot);}')
   text=text.replace('depthos-spatial-overview-toggle',toggle).replace('[depthos-overview]','['+identifier+']');actions=[toggle,probe]
  else:text=text.replace('depthos-spatial-poc-toggle',poc1+'-toggle').replace('Meta+Ctrl+Shift+F11','').replace('[depthos-spatial-poc]','['+poc1+']');actions=[poc1+'-toggle']
  qml.write_text(text);m=a.manifest();m['packages'].append({'kind':'effects','id':target});m['effects'].append(target);m['newActions']+=actions;a.save(m)
  a.command(['kpackagetool6','--type','KWin/Effect','--packageroot',str(a.DATA/'kwin/effects'),'--install',str(dest)])
 save('source-hashes.json',{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in (ROOT/'experiments/kwin-overview').rglob('*') if p.is_file()})
 p=subprocess.Popen([sys.executable,str(Path(__file__).with_name('journal.py'))],stdout=(root/'journal-filter.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True);a.process_record(p,'journal');time.sleep(.3)
 a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',identifier);check('overview-loaded',a.call('org.kde.KWin','/Effects',a.EFFECTS,'loadEffect',identifier));time.sleep(.4)
 (root/'record-enable').touch()
 open_overview=lambda:invoke(toggle)
 s=action('A-open','registered overview action',open_overview,'actual output and Z1',lambda s:s['effectActive'] and s['outputs']==1 and s['level']==1,method='registered KGlobalAccel action')
 owned={w['native']:w['pid'] for w in context['windows']};cards=lambda s:[i for i in s['items'] if i['name']=='native-window-card' and i['visible']]
 check('exact-two-owned-cards',len(cards(s))==2 and {c['uuid'].strip('{}') for c in cards(s)}==set(owned))
 base=invariant(s);grid=lambda s:item(s,'native-window-grid')
 for n,c,index in [('right',106,1),('left',105,0),('down',108,1),('up',103,0)]:s=action('A-'+n,'actual grid',lambda c=c:key(c),'grid index '+str(index),lambda s,i=index:grid(s)['index']==i and invariant(s)==base)
 for n,c in [('enter',28),('space',57)]:s=action('A-'+n,'selected card',lambda c=c:key(c),'local UUID selection only',lambda s:bool(s['selected']) and invariant(s)==base)
 for n,c,level in [('minus',12,2),('backspace',14,3),('equals',13,2),('plus',13,1)]:
  def send(c=c,n=n):
   if n=='plus':command('key 42 1')
   key(c)
   if n=='plus':command('key 42 0')
  s=action('A-'+n,'depth navigation',send,'Z'+str(level),lambda s,l=level:s['level']==l and invariant(s)==base)
 b=json.loads((root/'baseline.json').read_text());check('real-host-desktops',set(s['desktops'])=={str(d[1]) for d in b['desktops']});check('real-host-activity',set(s['activities'])==set(b['activities']))
 s=action('A-escape','native Escape',lambda:key(1),'inactive and delegates gone',lambda s:not s['effectActive'] and s['outputs']==0)
 s=action('B-open','registered action',open_overview,'reopened',lambda s:s['effectActive'],method='registered KGlobalAccel action')
 for n in range(2):
  c=cards(s)[n];s=action('B-hover-'+str(n),'owned card',lambda c=c:command('move %.2f %.2f'%(c['x']+c['w']/2,c['y']+c['h']/2)),'hover true',lambda s,n=n:cards(s)[n]['hovered'])
  s=action('B-card-'+str(n),'owned UUID',lambda c=c:click(c),'local UUID selected',lambda s,c=c:s['selected']==c['uuid'] and invariant(s)==base)
 s=action('B-zoom-out','zoom control',lambda:click(item(s,action='− Zoom out')),'Z2',lambda s:s['level']==2)
 idx=s['desktops'].index(context['desktop']);c=next(i for i in s['items'] if i.get('contextIndex')==idx and i['visible'])
 s=action('B-desktop','same controlled native desktop card',lambda:click(c),'inspect owned context; native context stays',lambda s:s['level']==1 and s['desktop']==context['desktop'] and invariant(s)==base)
 for n,level in [('z2',2),('z3',3)]:s=action('B-'+n,'zoom out',lambda:click(item(s,action='− Zoom out')),'Z'+str(level),lambda s,l=level:s['level']==l)
 c=next(i for i in s['items'] if i.get('contextIndex')==0 and i['visible']);s=action('B-activity','actual existing Activity',lambda:click(c),'Z2 same Activity',lambda s:s['level']==2 and s['activity'] in b['activities'])
 s=action('B-zoom-in','zoom in',lambda:click(item(s,action='+ Zoom in')),'Z1',lambda s:s['level']==1)
 s=action('B-motion','reduced-motion control',lambda:click(item(s,action='Reduce motion')),'motion off',lambda s:s['motionOff'])
 s=action('B-close','close button',lambda:click(item(s,action='Close · Esc')),'inactive',lambda s:not s['effectActive'])
 open_overview();time.sleep(.35);s=snap();scene=item(s,'depthos-overview-scene');point={'x':scene['x']+scene['w']-12,'y':scene['y']+scene['h']-12,'w':2,'h':2}
 s=action('B-backdrop','current mapped backdrop',lambda:click(point),'inactive',lambda s:not s['effectActive'])
 w=next(w for w in s['windows'] if w['uuid']==s['active']);native=w['uuid'].strip('{}');check('C-owned-target',native in owned and w['pid']==owned[native]);g=w['geometry'];ack=next(Path(w['ackFile']) for w in context['windows'] if w['native']==native)
 def marker():
  click(g);time.sleep(.2)
  codes={'d':32,'e':18,'p':25,'t':20,'h':35,'o':24,'s':31,'-':12,'0':11,'2':3,'f':33,'k':37}
  for ch in 'depthos-02f-ok':key(codes[ch])
  key(28)
 s=action('C-marker','verified foreground non-shell receiver',marker,'exact acknowledgement and same UUID',lambda s:ack.exists() and s['active']==w['uuid'] and not s['effectActive'])
 check('D-other-loaded',a.call('org.kde.KWin','/Effects',a.EFFECTS,'loadEffect',poc1));invoke(poc1+'-toggle');time.sleep(.4);check('D-other-active',a.active(poc1));open_overview();time.sleep(1.2);s=snap();check('D-refusal-reset',not s['requested'] and s['outputs']==0 and not s['effectActive'] and a.active(poc1));capture('D-refused')
 key(1);time.sleep(.4);check('D-native-dismissal',not a.active(poc1));a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',poc1)
 s=action('D-explicit-reopen','registered overview action',open_overview,'later explicit success',lambda s:s['effectActive'],method='registered KGlobalAccel action')
 s=action('D-final-escape','native Escape',lambda:key(1),'inactive',lambda s:not s['effectActive'])
 save('performance.json',{'framePacing':'PENDING: no validated public presentation collector','hostTestSeconds':time.monotonic()-started,'captureRateIsNotPresentationRate':True})
except Exception as e:
 failed=True;save('test-error.json',{'error':str(e),'traceback':traceback.format_exc()});print(traceback.format_exc(),flush=True)
finally:
 (root/'stop-portal').touch();time.sleep(.3)
 try:cleanup()
 except Exception as e:save('cleanup-error.json',{'error':str(e)});failed=True
 framesfile=root/'recording-frames.json'
 if framesfile.exists():
  try:
   frames=json.loads(framesfile.read_text());concat=root/'frames.ffconcat'
   if frames:
    concat.write_text('ffconcat version 1.0\n'+''.join("file '"+str(root/f['path'])+"'\nduration "+str(max(.04,frames[i+1]['monotonic']-f['monotonic']) if i+1<len(frames) else .2)+'\n' for i,f in enumerate(frames)))
    v=subprocess.run(['ffmpeg','-v','error','-y','-safe','0','-i',str(concat),'-fps_mode','vfr','-c:v','libvpx-vp9','-deadline','realtime','-cpu-used','8','-b:v','1M',str(root/'recording.webm')],capture_output=True,text=True,timeout=45)
    save('recording.json',{'success':v.returncode==0,'sampledCapture':True,'presentationTiming':False,'frames':len(frames)})
  except Exception as e:save('recording-error.json',{'error':str(e)})
 save('assertions.json',checks);print(json.dumps({'nativeHostAssertions':len(checks),'passed':sum(x['pass'] for x in checks),'failed':failed}),flush=True)
if failed:sys.exit(1)
