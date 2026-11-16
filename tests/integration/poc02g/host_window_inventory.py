"""Validate read-only protocol UUIDs against supported KWin D-Bus metadata."""
import json,os,subprocess
FIELDS=('uuid','pid','desktopFile','desktops','minimized','fullscreen','excludeFromCapture','x','y','width','height')
def validate(records,query):
 assert records,'Empty host window inventory is not preservation evidence'
 result={}
 for rec in records:
  identifier=rec['uuid'].strip('{}');assert identifier and identifier not in result
  info=query(identifier)
  assert info and str(info.get('uuid','')).strip('{}')==identifier and int(info.get('pid',0))==rec['pid'], 'Missing or changed host window identity'
  assert all(k in info for k in FIELDS),'Incomplete host window properties'
  result[identifier]={k:info[k] for k in FIELDS}
 return json.loads(json.dumps(result))
def collect(executable,query):
 records=json.loads(subprocess.check_output([str(executable)],env={**os.environ,'QT_QPA_PLATFORM':'wayland'},text=True,timeout=8))
 return validate(records,query)
