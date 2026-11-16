"""Recovery must not signal a recycled PID or confuse process comm parentheses."""
import os,sys,unittest,subprocess,time
from pathlib import Path
sys.path.insert(0,str(Path(__file__).parent))
from recovery import same_process,running_process
class RecoveryTests(unittest.TestCase):
 def test_current_process_identity(self):
  tick=Path('/proc/self/stat').read_text().rsplit(')',1)[1].split()[19]
  self.assertTrue(same_process({'pid':os.getpid(),'startTicks':tick}))
 def test_recycled_pid_refused(self):
  self.assertFalse(same_process({'pid':os.getpid(),'startTicks':'0'}))
 def test_missing_process_refused(self):
  self.assertFalse(same_process({'pid':2147483647,'startTicks':'0'}))
 def test_exited_unreaped_child_is_stopped(self):
  p=subprocess.Popen([sys.executable,'-c','pass'])
  try:
   tick=Path(f'/proc/{p.pid}/stat').read_text().rsplit(')',1)[1].split()[19]
   time.sleep(.15)
   self.assertFalse(running_process({'pid':p.pid,'startTicks':tick}))
  finally:p.wait()
if __name__=='__main__':unittest.main()
