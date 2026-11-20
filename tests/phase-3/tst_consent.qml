import QtQuick
import QtTest
import "../../experiments/phase-3-consent" as Phase3

Item {
    id: fixture
    width: 480
    height: 240

    Item { id: otherFocus }
    Component { id: promptComponent; Phase3.ConsentPrompt {} }
    SignalSpy { id: canceled; signalName: "cancelRequested" }

    TestCase {
        name: "DenyOnlyConsentPrompt"
        when: windowShown
        property var prompt

        function init() {
            prompt = createTemporaryObject(promptComponent, fixture, {
                sourceDesktop: "<b>Code</b>",
                destinationDesktop: "Review",
                activity: "Work"
            })
            verify(prompt)
            canceled.target = prompt
            canceled.clear()
            prompt.forceActiveFocus()
            verify(prompt.activeFocus)
        }

        function test_placementAndInput() {
            const source = findChild(prompt, "source")
            compare(source.text, "From <b>Code</b>")
            compare(source.textFormat, Text.PlainText)
            compare(findChild(prompt, "destination").text, "To Review")
            compare(findChild(prompt, "activity").text, "Activity Work stays the same")
            compare(typeof prompt.approve, "undefined")
            compare(typeof prompt.confirm, "undefined")
            keyClick(Qt.Key_Return)
            keyClick(Qt.Key_Space)
            verify(!prompt.dismissed)
            compare(canceled.count, 0)
        }

        function test_cancelAndRepeat() {
            const button = findChild(prompt, "cancel")
            mouseClick(button, button.width / 2, button.height / 2)
            verify(prompt.dismissed)
            verify(!prompt.visible)
            prompt.dismiss()
            compare(canceled.count, 1)
        }

        function test_escape() {
            keyClick(Qt.Key_Escape)
            verify(prompt.dismissed)
            compare(canceled.count, 1)
        }

        function test_focusLoss() {
            otherFocus.forceActiveFocus()
            verify(prompt.dismissed)
            compare(canceled.count, 1)
            prompt.forceActiveFocus()
            verify(!prompt.visible)
        }

        function test_placementChangeInvalidates() {
            prompt.destinationDesktop = "Unrelated"
            compare(findChild(prompt, "destination").text, "To Review")
            verify(prompt.dismissed)
            compare(canceled.count, 1)
        }

        function test_timeout() {
            canceled.wait(6500)
            verify(prompt.dismissed)
            compare(canceled.count, 1)
        }
    }
}
