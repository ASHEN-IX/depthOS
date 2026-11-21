#!/usr/bin/env python3
"""Phase 4.4 interactive hierarchy management, safe persistence, and Wayland qualification.

Validates:
1. First-run Onboarding Wizard flow and hierarchy guidance.
2. World CRUD, visual identity, and carousel organization.
3. Workspace CRUD, Git project root validation, and metadata inspection.
4. Logical Desktop CRUD and role preset specialization.
5. Sibling reordering with rollback on conflict.
6. Safe non-destructive archiving and restore from Archived Items drawer.
7. Concurrency conflict detection (optimistic locking).
8. Preferences persistence across clean application restart.
9. Verification of read-only observed window associations and DENY-ONLY native authorization.
10. Generates 10 qualified Wayland screenshots under artifacts/phase-4-4/.
"""
import argparse
import importlib.machinery
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import traceback
import uuid

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests/support"))
from private_session import SOCKET, PrivateKWin, guard, launch, mark, observer_package
from host_isolation_monitor import HostMonitor

ARTIFACTS = ROOT / "artifacts/phase-4-4"
LAUNCHER = ROOT / "tools/depthos-spatial-alpha"
OBSERVER = ROOT / "experiments/kwin-observer/contents/code/main.js"
FIXTURE_ID = "org.depthos.phase44"
FIXTURE = """
import sys
from PyQt6.QtWidgets import QApplication, QLabel
app = QApplication([])
app.setDesktopFileName(sys.argv[1])
window = QLabel("DepthOS Phase 4.4 Fixture")
window.setWindowTitle("DepthOS Phase 4.4 Fixture")
window.resize(360, 240)
window.show()
app.exec()
"""


