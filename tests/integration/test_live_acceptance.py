"""No native bus: regression for overloaded KWin scripting calls in live harness."""
import importlib.util
from pathlib import Path
import unittest
from unittest.mock import Mock, patch
import tempfile
import time

class AcceptanceHarnessTests(unittest.TestCase):
    def test_two_argument_load_uses_explicit_signature(self):
        path=Path(__file__).with_name('live-acceptance.py')
        spec=importlib.util.spec_from_file_location('live_acceptance_test',path)
        module=importlib.util.module_from_spec(spec)
        bus=Mock()
        with patch('dbus.SessionBus',return_value=bus):
            spec.loader.exec_module(module)
        module.call('org.kde.KWin','/Scripting',module.SCRIPTING,'loadScript','/test/owned.js','depthos-test')
        bus.call_blocking.assert_called_once_with('org.kde.KWin','/Scripting',module.SCRIPTING,'loadScript','ss',('/test/owned.js','depthos-test'),timeout=10)
        bus.get_object.assert_not_called()

    def test_comparison_ignores_only_recorded_test_effects(self):
        path=Path(__file__).with_name('live-acceptance.py')
        spec=importlib.util.spec_from_file_location('live_acceptance_compare_test',path)
        module=importlib.util.module_from_spec(spec)
        with patch('dbus.SessionBus',return_value=Mock()):
            spec.loader.exec_module(module)
        self.assertEqual(module.canonical([(0, 'id', 'Desktop 1')]), [[0, 'id', 'Desktop 1']])
        self.assertTrue(module.unrelated_effects_match(['blur'],['blur','depthos-test'],['depthos-test']))
        self.assertFalse(module.unrelated_effects_match(['blur'],['depthos-test'],['depthos-test']))
        self.assertFalse(module.unrelated_effects_match(['blur'],['blur','foreign-effect'],['depthos-test']))

    def test_cleaned_or_expired_runs_cannot_start_live_stages(self):
        path=Path(__file__).with_name('live-acceptance.py')
        spec=importlib.util.spec_from_file_location('live_acceptance_guard_test',path)
        module=importlib.util.module_from_spec(spec)
        with patch('dbus.SessionBus',return_value=Mock()):
            spec.loader.exec_module(module)
        with patch.object(module,'manifest',return_value={'cleaned':True}):
            with self.assertRaises(AssertionError):module.require_guard()
        with tempfile.TemporaryDirectory() as temp:
            (Path(temp)/'lease').write_text(str(time.time()-1))
            with patch.object(module,'manifest',return_value={'cleaned':False}), patch.object(module,'directory',return_value=Path(temp)):
                with self.assertRaises(AssertionError):module.require_guard()

    def test_session_identity_rejects_reused_bus_owner_and_changed_pid(self):
        path=Path(__file__).with_name('live-acceptance.py')
        spec=importlib.util.spec_from_file_location('live_acceptance_identity',path)
        module=importlib.util.module_from_spec(spec)
        with patch('dbus.SessionBus',return_value=Mock()):spec.loader.exec_module(module)
        manifest={'owner':':1.9','busId':'original-bus','kwinPid':123}
        with patch.object(module,'owner',return_value=':1.9'), patch.object(module,'call',side_effect=['new-bus',123]):
            with self.assertRaises(AssertionError):module.require_session(manifest)
        with patch.object(module,'owner',return_value=':1.9'), patch.object(module,'call',side_effect=['original-bus',456]):
            with self.assertRaises(AssertionError):module.require_session(manifest)
        with patch.object(module,'owner',return_value=':1.9'), patch.object(module,'call',side_effect=['original-bus',123]):
            module.require_session(manifest)

    def test_repeated_cleanup_never_mutates_a_cleaned_run(self):
        path=Path(__file__).with_name('live-acceptance.py')
        spec=importlib.util.spec_from_file_location('live_acceptance_cleanup',path)
        module=importlib.util.module_from_spec(spec)
        with patch('dbus.SessionBus',return_value=Mock()):spec.loader.exec_module(module)
        with patch.object(module,'manifest',return_value={'cleaned':True}), patch.object(module,'call') as call, patch.object(module,'command') as command:
            module.cleanup()
            call.assert_not_called();command.assert_not_called()

    def test_tiling_snapshot_is_scoped_and_nonempty_test_layout_is_refused(self):
        path=Path(__file__).with_name('live-acceptance.py')
        spec=importlib.util.spec_from_file_location('live_acceptance_tiling',path)
        module=importlib.util.module_from_spec(spec)
        with patch('dbus.SessionBus',return_value=Mock()):spec.loader.exec_module(module)
        config='[General]\nprivateCommand=not retained\n[Tiling][test-desktop][output]\npadding=4\ntiles={"layoutDirection":"horizontal","tiles":[]}\n[TabBox]\nLayoutName=unrelated\n'
        value=module.tiling_preferences(config)
        self.assertEqual(set(value),{'[Tiling][test-desktop][output]'})
        self.assertTrue(module.empty_saved_test_layout('test-desktop',value))
        self.assertFalse(module.empty_saved_test_layout('other-desktop',value))
        value['[Tiling][test-desktop][output]']['tiles']='{"layoutDirection":"horizontal","tiles":[{"width":0.5}]}'
        self.assertFalse(module.empty_saved_test_layout('test-desktop',value))

    def test_disposable_terminal_accepts_only_exact_marker_without_retaining_input(self):
        import subprocess,sys
        path=Path(__file__).with_name('marker-terminal.py')
        spec=importlib.util.spec_from_file_location('marker_terminal_test',path)
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        self.assertTrue(module.matches('depthos-02d-ok\n'))
        self.assertFalse(module.matches(' depthos-02d-ok\n'))
        self.assertFalse(module.matches('depthos-02d-ok; arbitrary command\n'))
        for line,accepted in [('depthos-02d-ok\n',True),('arbitrary private line\n',False)]:
            with tempfile.TemporaryDirectory() as temp:
                ack=Path(temp)/'ack.json'
                result=subprocess.run([sys.executable,str(path),'--pid-file',str(Path(temp)/'pid'),'--ack-file',str(ack)],input=line,text=True,capture_output=True,timeout=3)
                self.assertEqual(result.returncode,0)
                self.assertEqual(ack.exists(),accepted)
                self.assertNotIn('arbitrary private line',result.stdout+result.stderr)

    def test_empty_tiling_recovery_requires_exact_hash_and_preserves_unrelated_values(self):
        import hashlib
        path=Path(__file__).with_name('poc02d.py')
        spec=importlib.util.spec_from_file_location('poc02d_reconstruction',path)
        module=importlib.util.module_from_spec(spec)
        with patch('dbus.SessionBus',return_value=Mock()):spec.loader.exec_module(module)
        baseline='[General]\nvalue=keep\n[Tiling][test-desktop][output]\ntiles={"layoutDirection":"horizontal","tiles":[]}\n'
        current=baseline.replace('"tiles":[]','"tiles":[{"width":0.5}]')
        preferences=module.a.tiling_preferences(baseline)
        digest=hashlib.sha256(baseline.encode()).hexdigest()
        result=module.empty_tiling_reconstruction(current,preferences,'test-desktop',digest)
        self.assertEqual(result['section'],'[Tiling][test-desktop][output]')
        with self.assertRaises(ValueError):module.empty_tiling_reconstruction(current.replace('value=keep','value=user-change'),preferences,'test-desktop',digest)
        with self.assertRaises(ValueError):module.empty_tiling_reconstruction(current,preferences,'other-desktop',digest)

if __name__=='__main__':unittest.main()
