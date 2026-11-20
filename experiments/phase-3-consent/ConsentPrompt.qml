import QtQuick
import QtQuick.Controls

// Create one item per proposal. Only the fixture owner controls its lifetime.
FocusScope {
    id: prompt
    required property string sourceDesktop
    required property string destinationDesktop
    required property string activity

    signal cancelRequested()
    readonly property bool dismissed: lifecycle.dismissed
    implicitWidth: content.implicitWidth
    implicitHeight: content.implicitHeight
    visible: lifecycle.initialized && !dismissed

    function dismiss() {
        if (!lifecycle.initialized || lifecycle.dismissed)
            return
        lifecycle.dismissed = true
        cancelRequested()
    }

    QtObject {
        id: lifecycle
        property bool initialized: false
        property bool dismissed: false
        property bool heldFocus: prompt.activeFocus
        onHeldFocusChanged: if (!heldFocus) prompt.dismiss()
    }

    Component.onCompleted: {
        sourceLabel.text = qsTr("From %1").arg(sourceDesktop)
        destinationLabel.text = qsTr("To %1").arg(destinationDesktop)
        activityLabel.text = qsTr("Activity %1 stays the same").arg(activity)
        lifecycle.initialized = true
        if (!sourceDesktop.trim() || !destinationDesktop.trim()
                || !activity.trim() || sourceDesktop === destinationDesktop)
            dismiss()
    }

    onSourceDesktopChanged: dismiss()
    onDestinationDesktopChanged: dismiss()
    onActivityChanged: dismiss()
    Keys.onEscapePressed: event => {
        dismiss()
        event.accepted = true
    }

    // Presentation timeout only. Authority expiration uses its own monotonic clock.
    Timer {
        interval: 5000
        running: prompt.visible
        onTriggered: prompt.dismiss()
    }

    Column {
        id: content
        spacing: 8

        Label { id: sourceLabel; objectName: "source"; textFormat: Text.PlainText }
        Label { id: destinationLabel; objectName: "destination"; textFormat: Text.PlainText }
        Label { id: activityLabel; objectName: "activity"; textFormat: Text.PlainText }
        Label { text: qsTr("Approval unavailable") }
        Button {
            objectName: "cancel"
            text: qsTr("Cancel")
            onClicked: prompt.dismiss()
        }
    }
}
