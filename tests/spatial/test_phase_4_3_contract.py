#!/usr/bin/env python3
"""Exercise version negotiation and fail-closed shelf decoding in the launcher."""
import copy
import importlib.machinery
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import Mock, patch

import dbus

ROOT = Path(__file__).resolve().parents[2]
loader = importlib.machinery.SourceFileLoader("shelf_contract", str(ROOT / "tools/depthos-spatial-alpha"))
spec = importlib.util.spec_from_loader(loader.name, loader)
launcher = importlib.util.module_from_spec(spec)
loader.exec_module(launcher)


class ControllerContract(unittest.TestCase):
    def setUp(self):
        self.endpoint = Mock()
        self.bus = Mock()
        self.capability = {"readOnly": True, "assignmentMutations": False}
        self.hierarchy = {"items": [
            {"kind": "world", "id": "world", "name": "World", "parentId": None},
            {"kind": "workspace", "id": "workspace", "name": "Workspace", "parentId": "world"},
            {"kind": "desktop", "id": "desktop", "name": "Desktop", "parentId": "workspace"},
        ], "nextCursor": None}
        self.window = {"id": "060ac30e-56c2-43fc-b6da-3e821db17916", "workspaceId": "workspace",
                       "desktopId": "desktop", "applicationId": "org.example.Editor",
                       "assignment": "observed", "focused": True}
        self.shelves = {"items": [self.window], "nextCursor": None, "hierarchyRevision": "tree",
                        "titlePolicy": "withheld", "freshness": "live", "lastValidatedSequence": "19"}

    @staticmethod
    def envelope(data, revision="1", version=1):
        return json.dumps(dict(schemaVersion=version, instanceId="instance", revision=revision,
                               available=True, data=data))

    def read(self, version=1):
        self.endpoint.GetCapabilities.return_value = self.envelope(self.capability, version=version)
        self.endpoint.GetHierarchy.return_value = self.envelope(self.hierarchy, revision="tree")
        self.endpoint.GetControllerHealth.return_value = self.envelope({"health": {"observation": "Synchronized"}})
        self.endpoint.GetDesktopShelves.return_value = self.envelope(self.shelves)
        with patch.object(dbus, "SessionBus", return_value=self.bus), patch.object(dbus, "Interface", return_value=self.endpoint):
            return launcher.read_controller(":1.45")

    def test_old_and_future_shelf_versions_keep_hierarchy(self):
        for version in (None, 2, True):
            with self.subTest(version=version):
                if version is not None:
                    self.capability["desktopShelvesVersion"] = version
                cached, _, shelves, _ = self.read()
                self.assertEqual(cached[0][2][0]["id"], "desktop")
                self.assertEqual(shelves, ("unavailable", []))
        self.endpoint.GetDesktopShelves.assert_not_called()

    def test_unknown_controller_schema_is_rejected(self):
        with self.assertRaises(ValueError):
            self.read(version=2)
        self.endpoint.GetHierarchy.assert_not_called()
        self.bus.close.assert_called_once()

    def test_valid_projection_and_reject_title_duplicate_or_scope_mismatch(self):
        self.capability["desktopShelvesVersion"] = 1
        _, _, (freshness, rows), _ = self.read()
        self.assertEqual(freshness, "live")
        self.assertTrue(rows[0]["titleRedacted"])
        for bad in (dict(self.window, title="private"), dict(self.window, workspaceId="foreign")):
            self.shelves["items"] = [bad]
            self.assertEqual(self.read()[2], ("unavailable", []))
        self.shelves["items"] = [self.window, copy.copy(self.window)]
        self.assertEqual(self.read()[2], ("unavailable", []))


if __name__ == "__main__":
    unittest.main()