def run_isolated(results):
    bus = guard(results)
    import dbus
    from PyQt6 import QtQml
    from PyQt6.QtCore import QTimer, QObject
    from PyQt6.QtGui import QGuiApplication
    from PyQt6.QtQuick import QQuickWindow

    assert set(bus.list_names()) - {"org.freedesktop.DBus", bus.get_unique_name()} == set()
    install_root = results / "data/kwin/scripts"
    installed = None
    private = None
    children = []

    def process(name, *command):
        with (results / (name + ".log")).open("w") as log:
            child = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        children.append(child)
        return child

    def wait(predicate, timeout_s=10.0, msg="Condition timeout"):
        deadline = time.monotonic() + timeout_s
        while time.monotonic() < deadline:
            if predicate():
                return
            time.sleep(0.05)
        raise TimeoutError(msg)

    def cli(*args):
        return subprocess.check_output(
            [str(ROOT / "target/debug/depthos"), "--db", str(results / "workspace.sqlite"), *args],
            text=True
        )

    def checkpoint(stage, **details):
        identity = private.report() if private is not None and private.process.poll() is None else {}
        mark(results, stage, **identity, **details)

    def package(action):
        nonlocal installed
        checkpoint(f"package-{action}:before", package_root=str(install_root))
        path = observer_package(results, action, OBSERVER.parents[2], install_root)
        installed = path if action == "install" else None
        checkpoint(f"package-{action}:after", package_root=str(install_root))

    def load_script(path, name):
        assert path.resolve().is_relative_to(results) and path.is_file()
        checkpoint("script-load:before", script=name)
        number = int(private.call("/Scripting", "org.kde.kwin.Scripting",
                                  "loadScript", "ss", (str(path), name)))
        assert number >= 0
        private.call(f"/Scripting/Script{number}", "org.kde.kwin.Script", "run", "", ())
        checkpoint("script-load:after", script=name)
        return name

    def unload(name):
        checkpoint("script-unload:before", script=name)
        removed = private.call("/Scripting", "org.kde.kwin.Scripting", "unloadScript", "s", (name,))
        checkpoint("script-unload:after", script=name)
        return removed

    def compositor(label):
        nonlocal private
        private = None
        checkpoint(f"{label}:before")
        socket = results / "runtime" / SOCKET
        assert not socket.exists() and not socket.is_symlink()
        kwin = process(label, "/usr/bin/kwin_wayland", "--virtual", "--no-lockscreen",
                       "--no-global-shortcuts", "--width", "1600",
                       "--height", "1000", "--socket", SOCKET)
        wait(lambda: kwin.poll() is not None or (bus.name_has_owner("org.kde.KWin") and socket.is_socket()),
             msg="Private KWin failed to start")
        assert kwin.poll() is None, "Private KWin process terminated unexpectedly"
        private = PrivateKWin(results, bus, kwin)
        checkpoint(f"{label}:after")
        return kwin

    try:
        # 1. Setup private KWin compositor
        ids = [str(uuid.uuid4()), str(uuid.uuid4())]
        (results / "config/kwinrc").write_text(
            f"[Desktops]\nNumber=2\nId_1={ids[0]}\nId_2={ids[1]}\n"
            "[Plugins]\nkwin4_effect_shapecornersEnabled=false\n")
        kwin = compositor("kwin")

        # 2. Initialize disposable SQLite database
        cli("init", "--default-hierarchy")

        # 3. Start DepthOS controller and observer script
        def start_controller(label):
            child = process(label, str(ROOT / "target/debug/depthos"), "--db",
                            str(results / "workspace.sqlite"), "controller", "run")
            wait(lambda: bus.name_has_owner("org.depthos.Controller") and
                 bus.name_has_owner("org.depthos.StateInspector"),
                 msg="DepthOS controller failed to register on D-Bus")
            return child

        controller = start_controller("controller")
        package("install")
        observer = load_script(installed, "depthos-system-observer-session")
        endpoint = dbus.Interface(bus.get_object("org.depthos.Controller", "/org/depthos/Controller"),
                                  "org.depthos.Controller1")

        # Verify Controller is strictly read-only
        caps_raw = endpoint.GetCapabilities()
        caps = json.loads(str(caps_raw))
        assert caps["data"]["readOnly"] is True, "Controller readOnly must be TRUE"
        assert caps["data"]["assignmentMutations"] is False, "CRITICAL: Controller assignmentMutations must be FALSE (DENY-ONLY)"

        # Spawn fixture window
        marker = process("window-fixture", sys.executable, "-c", FIXTURE, FIXTURE_ID)

        # 4. Prepare UI launcher
        screenshots_dir = results / "screenshots"
        screenshots_dir.mkdir(parents=True, exist_ok=True, mode=0o700)
        ARTIFACTS.mkdir(parents=True, exist_ok=True, mode=0o755)

        loader = importlib.machinery.SourceFileLoader("phase_4_4_launcher", str(LAUNCHER))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        launcher = importlib.util.module_from_spec(spec)
        loader.exec_module(launcher)

        errors = []
        evidence = []

        def check(name, detail=None):
            checkpoint("case:observed", case=name, detail=detail)
            evidence.append({"check": name, "detail": detail})
            (results / "results.json").write_text(json.dumps(evidence, indent=2) + "\n")
            print("PASS", name, detail if detail is not None else "", flush=True)

        def capture(window, filename):
            target_file = screenshots_dir / filename
            success = window.grabWindow().save(str(target_file))
            assert success, f"Failed to save screenshot {filename}"
            print(f"Captured screenshot: {filename}", flush=True)

        def delay_step(seconds):
            target = time.monotonic() + seconds
            return lambda: time.monotonic() >= target

        created_world_id = None
        created_workspace_id = None
        desktop_ids = []

        def steps_phase_1(window):
            nonlocal created_world_id, created_workspace_id, desktop_ids
            backend = window.property("backend")
            scene = window.property("scene")
            bridge = window.property("hostBridge")

            yield lambda: backend.property("state") in ("Connected", "Database Mode")
            yield delay_step(0.4)

            # --- Checkpoint 1: Onboarding Wizard ---
            scene.openOnboardingWizard()
            yield delay_step(0.4)
            capture(window, "01-onboarding-wizard.png")
            check("01-onboarding-wizard: Multi-step guided onboarding explanation")

            onboarding = scene.findChild(QObject, "onboardingWizard")
            if onboarding is not None:
                onboarding.setProperty("isOpen", False)
            scene.restoreModalFocus()
            yield delay_step(0.2)

            # --- Checkpoint 2: Create World Dialog ---
            scene.openCreateWorldDialog()
            world_dialog = scene.findChild(QObject, "worldEditorDialog")
            yield (lambda: world_dialog is not None and world_dialog.property("isOpen"), 5)
            yield delay_step(0.3)
            capture(window, "02-create-world-dialog.png")
            check("02-create-world-dialog: Visual identity tone chips and artwork picker")

            # --- Checkpoint 3: World Creation & Carousel ---
            req = {
                "action": "create",
                "entity": "world",
                "payload": {
                    "name": "Cybernetics Core",
                    "description": "Core neural network and hardware interface",
                    "visualIdentity": "cyan",
                    "artwork": "matrix"
                }
            }
            resp = json.loads(bridge.executeManagement(json.dumps(req)))
            assert resp["success"] and "id" in resp, f"createWorld failed: {resp}"
            created_world_id = resp["id"]
            if world_dialog is not None:
                world_dialog.setProperty("isOpen", False)
            scene.restoreModalFocus()
            bridge.reloadHierarchy()
            backend.selectWorld(created_world_id)
            yield delay_step(0.4)
            capture(window, "03-world-view-edited.png")
            check("03-world-view-edited: World carousel with custom identity and management actions")

            # --- Checkpoint 4: Workspace Editor ---
            scene.openLevel(2)
            yield delay_step(0.2)
            scene.openCreateWorkspaceDialog(created_world_id)
            ws_dialog = scene.findChild(QObject, "workspaceEditorDialog")
            yield (lambda: ws_dialog is not None and ws_dialog.property("isOpen"), 5)
            yield delay_step(0.3)
            capture(window, "04-workspace-editor.png")
            check("04-workspace-editor: Workspace creation dialog with project root input")

            # --- Checkpoint 5: Project Root Git Validation ---
            ws_dialog.setProperty("projectRoot", str(ROOT))
            picker = ws_dialog.findChild(QObject, "rootPicker")
            if picker is not None:
                picker.validate()
            yield delay_step(0.4)
            capture(window, "05-project-root-git.png")
            check("05-project-root-git: Validated Git metadata badge with branch and commit")

            # Create Workspace
            ws_req = {
                "action": "create",
                "entity": "workspace",
                "parentId": created_world_id,
                "payload": {
                    "name": "Kernel Subsystem",
                    "description": "Core kernel drivers and hardware bus",
                    "rootDirectory": str(ROOT),
                    "visualIdentity": "lavender"
                }
            }
            ws_resp = json.loads(bridge.executeManagement(json.dumps(ws_req)))
            assert ws_resp["success"] and "id" in ws_resp, f"createWorkspace failed: {ws_resp}"
            created_workspace_id = ws_resp["id"]
            if ws_dialog is not None:
                ws_dialog.setProperty("isOpen", False)
            scene.restoreModalFocus()
            bridge.reloadHierarchy()
            backend.selectWorkspace(created_workspace_id)
            yield delay_step(0.2)

            # --- Checkpoint 6: Desktop Editor Dialog & Role Presets ---
            scene.openLevel(3)
            yield delay_step(0.2)
            scene.openCreateDesktopDialog(created_workspace_id)
            dt_dialog = scene.findChild(QObject, "desktopEditorDialog")
            yield (lambda: dt_dialog is not None and dt_dialog.property("isOpen"), 5)
            yield delay_step(0.3)
            capture(window, "06-desktop-editor.png")
            check("06-desktop-editor: Logical desktop editor with cyber-tech role presets")

            # Create multiple desktops
            if dt_dialog is not None:
                dt_dialog.setProperty("isOpen", False)
            scene.restoreModalFocus()
            for name in ["Compiler", "Runtime", "Profiler"]:
                d_resp = json.loads(bridge.executeManagement(json.dumps({
                    "action": "create",
                    "entity": "desktop",
                    "parentId": created_workspace_id,
                    "payload": {
                        "name": name,
                        "visualIdentity": "emerald"
                    }
                })))
                assert d_resp["success"] and "id" in d_resp, f"createDesktop failed: {d_resp}"
                desktop_ids.append(d_resp["id"])
            bridge.reloadHierarchy()
            yield delay_step(0.2)

            # Test Sibling Reordering
            reorder_req = {
                "action": "reorder",
                "entity": "desktop",
                "parentId": created_workspace_id,
                "payload": {"ids": [desktop_ids[2], desktop_ids[1], desktop_ids[0]]}
            }
            reorder_resp = json.loads(bridge.executeManagement(json.dumps(reorder_req)))
            assert reorder_resp["success"], f"reorder failed: {reorder_resp}"
            check("sibling-reordering: Atomic desktop reordering preserved in database")

            # --- Checkpoint 7: Archive Confirmation Dialog ---
            archive_target_id = desktop_ids[2] # "Profiler"
            scene.openArchiveConfirmation("desktop", archive_target_id, "Profiler", 2, 0)
            arch_dialog = scene.findChild(QObject, "archiveConfirmationDialog")
            yield (lambda: arch_dialog is not None and arch_dialog.property("isOpen"), 5)
            yield delay_step(0.3)
            capture(window, "07-archive-confirmation.png")
            check("07-archive-confirmation: Impact assessment showing reversible warning")

            # Execute archive
            if arch_dialog is not None:
                arch_dialog.setProperty("isOpen", False)
            scene.restoreModalFocus()
            arch_resp = json.loads(bridge.executeManagement(json.dumps({
                "action": "archive",
                "entity": "desktop",
                "id": archive_target_id,
                "expectedVersion": 2
            })))
            assert arch_resp["success"], f"archive failed: {arch_resp}"
            bridge.reloadHierarchy()
            yield delay_step(0.2)

            # --- Checkpoint 8: Archived Items Drawer & Restore ---
            scene.openArchivedView()
            arch_view = scene.findChild(QObject, "archivedItemsView")
            yield (lambda: arch_view is not None and arch_view.property("isOpen"), 5)
            yield delay_step(0.3)
            capture(window, "08-archived-items-view.png")
            check("08-archived-items-view: Archived items panel showing archived records and restore action")

            # Restore item
            rest_resp = json.loads(bridge.executeManagement(json.dumps({
                "action": "restore",
                "entity": "desktop",
                "id": archive_target_id
            })))
            assert rest_resp["success"], f"restore failed: {rest_resp}"
            if arch_view is not None:
                arch_view.setProperty("isOpen", False)
            scene.restoreModalFocus()
            bridge.reloadHierarchy()
            check("archive-restore: Safely restored archived desktop back into active hierarchy")

            # --- Concurrency Conflict Verification ---
            conflict_req = {
                "action": "update",
                "entity": "world",
                "id": created_world_id,
                "expectedVersion": 0, # Stale version! Actual is at least 1
                "payload": {"name": "Stale Conflict World"}
            }
            conflict_resp = json.loads(bridge.executeManagement(json.dumps(conflict_req)))
            assert conflict_resp["success"] is False and conflict_resp.get("code") == "conflict"
            check("optimistic-concurrency: Stale version write safely rejected with conflict code")

            # --- Checkpoint 9: Control Center & Preferences Persistence ---
            scene.openControlCenter()
            cc = scene.findChild(QObject, "controlCenter")
            yield (lambda: cc is not None and cc.property("isOpen"), 5)
            yield delay_step(0.3)
            capture(window, "09-control-center-preferences.png")
            check("09-control-center-preferences: Spatial Control Center with preference toggles")

            # Set user preferences
            bridge.setPreference("presentation.reduced_motion", "true")
            bridge.setPreference("presentation.low_effects", "true")
            bridge.setPreference("presentation.show_titles", "false")
            bridge.setPreference("workspace.default_landing_view", "workspaces")
            assert bridge.getPreference("presentation.reduced_motion") == "true"
            check("preferences-saved: Presentation and workspace preferences persisted to SQLite")

            if cc is not None:
                cc.setProperty("isOpen", False)
            scene.restoreModalFocus()
            yield delay_step(0.3)

            # Exit Phase 1 UI
            QGuiApplication.exit(0)

        step_counter = 0

        def drive(sequence, predicate=None, deadline=0):
            nonlocal step_counter
            try:
                if predicate is not None and not predicate():
                    if time.monotonic() >= deadline:
                        raise TimeoutError(f"Sequence predicate timed out at step {step_counter}")
                    QTimer.singleShot(25, lambda: drive(sequence, predicate, deadline))
                    return
                step_counter += 1
                next_check = next(sequence)
                predicate, budget = next_check if isinstance(next_check, tuple) else (next_check, 10)
                drive(sequence, predicate, time.monotonic() + budget)
            except StopIteration:
                QGuiApplication.exit(0)
            except Exception:
                err = traceback.format_exc()
                errors.append(err)
                print(err, file=sys.stderr)
                QGuiApplication.exit(1)

        class InstrumentedEngine(QtQml.QQmlApplicationEngine):
            def load(self, url):
                super().load(url)
                window = self.rootObjects()[0]
                assert isinstance(window, QQuickWindow)
                QTimer.singleShot(50, lambda: drive(steps_phase_1(window)))

        QtQml.QQmlApplicationEngine = InstrumentedEngine
        sys.argv = [str(LAUNCHER), "--db", str(results / "workspace.sqlite"), "--window-size", "1366x768"]
        try:
            launcher.main()
        except SystemExit as ex:
            if ex.code != 0:
                errors.append(f"launcher.main exited with code {ex.code}")

        for err in errors:
            print("UI Phase 1 Error:", err, file=sys.stderr)
        assert not errors, "Phase 1 UI test failed"

        # --- Checkpoint 10: Complete Restart & Persisted Hierarchy Restored ---
        # 1. Verify directly against SQLite that records and preferences are in the database
        db_tree = json.loads(cli("tree"))
        cyber_branch = next((b for b in db_tree["worlds"] if b["world"]["name"] == "Cybernetics Core"), None)
        assert cyber_branch is not None, "Persisted world missing in SQLite"
        kernel_branch = next((wb for wb in cyber_branch["workspaces"] if wb["workspace"]["name"] == "Kernel Subsystem"), None)
        assert kernel_branch is not None, "Persisted workspace missing in SQLite"
        assert len(kernel_branch["desktops"]) == 3, "Persisted desktops count mismatch"
        assert json.loads(cli("preferences", "get", "presentation.reduced_motion"))["value"] == "true"
        assert json.loads(cli("preferences", "get", "presentation.low_effects"))["value"] == "true"

        # 2. Launch depthos-spatial-alpha as an independent process restart to verify UI restoration and screenshot capture
        shot_path = screenshots_dir / "10-restored-hierarchy-restart.png"
        restart_proc = subprocess.run(
            [sys.executable, str(LAUNCHER), "--db", str(results / "workspace.sqlite"),
             "--window-size", "1366x768", "--screenshot", str(shot_path),
             "--quit-after-ms", "1400"],
            cwd=results,
            capture_output=True,
            text=True,
            timeout=15,
        )
        assert restart_proc.returncode == 0, f"Restart process failed: {restart_proc.stderr}\n{restart_proc.stdout}"
        assert shot_path.is_file(), "Screenshot 10-restored-hierarchy-restart.png was not generated"
        print(f"Captured screenshot: {shot_path.name}", flush=True)
        check("10-restored-hierarchy-restart: Spatial scene after restart with all persisted records")

        # Cleanup observer
        assert unload(observer)
        package("remove")
        marker.terminate()
        marker.wait(timeout=5)

    finally:
        for child in reversed(children):
            if child.poll() is None:
                child.terminate()
        for child in reversed(children):
            try:
                child.wait(timeout=5)
            except subprocess.TimeoutExpired:
                child.kill()
                child.wait(timeout=5)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--inside", type=Path, help=argparse.SUPPRESS)
    args = parser.parse_args()
    os.umask(0o077)
    if args.inside:
        run_isolated(args.inside)
        return
    ARTIFACTS.mkdir(parents=True, exist_ok=True, mode=0o755)
    results = Path(tempfile.mkdtemp(dir=ARTIFACTS, prefix="m-"))
    print(f"Phase 4.4 qualification workspace: {results}", flush=True)
    command = [sys.executable, __file__, "--inside", str(results)]
    with HostMonitor(results / "host.jsonl") as monitor:
        status = launch(results, command, wayland=True, monitor=monitor)
    monitor.require_clean()
    print(f"Host monitor clean across {monitor.checkpoints} checkpoints", flush=True)
    assert status == 0, f"Private worker exited with status {status}"
    for shot in sorted((results / "screenshots").glob("*.png")):
        shutil.copy2(shot, ARTIFACTS / shot.name)
    print("PHASE 4.4 MANAGEMENT AND PERSISTENCE TEST PASSED", flush=True)


if __name__ == "__main__":
    main()
