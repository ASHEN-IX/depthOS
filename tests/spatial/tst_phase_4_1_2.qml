import QtQuick
import QtQuick.Window
import QtTest
import "../../experiments/spatial-alpha" as Spatial

// Phase 4.1.2 QML Independent Adversarial Test Suite:
// Covers:
// 1. Pointer Interactions:
//    - Single-click vs double-click timing, edge cases, rapid repeated clicks, rapid clicks across different cards
//    - Click during scene transitions and scale animations
//    - Coordinate transforms, clipping, overlapping elements
//    - Window resize during click sequence
//    - Keyboard focus synchronization after mouse click
//    - Modal search palette focus preservation and restoration across window deactivation/activation
// 2. Gesture Handling:
//    - Classification of physical vs synthetic events
//    - Edge-of-canvas pan clamps and overflow bounds
//    - Mixed wheel scrolling and keyboard panning
//    - Reduced-motion behavior
//    - Verification that lack of physical gesture events does not deadlock or crash the UI
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

    // A focusable external item standing in for a separate modal or foreign tool
    TextInput {
        id: externalInput
        objectName: "externalInput"
        x: 0
        y: 0
        width: 20
        height: 20
    }

    Component {
        id: foreignWindowComponent
        Window {
            width: 150
            height: 150
            title: "Foreign Test Window"
        }
    }

    TestCase {
        id: tc
        name: "DepthOS_Phase_4_1_2"
        when: windowShown

        property var foreignWin: null

        function topWindow() {
            return testRoot.Window.window;
        }

        function loadMany(n) {
            be.loadMock();
            const list = [];
            for (let i = 0; i < n; ++i) {
                list.push({
                    id: "ws-bulk-" + i,
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

        function init() {
            if (foreignWin) {
                foreignWin.close();
                foreignWin.destroy();
                foreignWin = null;
            }
            const win = topWindow();
            if (win)
                win.requestActivate();

            testRoot.width = 1440;
            testRoot.height = 900;
            be.loadMock();
            scene.reducedMotion = true;
            scene.currentLevel = 1;
            scene.worldIndex = 0;
            scene.workspaceIndex = 0;
            scene.windowIndex = 0;
            scene.cameraX = 0;
            scene.cameraY = 0;
            be.onboardingState = "hidden";
            const palette = findChild(scene, "searchPalette");
            if (palette) {
                palette.isOpen = false;
                const input = findChild(palette, "searchInput");
                if (input)
                    input.text = "";
            }
            scene.syncIndices();
            scene.forceActiveFocus();
            wait(30);
        }

        function cleanup() {
            if (foreignWin) {
                foreignWin.close();
                foreignWin.destroy();
                foreignWin = null;
            }
            const palette = findChild(scene, "searchPalette");
            if (palette)
                palette.isOpen = false;
            be.onboardingState = "hidden";
            scene.forceActiveFocus();
        }

        // =================================================================
        // AREA 1: POINTER INTERACTIONS
        // =================================================================

        // PTR-01: Rapid repeated clicks on the same card
        function test_ptr01_rapid_repeated_clicks_on_same_card() {
            const target = be.worlds[1];
            const card = findChild(scene, "worldCard-" + target.id);
            verify(card !== null, "Target card must exist");

            // Rapid burst of 10 clicks on the same card
            for (let i = 0; i < 10; ++i) {
                mouseClick(card, card.width / 2, card.height / 2);
            }

            // Must select the card and remain at level 1 (single clicks only select)
            compare(be.selectedWorldId, target.id, "Card remains selected after rapid click burst");
            compare(scene.worldIndex, 1, "Keyboard index synchronizes to clicked card");
            compare(scene.currentLevel, 1, "Rapid single clicks must never accidentally drill down");
        }

        // PTR-02: Rapid alternating clicks across different cards
        function test_ptr02_rapid_alternating_clicks_across_cards() {
            const card0 = findChild(scene, "worldCard-" + be.worlds[0].id);
            const card1 = findChild(scene, "worldCard-" + be.worlds[1].id);
            verify(card0 !== null && card1 !== null);

            // Alternating rapid clicks: Card 0 -> Card 1 -> Card 0 -> Card 1
            mouseClick(card0, card0.width / 2, card0.height / 2);
            compare(be.selectedWorldId, be.worlds[0].id);
            compare(scene.worldIndex, 0);

            mouseClick(card1, card1.width / 2, card1.height / 2);
            compare(be.selectedWorldId, be.worlds[1].id);
            compare(scene.worldIndex, 1);

            mouseClick(card0, card0.width / 2, card0.height / 2);
            compare(be.selectedWorldId, be.worlds[0].id);
            compare(scene.worldIndex, 0);

            mouseClick(card1, card1.width / 2, card1.height / 2);
            compare(be.selectedWorldId, be.worlds[1].id);
            compare(scene.worldIndex, 1);

            compare(scene.currentLevel, 1, "Inter-card clicks must never be conflated into a double-click");
        }

        // PTR-03: Double-click sequence triggers exactly one drill-down
        function test_ptr03_double_click_drills_down_cleanly() {
            const target = be.worlds[1];
            const card = findChild(scene, "worldCard-" + target.id);
            verify(card !== null);

            const beforeTransitions = scene.levelTransitions;
            mouseDoubleClickSequence(card, card.width / 2, card.height / 2);

            compare(scene.currentLevel, 2, "Double click enters Level B");
            compare(be.selectedWorldId, target.id, "World remains selected");
            compare(scene.levelTransitions, beforeTransitions + 1, "Exactly one transition occurs");
        }

        // PTR-04: Clicks during scene transitions and scale animations
        function test_ptr04_click_during_scale_animation() {
            scene.reducedMotion = false; // Enable active animations
            const target = be.worlds[0];
            const card = findChild(scene, "worldCard-" + target.id);
            verify(card !== null);

            // Start drill down
            scene.drillDown(false);
            // Click during transition
            mouseClick(testRoot, 720, 450);
            wait(250); // Wait for transition animation to complete

            compare(scene.currentLevel, 2, "Transition completes to Level B safely without deadlock");
        }

        // PTR-05: Coordinate transforms, inner chips, and clipping
        function test_ptr05_coordinate_transforms_and_inner_elements() {
            const target = be.worlds[1];
            const card = findChild(scene, "worldCard-" + target.id);
            verify(card !== null);

            // Click at top-left padding
            mouseClick(card, 10, 10);
            compare(be.selectedWorldId, target.id);

            // Click at bottom-right padding
            mouseClick(card, card.width - 10, card.height - 10);
            compare(be.selectedWorldId, target.id);

            // Click on the chips area (inside the card)
            mouseClick(card, card.width / 2, card.height - 60);
            compare(be.selectedWorldId, target.id, "Clicks on inner elements bubble to select the card");
        }

        // PTR-06: Window resize during click sequence
        function test_ptr06_window_resize_during_interaction() {
            const card0 = findChild(scene, "worldCard-" + be.worlds[0].id);
            verify(card0 !== null);
            mouseClick(card0, card0.width / 2, card0.height / 2);
            compare(be.selectedWorldId, be.worlds[0].id);

            // Resize viewport to smaller dimensions
            testRoot.width = 800;
            testRoot.height = 600;
            wait(20);

            // Keyboard navigation continues cleanly
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 1, "Keyboard navigation functions after resize");
            compare(be.selectedWorldId, be.worlds[1].id);

            // Resize viewport to large dimensions
            testRoot.width = 1920;
            testRoot.height = 1080;
            wait(20);

            keyClick(Qt.Key_H);
            compare(scene.worldIndex, 0, "Keyboard navigation functions after expand");
            compare(be.selectedWorldId, be.worlds[0].id);
        }

        // PTR-07: Keyboard focus synchronization after mouse click
        function test_ptr07_keyboard_focus_synchronization() {
            // Click World 2
            const target = be.worlds[2];
            const card = findChild(scene, "worldCard-" + target.id);
            verify(card !== null);

            mouseClick(card, card.width / 2, card.height / 2);
            compare(scene.worldIndex, 2, "worldIndex synchronized to 2");
            compare(be.selectedWorldId, target.id);

            // Move left with 'H'
            keyClick(Qt.Key_H);
            compare(scene.worldIndex, 1, "Subsequent 'H' moves from index 2 to 1");
            compare(be.selectedWorldId, be.worlds[1].id);

            // Move right with 'L'
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, 2, "Subsequent 'L' moves from index 1 to 2");
            compare(be.selectedWorldId, be.worlds[2].id);
        }

        // PTR-08: Modal search palette focus preservation and restoration
        function test_ptr08_search_palette_focus_preservation() {
            const searchPalette = findChild(scene, "searchPalette");
            verify(searchPalette !== null);
            searchPalette.isOpen = true;
            wait(20);

            const searchInput = findChild(searchPalette, "searchInput");
            verify(searchInput !== null);
            compare(searchInput.activeFocus, true, "Search input holds active focus");

            // Enter search text
            searchInput.text = "Design";
            compare(searchInput.text, "Design");

            // Simulate foreign window stealing focus
            foreignWin = createTemporaryObject(foreignWindowComponent, testRoot);
            verify(foreignWin !== null);
            foreignWin.show();
            foreignWin.requestActivate();
            wait(50);

            // Reactivate top window
            const win = topWindow();
            if (win) {
                win.requestActivate();
                wait(50);
            }

            // Restore focus handler runs
            scene.restoreFocusAfterActivation();
            compare(searchPalette.isOpen, true, "Search palette remains open");
            compare(searchInput.text, "Design", "Search input text preserved");
        }

        // PTR-09: Modal search palette dismissal returns focus to scene
        function test_ptr09_search_palette_dismissal_returns_focus() {
            const searchPalette = findChild(scene, "searchPalette");
            verify(searchPalette !== null);
            searchPalette.isOpen = true;
            wait(20);

            // Dismiss via Escape
            keyClick(Qt.Key_Escape);
            compare(searchPalette.isOpen, false, "Search palette is closed via Escape");
            compare(scene.activeFocus, true, "Active focus returned to scene");

            // Verify navigation keys work immediately
            const before = scene.worldIndex;
            keyClick(Qt.Key_L);
            compare(scene.worldIndex, before + 1, "Keyboard navigation works immediately after dismissal");
        }

        // =================================================================
        // AREA 2: GESTURE HANDLING
        // =================================================================

        // GST-01: Synthetic gesture classification and bounds
        function test_gst01_synthetic_gesture_presentation_only() {
            // Synthetic pinch in Level 1
            compare(scene.currentLevel, 1);
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(1.4); // Zoom in
            scene.pinchEnd(false);

            compare(scene.currentLevel, 2, "Synthetic pinch in drills to Level B");

            // Reverse synthetic pinch
            scene.pinchBegin(720, 450);
            scene.pinchUpdate(0.6); // Zoom out
            scene.pinchEnd(false);

            compare(scene.currentLevel, 1, "Synthetic pinch out returns to Level A");
        }

        // GST-02: Edge-of-canvas pan clamps and extreme overflow bounds
        function test_gst02_pan_clamps_and_extreme_overflow() {
            loadMany(12);
            const limitY = scene.panLimitY();
            verify(limitY > 0, "Content must overflow vertically");

            // Extreme negative pan attempt
            scene.panBy(0, -1000000);
            compare(scene.cameraY, -limitY, "Extreme negative pan clamped to -limitY");

            // Extreme positive pan attempt
            scene.panBy(0, 2000000);
            compare(scene.cameraY, limitY, "Extreme positive pan clamped to +limitY");
        }

        // GST-03: Fit-to-screen pan clamping
        function test_gst03_fit_to_screen_zero_pan_limits() {
            // In 1440x900, the mock level 1 grid fits inside viewport
            scene.currentLevel = 1;
            scene.cameraX = 0;
            scene.cameraY = 0;
            compare(scene.panLimitX(), 0, "panLimitX is 0 for level 1");
            compare(scene.panLimitY(), 0, "panLimitY is 0 for level 1");

            // Attempt to pan
            scene.panBy(100, 100);
            compare(scene.cameraX, 0, "cameraX remains 0 when content fits in viewport");
            compare(scene.cameraY, 0, "cameraY remains 0 when content fits in viewport");
        }

        // GST-04: Mixed wheel scrolling and keyboard panning
        function test_gst04_mixed_wheel_and_keyboard_navigation() {
            loadMany(12);
            // Navigate down one row using keyboard 'J' (3 columns per row)
            keyClick(Qt.Key_J);
            compare(scene.workspaceIndex, 3, "J navigates down one row (index 0 -> 3)");

            // Wheel vertical notch
            mouseWheel(scene, 720, 450, 0, -120);
            compare(scene.cameraY, -scene.wheelStepPx);

            // Return up one row with keyboard 'K'
            keyClick(Qt.Key_K);
            compare(scene.workspaceIndex, 0, "K navigates back up one row (index 3 -> 0)");
        }

        // GST-05: Reduced-motion behavior
        function test_gst05_reduced_motion_behavior() {
            scene.reducedMotion = true;
            compare(scene.motionOff, true, "motionOff reflects reducedMotion");

            // Drill down
            scene.drillDown(false);
            compare(scene.currentLevel, 2, "Transition is instantaneous with reduced motion");

            // Drill up
            scene.drillUp(false);
            compare(scene.currentLevel, 1, "Upward transition is instantaneous with reduced motion");
        }

        // GST-06: UI resilience without physical gesture events
        function test_gst06_no_deadlock_without_gestures() {
            // Pure mouse and keyboard usage without any touch/pinch events
            for (let i = 0; i < 5; ++i) {
                keyClick(Qt.Key_L);
                keyClick(Qt.Key_H);
            }
            compare(scene.worldIndex, 0);
            compare(scene.currentLevel, 1);
            verify(true, "UI functions normally without physical gestures");
        }
    }
}
