import QtQuick
import QtQuick.Window
import QtTest
import "../../experiments/spatial-alpha" as Spatial

// Phase 4.1.1 QML interaction tests: DEF-07 focus, DEF-08 wheel, DEF-09 pinch
// logic (SYNTHETIC: drives the handler logic directly), DEF-10 onboarding and
// source indicators, DEF-11 double-click.  Window (de)activation is simulated by
// activating a second top-level window, which exercises Qt's real focus machinery
// but is NOT a compositor-driven activation; see docs/phase-4-1-1-ux-corrections.md.
Item {
    id: testRoot
    width: 1440
    height: 900

    Spatial.SpatialBackend {
        id: be
    }

    Spatial.SpatialScene {
        id: scene
        anchors.fill: parent
        backend: be
    }

    // A focusable control that lives OUTSIDE the scene (stands in for a rename
    // dialog / foreign text entry that legitimately owns keyboard focus).
    TextInput {
        id: foreignText
        objectName: "foreignText"
        x: 0
        y: 0
        width: 10
        height: 10
    }

    Item {
        id: driftItem
        objectName: "driftItem"
        width: 1
        height: 1
    }

    Component {
        id: freshBackendComponent
        Spatial.SpatialBackend {}
    }

    Component {
        id: otherWindowComponent
        Window {
            width: 120
            height: 120
            title: "another application"
        }
    }

    SignalSpy {
        id: closedSpy
        target: scene
        signalName: "closed"
    }

    SignalSpy {
        id: createSpy
        target: be
        signalName: "createWorkspaceRequested"
    }

    SignalSpy {
        id: demoSpy
        target: be
        signalName: "continueDemoRequested"
    }

    TestCase {
        id: tc
        name: "DepthOS_Phase_4_1_1"
        when: windowShown

        property var other: null

        function topWindow() {
            return testRoot.Window.window;
        }

        function init() {
            // qmltestrunner keeps one view for the whole file. A failed test can
            // leave the palette open, the search field focused, or a second
            // window active, and the next test then sees that leftover state.
            if (other) {
                other.close();
                other.destroy();
                other = null;
            }
            const win = topWindow();
            if (win)
                win.requestActivate();
            be.loadMock();
            scene.reducedMotion = true;
            scene.currentLevel = 1;
            scene.showTitles = false;
            scene.cameraX = 0;
            scene.cameraY = 0;
            scene.resetInteractionClocks();
            be.onboardingState = "hidden";
            be.onboardingMessage = "";
            be.onboardingDetail = "";
            be.selectWorld(be.worlds[0].id);
            scene.syncIndices();
            closedSpy.clear();
            createSpy.clear();
            demoSpy.clear();
            const palette = findChild(scene, "searchPalette");
            if (palette) {
                palette.isOpen = false;
                const input = findChild(palette, "searchInput");
                if (input)
                    input.text = "";
            }
            foreignText.text = "";
            scene.forceActiveFocus();
            wait(20);
        }

        function cleanup() {
            if (other) {
                other.close();
                other.destroy();
                other = null;
            }
            const palette = findChild(scene, "searchPalette");
            if (palette)
                palette.isOpen = false;
            be.onboardingState = "hidden";
            scene.forceActiveFocus();
        }

        // Simulates "focus another app, then click the DepthOS titlebar".
        function cycleActivation() {
            if (!topWindow().active) {
                topWindow().requestActivate();
                tryVerify(() => topWindow().active, 2000, "window must be active first");
            }
            if (!other) // parent=null: a transient child would keep us "active"
                other = otherWindowComponent.createObject(null);
            other.show();
            other.requestActivate();
            tryVerify(() => !topWindow().active, 2000, "window must deactivate");
            topWindow().requestActivate();
            tryVerify(() => topWindow().active, 2000, "window must reactivate");
            wait(40); // let Qt.callLater-based restoration run
        }

        function loadMany(n) {
            be.loadMock();
            const list = [];
            for (let i = 0; i < n; ++i) {
                list.push({
                    id: "ws-x" + i,
                    worldId: "world-engineering",
                    name: "Bulk " + i,
                    description: "bulk workspace",
                    rootDirectory: "~",
                    gitBranch: "main"
                });
            }
            be.workspaces = list.concat(be.workspaces.filter(w => w.worldId !== "world-engineering"));
            be.selectWorld("world-engineering");
            scene.currentLevel = 2;
            scene.syncIndices();
            scene.cameraX = 0;
            scene.cameraY = 0;
            wait(20);
        }

        // ---------------------------------------------------------------- DEF-07
        // Does not depend on Window.active. The reactivation tests below use a
        // second top-level window; if the offscreen QPA will not clear
        // Window.active they are marked Wayland-live-only and this one remains.
        function test_def07_synthetic_restoration_after_focus_drift() {
            driftItem.forceActiveFocus();
            verify(!scene.activeFocus, "precondition: scene lost focus");
            scene.restoreFocusAfterActivation();
            verify(scene.activeFocus, "restoration must return focus to the scene");
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 1);
            keyClick(Qt.Key_Return);
            compare(scene.currentLevel, 2);
        }

        function test_def07_synthetic_restoration_preserves_search_text_and_modal() {
            const palette = findChild(scene, "searchPalette");
            const input = findChild(palette, "searchInput");
            keyClick(Qt.Key_Slash);
            verify(input.activeFocus);
            scene.restoreFocusAfterActivation();
            verify(input.activeFocus, "search input keeps focus");
            verify(!scene.activeFocus);
            keyClick(Qt.Key_Escape);
            compare(palette.isOpen, false);
            verify(scene.activeFocus);

            foreignText.forceActiveFocus();
            scene.restoreFocusAfterActivation();
            verify(foreignText.activeFocus, "text entry keeps focus");
            keyClick(Qt.Key_Z);
            compare(foreignText.text, "z");
            compare(scene.worldIndex, 0);

            be.onboardingState = "offer";
            wait(20);
            scene.restoreFocusAfterActivation();
            verify(findChild(scene, "onboardingPanel").activeFocus);
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 0);
        }

        function test_def07_reactivation_restores_scene_focus_after_focus_drift() {
            driftItem.forceActiveFocus();
            verify(!scene.activeFocus, "precondition: scene lost focus");
            cycleActivation();
            verify(scene.activeFocus, "scene must own focus right after reactivation");
            const before = scene.worldIndex;
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, before + 1, "L must work immediately after reactivation");
            keyClick(Qt.Key_Return);
            compare(scene.currentLevel, 2, "Enter must work immediately after reactivation");
        }

        function test_def07_plain_reactivation_keeps_keyboard_working() {
            cycleActivation();
            verify(scene.activeFocus);
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 1);
            keyClick(Qt.Key_H);
            compare(scene.worldIndex, 0);
        }

        function test_def07_repeated_switching_is_stable() {
            for (let i = 0; i < 12; ++i) {
                if (i % 3 === 0)
                    driftItem.forceActiveFocus();
                cycleActivation();
                verify(scene.activeFocus, "cycle " + i);
            }
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 1);
        }

        function test_def07_reactivation_does_not_steal_focus_from_search_input() {
            const palette = findChild(scene, "searchPalette");
            const input = findChild(palette, "searchInput");
            verify(input !== null);
            keyClick(Qt.Key_Slash);
            compare(palette.isOpen, true);
            verify(input.activeFocus);
            cycleActivation();
            verify(input.activeFocus, "search input must keep focus");
            verify(!scene.activeFocus, "scene must not steal focus from the search input");
            keyClick(Qt.Key_X);
            compare(input.text, "x", "typing must still reach the search input");
            compare(scene.worldIndex, 0);
        }

        function test_def07_reactivation_does_not_steal_focus_from_foreign_text_entry() {
            foreignText.forceActiveFocus();
            verify(foreignText.activeFocus);
            for (let i = 0; i < 3; ++i) {
                cycleActivation();
                verify(foreignText.activeFocus, "text entry must keep focus on cycle " + i);
            }
            keyClick(Qt.Key_L);
            compare(foreignText.text, "l");
            compare(scene.worldIndex, 0);
        }

        function test_def07_reactivation_does_not_steal_focus_from_onboarding_modal() {
            be.onboardingState = "offer";
            wait(20);
            const panel = findChild(scene, "onboardingPanel");
            verify(panel !== null);
            const before = scene.currentLevel;
            cycleActivation();
            verify(panel.activeFocus, "onboarding modal must keep focus");
            verify(!scene.activeFocus);
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 0, "scene keys are inert while the modal is open");
            compare(scene.currentLevel, before);
        }

        function test_def07_escape_in_search_closes_palette_and_returns_keys() {
            const palette = findChild(scene, "searchPalette");
            keyClick(Qt.Key_Slash);
            compare(palette.isOpen, true);
            keyClick(Qt.Key_Escape);
            compare(palette.isOpen, false, "Esc must close the palette");
            verify(scene.activeFocus);
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 1, "scene keys must work after closing search");
        }

        function test_def07_backdrop_click_closes_palette_and_returns_keys() {
            const palette = findChild(scene, "searchPalette");
            keyClick(Qt.Key_Slash);
            compare(palette.isOpen, true);
            mouseClick(scene, 20, 700); // backdrop, outside the modal box
            compare(palette.isOpen, false);
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 1);
        }

        // ---------------------------------------------------------------- DEF-08
        function test_def08_vertical_wheel_pans_by_one_step_per_notch() {
            loadMany(12);
            verify(scene.panLimitY() > 0, "content must overflow for this test");
            const step = scene.wheelStepPx;
            mouseWheel(scene, 720, 450, 0, -120);
            compare(scene.cameraY, -step);
            compare(scene.cameraX, 0);
            mouseWheel(scene, 720, 450, 0, 120);
            compare(scene.cameraY, 0);
        }

        function test_def08_wheel_clamps_at_content_boundaries() {
            loadMany(12);
            const limit = scene.panLimitY();
            for (let i = 0; i < 60; ++i)
                mouseWheel(scene, 720, 450, 0, -120);
            compare(scene.cameraY, -limit);
            for (let i = 0; i < 120; ++i)
                mouseWheel(scene, 720, 450, 0, 120);
            compare(scene.cameraY, limit);
        }

        function test_def08_horizontal_wheel_and_shift_wheel_pan_horizontally() {
            loadMany(12);
            scene.panBy(0, 0);
            // Force horizontal overflow by shrinking nothing: use the function-level contract.
            const limitX = scene.panLimitX();
            mouseWheel(scene, 720, 450, 120, 0);
            compare(scene.cameraX, Math.min(limitX, scene.wheelStepPx));
            scene.cameraX = 0;
            mouseWheel(scene, 720, 450, 0, 120, Qt.NoButton, Qt.ShiftModifier);
            compare(scene.cameraY, 0, "shift+vertical wheel must not pan vertically");
            compare(scene.cameraX, Math.min(limitX, scene.wheelStepPx));
        }

        function test_def08_smooth_pixel_delta_pans_one_to_one() {
            loadMany(12);
            scene.handleWheel(0, 0, 0, -37, 0);
            compare(scene.cameraY, -37);
            scene.handleWheel(0, 0, 0, -13, 0);
            compare(scene.cameraY, -50);
            scene.handleWheel(0, 0, 0, 50, 0);
            compare(scene.cameraY, 0);
        }

        function test_def08_hires_angle_delta_is_proportional() {
            loadMany(12);
            const step = scene.wheelStepPx;
            for (let i = 0; i < 8; ++i)
                scene.handleWheel(0, -15, 0, 0, 0);
            compare(scene.cameraY, -step, "8 x 15 hi-res units equal one 120-unit notch");
        }

        function test_def08_pixel_delta_takes_precedence_over_angle_delta() {
            loadMany(12);
            // Qt delivers both for touchpads; they describe the SAME motion.
            scene.handleWheel(0, -120, 0, -30, 0);
            compare(scene.cameraY, -30, "pixelDelta must win; the event must not be processed twice");
        }

        function test_def08_one_wheel_event_is_processed_exactly_once() {
            loadMany(12);
            const before = scene.wheelEventsHandled;
            mouseWheel(scene, 720, 450, 0, -120);
            compare(scene.wheelEventsHandled, before + 1);
            compare(scene.cameraY, -scene.wheelStepPx);
        }

        function test_def08_ctrl_wheel_changes_hierarchy_level_not_camera() {
            compare(scene.currentLevel, 1);
            mouseWheel(scene, 720, 450, 0, 120, Qt.NoButton, Qt.ControlModifier);
            compare(scene.currentLevel, 2);
            compare(scene.cameraX, 0);
            compare(scene.cameraY, 0);
            wait(scene.levelStepCooldownMs + 30);
            mouseWheel(scene, 720, 450, 0, 120, Qt.NoButton, Qt.ControlModifier);
            compare(scene.currentLevel, 3);
            wait(scene.levelStepCooldownMs + 30);
            mouseWheel(scene, 720, 450, 0, 120, Qt.NoButton, Qt.ControlModifier);
            compare(scene.currentLevel, 3, "no window cycling / overflow at the deepest level");
            wait(scene.levelStepCooldownMs + 30);
            mouseWheel(scene, 720, 450, 0, -120, Qt.NoButton, Qt.ControlModifier);
            compare(scene.currentLevel, 2);
            wait(scene.levelStepCooldownMs + 30);
            mouseWheel(scene, 720, 450, 0, -120, Qt.NoButton, Qt.ControlModifier);
            compare(scene.currentLevel, 1);
            wait(scene.levelStepCooldownMs + 30);
            mouseWheel(scene, 720, 450, 0, -120, Qt.NoButton, Qt.ControlModifier);
            compare(scene.currentLevel, 1);
            compare(closedSpy.count, 0, "wheel must never close the window");
        }

        function test_def08_ctrl_wheel_burst_makes_a_single_predictable_step() {
            mouseWheel(scene, 720, 450, 0, 120, Qt.NoButton, Qt.ControlModifier);
            mouseWheel(scene, 720, 450, 0, 120, Qt.NoButton, Qt.ControlModifier);
            mouseWheel(scene, 720, 450, 0, 120, Qt.NoButton, Qt.ControlModifier);
            compare(scene.currentLevel, 2, "a burst inside the cooldown is one level step");
        }

        function test_def08_wheel_is_inert_while_search_palette_is_open() {
            loadMany(12);
            keyClick(Qt.Key_Slash);
            mouseWheel(scene, 720, 450, 0, -120);
            compare(scene.cameraY, 0);
            keyClick(Qt.Key_Escape);
        }

        function test_def08_keyboard_navigation_keeps_focused_card_visible_after_panning() {
            loadMany(12);
            scene.cameraY = -scene.panLimitY();
            keyClick(Qt.Key_L);
            const card = findChild(scene, "workspaceCard-" + scene.currentWorkspaces[scene.workspaceIndex].id);
            verify(card !== null);
            const viewport = findChild(scene, "canvasViewport");
            const p = card.mapToItem(viewport, 0, 0);
            verify(p.y >= -1 && p.y + card.height <= viewport.height + 1, "focused card must be inside the viewport: y=" + p.y);
        }

        function test_def08_wheel_pan_resets_on_level_change_and_home() {
            loadMany(12);
            scene.handleWheel(0, 0, 0, -100, 0);
            verify(scene.cameraY !== 0);
            scene.returnHome();
            compare(scene.cameraY, 0);
            scene.handleWheel(0, 0, 0, -100, 0);
            scene.returnToParent();
            compare(scene.cameraY, 0, "level change resets the camera predictably");
        }

        function test_def08_motion_is_instant_with_reduced_motion_and_direct_otherwise() {
            loadMany(12);
            scene.reducedMotion = false;
            scene.handleWheel(0, 0, 0, -40, 0);
            compare(scene.cameraY, -40, "wheel pans are applied directly, never queued behind an animation");
            scene.reducedMotion = true;
            scene.handleWheel(0, 0, 0, -10, 0);
            compare(scene.cameraY, -50);
        }

        // ---------------------------------------------------------------- DEF-09 (SYNTHETIC logic)
        function test_def09_synthetic_pinch_in_drills_down_once_per_gesture() {
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(1.1);
            compare(scene.currentLevel, 1, "below threshold: no transition");
            scene.pinchUpdate(1.3);
            compare(scene.currentLevel, 2);
            scene.pinchUpdate(1.9);
            scene.pinchUpdate(2.4);
            compare(scene.currentLevel, 2, "latched: no repeated drilldown inside one gesture");
            scene.pinchUpdate(0.5);
            compare(scene.currentLevel, 2, "no reverse inside the same gesture");
            scene.pinchEnd(false);
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(1.4);
            compare(scene.currentLevel, 3, "a new gesture may transition again");
            scene.pinchEnd(false);
        }

        function test_def09_synthetic_pinch_out_returns_up_and_never_closes() {
            scene.currentLevel = 3;
            scene.syncIndices();
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(0.7);
            compare(scene.currentLevel, 2);
            scene.pinchEnd(false);
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(0.7);
            compare(scene.currentLevel, 1);
            scene.pinchEnd(false);
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(0.5);
            compare(scene.currentLevel, 1);
            compare(closedSpy.count, 0, "pinch must never close the window");
            scene.pinchEnd(false);
        }

        function test_def09_synthetic_pinch_in_at_deepest_level_is_a_noop() {
            scene.currentLevel = 3;
            scene.syncIndices();
            const before = be.selectedWindowId;
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(2.0);
            compare(scene.currentLevel, 3);
            compare(be.selectedWindowId, before, "pinch must not cycle windows");
            scene.pinchEnd(false);
        }

        function test_def09_synthetic_cancelled_or_interrupted_gesture_does_not_commit() {
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(1.1);
            scene.pinchEnd(true);
            compare(scene.currentLevel, 1);

            scene.pinchBegin(720, 450);
            scene.cancelGestures(); // e.g. window deactivated mid-gesture
            scene.pinchUpdate(1.6);
            compare(scene.currentLevel, 1, "updates after cancellation are ignored");
            scene.pinchEnd(false);
            compare(scene.currentLevel, 1);
        }

        function test_def09_synthetic_pinch_is_ignored_while_modal_is_open() {
            scene.openSearch();
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(1.8);
            compare(scene.currentLevel, 1);
            scene.pinchEnd(false);
            scene.returnToParent();
        }

        function test_def09_synthetic_pinch_targets_card_under_the_pointer() {
            const target = be.worlds[2];
            const card = findChild(scene, "worldCard-" + target.id);
            verify(card !== null);
            const p = card.mapToItem(scene, card.width / 2, card.height / 2);
            scene.pinchBegin(p.x, p.y);
            scene.pinchUpdate(1.5);
            compare(scene.currentLevel, 2);
            compare(be.selectedWorldId, target.id, "pinch drills into the world under the pointer");
            scene.pinchEnd(false);
        }

        // ---------------------------------------------------------------- DEF-11
        function test_def11_single_click_selects_without_entering() {
            const target = be.worlds[1];
            const card = findChild(scene, "worldCard-" + target.id);
            verify(card !== null);
            mouseClick(card);
            compare(be.selectedWorldId, target.id);
            compare(scene.worldIndex, 1, "mouse selection syncs the keyboard index");
            compare(scene.currentLevel, 1, "single click only selects");
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 2, "keyboard continues from the clicked card");
        }

        function test_def11_double_click_world_enters_level_b_exactly_once() {
            const target = be.worlds[1];
            const card = findChild(scene, "worldCard-" + target.id);
            const before = scene.levelTransitions;
            mouseDoubleClickSequence(card);
            compare(scene.currentLevel, 2);
            compare(be.selectedWorldId, target.id);
            compare(scene.levelTransitions, before + 1, "one double-click is exactly one transition");
        }

        function test_def11_double_click_workspace_enters_level_c_exactly_once() {
            scene.enterDeeper();
            compare(scene.currentLevel, 2);
            const target = scene.currentWorkspaces[1];
            const card = findChild(scene, "workspaceCard-" + target.id);
            verify(card !== null);
            mouseClick(card);
            compare(scene.currentLevel, 2);
            compare(scene.workspaceIndex, 1);
            const before = scene.levelTransitions;
            mouseDoubleClickSequence(card);
            compare(scene.currentLevel, 3);
            compare(be.selectedWorkspaceId, target.id);
            compare(scene.levelTransitions, before + 1);
        }

        function test_def11_unavailable_cards_reject_interaction() {
            const worlds = be.worlds.slice();
            worlds[1] = Object.assign({}, worlds[1], {
                unavailable: true
            });
            be.worlds = worlds;
            wait(20);
            const card = findChild(scene, "worldCard-" + worlds[1].id);
            verify(card !== null);
            compare(card.available, false);
            mouseClick(card);
            compare(be.selectedWorldId !== worlds[1].id, true, "click on unavailable card is rejected");
            mouseDoubleClickSequence(card);
            compare(scene.currentLevel, 1, "double-click on unavailable card is rejected");
        }

        function test_def11_window_card_double_click_never_activates_native_window() {
            scene.currentLevel = 3;
            scene.syncIndices();
            const target = scene.currentWindows[1];
            const card = findChild(scene, "windowCard-" + target.id);
            verify(card !== null);
            const before = scene.levelTransitions;
            mouseDoubleClickSequence(card);
            compare(scene.currentLevel, 3);
            compare(scene.levelTransitions, before);
            compare(be.selectedWindowId, target.id, "window cards select only (read-only integration)");
        }

        // ---------------------------------------------------------------- source indicators
        function test_source_backend_starts_with_no_data_and_no_fake_source() {
            const fresh = createTemporaryObject(freshBackendComponent, testRoot);
            verify(fresh !== null);
            compare(fresh.sourceKind, "none");
            compare(fresh.worlds.length, 0, "no implicit mock data");
            compare(fresh.observerLive, false);
            compare(fresh.mockMode, false);
        }

        function test_source_demo_is_labelled_and_watermarked() {
            compare(be.sourceKind, "demo");
            compare(be.sourceLabel, "Demo");
            compare(be.observerLabel, "Native Observer Unavailable", "demo data is never native observation");
            const label = findChild(scene, "sourceLabel");
            const observer = findChild(scene, "observerLabel");
            verify(label !== null && observer !== null);
            compare(label.text, "Demo");
            compare(observer.text, "Native Observer Unavailable");
            const watermark = findChild(scene, "demoWatermark");
            verify(watermark !== null);
            compare(watermark.visible, true);
            verify(watermark.text.indexOf("DEMO DATA") >= 0);
        }

        function test_source_local_sqlite_is_not_demo_and_has_no_watermark() {
            be.worlds = [{id: "w", name: "Default World", description: "d", icon: "x", color: "#fff", accent: "#fff"}];
            be.workspaces = [];
            be.desktops = [];
            be.windows = [];
            be.mockMode = false;
            be.sourceKind = "sqlite";
            be.sourceDetail = "/tmp/x.sqlite3";
            compare(be.sourceLabel, "Local SQLite");
            compare(findChild(scene, "sourceLabel").text, "Local SQLite");
            compare(findChild(scene, "observerLabel").text, "Native Observer Unavailable");
            compare(findChild(scene, "demoWatermark").visible, false);
        }

        function test_source_controller_connection_never_implies_native_observation() {
            be.mockMode = false;
            be.sourceKind = "controller";
            be.observerLive = false;
            be.observerDetail = "WaitingForObserver";
            compare(findChild(scene, "sourceLabel").text, "Connected Controller");
            compare(findChild(scene, "observerLabel").text, "Native Observer Unavailable");
            verify(findChild(scene, "observerLabel").text.indexOf("Live") < 0);
            be.observerLive = true;
            compare(findChild(scene, "observerLabel").text, "Native Observer Live");
        }

        function test_source_workspace_badges_are_not_hardcoded() {
            // Phase 4.1 hard-coded "Stale" for workspace index 1 and "Live" otherwise.
            be.mockMode = false;
            be.sourceKind = "sqlite";
            be.observerLive = false;
            scene.currentLevel = 2;
            scene.syncIndices();
            wait(20);
            for (let i = 0; i < scene.currentWorkspaces.length; ++i) {
                const card = findChild(scene, "workspaceCard-" + scene.currentWorkspaces[i].id);
                verify(card !== null);
                verify(card.statusBadge !== "Live", "no workspace may claim Live without a live observer");
                compare(card.statusBadge, "Unavailable");
            }
            be.sourceKind = "demo";
            be.mockMode = true;
            for (let i = 0; i < scene.currentWorkspaces.length; ++i)
                compare(findChild(scene, "workspaceCard-" + scene.currentWorkspaces[i].id).statusBadge, "Demo");
        }

        function test_demo_window_cards_are_not_painted_live() {
            be.loadMock();
            let desktopId = "";
            for (let i = 0; i < be.desktops.length; ++i) {
                if (be.getWindowsForDesktop(be.desktops[i].id).length > 0) {
                    desktopId = be.desktops[i].id;
                    break;
                }
            }
            verify(desktopId !== "");
            be.selectDesktop(desktopId);
            scene.currentLevel = 3;
            scene.syncIndices();
            wait(30);
            verify(scene.currentWindows.length > 0);
            for (let i = 0; i < scene.currentWindows.length; ++i) {
                const card = findChild(scene, "windowCard-" + scene.currentWindows[i].id);
                verify(card !== null);
                compare(card.demoSource, true);
                compare(card.paintedStatus, "Demo");
            }
            be.mockMode = false;
            be.sourceKind = "sqlite";
            wait(20);
            compare(findChild(scene, "windowCard-" + scene.currentWindows[0].id).demoSource, false);
            compare(findChild(scene, "windowCard-" + scene.currentWindows[0].id).paintedStatus, scene.currentWindows[0].statusBadge);
        }

        // ---------------------------------------------------------------- onboarding & empty states
        function test_onboarding_offer_shows_two_explicit_choices_and_nothing_is_created_implicitly() {
            be.worlds = [];
            be.workspaces = [];
            be.desktops = [];
            be.windows = [];
            be.mockMode = false;
            be.sourceKind = "none";
            be.databasePath = "/tmp/depthos-test/workspaces.sqlite3";
            be.databaseState = "missing";
            be.onboardingState = "offer";
            wait(20);
            const panel = findChild(scene, "onboardingPanel");
            verify(panel !== null);
            compare(panel.visible, true);
            const create = findChild(panel, "createWorkspaceButton");
            const demo = findChild(panel, "continueDemoButton");
            verify(create !== null && demo !== null);
            compare(create.text, "Create my workspace");
            compare(demo.text, "Continue in Demo Mode");
            compare(createSpy.count, 0, "no creation before the explicit click");
            verify(panel.messageText.indexOf("/tmp/depthos-test/workspaces.sqlite3") >= 0);
        }

        function test_onboarding_create_click_emits_request_and_enters_creating_state() {
            be.onboardingState = "offer";
            wait(20);
            mouseClick(findChild(scene, "createWorkspaceButton"));
            compare(createSpy.count, 1);
            compare(be.onboardingState, "creating");
            // A second click while creating must not issue a second request.
            mouseClick(findChild(scene, "createWorkspaceButton"));
            compare(createSpy.count, 1);
        }

        function test_onboarding_demo_click_loads_labelled_demo_data() {
            be.worlds = [];
            be.mockMode = false;
            be.sourceKind = "none";
            be.onboardingState = "offer";
            wait(20);
            mouseClick(findChild(scene, "continueDemoButton"));
            compare(demoSpy.count, 1);
            compare(be.onboardingState, "hidden");
            compare(be.sourceKind, "demo");
            verify(be.worlds.length >= 4);
            compare(findChild(scene, "demoWatermark").visible, true);
        }

        function test_onboarding_error_and_success_feedback_are_accurate() {
            be.onboardingState = "error";
            be.onboardingMessage = "Could not create the workspace";
            be.onboardingDetail = "depthos: database path is unsafe or inaccessible";
            wait(20);
            const panel = findChild(scene, "onboardingPanel");
            compare(panel.visible, true);
            verify(panel.messageText.indexOf("Could not create the workspace") >= 0);
            verify(panel.detailText.indexOf("unsafe or inaccessible") >= 0);
            verify(findChild(panel, "retryButton") !== null);
            be.onboardingState = "success";
            be.onboardingMessage = "Workspace created";
            wait(20);
            verify(panel.messageText.indexOf("Workspace created") >= 0);
            verify(findChild(panel, "openWorkspaceButton") !== null);
        }

        function test_onboarding_blocks_scene_keys_until_dismissed() {
            be.onboardingState = "offer";
            wait(20);
            keyClick(Qt.Key_Return);
            compare(scene.currentLevel, 1);
            be.onboardingState = "hidden";
            wait(20);
            scene.forceActiveFocus();
            keyClick(Qt.Key_Return);
            compare(scene.currentLevel, 2);
        }

        function test_empty_hierarchy_is_handled_gracefully() {
            be.worlds = [];
            be.workspaces = [];
            be.desktops = [];
            be.windows = [];
            be.selectedWorldId = "";
            be.selectedWorkspaceId = "";
            be.selectedDesktopId = "";
            be.mockMode = false;
            be.sourceKind = "sqlite";
            scene.currentLevel = 1;
            scene.syncIndices();
            wait(20);
            keyClick(Qt.Key_Return);
            keyClick(Qt.Key_L);
            keyClick(Qt.Key_J);
            compare(scene.currentLevel, 1, "no world to enter");
            verify(findChild(scene, "emptyHierarchyHint") !== null);
            compare(findChild(scene, "emptyHierarchyHint").visible, true);
        }

        function test_fresh_default_hierarchy_has_honest_empty_window_state() {
            be.worlds = [{id: "w1", name: "Default World", description: "Your default logical world", icon: "", color: "#6366F1", accent: "#818CF8"}];
            be.workspaces = [{id: "p1", worldId: "w1", name: "Primary Workspace", description: "Your primary logical workspace", rootDirectory: "", gitBranch: ""}];
            be.desktops = [{id: "d1", workspaceId: "p1", name: "Desktop 1", role: "Placeholder desktop"}];
            be.windows = [];
            be.mockMode = false;
            be.sourceKind = "sqlite";
            be.observerLive = false;
            be.selectWorld("w1");
            scene.currentLevel = 1;
            scene.syncIndices();
            keyClick(Qt.Key_Return);
            keyClick(Qt.Key_Return);
            compare(scene.currentLevel, 3);
            const hint = findChild(scene, "emptyWindowsHint");
            verify(hint !== null);
            compare(hint.visible, true);
            verify(hint.text.toLowerCase().indexOf("observer") >= 0, "must say the observer is unavailable, not that there are no windows: " + hint.text);
            be.observerLive = true;
            verify(hint.text.toLowerCase().indexOf("no native windows") >= 0, hint.text);
        }
    }
}
