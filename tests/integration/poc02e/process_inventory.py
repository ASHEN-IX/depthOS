"""Read-only identity inventory of the launcher's own process group."""
from pathlib import Path
import json, threading

def members(group):
 result=[]
 for path in Path('/proc').glob('[0-9]*/stat'):
  try:
   fields=path.read_text().rsplit(')',1)[1].split()
   if int(fields[2])==group:
    result.append({'pid':int(path.parent.name),'startTicks':fields[19],'state':fields[0],'group':group})
  except (FileNotFoundError,ProcessLookupError,PermissionError):pass
 return result

class Inventory:
 def __init__(self,group,path):
  self.group,self.path,self.records=group,path,{}
  self.stop=threading.Event();self.thread=threading.Thread(target=self.collect,daemon=True)
 def collect(self):
  while True:
   for rec in members(self.group):self.records[(rec['pid'],rec['startTicks'])]=rec
   self.path.write_text(json.dumps(list(self.records.values()),indent=2))
   if self.stop.wait(.1):return
 def start(self):self.thread.start()
 def close(self):self.stop.set();self.thread.join()
