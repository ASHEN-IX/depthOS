"""One explicit fresh host attempt. KDE consent is the only operator checkpoint.
A denied/timed-out attempt is cleaned and never automatically retried.
"""
import json,sys,subprocess,time
from pathlib import Path
from host import ROOT,a,cleanup,save
HERE=Path(__file__).resolve().parent

def main():
 operation='preflight' if '--preflight-only' in sys.argv[1:] else 'prepare'
 preparation=subprocess.run([sys.executable,str(HERE/'host.py'),operation],cwd=ROOT)
 if preparation.returncode or operation=='preflight':return preparation.returncode
 portal=None;driver=None;result=1
 try:
  a.require_guard()
  portal=subprocess.Popen([sys.executable,str(HERE/'portal.py')],cwd=ROOT,stdout=(a.directory()/'portal.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True);a.process_record(portal,'portal')
  print('Complete the nonpersistent authorization in KDE. No chat reply grants access.',flush=True)
  deadline=time.monotonic()+205
  while time.monotonic()<deadline:
   path=a.directory()/'portal-status.json'
   try:status=json.loads(path.read_text())
   except (FileNotFoundError,json.JSONDecodeError):status={}
   if status.get('status')=='AUTHORIZED_READY':break
   if status.get('status') in ['BLOCKED','STOPPED_ERROR'] or portal.poll() is not None:
    save('automation-result.json',{'result':'BLOCKED','portal':status,'nativeHostAssertionsExecuted':0});result=2;return result
   time.sleep(.1)
  else:
   save('automation-result.json',{'result':'BLOCKED','reason':'No authorized ready state before bounded launcher deadline','nativeHostAssertionsExecuted':0});result=2;return result
  driver=subprocess.Popen([sys.executable,str(HERE/'run.py')],cwd=ROOT,stdout=(a.directory()/'test-runner.log').open('w'),stderr=subprocess.STDOUT,start_new_session=True)
  save('driver-process.json',{'pid':driver.pid,'startTicks':Path(f'/proc/{driver.pid}/stat').read_text().rsplit(')',1)[1].split()[19]})
  try:result=driver.wait(timeout=240)
  except subprocess.TimeoutExpired:
   driver.terminate();driver.wait(timeout=10);save('automation-result.json',{'result':'STOPPED','reason':'Native driver deadline exceeded'});result=1
  return result
 finally:
  (a.directory()/'stop-portal').touch()
  if portal and portal.poll() is None:
   try:portal.wait(timeout=4)
   except subprocess.TimeoutExpired:portal.terminate();portal.wait(timeout=4)
  cleanup()

if __name__=='__main__':sys.exit(main())
