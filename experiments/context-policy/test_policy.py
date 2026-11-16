"""Deterministic synthetic ownership scenarios. Run using standard unittest."""
from dataclasses import asdict
from itertools import permutations
import json
import unittest

from policy import Claim, Source, Window, WindowKey, decide, project_window


A = WindowKey("epoch-a", "window-1")
B = WindowKey("epoch-a", "window-2")
REPLACEMENT = WindowKey("epoch-b", "window-1")
PROJECTS = frozenset({"alpha", "beta"})


def claim(window, source, *targets):
    return Claim(window, source, frozenset(targets))


def fixture(window, app, **metadata):
    return {
        "runtimeId": window.runtime_id,
        "applicationId": app,
        "classification": "application",
        "excludeFromCapture": False,
        "skipTaskbar": False,
    } | metadata


def decision(key=A, claims=(), *, epoch=A.epoch, ready=True, windows=None, projects=PROJECTS):
    return decide(key, epoch=epoch, ready=ready, claims=claims,
                  windows={A: Window(True), B: Window(True)} if windows is None else windows,
                  workspaces=projects)


class PolicyTests(unittest.TestCase):
    def test_two_editor_windows_with_same_application_id(self):
        windows = dict(project_window(fixture(key, "org.example.Editor"), key.epoch) for key in (A, B))
        claims = (claim(A, Source.USER, "alpha"), claim(B, Source.USER, "beta"))
        self.assertEqual(decision(A, claims, windows=windows).targets, ("alpha",))
        self.assertEqual(decision(B, claims, windows=windows).targets, ("beta",))

    def test_identical_terminal_app_ids_and_shared_process(self):
        windows = dict(project_window(fixture(key, "org.kde.konsole", pid=321), key.epoch) for key in (A, B))
        claims = (claim(A, Source.VERIFIED_LAUNCH, "alpha"),)
        self.assertEqual(decision(A, claims, windows=windows).status, "confirmed")
        self.assertEqual(decision(B, claims, windows=windows).status, "unknown")

    def test_browser_unknown_profiles(self):
        windows = dict(project_window(fixture(key, "org.example.Browser"), key.epoch) for key in (A, B))
        for key in windows:
            self.assertEqual(decision(key, windows=windows).status, "unknown")

    def test_explicit_shared_target_is_one_claim(self):
        shared = claim(A, Source.EXPLICIT_POLICY, "beta", "alpha")
        result = decision(claims=(shared,))
        self.assertEqual((result.status, result.targets), ("confirmed", ("alpha", "beta")))
        separate = (claim(A, Source.EXPLICIT_POLICY, "alpha"), claim(A, Source.EXPLICIT_POLICY, "beta"))
        self.assertEqual(decision(claims=separate).status, "conflicted")
        with self.assertRaises(ValueError):
            claim(A, Source.HEURISTIC, "alpha", "beta")

    def test_restart_with_window_uuid_replacement_does_not_inherit(self):
        old = project_window(fixture(A, "editor", nativeId="uuid-before"), A.epoch)
        new = project_window(fixture(B, "editor", nativeId="uuid-after"), B.epoch)
        claims = (claim(A, Source.USER, "alpha"),)
        self.assertEqual(decision(A, claims, windows=dict([new])).status, "stale")
        self.assertEqual(decision(B, claims, windows=dict([old, new])).status, "unknown")
        rebound = (*claims, claim(B, Source.EXPLICIT_POLICY, "alpha"))
        self.assertEqual(decision(B, rebound, windows=dict([new])).status, "confirmed")

    def test_pid_reuse_cannot_carry_a_launch_claim(self):
        windows = dict(project_window(fixture(key, "terminal", pid=321, processStartTick=tick), key.epoch)
                       for key, tick in ((A, 42), (B, 90)))
        self.assertEqual(decision(B, (claim(A, Source.VERIFIED_LAUNCH, "alpha"),), windows=windows).status, "unknown")

    def test_invalid_parent_references_never_inherit(self):
        choices = [
            {A: Window(True, A, dialog=True)},
            {A: Window(True, B, dialog=True)},
            {A: Window(True, B, dialog=True), B: Window(True, A, dialog=True)},
            {A: Window(True, B, dialog=True), B: Window(False)},
            {A: Window(True, REPLACEMENT, dialog=True), REPLACEMENT: Window(True)},
        ]
        for windows in choices:
            with self.subTest(windows=windows):
                self.assertEqual(decision(claims=(claim(A, Source.PARENT_HINT, "alpha"),), windows=windows).status, "unknown")
        choices_without_dialog = {A: Window(True, B), B: Window(True)}
        self.assertEqual(decision(claims=(claim(A, Source.PARENT_HINT, "alpha"),),
                                  windows=choices_without_dialog).status, "unknown")
        valid = {A: Window(True, B, dialog=True), B: Window(True)}
        self.assertEqual(decision(claims=(claim(A, Source.PARENT_HINT, "alpha"),), windows=valid).status, "suggested")

    def test_user_reassignment_wins_and_unassignment_suppresses_fallback(self):
        launch = claim(A, Source.VERIFIED_LAUNCH, "alpha")
        assignment = claim(A, Source.USER, "beta")
        self.assertEqual(decision(claims=(launch, assignment)).targets, ("beta",))
        self.assertEqual(decision(claims=(launch, claim(A, Source.USER))).reason, "user_unassigned")
        self.assertEqual(decision(claims=(launch, assignment, claim(A, Source.USER, "alpha"))).status, "conflicted")

    def test_verified_launch_precedes_policy_and_duplicate_claims_agree(self):
        launch = claim(A, Source.VERIFIED_LAUNCH, "beta")
        binding = claim(A, Source.EXPLICIT_POLICY, "alpha")
        result = decision(claims=(binding, launch, launch))
        self.assertEqual((result.status, result.reason, result.targets),
                         ("confirmed", "verified_window_launch", ("beta",)))
        self.assertEqual(decision(claims=(launch, claim(A, Source.VERIFIED_LAUNCH, "alpha"))).status,
                         "conflicted")

    def test_conflicting_automatic_suggestions_are_order_independent(self):
        claims = (claim(A, Source.HEURISTIC, "beta"), claim(A, Source.PARENT_HINT, "alpha"),
                  claim(B, Source.USER, "alpha"))
        windows = {A: Window(True, B, dialog=True), B: Window(True)}
        results = {decision(claims=ordered, windows=windows) for ordered in permutations(claims)}
        self.assertEqual(len(results), 1)
        result = results.pop()
        self.assertEqual((result.status, result.targets, result.alternatives),
                         ("conflicted", (), (("alpha",), ("beta",))))

    def test_unknown_application_requires_real_evidence(self):
        windows = dict([project_window(fixture(A, None), A.epoch)])
        self.assertEqual(decision(windows=windows).status, "unknown")
        self.assertEqual(decision(claims=(claim(A, Source.USER, "alpha"),), windows=windows).status, "confirmed")

    def test_reconnect_requires_reconciled_snapshot_and_retained_epoch(self):
        claims = (claim(A, Source.USER, "alpha"),)
        self.assertEqual(decision(claims=claims, ready=False).status, "stale")
        self.assertEqual(decision(claims=claims, ready=True).status, "confirmed")
        self.assertEqual(decision(A, claims, epoch=REPLACEMENT.epoch).status, "stale")
        self.assertEqual(decision(REPLACEMENT, claims, epoch=REPLACEMENT.epoch,
                                  windows={REPLACEMENT: Window(True)}).status, "unknown")

    def test_workspace_deletion_leaves_active_window_unowned(self):
        claims = (claim(A, Source.USER, "alpha"), claim(A, Source.EXPLICIT_POLICY, "beta"))
        result = decision(claims=claims, projects=frozenset({"beta"}))
        self.assertEqual((result.status, result.reason, result.targets), ("stale", "workspace_removed", ()))
        shared = (claim(A, Source.USER, "alpha", "beta"),)
        self.assertEqual(decision(claims=shared, projects=frozenset({"beta"})).status, "stale")

    def test_heuristic_has_no_write_path_or_confirmation(self):
        hints = (claim(A, Source.HEURISTIC, "alpha"),)
        advisory = decision(claims=hints)
        self.assertEqual(advisory.status, "suggested")
        self.assertEqual(decision().status, "unknown")
        self.assertEqual(hints[0].source, Source.HEURISTIC)

    def test_projection_redacts_secrets_and_ignores_claim_like_fields(self):
        private = "sensitive-fragment"
        key, window = project_window(
            fixture(A, private, title=private, url=private, cwd=private, profile=private,
                    commandLine=private, nativeId=private, ownerAssertion="alpha",
                    source="USER", pid=321), A.epoch)
        result = decision(key, windows={key: window})
        self.assertNotIn(private, json.dumps([asdict(window), asdict(result)]))
        self.assertEqual((result.status, result.sources), ("unknown", ()))

    def test_protected_or_unclassified_windows_are_excluded(self):
        for metadata in ({"excludeFromCapture": True}, {"excludeFromCapture": None},
                         {"skipTaskbar": True}, {"classification": "system"}):
            key, window = project_window(fixture(A, "editor", **metadata), A.epoch)
            self.assertEqual(decision(key, (claim(A, Source.USER, "alpha"),),
                                      windows={key: window}).reason, "window_outside_policy_scope")


if __name__ == "__main__":
    unittest.main()
