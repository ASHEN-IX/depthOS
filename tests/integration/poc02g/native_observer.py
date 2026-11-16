"""Private native observer and owned-window fixtures; never a host entry point."""
import json, os, shutil, subprocess, time, uuid
from pathlib import Path
import dbus

class Observer:
 def __init__(self,bus,run,root,track):
  self.bus,self.run,self.root,self.track=bus,run,root,track
  assert Path(os.environ['HOME']).resolve()==run/'home'
  self.directory=run/'observer';self.directory.mkdir(mode=0o700)
  self.identifier='depthos-02g-observer-'+uuid.uuid4().hex[:10]
  self.receiver=None
 def call(self,method,*args):
  if method=='loadScript':return self.bus.call_blocking('org.kde.KWin','/Scripting','org.kde.kwin.Scripting',method,'ss',args,timeout=10)
  return getattr(dbus.Interface(self.bus.get_object('org.kde.KWin','/Scripting'),'org.kde.kwin.Scripting'),method)(*args)
 def wait(self,predicate):
  end=time.monotonic()+12
  while time.monotonic()<end:
   try:
    value=predicate()
    if value:return value
   except (FileNotFoundError,json.JSONDecodeError):pass
   time.sleep(.1)
  raise TimeoutError('private observer condition')
 def state(self):return json.loads((self.directory/'latest-snapshot.json').read_text())['state']
 def windows(self):return [w for w in self.state()['windows'] if w['classification'] in ('application','dialog')]
 def start(self):
  self.receiver=subprocess.Popen(['python3',str(self.root/'tests/integration/live-observer.py'),'--output',str(self.directory)],stdout=(self.directory/'receiver.log').open('w'),stderr=subprocess.STDOUT)
  self.track(self.receiver,'observer-receiver')
  self.wait(lambda:self.bus.name_has_owner('org.depthos.StateInspector'))
  src=self.run/'package-source'/self.identifier;shutil.copytree(self.root/'experiments/kwin-observer',src)
  metadata=json.loads((src/'metadata.json').read_text());metadata['KPlugin']['Id']=self.identifier;(src/'metadata.json').write_text(json.dumps(metadata))
  subprocess.run(['kpackagetool6','--type','KWin/Script','--packageroot',str(self.run/'data/kwin/scripts'),'--install',str(src)],check=True)
  number=int(self.call('loadScript',str(src/'contents/code/main.js'),self.identifier+'-session'));assert number>=0
  dbus.Interface(self.bus.get_object('org.kde.KWin',f'/Scripting/Script{number}'),'org.kde.kwin.Script').run()
  self.wait(lambda:(self.directory/'latest-snapshot.json').exists())
 def close(self):
  self.call('unloadScript',self.identifier+'-session')
  if self.receiver and self.receiver.poll() is None:self.receiver.terminate();self.receiver.wait(timeout=5)
  installed=self.run/'data/kwin/scripts'/self.identifier
  if installed.exists():shutil.rmtree(installed)
 def fixture(self,target,action,value):
  assert target['pid'] in self.owned and action in ('minimized','protected','fullscreen','desktops')
  params=json.dumps({'id':target['uuid'].strip('{}'),'pid':target['pid'],'action':action,'value':value})
  source='''const p=PARAMS;const ws=workspace.stackingOrder.filter(w=>String(w.internalId).replace(/[{}]/g,"")===p.id&&w.pid===p.pid&&w.desktopFileName==="org.kde.konsole");if(ws.length!==1)throw new Error("Owned target missing");const w=ws[0];if(p.action==="desktops")w.desktops=workspace.desktops.filter(d=>p.value.includes(d.id));else if(p.action==="minimized")w.minimized=p.value;else if(p.action==="protected")w.excludeFromCapture=p.value;else if(p.action==="fullscreen")w.fullScreen=p.value;'''.replace('PARAMS',params)
  name='depthos-02g-fixture-'+uuid.uuid4().hex[:10];path=self.run/(name+'.js');path.write_text(source)
  number=int(self.call('loadScript',str(path),name));assert number>=0
  try:dbus.Interface(self.bus.get_object('org.kde.KWin',f'/Scripting/Script{number}'),'org.kde.kwin.Script').run()
  finally:self.call('unloadScript',name)
  with (self.run/'fixtures.jsonl').open('a') as f:f.write(json.dumps({'target':target,'action':action,'value':value,'method':'public Window fixture, not input acceptance'})+'\n')
  time.sleep(.65)

