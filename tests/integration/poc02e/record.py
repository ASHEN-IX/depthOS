"""Continuous sampled capture of the private compositor, never host desktop."""
import sys,os,time,json,threading
from pathlib import Path
import dbus
from PyQt6.QtGui import QImage
run=Path(sys.argv[1]).resolve();assert Path(os.environ['HOME']).resolve()==run/'home'
bus=dbus.SessionBus();folder=run/'recorded-frames';folder.mkdir();start=time.monotonic();frames=[];perf=[];pid=json.loads((run/'manifest.json').read_text())['kwinPid']
try:
 while not (run/'stop-recording').exists() and time.monotonic()-start<180:
  if (run/'pause-recording').exists():time.sleep(.05);continue
  begin=time.monotonic();r,w=os.pipe();chunks=[]
  def drain():
   try:
    while True:
     b=os.read(r,65536)
     if not b:break
     chunks.append(b)
   finally:os.close(r)
  t=threading.Thread(target=drain,daemon=True);t.start()
  try:meta=dbus.Interface(bus.get_object('org.kde.KWin','/org/kde/KWin/ScreenShot2'),'org.kde.KWin.ScreenShot2').CaptureWorkspace(dbus.Dictionary({'include-cursor':dbus.Boolean(True)},signature='sv'),dbus.types.UnixFd(w),signature='a{sv}h',timeout=8)
  finally:os.close(w)
  t.join(8);assert not t.is_alive()
  data=b''.join(chunks);path=folder/f'{len(frames):05d}.png'
  assert QImage(data,int(meta['width']),int(meta['height']),int(meta['stride']),QImage.Format(int(meta['format']))).copy().save(str(path))
  stat=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split();rss=next(l for l in Path(f'/proc/{pid}/status').read_text().splitlines() if l.startswith('VmRSS:'));perf.append({'monotonic':time.monotonic(),'cpuTicks':int(stat[11])+int(stat[12]),'rssKiB':int(rss.split()[1])})
  frames.append({'path':str(path),'requestMonotonic':begin,'receivedMonotonic':time.monotonic()})
  with (run/'capture-timestamps.jsonl').open('a') as f:f.write(json.dumps(frames[-1])+'\n')
  (run/'recording-ready').touch()
  time.sleep(max(0,.2-(time.monotonic()-begin)))
except Exception as e:(run/'recording-error.json').write_text(json.dumps({'error':str(e)}))
finally:
 (run/'resource-samples.json').write_text(json.dumps({'clockTicksPerSecond':os.sysconf('SC_CLK_TCK'),'samples':perf,'includesCaptureOverhead':True}))
 (run/'continuous-frames.json').write_text(json.dumps(frames))
