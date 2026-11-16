import importlib.util
import hashlib
import json
from pathlib import Path
import tempfile
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("overview_dev", ROOT / "tools/overview-dev.py")
dev = importlib.util.module_from_spec(spec)
spec.loader.exec_module(dev)


class PackageTests(unittest.TestCase):
    def test_source_hash_changes_with_qml_or_relative_component_edits(self):
        files = dev.source_files()
        first = dev.revision(files)
        files["contents/ui/WindowCard.qml"] += b"\n// changed fixture\n"
        self.assertNotEqual(first, dev.revision(files))
        self.assertEqual(dev.revision(files), dev.revision(dict(reversed(list(files.items())))))

    def test_prepare_is_repeatable_and_never_overwrites_modified_revisions(self):
        with tempfile.TemporaryDirectory() as temporary, patch.object(dev, "ARTIFACTS", Path(temporary)):
            current = dev.prepare()
            self.assertEqual(current, dev.prepare())
            path = Path(current["path"])
            metadata = json.loads((path / "metadata.json").read_text())
            self.assertFalse(metadata["KPlugin"]["EnabledByDefault"])
            self.assertEqual(metadata["KPlugin"]["Id"], current["id"])
            (path / "contents/ui/main.qml").write_text("modified fixture")
            with self.assertRaises(ValueError):
                dev.prepare()

    def test_pending_acceptance_cannot_reach_any_session_query(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "pending.json"
            path.write_text(json.dumps({"status": "PENDING"}))
            with patch.object(dev, "dbus", side_effect=AssertionError("must reject before querying KWin")):
                with self.assertRaises(ValueError):
                    dev.require_acceptance(path)

    def test_attestation_must_match_both_compositor_owner_and_session_bus(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fixture.json"
            value = {"status": "PASS", "kwinVersion": "6.8.1", "compositorOwner": ":1.fixture", "sessionBusId": "fixture-bus",
                "observerSourceHash": hashlib.sha256((dev.ROOT / "experiments/kwin-observer/contents/code/main.js").read_bytes()).hexdigest(),
                "checks": {key: True for key in dev.REQUIRED}}
            path.write_text(json.dumps(value))
            with patch.object(dev, "dbus", side_effect=[":1.fixture", "fixture-bus"]):
                dev.require_acceptance(path)
            with patch.object(dev, "dbus", return_value=":1.other"):
                with self.assertRaises(ValueError):
                    dev.require_acceptance(path)
            with patch.object(dev, "dbus", side_effect=[":1.fixture", "other-bus"]):
                with self.assertRaises(ValueError):
                    dev.require_acceptance(path)

    def test_cleanup_refuses_unrelated_identifiers_and_symlink_packages(self):
        for identifier in ("codex-glass", "depthos-spatial-poc", "../other", "depthos-overview-dev-../../"):
            with self.assertRaises(ValueError):
                dev.own(identifier)
        with tempfile.TemporaryDirectory() as temporary, patch.object(dev, "package_root", return_value=Path(temporary)):
            identifier = dev.PREFIX + "0" * 12
            (Path(temporary) / identifier).symlink_to(ROOT / "experiments/kwin-effect", target_is_directory=True)
            with self.assertRaises(ValueError):
                dev.own(identifier)

    def test_one_stable_action_and_no_default_global_binding(self):
        source = (dev.SOURCE / "contents/ui/main.qml").read_text()
        self.assertIn('name: "' + dev.ACTION + '"', source)
        self.assertIn('sequence: ""', source)
        self.assertNotIn("Workspace.activeWindow =", source)
        self.assertNotIn("Workspace.currentDesktop =", source)

    def test_emergency_unload_is_independent_of_edited_source_and_acceptance(self):
        identifier = dev.PREFIX + "1" * 12
        with patch.object(sys, "argv", ["overview-dev.py", "unload", "--id", identifier]), \
             patch.object(dev, "package_data", side_effect=AssertionError("do not read edited source")), \
             patch.object(dev, "own", return_value=Path("/fixture")), \
             patch.object(dev.os, "getuid", return_value=1000), \
             patch.object(dev, "dbus", side_effect=["", "false"]) as native:
            dev.main()
            self.assertEqual(native.call_count, 2)
            self.assertEqual(native.call_args_list[0].args[-2:], ("org.kde.kwin.Effects.unloadEffect", identifier))


if __name__ == "__main__":
    unittest.main()
