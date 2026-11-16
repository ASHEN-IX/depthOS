import QtQuick
import QtTest
import org.kde.kwin
import "Policy.js" as Policy

Item {
    width: 1920; height: 1080
    Component {
        id: fakeWindow
        QtObject {
            property string internalId: "window-test"
            property var caption: "Synthetic test title"
            property string desktopFileName: "test.application"
            property string resourceClass: "test"
            property var icon: "application-x-executable"
            property bool normalWindow: true
            property bool dialog: false
            property bool onAllDesktops: false
            property var desktops: [Workspace.firstDesktop]
            property var activities: []
            property var frameGeometry: Qt.rect(100, 80, 800, 600)
            property var output: Workspace.firstOutput
            property var minimized: false
            property var fullScreen: false
            property var excludeFromCapture: false
            property int stackingOrder: 1
        }
    }
    TestCase {
        id: testCase
        name: "DepthOS_Native_Overview_MockBoundary"
        when: windowShown
        property var effect
        property var view
        property var scene
        property var grid
        property var created: []
        property var extraViews: []

        function init() {
            failOnWarning(/.*/);
            Workspace.desktops = [Workspace.firstDesktop, Workspace.secondDesktop];
            Workspace.activities = ["activity-a", "activity-b"];
            Workspace.currentDesktop = Workspace.firstDesktop;
            Workspace.currentActivity = "activity-a";
            Workspace.screens = [Workspace.firstOutput];
            Workspace.activeWindow = null;
            SceneView.screen = Workspace.firstOutput;
            const component = Qt.createComponent("main.qml");
            compare(component.status, Component.Ready, component.errorString());
            effect = component.createObject(testCase, {reducedMotion: true, backendEnabled: false});
            verify(effect !== null);
            effect.open();
            view = effect.delegate.createObject(testCase.parent, {width: 1920, height: 1080});
            verify(view !== null);
            tryVerify(() => findChild(view, "depthos-overview-scene") !== null);
            scene = findChild(view, "depthos-overview-scene");
            grid = findChild(view, "native-window-grid");
            verify(grid !== null);
            scene.forceActiveFocus();
            wait(20);
        }
        function cleanup() {
            for (const item of extraViews) item.destroy();
            extraViews = [];
            view.destroy(); effect.destroy();
            for (const w of created) w.destroy();
            created = [];
            wait(0);
        }
        function add(properties) {
            const w = fakeWindow.createObject(testCase, properties || {});
            created.push(w); effect.windowSource.append({window: w}); return w;
        }
        function card(index) {
            grid.forceLayout();
            return findChild(grid.itemAtIndex(index || 0), "native-window-card");
        }

        function test_initial_empty_and_dynamic_add_remove() {
            compare(grid.count, 0);
            const w = add();
            tryCompare(grid, "count", 1);
            verify(card(0) !== null);
            effect.selectedWindowId = w.internalId;
            effect.windowSource.remove(0); Workspace.windowRemoved(w);
            tryCompare(grid, "count", 0);
            compare(effect.selectedWindowId, "");
        }
        function test_titles_privacy_focus_and_window_state() {
            const w = add(); tryCompare(grid, "count", 1);
            const c = card(0), title = findChild(c, "window-title");
            compare(title.text, "test.application");
            effect.showTitles = true; compare(title.text, "Synthetic test title");
            w.caption = "Updated synthetic title"; compare(title.text, "Updated synthetic title");
            Workspace.activeWindow = w; compare(c.focusedWindow, true);
            w.minimized = true; w.fullScreen = true;
            verify(findChild(c, "window-state").text.indexOf("Minimized") >= 0);
            verify(findChild(c, "window-state").text.indexOf("Fullscreen") >= 0);
            w.excludeFromCapture = true; compare(title.text, "test.application");
            compare(findChild(c, "native-preview-loader").active, false);
        }
        function test_membership_selection_does_not_switch_native_state() {
            const w = add({desktops: [Workspace.secondDesktop]});
            compare(grid.count, 0);
            effect.inspectDesktop(Workspace.secondDesktop);
            tryCompare(grid, "count", 1);
            compare(Workspace.currentDesktop, Workspace.firstDesktop);
            compare(effect.intendedDesktopId, "desk-b");
            w.desktops = [Workspace.firstDesktop]; effect.windowSource.touch();
            tryCompare(grid, "count", 0);
            w.onAllDesktops = true; effect.windowSource.touch();
            tryCompare(grid, "count", 1);
            compare(Policy.membership(w), "All desktops");
        }
        function test_activity_filter_and_removed_desktop_reconcile() {
            add({activities: ["activity-b"]}); compare(grid.count, 0);
            effect.inspectActivity("activity-b"); tryCompare(grid, "count", 1);
            compare(Workspace.currentActivity, "activity-a");
            effect.inspectDesktop(Workspace.secondDesktop);
            Workspace.desktops = [Workspace.firstDesktop];
            compare(effect.selectedDesktop, Workspace.firstDesktop);
            compare(effect.intendedDesktopId, "");
            Workspace.activities = ["activity-a"];
            compare(effect.selectedActivity, "activity-a");
        }
        function test_active_context_changes_preserve_local_inspection() {
            Workspace.currentDesktop = Workspace.secondDesktop;
            Workspace.currentActivity = "activity-b";
            compare(effect.selectedDesktop, Workspace.firstDesktop);
            compare(effect.selectedActivity, "activity-a");
            const status = findChild(view, "native-context-status");
            verify(status.text.indexOf("Active Activity: activity-b") >= 0);
            verify(status.text.indexOf("Inspecting Activity: activity-a") >= 0);
            Workspace.activities = ["activity-a"];
            effect.inspectActivity("activity-a");
            Workspace.activities = ["activity-c"];
            compare(effect.selectedActivity, "activity-c");
        }
        function test_arrow_enter_zoom_escape_and_shortcut() {
            add(); add({internalId: "second"}); tryCompare(grid, "count", 2);
            grid.forceLayout(); scene.forceActiveFocus();
            keyClick(Qt.Key_Right); compare(grid.currentIndex, 1);
            keyClick(Qt.Key_Return); compare(effect.selectedWindowId, "second");
            compare(Workspace.activeWindow, null);
            keyClick(Qt.Key_Minus); compare(effect.level, 2);
            keyClick(Qt.Key_Right); keyClick(Qt.Key_Return);
            compare(effect.level, 1); compare(effect.selectedDesktop, Workspace.secondDesktop);
            keyClick(Qt.Key_Minus); keyClick(Qt.Key_Minus); compare(effect.level, 3);
            keyClick(Qt.Key_Plus); compare(effect.level, 2);
            const shortcut = findChild(effect, "depthos-overview-shortcut");
            compare(shortcut.sequence, "");
            keyClick(Qt.Key_Escape); tryCompare(effect, "visible", false);
            shortcut.activated(); tryCompare(effect, "visible", true);
            shortcut.activated(); tryCompare(effect, "visible", false);
        }
        function test_preview_policy_and_metadata_fallbacks() {
            const w = add(); tryCompare(grid, "count", 1);
            compare(Policy.previewReason(w, Workspace, true, 0, 6), "");
            verify(Policy.previewReason(w, Workspace, true, 6, 6).indexOf("budget") >= 0);
            w.minimized = true; verify(card(0).previewReason.indexOf("metadata only") >= 0);
            w.minimized = false; effect.inspectDesktop(Workspace.secondDesktop);
            w.onAllDesktops = true; effect.windowSource.touch(); tryCompare(grid, "count", 1);
            // Sticky is actually on the active desktop, so preview is allowed.
            compare(Policy.previewReason(w, Workspace, true, 0, 6), "");
            w.onAllDesktops = false; w.desktops = [Workspace.secondDesktop];
            verify(Policy.previewReason(w, Workspace, true, 0, 6).indexOf("Inactive desktop") >= 0);
            w.excludeFromCapture = null; verify(Policy.previewReason(w, Workspace, true, 0, 6).indexOf("protected") >= 0);
            w.desktopFileName = ""; w.resourceClass = ""; w.caption = null;
            compare(Policy.title(w, true), "Application identifier unavailable");
        }
        function test_native_unavailable_is_not_stale_observer_state() {
            add(); tryCompare(grid, "count", 1);
            const nativeSource = effect.windowSource;
            effect.windowSource = null; tryCompare(grid, "count", 0);
            compare(scene.available, false);
            effect.windowSource = nativeSource; tryCompare(grid, "count", 1);
            compare(scene.available, true);
        }
        function test_motion_reversal_cancellation_and_geometry() {
            effect.reducedMotion = false;
            effect.close(); wait(50); effect.open(); wait(350);
            compare(effect.visible, true); compare(effect.reveal, 1);
            for (let i = 0; i < 20; ++i) { effect.toggle(); wait(5); }
            wait(350); compare(effect.visible, true);
            effect.reducedMotion = true;
            effect.close(); tryCompare(effect, "visible", false);
            effect.open(); tryCompare(effect, "reveal", 1);
            view.width = 1280; view.height = 720;
            compare(scene.width, 1280); compare(scene.height, 720);
            Workspace.activities = []; effect.zoomOut(); effect.zoomOut(); compare(effect.level, 2);
        }
        function test_multiple_outputs_and_live_metadata_count() {
            add(); add({internalId: "window-2", onAllDesktops: true}); tryCompare(grid, "count", 2);
            const second = Qt.createQmlObject('import QtQuick; QtObject {property string name: "output-b"; property var geometry: Qt.rect(1920,0,1280,720)}', testCase);
            Workspace.screens = [Workspace.firstOutput, second];
            const component = Qt.createComponent("OverviewScene.qml");
            const other = component.createObject(testCase.parent, {controller: effect, system: Workspace, windowSource: effect.windowSource, output: second, width: 1280, height: 720});
            verify(other !== null);
            extraViews.push(other);
            tryCompare(findChild(other, "native-window-grid"), "count", 2);
            created.push(second);
        }
        function test_mouse_selection_hover_and_outside_dismissal() {
            add(); add({internalId: "mouse-target"}); tryCompare(grid, "count", 2);
            const target = card(1);
            verify(target !== null);
            mouseMove(target, target.width / 2, target.height / 2);
            mouseClick(target, target.width / 2, target.height / 2);
            compare(effect.selectedWindowId, "mouse-target");
            compare(Workspace.activeWindow, null);
            mouseClick(view, 5, 350);
            tryCompare(effect, "visible", false);
        }
        function test_rejected_start_resets_request_without_automatic_retry() {
            effect.close(); tryCompare(effect, "visible", false);
            if (view) { view.destroy(); view = null; wait(20); }
            compare(effect.constructedOutputs, 0);
            effect.open();
            compare(effect.visible, true); compare(effect.requestedOpen, true);
            tryCompare(effect, "visible", false, 1800);
            compare(effect.requestedOpen, false); compare(effect.reveal, 0);
            wait(100); compare(effect.visible, false);
            effect.open(); compare(effect.visible, true);
            view = effect.delegate.createObject(testCase.parent, {width: 1920, height: 1080});
            verify(view !== null); compare(effect.constructedOutputs, 1);
            wait(1100); compare(effect.visible, true); compare(effect.requestedOpen, true);
        }
        function test_preview_status_does_not_cover_frame_or_claim_no_frame() {
            const w = add(); tryCompare(grid, "count", 1); grid.forceLayout(); wait(30);
            const c = card(0);
            const loader = findChild(c, "native-preview-loader");
            const status = findChild(c, "native-preview-status");
            const fallback = findChild(c, "native-preview-fallback");
            verify(loader.active); verify(status.visible); verify(!fallback.visible);
            verify(loader.y + loader.height <= status.y + 0.5);
            verify(status.text.indexOf("No compositor frame") < 0);
            w.excludeFromCapture = true;
            tryCompare(loader, "active", false);
            verify(fallback.visible); verify(!status.visible);
        }
        function test_visual_fixture_capture() {
            if (!Workspace.previewPath) return;
            add({desktopFileName: "org.example.Editor"});
            add({internalId: "fixture-terminal", desktopFileName: "org.example.Terminal", minimized: true});
            add({internalId: "fixture-protected", desktopFileName: "org.example.Protected", excludeFromCapture: true});
            tryCompare(grid, "count", 3); grid.forceLayout();
            const watermark = Qt.createQmlObject('import QtQuick; Text {text: "OFFSCREEN FIXTURE — NOT LIVE KWIN"; color: "#F8FAFC"; font.pixelSize: 16; x: 32; y: 120; z: 100}', view);
            let captured = false;
            view.grabToImage(result => { captured = result.saveToFile(Workspace.previewPath); });
            tryVerify(() => captured, 5000);
            watermark.destroy();
        }
        function test_metadata_is_literal_text_without_rich_text_or_control_injection() {
            const w = add({caption: '<img src="https://example.invalid/never-fetch">\n\u202eText'});
            effect.showTitles = true; tryCompare(grid, "count", 1);
            const title = findChild(card(0), "window-title");
            compare(title.textFormat, Text.PlainText);
            verify(title.text.indexOf('<img src="https://example.invalid/never-fetch">') === 0);
            verify(title.text.indexOf("\n") < 0);
            verify(title.text.indexOf("\u202e") < 0);
        }
        function test_per_output_desktop_policy_and_missing_output_fallback() {
            const w = add({desktops: [Workspace.secondDesktop]});
            const perOutput = {currentDesktop: Workspace.firstDesktop, currentActivity: "activity-a",
                currentDesktopForScreen: function(output) { return Workspace.secondDesktop; }};
            compare(Policy.previewReason(w, perOutput, true, 0, 6), "");
            w.output = null;
            verify(Policy.previewReason(w, perOutput, true, 0, 6).indexOf("Output unavailable") >= 0);
        }
    }
}
