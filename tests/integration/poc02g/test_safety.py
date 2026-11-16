"""Safety regressions for the added isolated qualification entry point."""
import importlib.util,json,os,subprocess,tempfile,time,unittest
from host_window_inventory import validate,FIELDS
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
spec=importlib.util.spec_from_file_location('inventory',ROOT/'tests/integration/poc02e/process_inventory.py');inventory=importlib.util.module_from_spec(spec);spec.loader.exec_module(inventory)
class SafetyTests(unittest.TestCase):
 def test_missing_host_windows_fail_closed(self):
  for records,info in [([],{}),([{'uuid':'{id}','pid':42}],{})]:
   with self.assertRaises(AssertionError):validate(records,lambda _:info)
 def test_changed_host_identity_fails_closed(self):
  info={k:0 for k in FIELDS};info.update(uuid='{id}',pid=99)
  with self.assertRaises(AssertionError):validate([{'uuid':'id','pid':42}],lambda _:info)
 def test_valid_host_inventory_is_complete_and_normalized(self):
  info={k:0 for k in FIELDS};info.update(uuid='{id}',pid=42)
  result=validate([{'uuid':'{id}','pid':42}],lambda _:info);self.assertEqual(result['id'],info)
 def test_inventory_records_exact_process_identity(self):
  own=next(x for x in inventory.members(os.getpgrp()) if x['pid']==os.getpid())
  self.assertEqual(own['startTicks'],Path(f'/proc/{os.getpid()}/stat').read_text().rsplit(')',1)[1].split()[19])
  self.assertEqual(inventory.members(-12345),[])
 def test_inventory_stops_and_persists(self):
  with tempfile.TemporaryDirectory() as d:
   path=Path(d)/'inventory.json';i=inventory.Inventory(os.getpgrp(),path);i.start();time.sleep(.15);i.close()
   self.assertFalse(i.thread.is_alive());self.assertIn(os.getpid(),[x['pid'] for x in json.loads(path.read_text())])
 def test_qualification_rejects_unspecified_version_before_host_access(self):
  p=subprocess.run(['python3',str(ROOT/'tests/integration/poc02e/launch.py'),'--artifact-group','poc-02g'],capture_output=True,text=True)
  self.assertEqual(p.returncode,2);self.assertIn('requires explicit --expected-version 6.8.3',p.stderr)
if __name__=='__main__':unittest.main()
