import QtQuick
import "../../experiments/phase-3-consent" as Phase3

Item {
    id: fixture
    width: 440
    height: 160

    property var preview: null
    readonly property bool previewVisible: prompt.item !== null && prompt.item.visible
    signal cancelled()

    function showPreview(placement) {
        if (!preview) {
            preview = Object.freeze({
                sourceDesktop: placement.sourceDesktop,
                destinationDesktop: placement.destinationDesktop,
                activity: placement.activity
            })
            prompt.active = true
        }
    }

    function dismissPreview() {
        if (prompt.item)
            prompt.item.dismiss()
    }

    function invalidatePreview() {
        prompt.active = false
    }

    Loader {
        id: prompt
        anchors.centerIn: parent
        active: false
        sourceComponent: Phase3.ConsentPrompt {
            sourceDesktop: fixture.preview.sourceDesktop
            destinationDesktop: fixture.preview.destinationDesktop
            activity: fixture.preview.activity
            onCancelRequested: fixture.cancelled()
        }
        onLoaded: item.forceActiveFocus()
    }
}
