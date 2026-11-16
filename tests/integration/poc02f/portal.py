"""Nonpersistent OS-consented portal broker. No private EIS, uinput, network or autostart.
Only this run's bounded input commands are accepted, with native privacy checks.
"""
import os,sys,time,json,signal,subprocess,select
from pathlib import Path
import dbus
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib
DBusGMainLoop(set_as_default=True)
from host import a,save
from policy import valid_command,native_capture_scope
a.require_guard() # Refuse a cleaned/stale run before any session or evidence mutation.
a.compatibility_gate() # Recheck before requesting OS authorization.
bus=dbus.SessionBus();root=a.directory();session=None;helper=None;stop=False;pipeline=None;recordFrames=[];authorized=False
P='org.freedesktop.portal.Desktop';PATH='/org/freedesktop/portal/desktop';RD='org.freedesktop.portal.RemoteDesktop';SC='org.freedesktop.portal.ScreenCast'
iface=dbus.Interface(bus.get_object(P,PATH),RD)
def stopped(*args):
 global stop;stop=True
signal.signal(signal.SIGTERM,stopped);signal.signal(signal.SIGINT,stopped)
def events():
 ctx=GLib.MainContext.default()
 while ctx.pending():ctx.iteration(False)
def request(method,args,options,timeout=180):
 token='depthos02f_'+os.urandom(6).hex();options={**options,'handle_token':dbus.String(token)}
 sender=bus.get_unique_name().lstrip(':').replace('.','_');expected=f'/org/freedesktop/portal/desktop/request/{sender}/{token}';result=[]
 match=bus.add_signal_receiver(lambda code,data:result.append((int(code),data)),signal_name='Response',dbus_interface='org.freedesktop.portal.Request',path=expected,bus_name=P)
 save('portal-last-request.json',{'method':method.__name__ if hasattr(method,'__name__') else str(method),'monotonic':time.monotonic(),'timeoutSeconds':timeout})
 handle=method(*args,dbus.Dictionary(options,signature='sv'));assert str(handle)==expected
 deadline=time.monotonic()+timeout
 try:
  while not result and not stop and time.monotonic()<deadline:
   events();a.renew();time.sleep(.05)
  if not result:
   dbus.Interface(bus.get_object(P,handle),'org.freedesktop.portal.Request').Close();raise TimeoutError('Portal response not received; authorization was not established')
  code,data=result[0];return code,data
 finally:match.remove()
def safe_context():
 try:
  a.require_session(a.manifest());c=json.loads((root/'host-context.json').read_text());s=a.replica()['state']
  if s['activeDesktopId']!=c['desktop']:return False
  health=a.summary()
  if health.get('needSnapshot') or health.get('errors') or health.get('packetAgeSeconds') is None or health['packetAgeSeconds']>3:return False
  normal=[w for w in s['windows'] if not (w.get('typeFlags',{}).get('desktopWindow') is True or w.get('typeFlags',{}).get('dock') is True)]
  owned={w['native']:w['pid'] for w in c['windows']}
  for w in normal:
   visible=w['onAllDesktops'] or w['desktopIds'] is None or c['desktop'] in w['desktopIds']
   if visible and (w['nativeId'] not in owned or int(a.test_info(w['nativeId']).get('pid',-1))!=owned[w['nativeId']]):return False
  active=next((w for w in s['windows'] if w['runtimeId']==s['activeWindowId']),None)
  if active is not None:return active.get('nativeId') in owned
  return False
 except Exception:return False
