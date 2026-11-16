"""Deliberately stall an isolated input worker to exercise external recovery."""
import os,sys,time,json,subprocess,shutil,re
from pathlib import Path
import dbus
run=Path(sys.argv[1]);assert Path(os.environ['HOME'])==run/'home'
bus=dbus.SessionBus();repo=Path(__file__).resolve().parents[3];identifier='depthos-02e-watchdog-'+run.name[-6:];source=run/'package-source'/identifier;shutil.copytree(repo/'experiments/kwin-effect',source)
meta=json.loads((source/'metadata.json').read_text());meta['KPlugin']['Id']=identifier;(source/'metadata.json').write_text(json.dumps(meta))
qml=source/'contents/ui/main.qml';qml.write_text(qml.read_text().replace('depthos-spatial-poc-toggle',identifier+'-toggle').replace('Meta+Ctrl+Shift+F11',''))
subprocess.run(['kpackagetool6','--type','KWin/Effect','--packageroot',str(run/'data/kwin/effects'),'--install',str(source)],check=True)
p=subprocess.Popen(['kwin_wayland','--virtual','--no-lockscreen','--socket','depthos-watchdog-test'],stdout=(run/'scoped-kwin.log').open('w'),stderr=subprocess.STDOUT)
for _ in range(100):
 if bus.name_has_owner('org.kde.KWin'):break
 time.sleep(.05)
db=dbus.Interface(bus.get_object('org.freedesktop.DBus','/org/freedesktop/DBus'),'org.freedesktop.DBus');assert int(db.GetConnectionUnixProcessID('org.kde.KWin'))==p.pid
support=str(dbus.Interface(bus.get_object('org.kde.KWin','/KWin'),'org.kde.KWin').supportInformation());version=re.search(r'^KWin version: (.+)$',support,re.MULTILINE).group(1).strip();assert version==os.environ.get('DEPTHOS_ISOLATED_EXPECTED_VERSION',version)
(run/'version-verification.json').write_text(json.dumps({'running':version,'privatePid':p.pid,'ownerPidMatched':True}))
(run/'manifest.json').write_text(json.dumps({'scope':'isolated virtual KWin','runtime':str(run/'runtime'),'kwinPid':p.pid,'processStartTicks':Path(f'/proc/{p.pid}/stat').read_text().rsplit(')',1)[1].split()[19],'busAddress':os.environ['DBUS_SESSION_BUS_ADDRESS'],'busId':str(db.GetId()),'packages':[identifier]}))
assert dbus.Interface(bus.get_object('org.kde.KWin','/Effects'),'org.kde.kwin.Effects').loadEffect(identifier)
time.sleep(.2)
dbus.Interface(bus.get_object('org.kde.kglobalaccel','/component/kwin'),'org.kde.kglobalaccel.Component').invokeShortcut(identifier+'-toggle')
time.sleep(.4)
fd=dbus.Interface(bus.get_object('org.kde.KWin','/org/kde/KWin/EIS/RemoteDesktop'),'org.kde.KWin.EIS.RemoteDesktop').connectToEIS(dbus.Int32(63))[0].take()
h=subprocess.Popen([str(run/'native-input'),str(fd)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(run/'input.log').open('w'),pass_fds=[fd],text=True);os.close(fd)
assert h.stdout.readline().strip()=='READY'
(run/'input-process.json').write_text(json.dumps({'pid':h.pid,'startTicks':Path(f'/proc/{h.pid}/stat').read_text().rsplit(')',1)[1].split()[19]}))
h.stdin.write('key 42 1\n');h.stdin.flush();assert h.stdout.readline().strip()=='OK'
(run/'deliberately-stalled').touch();time.sleep(90)
