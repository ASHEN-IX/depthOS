"""POC-02F scoped host preparation/recovery. No private KWin input interface."""
import sys,os,json,time,subprocess,importlib.util,signal
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
spec=importlib.util.spec_from_file_location('operator',ROOT/'tests/integration/poc02d.py');d=importlib.util.module_from_spec(spec);spec.loader.exec_module(d)
a=d.a;a.ART=ROOT/'artifacts/poc-02f'
def save(name,value):a.atomic(a.directory()/name,value)
def original_state():
 result={}
 if not (a.directory()/'original-windows.json').exists():return result
 for w in json.loads((a.directory()/'original-windows.json').read_text()):
  if not w.get('nativeId'):continue
  v=a.test_info(w['nativeId']);result[w['nativeId']]={k:v.get(k) for k in ['uuid','pid','desktopFile','desktops','minimized','fullscreen','x','y','width','height']}
 return a.canonical(result)
def stop_role(role):
 for rec in a.manifest()['processes']:
  if rec['role']==role:a.terminate_owned(rec)
def cleanup():
 if a.manifest().get('cleaned'):return
 a.require_session(a.manifest())
 # Sender is stopped and releases tracked held input before capture/control teardown.
 stop_role('native-input');time.sleep(.15)
 stop_role('recording');time.sleep(.15)
 stop_role('portal');time.sleep(.25)
 before=json.loads((a.directory()/'original-window-properties.json').read_text()) if (a.directory()/'original-window-properties.json').exists() else {}
 try:d.cleanup()
 finally:
  after=original_state();save('original-windows-after.json',after);save('original-window-comparison.json',{'equal':before==after,'before':before,'after':after})
 # Audit live processes, including unreaped-but-exited state, independently of normal a.cleanup.
 remaining=[]
 for rec in a.manifest()['processes']:
  if rec['role']=='guard':continue
  try:
   fields=Path(f"/proc/{rec['pid']}/stat").read_text().rsplit(')',1)[1].split()
   if fields[19]==rec['startTicks'] and fields[0]!='Z':remaining.append(rec)
  except FileNotFoundError:pass
 save('owned-process-check.json',{'remaining':remaining,'pass':not remaining})
def guard():
 from types import SimpleNamespace
 a.process_record(SimpleNamespace(pid=os.getpid()),'guard')
 while not a.manifest().get('cleaned'):
  if time.time()>float((a.directory()/'lease').read_text()):cleanup();return
  time.sleep(.5)
