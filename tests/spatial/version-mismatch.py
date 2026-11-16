#!/usr/bin/env python3
"""A real incompatible bus peer must leave the production QML client unavailable."""
import os,sys,tempfile,subprocess,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
if '--inner' not in sys.argv:
 subprocess.run(['dbus-run-session','--',sys.executable,__file__,'--inner'],check=True);raise SystemExit()
import dbus,dbus.service
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib
DBusGMainLoop(set_as_default=True);bus=dbus.SessionBus();name=dbus.service.BusName('org.depthos.Controller',bus,do_not_queue=True)
class Incompatible(dbus.service.Object):
 @dbus.service.method('org.depthos.Controller1',in_signature='',out_signature='s')
 def GetCapabilities(self):return json.dumps({'schemaVersion':2,'instanceId':'fake','revision':'1','available':True,'data':{'readOnly':True,'assignmentMutations':False}})
peer=Incompatible(bus,'/org/depthos/Controller')
with tempfile.TemporaryDirectory(prefix='depthos-api-version-') as t:
 qml=Path(t)/'tst_version.qml';source=(ROOT/'experiments/kwin-overview/contents/ui/ControllerClient.qml').as_uri()
 qml.write_text('''import QtQuick
import QtTest
Item {Loader{id:b;source:"SOURCE"}TestCase{name:"RealVersionMismatch";when:windowShown;function test_refuse(){tryVerify(()=>b.item && b.item.state==="Backend unavailable",5000);compare(b.item.rows.length,0);compare(b.item.contextAvailable,false);verify(b.item.lastError.includes("schema"));}}}'''.replace('SOURCE',source))
 p=subprocess.Popen(['/usr/lib/qt6/bin/qmltestrunner','-input',str(qml)],env=dict(os.environ,QT_QPA_PLATFORM='offscreen',QT_QUICK_BACKEND='software'),stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
 loop=GLib.MainLoop();GLib.timeout_add(30,lambda:True if p.poll() is None else (loop.quit() or False));GLib.timeout_add_seconds(9,lambda:(loop.quit() or False));loop.run()
 if p.poll() is None:p.kill()
 out=p.communicate(timeout=3)[0];assert p.returncode==0,out
 print('PASS real schema-2 service rejected by production Plasma QML adapter')
