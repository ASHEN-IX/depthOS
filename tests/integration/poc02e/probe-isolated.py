import os,sys,subprocess,time,json
from pathlib import Path
import dbus
root=Path(sys.argv[1]).resolve();assert Path(os.environ['HOME']).resolve()==root/'home';assert Path(os.environ['XDG_RUNTIME_DIR']).resolve()==root/'runtime'
out=(root/'scoped-kwin.log').open('w')
p=subprocess.Popen(['kwin_wayland','--virtual','--no-lockscreen','--width','1920','--height','1080','--socket','depthos-isolated'],stdout=out,stderr=subprocess.STDOUT)
(root/'isolated-kwin.pid').write_text(str(p.pid))
try:
    bus=dbus.SessionBus()
    for _ in range(150):
        if p.poll() is not None:raise RuntimeError('isolated KWin exited')
        if bus.name_has_owner('org.kde.KWin'):break
        time.sleep(.1)
    owner=bus.get_name_owner('org.kde.KWin')
    print('Isolated KWin ready',owner,flush=True)
    iface=dbus.Interface(bus.get_object('org.kde.KWin','/org/kde/KWin/EIS/RemoteDesktop'),'org.kde.KWin.EIS.RemoteDesktop')
    result=iface.connectToEIS(dbus.Int32(63));fd=result[0].take();print('Isolated EIS fd acquired',flush=True)
    helper=subprocess.Popen([str(root/'native-input'),str(fd)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,pass_fds=[fd]);os.close(fd)
    ready=helper.stdout.readline().strip();print('helper',ready,flush=True)
    if not ready: print(helper.communicate(timeout=5),helper.returncode,flush=True);raise RuntimeError('helper handshake failed')
    helper.stdin.write('key 1 1\nkey 1 0\nquit\n');helper.stdin.flush();print(helper.communicate(timeout=5),flush=True)
    (root/'probe-ready.json').write_text(json.dumps({'owner':str(owner),'pid':p.pid,'bus':os.environ['DBUS_SESSION_BUS_ADDRESS']}))
finally:
    p.terminate();p.wait(timeout=10)