def prepare():
 a.baseline();m=a.manifest();m['phase']='POC-02F';a.save(m)
 out=(a.directory()/'guard.log').open('w');p=subprocess.Popen([sys.executable,__file__,'guard'],env={**os.environ,'DEPTHOS_ACCEPTANCE_RUN_DIRECTORY':str(a.directory())},stdout=out,stderr=subprocess.STDOUT,start_new_session=True)
 a.wait_for(lambda:any(r['role']=='guard' for r in a.manifest()['processes']));a.require_guard()
 flags=subprocess.check_output(['pkg-config','--cflags','--libs','libei-1.0'],text=True).split()
 subprocess.run(['gcc','-O2','-Wall','-Wextra','-Werror',str(ROOT/'tests/integration/poc02e/native-input.c'),'-o',str(a.directory()/'native-input'),*flags],check=True)
 m=a.manifest();m['packages'].append({'kind':'scripts','id':'depthos-system-observer'});m['scripts'].append('depthos-system-observer-session');a.save(m)
 a.command(['fish','scripts/install-observer.fish','--install']);a.start_receiver();a.command(['fish','scripts/observer-session.fish','--start']);a.wait_for(lambda:a.summary()['snapshots']>=1)
 initial=a.replica()['state'];save('observer-initial.json',a.replica());windows=[w for w in initial['windows'] if w['classification'] in ('application','dialog')];save('original-windows.json',windows);save('original-window-properties.json',original_state())
 b=json.loads((a.directory()/'baseline.json').read_text())
 empty=[str(x[1]) for x in b['desktops'] if a.empty_saved_test_layout(str(x[1]),b['tilingPreferences']) and not any(w['onAllDesktops'] or w['desktopIds'] is None or str(x[1]) in w['desktopIds'] for w in windows)]
 if not empty:raise RuntimeError('No existing empty desktop with safely restorable empty saved tiling; no host capture/test context created')
 desktop=empty[0];save('poc02d.json',{'desktop':desktop});m=a.manifest();m['testDesktop']=desktop;a.save(m)
 a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(desktop,variant_level=1));a.wait_for(lambda:a.replica()['state']['activeDesktopId']==desktop);time.sleep(1.3)
 # Receiver is a fixed literal parser, never a shell. Retain only its exact acknowledgement.
 marker=a.directory()/'marker.py';marker.write_text('import sys,os,json,time,select\nfrom pathlib import Path\nPath(sys.argv[1]).write_text(str(os.getpid()))\nprint("DEPTHOS POC-02F SAFE INPUT RECEIVER — not a shell",flush=True)\nend=time.monotonic()+600\nwhile time.monotonic()<end:\n if not select.select([sys.stdin],[],[],.25)[0]:continue\n line=sys.stdin.readline(256)\n if not line:break\n if line.rstrip("\\r\\n")=="depthos-02f-ok":\n  Path(sys.argv[2]).write_text(json.dumps({"matched":True,"marker":"depthos-02f-ok","time":time.time()}));print("INPUT VERIFIED",flush=True)\n del line\n')
 owned=[]
 for i in range(2):
  a.renew();pidfile=a.directory()/f'marker-{i}.pid';ack=a.directory()/f'ack-{i}.json';out=(a.directory()/f'terminal-{i}.log').open('w')
  proc=subprocess.Popen(['konsole','--separate','--workdir','/tmp','-e',sys.executable,str(marker),str(pidfile),str(ack)],stdout=out,stderr=subprocess.STDOUT,start_new_session=True);record=a.process_record(proc,'terminal')
  def find():
   for w in a.replica()['state']['windows']:
    if w['applicationId']=='org.kde.konsole' and w['nativeId'] and int(a.test_info(w['nativeId']).get('pid',-1))==proc.pid:return w
  w=a.wait_for(find);a.wait_for(pidfile.exists);child=int(pidfile.read_text());m=a.manifest();m['processes'].append({'pid':child,'startTicks':Path(f'/proc/{child}/stat').read_text().rsplit(')',1)[1].split()[19],'role':'testchild'});target={'pid':proc.pid,'native':w['nativeId']};m.setdefault('testWindows',[]).append(target);a.save(m)
  owned.append({**target,'startTicks':record['startTicks'],'appId':w['applicationId'],'desktopIds':w['desktopIds'],'ackFile':str(ack)})
 save('host-context.json',{'desktop':desktop,'windows':owned,'originalDesktop':b['currentDesktop'],'backgroundWindows':[{'native':w['nativeId'],'pid':int(a.test_info(w['nativeId']).get('pid',-1))} for w in initial['windows'] if w.get('nativeId') and (w.get('typeFlags',{}).get('desktopWindow') is True or w.get('typeFlags',{}).get('dock') is True)]});save('poc02d.json',{'desktop':desktop,'windows':owned});a.renew()
 print(json.dumps({'directory':str(a.directory()),'testDesktop':desktop,'ownedWindows':owned}),flush=True)
def preflight():
 result=a.detect_kwin_compatibility();print(json.dumps(result,indent=2),flush=True)
 if not result['approved']:raise a.CompatibilityError(result['reason'])
if __name__=='__main__':
 try:globals()[sys.argv[1]]()
 except a.CompatibilityError as error:
  print('Compatibility preflight BLOCKED: '+str(error),file=sys.stderr);sys.exit(2)
 except Exception:
  import traceback;traceback.print_exc()
  if sys.argv[1]=='prepare':
   try:cleanup()
   except Exception:traceback.print_exc()
  raise
