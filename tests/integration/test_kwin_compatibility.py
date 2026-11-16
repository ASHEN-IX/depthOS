"""Bus-free version policy and preflight boundary regressions."""
import importlib.util,json,os,subprocess,tempfile,unittest
from pathlib import Path
from unittest.mock import Mock,patch
from contextlib import ExitStack

def harness():
 spec=importlib.util.spec_from_file_location('version_gate',Path(__file__).with_name('live-acceptance.py'));m=importlib.util.module_from_spec(spec)
 with patch('dbus.SessionBus',return_value=Mock()):spec.loader.exec_module(m)
 return m
class KWinCompatibilityTests(unittest.TestCase):
 def setUp(self):self.a=harness()
 def test_verified_release_and_whitespace(self):
  for raw in ['kwin 6.8.1\n','  kwin   6.8.1 \n','kwin_wayland 6.8.1']:
   p=self.a.kwin_compatibility(raw,'System\nKWin version: 6.8.1\nQt Version: 6.12.0');self.assertTrue(p['approved']);self.assertEqual(p['running'],'6.8.1')
 def test_unaudited_release_is_not_automatically_accepted(self):
  for v in ['6.8.10','6.9.0','7.0.0','6.8.0']:
   p=self.a.kwin_compatibility('kwin '+v,'KWin version: '+v);self.assertFalse(p['approved']);self.assertIn('not source-audited',p['reason'])
 def test_isolated_qualification_does_not_authorize_host_preparation(self):
  p=self.a.kwin_compatibility('kwin 6.8.3','KWin version: 6.8.3')
  self.assertEqual(p['runningQualification'],'ISOLATED_QUALIFIED');self.assertFalse(p['approved']);self.assertFalse(p['hostSessionQualified']);self.assertIn('host-session qualification is pending',p['reason'])
 def test_qualification_does_not_waive_mismatch(self):
  p=self.a.kwin_compatibility('kwin 6.8.3','KWin version: 6.8.1')
  self.assertEqual(p['installedQualification'],'ISOLATED_QUALIFIED');self.assertEqual(p['runningQualification'],'HISTORICALLY_VALIDATED');self.assertFalse(p['approved']);self.assertIn('mismatch',p['reason'])
 def test_qualification_registry_has_no_wildcards_or_shared_mutable_results(self):
  p=self.a.kwin_compatibility('kwin 6.8.1','KWin version: 6.8.1');p['releaseQualifications']['6.8.3']['classification']='HOST_PASS'
  self.assertEqual(self.a.KWIN_QUALIFICATIONS['6.8.3']['classification'],'ISOLATED_QUALIFIED');self.assertEqual(set(self.a.KWIN_QUALIFICATIONS),{'6.8.1','6.8.3'})
 def test_mismatch_both_directions(self):
  for i,r in [('6.8.3','6.8.1'),('6.8.1','6.8.3')]:
   p=self.a.kwin_compatibility('kwin '+i,'KWin version: '+r);self.assertFalse(p['approved']);self.assertIn('mismatch',p['reason'])
 def test_unexpected_installed_output_fails_closed(self):
  for raw in ['', 'KWin version: 6.8.1','kwin 6.8.1-rc1','kwin 6.8.1.2','kwin 6.8.1\nunexpected','kwin 6.8']:
   self.assertFalse(self.a.kwin_compatibility(raw,'KWin version: 6.8.1')['approved'],raw)
 def test_ambiguous_or_missing_running_output_fails_closed(self):
  for raw in ['', 'Qt Version: 6.8.1','KWin version: unknown','KWin version: 6.8.1\nKWin version: 6.8.1']:
   self.assertFalse(self.a.kwin_compatibility('kwin 6.8.1',raw)['approved'],raw)
 def test_detection_does_not_log_to_previous_run(self):
  with patch.object(self.a,'owner',return_value=':1.9'),patch.object(self.a,'call',side_effect=['bus',12,'KWin version: 6.8.1']),patch.object(self.a,'require_session'),patch.object(self.a.subprocess,'run',return_value=subprocess.CompletedProcess([],0,'kwin 6.8.1\n','')),patch.object(self.a,'command') as command:
   p=self.a.detect_kwin_compatibility();self.assertTrue(p['approved']);command.assert_not_called()
 def test_failed_binary_probe_and_changed_identity_block(self):
  for rc,changed in [(1,False),(0,True)]:
   with patch.object(self.a,'owner',return_value=':1.9'),patch.object(self.a,'call',side_effect=['bus',12,'KWin version: 6.8.1']),patch.object(self.a,'require_session',side_effect=AssertionError if changed else None),patch.object(self.a.subprocess,'run',return_value=subprocess.CompletedProcess([],rc,'kwin 6.8.1\n','')):
    self.assertFalse(self.a.detect_kwin_compatibility()['approved'])
 def baseline_case(self,approved):
  a=self.a;p=a.kwin_compatibility('kwin 6.8.1' if approved else 'kwin 6.8.3','KWin version: 6.8.1');p.update(owner=':1.9',busId='bus',kwinPid=12)
  with tempfile.TemporaryDirectory() as temp,ExitStack() as stack:
   a.ART=Path(temp)/'runs'
   stack.enter_context(patch.dict(os.environ,{'XDG_SESSION_TYPE':'wayland','DEPTHOS_ACCEPTANCE_RUN_DIRECTORY':''}))
   stack.enter_context(patch.object(a.os,'getuid',return_value=1000));stack.enter_context(patch.object(a,'detect_kwin_compatibility',return_value=p));stack.enter_context(patch.object(a,'require_session'))
   stack.enter_context(patch.object(a,'state',return_value={'owner':':1.9','busId':'bus','kwinPid':12,'activeEffects':[]}));stack.enter_context(patch.object(a.subprocess,'run',return_value=subprocess.CompletedProcess([],0,'','')))
   stack.enter_context(patch.object(a,'scripting_loaded',return_value=False));calls=stack.enter_context(patch.object(a,'call',return_value=False))
   if approved:a.baseline()
   else:
    with self.assertRaises(a.CompatibilityError):a.baseline()
    calls.assert_not_called() # no script, effect, portal, or desktop operation
   directory=Path(json.loads((a.ART/'current.json').read_text())['directory']);m=json.loads((directory/'manifest.json').read_text());b=json.loads((directory/'baseline.json').read_text())
   self.assertEqual(m['kwinCompatibility'],p);self.assertEqual(b['kwinCompatibility'],p);self.assertEqual(b['kwinVersion'],'6.8.1');self.assertEqual(m['cleaned'],not approved)
   if not approved:self.assertFalse(m['preparationStarted']);self.assertEqual(m['processes'],[]);self.assertFalse((a.ART/'baseline.json').exists())
 def test_supported_manifest_records_approved_versions(self):self.baseline_case(True)
 def test_blocked_manifest_records_mismatch_without_live_setup(self):self.baseline_case(False)
 def test_accept_stops_before_portal_when_preparation_blocks(self):
  spec=importlib.util.spec_from_file_location('accept_gate',Path(__file__).parent/'poc02f/accept.py');m=importlib.util.module_from_spec(spec)
  with patch.dict('sys.modules',{'host':Mock(ROOT=Path('/repo'),a=Mock())}):spec.loader.exec_module(m)
  with patch.object(m.sys,'argv',['accept.py']),patch.object(m.subprocess,'run',return_value=subprocess.CompletedProcess([],2)),patch.object(m.subprocess,'Popen') as spawn:
   self.assertEqual(m.main(),2);spawn.assert_not_called()
if __name__=='__main__':unittest.main()