try:
 a.require_guard()
 props=dbus.Interface(bus.get_object(P,PATH),'org.freedesktop.DBus.Properties')
 save('portal-capabilities.json',{'remoteDesktop':dict(props.GetAll(RD)),'screenCast':dict(props.GetAll(SC)),'persistMode':0,'clipboardRequested':False})
 code,data=request(iface.CreateSession,[],{'session_handle_token':dbus.String('depthos02f_'+os.urandom(6).hex())})
 if code:raise RuntimeError('CreateSession response '+str(code))
 session=dbus.ObjectPath(data['session_handle']);m=a.manifest();m['portalSession']=str(session);a.save(m)
 bus.add_signal_receiver(stopped,signal_name='Closed',dbus_interface='org.freedesktop.portal.Session',path=str(session),bus_name=P)
 code,_=request(iface.SelectDevices,[session],{'types':dbus.UInt32(3),'persist_mode':dbus.UInt32(0)})
 if code:raise RuntimeError('SelectDevices response '+str(code))
 # This is a request for consent, never consent fabrication. No stream is consumed yet.
 sc=dbus.Interface(bus.get_object(P,PATH),SC)
 code,_=request(sc.SelectSources,[session],{'types':dbus.UInt32(1),'multiple':dbus.Boolean(False),'cursor_mode':dbus.UInt32(2)})
 if code:raise RuntimeError('SelectSources response '+str(code))
 save('portal-status.json',{'status':'WAITING_FOR_OS_AUTHORIZATION','inputDevices':['keyboard','pointer'],'capture':'single monitor, consumed only under test-context guards','persistMode':0})
 code,data=request(iface.Start,[session,dbus.String('')],{})
 save('portal-start-result.json',{'response':code,'devices':int(data.get('devices',0)),'streams':a.canonical(data.get('streams',[])),'restoreTokenReturned':'restore_token' in data,'persistentPermissionRequested':False})
 if code!=0:raise RuntimeError('OS portal authorization was denied or failed (response '+str(code)+')')
 if int(data.get('devices',0))&3!=3:raise RuntimeError('OS did not grant both keyboard and pointer')
 authorized=True
 fd=iface.ConnectToEIS(session,dbus.Dictionary({},signature='sv')).take()
 helper=subprocess.Popen([str(root/'native-input'),str(fd)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=(root/'input.log').open('w'),text=True,bufsize=1,pass_fds=[fd]);os.close(fd);a.process_record(helper,'native-input')
 if not select.select([helper.stdout],[],[],7)[0] or helper.stdout.readline().strip()!='READY':raise RuntimeError('Portal EIS device negotiation failed')
 save('portal-status.json',{'status':'AUTHORIZED_READY','devices':3,'streamCount':len(data.get('streams',[])),'persistMode':0})
 streams=data.get('streams',[])
 if streams:
  import gi
  gi.require_version('Gst','1.0')
  from gi.repository import Gst
  from PyQt6.QtGui import QImage
  Gst.init(None)
  pwfd=sc.OpenPipeWireRemote(session,dbus.Dictionary({},signature='sv')).take()
  pipeline=Gst.parse_launch('pipewiresrc fd='+str(pwfd)+' path='+str(int(streams[0][0]))+' do-timestamp=true ! videoconvert ! videorate ! video/x-raw,format=RGB,framerate=5/1 ! appsink name=capture emit-signals=false max-buffers=1 drop=true sync=false')
  sink=pipeline.get_by_name('capture');(root/'recorded-frames').mkdir(exist_ok=True)
  pipelineStarted=False
 def native_capture_guard():
  if not safe_context() or not (root/'effects.json').exists():return False
  cfg=json.loads((root/'effects.json').read_text());file=root/'scoped-kwin.log'
  if not file.exists():return False
  before=file.stat().st_size
  a.call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','invokeShortcut',cfg['probe'])
  for _ in range(40):
   time.sleep(.015);events()
   for line in file.read_bytes()[before:].decode(errors='replace').splitlines():
    if 'DEPTHOS02F_STATE ' in line:
     state=json.loads(line.split('DEPTHOS02F_STATE ',1)[1]);ctx=json.loads((root/'host-context.json').read_text())
     return native_capture_scope(state,ctx,str(a.prop('/VirtualDesktopManager',a.VDM,'current')))
  return False
 deadline=time.monotonic()+600;sequence=0
 while not stop and time.monotonic()<deadline:
  events();a.renew()
  if (root/'stop-portal').exists():break
  path=root/'input-request.json'
  if path.exists():
   try:r=json.loads(path.read_text())
   except json.JSONDecodeError:time.sleep(.01);continue
   if r['sequence']>sequence:
    if r['sequence']!=sequence+1 or time.monotonic()-r['monotonic']>3 or not valid_command(r['command']) or not safe_context():raise RuntimeError('Native input request failed sequence/age/allowlist/privacy guard')
    helper.stdin.write(r['command']+'\n');helper.stdin.flush()
    if not select.select([helper.stdout],[],[],3)[0] or helper.stdout.readline().strip()!='OK':raise RuntimeError('Native sender acknowledgement missing')
    sequence=r['sequence'];save('input-result.json',{'sequence':sequence,'success':True,'monotonic':time.monotonic()})
  if streams and (root/'record-enable').exists():
   if not native_capture_guard():raise RuntimeError('Capture stopped: fresh native privacy guard rejected context')
   if not pipelineStarted:
    pipeline.set_state(Gst.State.PLAYING);pipelineStarted=True
   sample=sink.emit('try-pull-sample',0)
   if sample:
    stamp=time.monotonic();caps=sample.get_caps().get_structure(0);width=caps.get_value('width');height=caps.get_value('height');buffer=sample.get_buffer();ok,mapping=buffer.map(Gst.MapFlags.READ)
    if ok:
     try:
      # An accepted frame is copied in memory, then guarded again before saving.
      image=QImage(bytes(mapping.data),width,height,len(mapping.data)//height,QImage.Format.Format_RGB888).copy()
     finally:buffer.unmap(mapping)
     if not native_capture_guard():raise RuntimeError('Capture discarded: post-frame privacy guard rejected context')
     f=root/'recorded-frames'/('%05d.png'%len(recordFrames));assert image.save(str(f));row={'path':str(f.relative_to(root)),'monotonic':stamp,'pipewirePtsNanoseconds':int(buffer.pts),'requestScope':'authorized monitor on controlled test desktop','preAndPostNativeGuard':True};recordFrames.append(row)
     with (root/'capture-timestamps.jsonl').open('a') as output:output.write(json.dumps(row)+'\n')
  time.sleep(.005)
except Exception as e:
 save('portal-status.json',{'status':'BLOCKED' if not authorized else 'STOPPED_ERROR','reason':str(e),'authorized':authorized})
finally:
 if helper and helper.poll() is None:
  try:helper.stdin.write('quit\n');helper.stdin.flush();helper.wait(timeout=3)
  except Exception:helper.terminate();helper.wait(timeout=3)
 if pipeline:
  pipeline.set_state(Gst.State.NULL)
  save('recording-frames.json',recordFrames)
 if session:
  try:dbus.Interface(bus.get_object(P,session),'org.freedesktop.portal.Session').Close();save('portal-close.json',{'closed':True})
  except dbus.DBusException as e:save('portal-close.json',{'alreadyClosedOrError':str(e)})
 save('portal-process-exit.json',{'inputStopped':helper is None or helper.poll() is not None,'authorized':authorized,'normalStop':stop or (root/'stop-portal').exists()})
