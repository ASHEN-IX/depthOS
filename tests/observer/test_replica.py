import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("depthos_state", ROOT / "tools/state-inspector/state.py")
state = importlib.util.module_from_spec(spec)
spec.loader.exec_module(state)


def production_trace():
    code = """
const {fixture,window} = require('./tests/observer/fixtures.cjs');
const a=window({caption:'SENSITIVE TEST TITLE'}), f=fixture([a]);
const frames=[f.initial];
a.caption='Updated title'; a.captionChanged.emit(); frames.push(...f.events());
const b=window(); f.workspace.windowAdded.emit(b); frames.push(...f.events());
f.workspace.activeWindow=b; f.workspace.windowActivated.emit(b); frames.push(...f.events());
a.closed.emit(); frames.push(...f.events());
console.log(JSON.stringify({frames, final:f.observer.snapshot('comparison')}));
"""
    return json.loads(subprocess.check_output(["node", "-e", code], cwd=ROOT, text=True))


class ReplicaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.trace = production_trace()

    def setUp(self):
        self.replica = state.Replica()
        self.initial = copy.deepcopy(self.trace["frames"][0])

    def test_production_js_frames_reconcile_to_actual_final_snapshot(self):
        for frame in self.trace["frames"]:
            self.replica.apply(frame)
        self.assertEqual(self.replica.state, self.trace["final"]["state"])
        self.assertFalse(self.replica.need_snapshot)

    def test_gap_requires_snapshot_and_a_later_snapshot_recovers(self):
        self.replica.apply(self.initial)
        self.replica.apply(self.trace["frames"][2])
        self.assertTrue(self.replica.need_snapshot)
        self.replica.apply(self.trace["final"])
        self.assertFalse(self.replica.need_snapshot)
        self.assertEqual(self.replica.state, self.trace["final"]["state"])

    def test_duplicate_event_does_not_change_revision_or_state(self):
        for frame in self.trace["frames"][:2]:
            self.replica.apply(frame)
        before = (self.replica.sequence, self.replica.revision, copy.deepcopy(self.replica.state))
        self.replica.apply(self.trace["frames"][1])
        self.assertEqual(before, (self.replica.sequence, self.replica.revision, self.replica.state))

    def test_reconnect_requires_initial_snapshot_and_new_session_resets_ids(self):
        self.replica.apply(self.trace["frames"][1])
        self.assertTrue(self.replica.need_snapshot)
        snapshot = copy.deepcopy(self.initial)
        snapshot["sessionId"] = "another-observer-runtime"
        self.replica.apply(snapshot)
        self.assertEqual(self.replica.session_id, "another-observer-runtime")
        self.assertFalse(self.replica.need_snapshot)

    def test_heartbeat_detects_missing_events_without_further_window_changes(self):
        self.replica.apply(self.initial)
        answer = self.replica.accept({"schemaVersion": 1, "sessionId": self.initial["sessionId"],
            "consumerId": "consumer", "frames": [], "position": {"sequence": 99, "revision": 99}}, "consumer")
        self.assertTrue(answer["needSnapshot"])

    def test_invalid_schema_and_malformed_json_fail_without_state(self):
        with self.assertRaises(ValueError):
            self.replica.apply({"schemaVersion": 99})
        for raw in ("not json", '{"value":NaN}', '"' + "x" * state.MAX_BYTES + '"'):
            with self.assertRaises(ValueError):
                state.decode(raw)
        self.assertIsNone(self.replica.state)

    def test_snapshot_duplicate_ids_and_wrong_consumer_rejected(self):
        self.initial["state"]["windows"].append(copy.deepcopy(self.initial["state"]["windows"][0]))
        with self.assertRaises(ValueError):
            self.replica.apply(self.initial)
        with self.assertRaises(ValueError):
            self.replica.accept({"schemaVersion": 1, "consumerId": "wrong"}, "expected")

    def test_unknown_metadata_and_title_redaction(self):
        self.initial["state"]["windows"][0]["applicationId"] = None
        self.replica.apply(self.initial)
        redacted = state.redacted(self.replica.state)
        self.assertEqual(redacted["windows"][0]["title"], "<redacted>")
        self.assertIsNone(redacted["windows"][0]["applicationId"])
        self.assertEqual(self.replica.state["windows"][0]["title"], "SENSITIVE TEST TITLE")

    def test_terminal_control_and_bidi_characters_are_removed(self):
        clean = state.safe_terminal("Editor\x1b[2J\n\r\u009b\u202esecret")
        self.assertNotIn("\x1b", clean)
        self.assertNotIn("\n", clean)
        self.assertNotIn("\u202e", clean)

    def test_invalid_native_field_types_and_regressing_snapshot_rejected(self):
        malformed = copy.deepcopy(self.initial)
        malformed["state"]["windows"][0]["title"] = {"unexpected": "object"}
        with self.assertRaises(ValueError):
            self.replica.apply(malformed)
        self.replica.apply(self.trace["final"])
        old = copy.deepcopy(self.initial)
        old["sequence"] = self.replica.sequence + 1
        with self.assertRaises(ValueError):
            self.replica.apply(old)

    def test_payload_limit_status_visible_without_title_payload(self):
        answer = self.replica.accept({"schemaVersion": 1, "sessionId": "test-session", "consumerId": "consumer",
            "frames": [], "position": {"sequence": 100}, "status": "payload-too-large"}, "consumer")
        self.assertTrue(answer["needSnapshot"])
        self.assertEqual(self.replica.status, "payload-too-large")


if __name__ == "__main__":
    unittest.main()
