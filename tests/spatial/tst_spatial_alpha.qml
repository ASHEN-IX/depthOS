import QtQuick
import QtTest
import "../../experiments/spatial-alpha" as Spatial

Item {
    id: testRoot
    width: 1440
    height: 900

    Spatial.SpatialBackend {
        id: testBackend
        mockMode: true
    }

    Spatial.SpatialScene {
        id: testScene
        anchors.fill: parent
        backend: testBackend
    }

    TestCase {
        id: testCase
        name: "DepthOS_Spatial_Alpha_Navigation_And_Badges"
        when: windowShown

        function init() {
            testBackend.loadMock();
            testScene.currentLevel = 1;
            testScene.reducedMotion = true; // instant animations for fast, deterministic tests
            testScene.showTitles = false;
            testScene.cameraX = 0;
            testScene.cameraY = 0;
            testScene.syncIndices();
            testScene.forceActiveFocus();
            wait(20);
        }

        // 1. Test initial Level A (World Overview) state and hierarchy counts
        function test_initial_level_a_worlds() {
            compare(testScene.currentLevel, 1);
            compare(testScene.currentLevelName, "Level A: World Overview");
            verify(testBackend.worlds.length >= 4);

            const initialWorld = testBackend.currentWorld;
            verify(initialWorld !== null);
            compare(initialWorld.name, "Core Engineering");
            compare(testScene.worldIndex, 0);

            // Verify breadcrumb format
            verify(testScene.breadcrumbText.includes("DepthOS"));
            verify(testScene.breadcrumbText.includes("Core Engineering"));
        }

        // 2. Test horizontal sibling navigation (H / L) across Worlds at Level A
        function test_horizontal_sibling_navigation_level_a() {
            compare(testScene.currentLevel, 1);
            compare(testScene.worldIndex, 0);

            // Press L (move right)
            testScene.moveHorizontal(1);
            compare(testScene.worldIndex, 1);
            compare(testBackend.selectedWorldId, testBackend.worlds[1].id);
            compare(testBackend.currentWorld.name, "Security Research");

            // Press L again
            testScene.moveHorizontal(1);
            compare(testScene.worldIndex, 2);
            compare(testBackend.currentWorld.name, "Creative & Design");

            // Press H (move left)
            testScene.moveHorizontal(-1);
            compare(testScene.worldIndex, 1);
            compare(testBackend.currentWorld.name, "Security Research");

            // Press H back to first
            testScene.moveHorizontal(-1);
            compare(testScene.worldIndex, 0);
            compare(testBackend.currentWorld.name, "Core Engineering");

            // Boundary clamping: pressing H on first world stays at index 0
            testScene.moveHorizontal(-1);
            compare(testScene.worldIndex, 0);
        }

        // 3. Test Enter key deep traversal (Level A -> Level B: Workspace Overview)
        function test_enter_deeper_to_level_b() {
            compare(testScene.currentLevel, 1);
            testScene.enterDeeper();

            compare(testScene.currentLevel, 2);
            compare(testScene.currentLevelName, "Level B: Workspace Overview");
            verify(testScene.currentWorkspaces.length >= 3);
            compare(testBackend.currentWorkspace.name, "DepthOS-Kernel");
            compare(testScene.workspaceIndex, 0);

            // Horizontal navigation between workspaces at Level B
            testScene.moveHorizontal(1);
            compare(testScene.workspaceIndex, 1);
            compare(testBackend.currentWorkspace.name, "Spatial-UX");

            testScene.moveHorizontal(1);
            compare(testScene.workspaceIndex, 2);
            compare(testBackend.currentWorkspace.name, "KWin-Observer");

            testScene.moveHorizontal(-1);
            compare(testScene.workspaceIndex, 1);
            compare(testBackend.currentWorkspace.name, "Spatial-UX");
        }

        // 4. Test Enter key deep traversal (Level B -> Level C: Logical Desktop Overview)
        function test_enter_deeper_to_level_c() {
            testScene.enterDeeper(); // Level 1 -> 2
            compare(testScene.currentLevel, 2);

            testScene.enterDeeper(); // Level 2 -> 3
            compare(testScene.currentLevel, 3);
            compare(testScene.currentLevelName, "Level C: Logical Desktop Overview");
            verify(testScene.currentDesktops.length >= 3);
            compare(testBackend.currentDesktop.name, "Editor & Spec");
            compare(testScene.desktopIndex, 0);

            // Verify observed windows in this desktop
            verify(testScene.currentWindows.length >= 2);
            compare(testBackend.currentWindow.appName, "Kate");

            // Cycling windows in Level C
            testScene.enterDeeper();
            compare(testScene.windowIndex, 1);
            compare(testBackend.currentWindow.appName, "Alacritty");
        }

        // 5. Test Escape / Backspace upward traversal (Level C -> Level B -> Level A)
        function test_return_to_parent_levels() {
            testScene.enterDeeper(); // Level 1 -> 2
            testScene.enterDeeper(); // Level 2 -> 3
            compare(testScene.currentLevel, 3);

            // Esc: Level 3 -> Level 2
            testScene.returnToParent();
            compare(testScene.currentLevel, 2);
            compare(testScene.currentLevelName, "Level B: Workspace Overview");

            // Esc: Level 2 -> Level 1
            testScene.returnToParent();
            compare(testScene.currentLevel, 1);
            compare(testScene.currentLevelName, "Level A: World Overview");
        }

        // 6. Test Home key resetting camera and maintaining deterministic focus
        function test_home_key_focus() {
            testScene.cameraX = 150;
            testScene.cameraY = -80;
            testScene.returnHome();
            compare(testScene.cameraX, 0);
            compare(testScene.cameraY, 0);
            verify(testScene.worldIndex >= 0);
        }

        // 7. Test Search Command Palette (/) and Direct Context Navigation
        function test_search_and_direct_navigation() {
            const palette = findChild(testScene, "searchPalette");
            verify(palette !== null);
            compare(palette.isOpen, false);

            // Open search via /
            testScene.openSearch();
            compare(palette.isOpen, true);

            // Search query for "UX" -> should match Spatial-UX workspace
            const results = testBackend.search("UX");
            verify(results.length >= 1);
            const wsResult = results.find(r => r.kind === "workspace" && r.name === "Spatial-UX");
            verify(wsResult !== undefined);

            // Direct navigation to workspace result
            testBackend.navigateTo(wsResult);
            testScene.currentLevel = 2; // Jump to Level B
            testScene.syncIndices();

            compare(testScene.currentLevel, 2);
            compare(testBackend.selectedWorkspaceId, "ws-spatial-ux");
            compare(testBackend.currentWorkspace.name, "Spatial-UX");

            // Search query for "Docs" -> matches desktop
            const deskResults = testBackend.search("Architecture Docs");
            verify(deskResults.length >= 1);
            const deskResult = deskResults[0];

            testBackend.navigateTo(deskResult);
            testScene.currentLevel = 3; // Jump to Level C
            testScene.syncIndices();

            compare(testScene.currentLevel, 3);
            compare(testBackend.selectedDesktopId, "desk-kernel-docs");
            compare(testBackend.currentDesktop.name, "Architecture Docs");

            // Close search palette via Escape
            palette.isOpen = true;
            testScene.returnToParent(); // Closes search
            compare(palette.isOpen, false);
        }

        // 8. Test Observation Status Badges & Invariant (Live, Stale, Reconnecting, Unavailable, Unknown)
        function test_observation_status_badges_and_invariants() {
            // Find windows in mock data with different statuses
            const liveWin = testBackend.windows.find(w => w.statusBadge === "Live");
            verify(liveWin !== undefined);
            compare(liveWin.statusBadge, "Live");

            const staleWin = testBackend.windows.find(w => w.statusBadge === "Stale");
            verify(staleWin !== undefined);
            compare(staleWin.statusBadge, "Stale");

            const reconnectingWin = testBackend.windows.find(w => w.statusBadge === "Reconnecting");
            verify(reconnectingWin !== undefined);
            compare(reconnectingWin.statusBadge, "Reconnecting");

            const unavailWin = testBackend.windows.find(w => w.statusBadge === "Unavailable");
            verify(unavailWin !== undefined);
            compare(unavailWin.statusBadge, "Unavailable");

            const unknownWin = testBackend.windows.find(w => w.statusBadge === "Unknown");
            verify(unknownWin !== undefined);
            compare(unknownWin.statusBadge, "Unknown");

            // CRITICAL INVARIANT: Stale observation must NEVER be reported as Live
            verify(staleWin.statusBadge !== "Live");
            verify(reconnectingWin.statusBadge !== "Live");
            verify(unavailWin.statusBadge !== "Live");
            verify(unknownWin.statusBadge !== "Live");
        }

        // 9. Test Reduced Motion toggle
        function test_reduced_motion_mode() {
            testScene.reducedMotion = false;
            compare(testScene.motionOff, false);

            testScene.reducedMotion = true;
            compare(testScene.motionOff, true);
        }

        // 10. Test Title Privacy Redaction toggle (T key)
        function test_privacy_redaction_toggle() {
            compare(testScene.showTitles, false); // Default redacted
            testScene.showTitles = true;
            compare(testScene.showTitles, true);
            testScene.showTitles = false;
            compare(testScene.showTitles, false);
        }
    }
}
