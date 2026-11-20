import QtQuick
import QtTest
import "../../experiments/spatial-alpha" as Spatial

Item {
    id: root
    width: 1440
    height: 900

    Spatial.SpatialBackend { id: backend }
    Spatial.SpatialScene {
        id: scene
        anchors.fill: parent
        backend: backend
    }

    TestCase {
        id: tests
        name: "DepthOS_Phase_4_2"
        when: windowShown

        function init() {
            root.width = 1440;
            root.height = 900;
            backend.loadMock();
            scene.setLevel(1);
            scene.resetInteractionClocks();
            scene.reducedMotion = true;
            scene.lowEffects = false;
            scene.highContrast = false;
            scene.showTitles = false;
            scene.forceActiveFocus();
            wait(20);
        }

        function test_peripheral_mouse_selection_keeps_double_click_target() {
            scene.reducedMotion = false;
            const card = findChild(scene, "worldCard-world-security");
            verify(card !== null);
            const first = card.mapToItem(scene, card.width / 2, card.height / 2);
            mouseClick(scene, first.x, first.y);
            compare(backend.selectedWorldId, "world-security");
            compare(scene.carouselIndex, 0);
            wait(120);
            const settled = card.mapToItem(scene, card.width / 2, card.height / 2);
            verify(Math.abs(settled.x - first.x) < 0.01 && Math.abs(settled.y - first.y) < 0.01,
                   "The projected hit target stays within the same physical pixel");
            mouseDoubleClickSequence(scene, first.x, first.y);
            compare(scene.currentLevel, 2);
            compare(backend.selectedWorldId, "world-security");
        }

        function test_wheel_browses_and_accumulates_smooth_deltas_once() {
            scene.handleWheel(0, -120, 0, -35, 0);
            compare(scene.worldIndex, 0);
            scene.handleWheel(0, -120, 0, -35, 0);
            compare(scene.worldIndex, 1);
            scene.handleWheel(0, 120, 0, 0, 0);
            compare(scene.worldIndex, 0);
        }

        function test_wayland_discrete_wheel_with_pixel_fallback_browses_once() {
            scene.handleWheel(0, -120, 0, -15, 0, Qt.NoScrollPhase, PointerDevice.TouchPad);
            compare(scene.worldIndex, 1);
            scene.handleWheel(0, -420, 0, -35, 0, Qt.NoScrollPhase, PointerDevice.TouchPad);
            compare(scene.worldIndex, 1);
            scene.handleWheel(0, -420, 0, -35, 0, Qt.NoScrollPhase, PointerDevice.TouchPad);
            compare(scene.worldIndex, 2);
        }

        function test_horizontal_wheel_reaches_carousel_once() {
            const before = scene.wheelEventsHandled;
            mouseWheel(scene, 550, 450, -120, 0);
            compare(scene.worldIndex, 1);
            compare(scene.wheelEventsHandled, before + 1);
        }

        function test_header_short_labels_fit_their_buttons() {
            for (const name of ["searchButton", "controlCenterButton"]) {
                const button = findChild(scene, name);
                verify(button !== null);
                tryCompare(button.contentItem, "truncated", false);
            }
        }

        function test_short_worlds_view_keeps_heading_and_card_counts_visible() {
            root.width = 630;
            root.height = 470;
            const heading = findChild(scene, "worldsHeading");
            const card = findChild(scene, "worldCard-world-engineering");
            const counts = findChild(card, "worldCardCounts");
            tryVerify(() => card.mapToItem(scene, 0, 0).y >= heading.mapToItem(scene, 0, heading.height).y + 8);
            verify(counts.mapToItem(card, 0, counts.height).y <= card.height - 8);
        }

        function test_worlds_responsive_controls_bounds_data() {
            return [{ w: 1920, h: 1080 }, { w: 1440, h: 900 }, { w: 1366, h: 768 },
                    { w: 1024, h: 768 }, { w: 800, h: 600 }, { w: 630, h: 470 }];
        }

        function test_worlds_responsive_controls_bounds(size) {
            root.width = size.w;
            root.height = size.h;
            const paging = findChild(scene, "worldPaging");
            const open = findChild(scene, "openWorldButton");
            const card = findChild(scene, "worldCard-world-engineering");
            const neighbor = findChild(scene, "worldCard-world-security");
            for (const item of [paging, open, card]) {
                verify(item.visible, item.objectName);
                const top = item.mapToItem(scene, 0, 0);
                const bottom = item.mapToItem(scene, item.width, item.height);
                verify(top.x >= 0 && top.y >= 0 && bottom.x <= size.w && bottom.y <= size.h,
                       item.objectName + " remains inside the viewport");
            }
            compare(neighbor.visible, size.w >= 740);
            keyClick(Qt.Key_Right);
            compare(backend.selectedWorldId, "world-security");
        }

        function test_reference_catalog_and_live_chrome() {
            root.width = 1672;
            root.height = 941;
            backend.loadReferenceDemo();
            compare(backend.selectedWorldId, "reference-research");
            compare(backend.worlds.length, 4);
            compare(backend.workspaces.length, 12);
            compare(backend.desktops.length, 36);
            mouseClick(findChild(scene, "railSettings"));
            compare(findChild(scene, "controlCenter").isOpen, true);
            keyClick(Qt.Key_Escape);
            mouseClick(findChild(scene, "searchButton"));
            compare(findChild(scene, "searchPalette").isOpen, true);
            keyClick(Qt.Key_Escape);
            mouseClick(findChild(scene, "topLevel3"));
            compare(scene.currentLevel, 3);
            mouseClick(findChild(scene, "railLevel1"));
            compare(scene.currentLevel, 1);
            const options = findChild(scene, "worldPreview-reference-research-2-options");
            const preview = findChild(scene, "worldPreview-reference-research-2");
            mouseClick(options);
            tryCompare(preview, "optionsOpen", true);
            keyClick(Qt.Key_Escape);
            tryCompare(preview, "optionsOpen", false);
            mouseClick(findChild(scene, "openWorldButton"));
            compare(scene.currentLevel, 2);
        }

        function test_workspace_preview_selects_workspace_and_opens_matrix() {
            const preview = findChild(scene, "worldPreview-ws-spatial-ux");
            verify(preview && preview.visible);
            mouseClick(preview);
            compare(scene.currentLevel, 2);
            compare(backend.selectedWorkspaceId, "ws-spatial-ux");
            scene.returnToParent();
            compare(backend.selectedWorkspaceId, "ws-spatial-ux");
        }

        function test_workspace_rows_follow_viewport_columns() {
            const base = backend.currentWorkspace;
            backend.workspaces = backend.workspaces.concat(
                { id: "extra-workspace", worldId: base.worldId, name: "Extra" });
            scene.enterDeeper();
            const matrix = findChild(scene, "workspacesView");
            compare(matrix.columns, 3);
            root.width = 800;
            tryCompare(matrix, "columns", 2);
            scene.moveVertical(1);
            compare(scene.workspaceIndex, 2);
            root.width = 560;
            tryCompare(matrix, "columns", 1);
            scene.moveVertical(-1);
            compare(scene.workspaceIndex, 1);
        }

        function test_search_reindexes_and_jumps_to_exact_cross_world_desktop() {
            const target = backend.desktops.find(d => d.id === "desk-formal-proofs");
            verify(target !== undefined);
            const oldName = target.name;
            backend.desktops = backend.desktops.map(d => d.id === target.id
                ? Object.assign({}, d, { name: "New <b>proof</b> area" }) : d);
            const jump = backend.search("! new <b>proof</b>")[0];
            verify(jump !== undefined);
            verify(jump.breadcrumb.includes("Security Research"));
            verify(!backend.search(oldName).some(result => result.id === target.id));
            keyClick(Qt.Key_Slash);
            const palette = findChild(scene, "searchPalette");
            const field = findChild(palette, "searchInput");
            field.text = "new <b>proof</b>";
            verify(palette.highlightName(jump.name).includes("&lt;b&gt;"));
            keyClick(Qt.Key_Return);
            compare(scene.currentLevel, 3);
            compare(backend.selectedDesktopId, target.id);
            compare(backend.selectedWorkspaceId, target.workspaceId);
            compare(backend.selectedWorldId, "world-security");
        }

        function test_control_center_presentation_toggles_and_focus() {
            const source = backend.sourceKind;
            const selected = backend.selectedWorldId;
            keyClick(Qt.Key_F2);
            const center = findChild(scene, "controlCenter");
            verify(center.isOpen);
            const low = findChild(center, "lowEffectsToggle");
            const contrast = findChild(center, "highContrastToggle");
            mouseClick(low);
            mouseClick(contrast);
            compare(scene.lowEffects, true);
            compare(scene.highContrast, true);
            keyClick(Qt.Key_Escape);
            compare(center.isOpen, false);
            verify(scene.activeFocus);
            compare(backend.sourceKind, source);
            compare(backend.selectedWorldId, selected);
        }

        function test_narrow_control_center_keeps_keyboard_focused_toggle_visible() {
            root.width = 630;
            root.height = 470;
            keyClick(Qt.Key_F2);
            const center = findChild(scene, "controlCenter");
            const viewport = findChild(center, "controlCenterViewport");
            for (const name of ["lowEffectsToggle", "highContrastToggle", "showTitlesToggle"]) {
                keyClick(Qt.Key_Tab);
                const toggle = findChild(center, name);
                tryCompare(toggle, "activeFocus", true);
                verify(toggle.mapToItem(viewport, 0, 0).y >= 0);
                verify(toggle.mapToItem(viewport, 0, toggle.height).y <= viewport.height);
            }
            keyClick(Qt.Key_Tab, Qt.ShiftModifier);
            tryCompare(findChild(center, "highContrastToggle"), "activeFocus", true);
            keyClick(Qt.Key_Escape);
            verify(scene.activeFocus);
        }

        function test_privacy_rendering_strips_control_codes_and_protects_identity() {
            scene.enterDeeper();
            scene.enterDeeper();
            const first = scene.currentWindows[0];
            backend.windows = backend.windows.map(w => w.id === first.id
                ? Object.assign({}, w, { title: "Left\u202eRight\u001b[2K", titleRedacted: false }) : w);
            let card = findChild(scene, "windowCard-" + first.id);
            verify(card !== null);
            compare(card.safeCaption, "");
            scene.showTitles = true;
            compare(card.safeCaption, "Left Right [2K");
            backend.windows = backend.windows.map(w => w.id === first.id
                ? Object.assign({}, w, { excludeFromCapture: true }) : w);
            card = findChild(scene, "windowCard-" + first.id);
            compare(card.safeCaption, "");
            compare(card.appIdentity, "Protected window");
        }

        function test_rapid_transitions_leave_a_single_enabled_view() {
            scene.reducedMotion = false;
            keyClick(Qt.Key_Return);
            keyClick(Qt.Key_Return);
            keyClick(Qt.Key_Escape);
            keyClick(Qt.Key_Return);
            keyClick(Qt.Key_Escape);
            keyClick(Qt.Key_Escape);
            keyClick(Qt.Key_Right);
            compare(scene.currentLevel, 1);
            compare(scene.worldIndex, 1);
            tryCompare(findChild(scene, "worldsView"), "opacity", 1);
            tryCompare(findChild(scene, "workspacesView"), "visible", false);
            tryCompare(findChild(scene, "desktopsView"), "visible", false);
        }
    }
}
