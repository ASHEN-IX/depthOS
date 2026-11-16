"""Run a bounded worker on a private bus/runtime. Never target the host EIS."""
import os,sys,json,subprocess,datetime,uuid,signal,time,shutil,argparse,hashlib
from recovery import recover
from process_inventory import Inventory, members
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
sys.path.insert(0,str(ROOT/'tests/integration'))
import importlib.util
spec=importlib.util.spec_from_file_location('acceptance',ROOT/'tests/integration/live-acceptance.py');a=importlib.util.module_from_spec(spec);spec.loader.exec_module(a)
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('worker',nargs='?',default='runner.py',choices=['runner.py','watchdog-test.py','probe-isolated.py']);parser.add_argument('--artifact-group',default='poc-02e',choices=['poc-02e','poc-02g']);parser.add_argument('--expected-version',choices=['6.8.1','6.8.3']);args=parser.parse_args()
if args.artifact_group=='poc-02g' and args.expected_version!='6.8.3':parser.error('POC-02G requires explicit --expected-version 6.8.3')
run=ROOT/'artifacts'/args.artifact_group/('run-'+datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')+'-'+uuid.uuid4().hex[:6]);run.mkdir(parents=True,mode=0o700)
for d in ['home','runtime','config','data','cache','state','screenshots']:(run/d).mkdir(mode=0o700)
sys.path.insert(0,str(ROOT/'tests/integration/poc02g'))
from host_window_inventory import collect
qtflags=subprocess.check_output(['pkg-config','--cflags','--libs','Qt6Gui'],text=True).split()
subprocess.run(['g++','-fPIC','-O2','-Wall','-Wextra',str(ROOT/'tests/integration/poc02g/window-inventory.cpp'),'-I/usr/include/KWayland','-lKWaylandClient',*qtflags,'-o',str(run/'host-window-inventory')],check=True)
def original_windows():
 return collect(run/'host-window-inventory',lambda identifier:a.call('org.kde.KWin','/KWin','org.kde.KWin','getWindowInfo',identifier))
windows_before=original_windows();(run/'original-windows-before.json').write_text(json.dumps(windows_before,indent=2))
before=a.state();(run/'baseline.json').write_text(json.dumps(before,indent=2))
flags=subprocess.check_output(['pkg-config','--cflags','--libs','libei-1.0'],text=True).split()
subprocess.run(['gcc','-O2','-Wall','-Wextra',str(Path(__file__).with_name('native-input.c')),'-o',str(run/'native-input'),*flags],check=True)
env={'PATH':os.environ['PATH'],'LANG':'C.UTF-8','QT_FORCE_STDERR_LOGGING':'1','QT_LOGGING_TO_CONSOLE':'1','HOME':str(run/'home'),'XDG_RUNTIME_DIR':str(run/'runtime'),'XDG_CONFIG_HOME':str(run/'config'),'XDG_DATA_HOME':str(run/'data'),'XDG_CACHE_HOME':str(run/'cache'),'XDG_STATE_HOME':str(run/'state'),'XDG_DATA_DIRS':'/usr/local/share:/usr/share','XDG_SESSION_TYPE':'wayland','XDG_CURRENT_DESKTOP':'KDE','KWIN_WAYLAND_NO_PERMISSION_CHECKS':'1','KWIN_SCREENSHOT_NO_PERMISSION_CHECKS':'1','KWIN_XKB_DEFAULT_KEYMAP':'1'}
if args.expected_version:env['DEPTHOS_ISOLATED_EXPECTED_VERSION']=args.expected_version
if args.artifact_group=='poc-02g':env['DEPTHOS_ISOLATED_QUALIFICATION']='6.8.3'
worker=Path(__file__).with_name(args.worker)
(run/'harness-source-hashes.json').write_text(json.dumps({str(f.relative_to(ROOT)):hashlib.sha256(f.read_bytes()).hexdigest() for folder in [worker.parent,ROOT/'tests/integration/poc02g'] for f in folder.glob('*') if f.is_file()},indent=2))
(run.parent/'current.json').write_text(json.dumps({'directory':str(run)}))
with (run/'runner.log').open('w') as log:
 p=subprocess.Popen(['dbus-run-session','--','python3',str(worker),str(run)],env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 (run/'launcher-process.json').write_text(json.dumps({'pid':p.pid,'startTicks':Path(f'/proc/{p.pid}/stat').read_text().rsplit(')',1)[1].split()[19],'group':p.pid}))
 inventory=Inventory(p.pid,run/'process-group-inventory.json');inventory.start()
 print(str(run),flush=True)
 try:p.wait(timeout=15 if worker.name=='watchdog-test.py' else (240 if worker.name=='runner.py' else 35))
 except subprocess.TimeoutExpired:
  try:recover(run)
  finally:
   os.killpg(p.pid,signal.SIGTERM)
   try:p.wait(timeout=5)
   except subprocess.TimeoutExpired:os.killpg(p.pid,signal.SIGKILL);p.wait()
 if worker.name=='watchdog-test.py':
  writeback=json.loads((run/'watchdog-recovery.json').read_text());writeback['workerStopped']=p.poll() is not None
  for identifier in json.loads((run/'manifest.json').read_text())['packages']:
   assert identifier.startswith('depthos-02e-');shutil.rmtree(run/'data/kwin/effects'/identifier,ignore_errors=True)
  writeback['packagesRemoved']=not any((run/'data/kwin/effects').glob('depthos*'))
  (run/'cleanup.json').write_text(json.dumps(writeback,indent=2))
 # Kill only the group created above; catches orphaned isolated clients/services.
 try:os.killpg(p.pid,signal.SIGTERM)
 except ProcessLookupError:pass
 time.sleep(.3)
 remaining=[x for x in members(p.pid) if x['state']!='Z']
 if remaining:
  try:os.killpg(p.pid,signal.SIGKILL)
  except ProcessLookupError:pass
  time.sleep(.3)
 inventory.close()
 remaining=[x for x in members(p.pid) if x['state']!='Z']
 (run/'process-group-cleanup.json').write_text(json.dumps({'remaining':remaining,'pass':not remaining},indent=2))
 windows_after=original_windows();(run/'original-windows-after.json').write_text(json.dumps(windows_after,indent=2))
 after=a.state();(run/'host-after.json').write_text(json.dumps(after,indent=2))
 (run/'host-comparison.json').write_text(json.dumps({'equal':before==after,'originalWindowsEqual':windows_before==windows_after,'changedKeys':[k for k in before if before[k]!=after.get(k)]},indent=2))
 (run/'evidence-index.json').write_text(json.dumps({'run':str(run),'assertions':'assertions.json','actions':'actions.jsonl','state':'native-state.jsonl','video':'recording.webm','cleanup':'cleanup.json','hostComparison':'host-comparison.json','screenshots':sorted(str(x.relative_to(run)) for x in (run/'screenshots').glob('*.png'))},indent=2))
 print('exit',p.returncode,flush=True)
 print((run/'runner.log').read_text()[-5000:])

if (p.returncode and worker.name!='watchdog-test.py') or before!=after or windows_before!=windows_after or remaining:sys.exit(1)
