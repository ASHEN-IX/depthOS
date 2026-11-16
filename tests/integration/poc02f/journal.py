"""Retain only this run's fixed title-free diagnostic prefixes."""
import sys,subprocess,signal
from host import a
root=a.directory()
import json
cfg=json.loads((root/'effects.json').read_text());prefixes=[cfg['overview'],cfg['poc1'],'DEPTHOS02F_STATE']
p=subprocess.Popen(['journalctl','--user','-f','--since','now','-o','cat','_PID='+str(a.manifest()['kwinPid'])],stdout=subprocess.PIPE,text=True)
def stop(*args):p.terminate()
signal.signal(signal.SIGTERM,stop)
with (root/'scoped-kwin.log').open('a') as out:
 for line in p.stdout:
  if any(x in line for x in prefixes):out.write(line);out.flush()
p.wait()