def qualify(o,g):
 check,snap,action,key,toggle,capture=[g[k] for k in ('check','snap','action','key','toggle','capture')]
 cards=lambda s:[i for i in s['items'] if i['name']=='native-window-card' and i['visible']]
 o.owned={p.pid for p in g['children']}
 s=action('G-open','registered action',toggle,'overview active',lambda s:s['visible'] and s['effectActive'])
 s=action('G-previews','native preview control',lambda:g['click'](g['item'](s,action='Enable previews')),'native thumbnails eligible',lambda s:all(c['previewReason']=='' for c in cards(s)))
 check('G-model-observer-identities',{w['uuid'].strip('{}') for w in s['windows']}=={w['nativeId'].strip('{}') for w in o.windows()})
 p=subprocess.Popen(['konsole','--separate','--hide-menubar','--hide-tabbar','-e','python3',str(g['marker']),str(o.run/'ack-extra.json')],stdout=(o.run/'app-extra.log').open('w'),stderr=subprocess.STDOUT);g['children'].append(p);g['track'](p,'dynamic-test-window')
 o.wait(lambda:len(o.windows())==3);time.sleep(.4);s=snap();check('G-native-add',len(s['windows'])==3 and len(cards(s))==3);capture('G-added-three')
 p.terminate();p.wait(timeout=5);o.wait(lambda:len(o.windows())==2);time.sleep(.4);s=snap();check('G-native-remove',len(s['windows'])==2 and len(cards(s))==2);capture('G-removed-two')
 target=s['windows'][0]
 for flag,field,reason in [('protected','excludeFromCapture','protected'),('minimized','minimized','Minimized')]:
  o.fixture(target,flag,True);o.wait(lambda:any(w['nativeId'].strip('{}')==target['uuid'].strip('{}') and w[field] for w in o.windows()))
  s=snap();check('G-'+flag+'-fallback',any(c['uuid']==target['uuid'] and reason in c['previewReason'] for c in cards(s)));capture('G-'+flag)
  o.fixture(target,flag,False)
 o.fixture(target,'desktops',[g['ids'][1]])
 o.wait(lambda:any(w['nativeId'].strip('{}')==target['uuid'].strip('{}') and w['desktopIds']==[g['ids'][1]] for w in o.windows()))
 key(12);time.sleep(.25);key(106);key(28);time.sleep(.4);s=snap()
 check('G-inactive-desktop',s['desktop']==g['ids'][1] and any('Inactive desktop' in c['previewReason'] for c in cards(s)));capture('G-inactive-desktop')
 o.fixture(target,'desktops',[g['ids'][0]])
 key(12);time.sleep(.2);key(105);key(28);time.sleep(.3)
 action('G-fullscreen-dismiss','Escape',lambda:key(1),'closed',lambda s:not s['effectActive'])
 o.fixture(target,'fullscreen',True);o.wait(lambda:any(w['fullscreen'] for w in o.windows()))
 action('G-fullscreen-window','registered action',toggle,'normal fullscreen client coexists',lambda s:s['effectActive'] and len(cards(s))==2);capture('G-fullscreen-client')
 o.fixture(target,'fullscreen',False)
 starts=0
 for n in range(6):
  action('G-cycle-close-'+str(n),'Escape',lambda:key(1),'no delegates',lambda s:not s['effectActive'] and s['outputs']==0)
  s=action('G-cycle-open-'+str(n),'registered action',toggle,'thumbnails re-created',lambda s:s['effectActive'] and s['outputs']==1 and len(cards(s))==2 and all(c['previewReason']=='' for c in cards(s)));starts+=1
 action('G-final-escape','Escape',lambda:key(1),'inactive',lambda s:not s['effectActive'] and s['outputs']==0)
 initial=snap()['active']
 def alternate():
  g['command']('key 56 1');key(15);g['command']('key 56 0')
 s=action('G-focus-alt-tab','native Alt+Tab',alternate,'other owned client receives focus',lambda s:s['active']!=initial and s['active'] in [w['uuid'] for w in s['windows']])
 def synchronized_focus():
  state=o.state();return any(w['nativeId'].strip('{}')==s['active'].strip('{}') and w['runtimeId']==state['activeWindowId'] for w in state['windows'])
 check('G-observer-focus',o.wait(synchronized_focus))
 summary=json.loads((o.directory/'summary.json').read_text())
 check('G-observer-clean-stream',summary['errors']==0 and summary['removalConsistent'] and not summary['needSnapshot'],summary)
 (o.run/'qualification.json').write_text(json.dumps({'settledCycleStarts':starts,'expectedContentionRefusals':1,'observer':summary,'framePacing':'PENDING'},indent=2))
