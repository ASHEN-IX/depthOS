import sys,unittest,copy
from pathlib import Path
sys.path.insert(0,str(Path(__file__).parent))
from policy import valid_command,native_capture_scope
class PolicyTests(unittest.TestCase):
 def test_marker_and_navigation_keys_allowed(self):
  for k in [33,32,1,103,105,106,108,42]:self.assertTrue(valid_command(f'key {k} 1'))
 def test_arbitrary_shortcuts_and_buttons_refused(self):
  for text in ['key 125 1','key 29 1','button 273 1','key 1 2','quit','key 1 1\nkey 1 0']:self.assertFalse(valid_command(text),text)
 def test_offscreen_nonfinite_pointer_refused(self):
  for text in ['move -1 0','move 1920 0','move 0 1080','move nan 1','move inf 1']:self.assertFalse(valid_command(text),text)
 def setUp(self):
  self.ctx={'desktop':'test','windows':[{'native':'owned','pid':7}]};self.state={'active':'{owned}','windows':[{'uuid':'{owned}','pid':7,'desktops':['test']} ]}
 def test_owned_capture_allowed(self):self.assertTrue(native_capture_scope(self.state,self.ctx,'test'))
 def test_unknown_sticky_or_wrong_pid_refused(self):
  for window in [{'uuid':'other','pid':9,'desktops':['test']},{'uuid':'other','pid':9,'desktops':[]},{'uuid':'owned','pid':8,'desktops':['test']}]:
   s=copy.deepcopy(self.state);s['windows'].append(window);self.assertFalse(native_capture_scope(s,self.ctx,'test'))
 def test_wrong_desktop_or_foreign_focus_refused(self):
  self.assertFalse(native_capture_scope(self.state,self.ctx,'other'));self.state['active']='foreign';self.assertFalse(native_capture_scope(self.state,self.ctx,'test'))
 def test_unowned_popup_or_notification_refused(self):
  s=copy.deepcopy(self.state);s['windows'].append({'uuid':'notification','pid':44,'desktops':[],'background':False});self.assertFalse(native_capture_scope(s,self.ctx,'test'))
 def test_background_requires_recorded_uuid_and_pid(self):
  s=copy.deepcopy(self.state);s['windows'].append({'uuid':'{panel}','pid':44,'desktops':[],'background':True});self.assertFalse(native_capture_scope(s,self.ctx,'test'))
  self.ctx['backgroundWindows']=[{'native':'panel','pid':44}];self.assertTrue(native_capture_scope(s,self.ctx,'test'))
  self.ctx['backgroundWindows'][0]['pid']=45;self.assertFalse(native_capture_scope(s,self.ctx,'test'))
if __name__=='__main__':unittest.main()
