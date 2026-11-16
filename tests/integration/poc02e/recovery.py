"""Independent, identity-checked recovery for the isolated process group only."""
import os,json,signal,time
from pathlib import Path
import dbus

def same_process(record):
 try:return Path(f"/proc/{record['pid']}/stat").read_text().rsplit(')',1)[1].split()[19]==str(record['startTicks'])
 except (FileNotFoundError,ProcessLookupError):return False

def running_process(record):
 if not same_process(record):return False
 try:return Path(f"/proc/{record['pid']}/stat").read_text().rsplit(')',1)[1].split()[0]!='Z'
 except FileNotFoundError:return False

def recover(run):
 result={'inputStopped':True,'effectUnloadAttempted':False,'scope':'private isolated compositor'}
 ip=run/'input-process.json'
 if ip.exists():
  record=json.loads(ip.read_text())
  if running_process(record):
   os.kill(record['pid'],signal.SIGTERM)
   for _ in range(30):
    if not running_process(record):break
    time.sleep(.05)
   result['inputStopped']=not running_process(record)
 if (run/'manifest.json').exists():
  m=json.loads((run/'manifest.json').read_text())
  assert m['scope']=='isolated virtual KWin' and Path(m['runtime']).resolve()==run/'runtime'
  if same_process({'pid':m['kwinPid'],'startTicks':m['processStartTicks']}):
   bus=dbus.bus.BusConnection(m['busAddress'])
   daemon=dbus.Interface(bus.get_object('org.freedesktop.DBus','/org/freedesktop/DBus'),'org.freedesktop.DBus')
   assert str(daemon.GetId())==m['busId']
   assert int(daemon.GetConnectionUnixProcessID('org.kde.KWin'))==m['kwinPid']
   effect=dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.kde.kwin.Effects')
   for identifier in m['packages']:
    assert identifier.startswith('depthos-02e-')
    effect.unloadEffect(identifier,timeout=3)
   result['effectUnloadAttempted']=True
   result['allOwnedEffectsUnloaded']=all(not effect.isEffectLoaded(i,timeout=3) for i in m['packages'])
 (run/'watchdog-recovery.json').write_text(json.dumps(result,indent=2))
 return result
