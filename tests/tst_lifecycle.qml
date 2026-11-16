import QtQuick
import QtTest

Item {
    width: 1920
    height: 1080

    TestCase {
        id: testCase
        name: "DepthOS_QtQuick_MockKWin"
        anchors.fill: parent
        when: windowShown
        property var effect
        property var view
        property var shortcut

        function init() {
            failOnWarning(/.*/);
            const component = Qt.createComponent("Effect.qml");
            compare(component.status, Component.Ready, component.errorString());
            effect = component.createObject(testCase);
            verify(effect !== null);
            effect.diagnosticsEnabled = false;
            shortcut = findChild(effect, "depthos-spatial-poc-shortcut");
            verify(shortcut !== null);
            // TestCase itself is invisible; parent the UI to the visible root Item.
            view = effect.delegate.createObject(testCase.parent, {
                width: 1920,
                height: 1080
            });
            verify(view !== null);
            view.forceActiveFocus();
        }

        function cleanup() {
            view.destroy();
            effect.destroy();
        }

        function test_entrance_and_exit() {
            compare(effect.visible, false);
            effect.toggle();
            compare(effect.visible, true);
            compare(effect.cardScale, 0.75);
            wait(350);
            compare(effect.cardScale, 1);
            compare(effect.cardOpacity, 1);
            compare(effect.backdropOpacity, 1);
            effect.toggle();
            compare(effect.visible, true); // Input must remain owned until exit finishes.
            wait(100);
            compare(effect.visible, true);
            tryCompare(effect, "visible", false, 500);
            compare(effect.cardScale, 0.85);
            compare(effect.cardOpacity, 0);
        }

        function test_shortcut_handler_wiring() {
            compare(shortcut.name, "depthos-spatial-poc-toggle");
            compare(shortcut.text, "Toggle DepthOS Spatial POC");
            compare(shortcut.sequence, "Meta+Ctrl+Shift+F11");
            shortcut.activated(); // Invoke production onActivated, not effect.toggle().
            compare(effect.requestedOpen, true);
            compare(effect.visible, true);
            wait(350);
            compare(effect.cardScale, 1);
            shortcut.activated();
            compare(effect.requestedOpen, false);
            compare(effect.visible, true);
            tryCompare(effect, "visible", false, 500);
        }

        function test_external_hide_cancels_transitions() {
            shortcut.activated();
            wait(80);
            effect.visible = false;
            compare(effect.requestedOpen, false);
            const stoppedScale = effect.cardScale;
            wait(350);
            compare(effect.cardScale, stoppedScale);
            shortcut.activated();
            wait(350);
            compare(effect.cardScale, 1);
            shortcut.activated();
            wait(60);
            effect.visible = false;
            compare(effect.requestedOpen, false);
            const stoppedOpacity = effect.cardOpacity;
            wait(300);
            compare(effect.cardOpacity, stoppedOpacity);
            shortcut.activated();
            wait(350);
            compare(effect.visible, true);
            compare(effect.cardOpacity, 1);
        }

        function test_interrupted_close_reopens() {
            effect.open();
            wait(80);
            effect.close();
            wait(60);
            effect.toggle();
            wait(350); // A stale exit completion must not hide the reopened view.
            compare(effect.visible, true);
            compare(effect.requestedOpen, true);
            compare(effect.cardScale, 1);
            effect.close();
            tryCompare(effect, "visible", false, 500);
        }

        function test_rapid_toggle() {
            for (let i = 0; i < 30; ++i) {
                effect.toggle();
                wait(5);
            }
            tryCompare(effect, "visible", false, 500);
            effect.open();
            wait(350);
            compare(effect.visible, true);
            compare(effect.cardOpacity, 1);
        }

        function test_escape_and_selection() {
            effect.open();
            wait(350);
            mouseClick(view, view.width / 2, view.height / 2);
            compare(effect.selected, true);
            compare(effect.visible, true);
            keyClick(Qt.Key_Space);
            compare(effect.selected, false);
            keyClick(Qt.Key_Escape);
            tryCompare(effect, "visible", false, 500);
        }

        function test_outside_click_and_reset() {
            effect.open();
            wait(350);
            mouseClick(view, view.width / 2, view.height / 2);
            compare(effect.selected, true);
            mouseClick(view, 10, 10);
            tryCompare(effect, "visible", false, 500);
            effect.open();
            compare(effect.selected, false);
            wait(350);
            compare(effect.cardOpacity, 1);
        }

        function test_repeated_activation_and_resize() {
            for (let i = 0; i < 5; ++i) {
                effect.open();
                wait(350);
                compare(effect.cardScale, 1);
                view.width = i % 2 === 0 ? 1280 : 1920;
                view.height = i % 2 === 0 ? 720 : 1080;
                verify(view.unit > 0 && view.unit <= 1);
                effect.close();
                tryCompare(effect, "visible", false, 500);
            }
        }
    }
}
