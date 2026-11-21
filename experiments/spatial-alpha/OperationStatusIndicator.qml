import QtQuick
import "theme"

Item {
    id: root
    property string status: "idle" // "idle" | "saving" | "success" | "conflict" | "error"
    property string message: ""
    property var onRefreshCallback: null
    property bool motionOff: false

    visible: status !== "idle"
    opacity: status !== "idle" ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: NeonTheme.ms(root.motionOff, NeonTheme.modalMs) }
    }

    implicitWidth: Math.min(620, contentRow.implicitWidth + 32)
    implicitHeight: 46

    Timer {
        id: autoDismiss
        interval: 3500
        repeat: false
        onTriggered: root.status = "idle"
    }

    function showSaving(msg) {
        autoDismiss.stop();
        root.message = msg || "Saving changes…";
        root.status = "saving";
    }

    function showSuccess(msg) {
        root.message = msg || "Changes saved successfully";
        root.status = "success";
        autoDismiss.restart();
    }

    function showConflict(msg, refreshCb) {
        autoDismiss.stop();
        root.message = msg || "Conflict: Record was modified concurrently.";
        root.onRefreshCallback = refreshCb;
        root.status = "conflict";
    }

    function showError(msg) {
        autoDismiss.stop();
        root.message = msg || "Operation failed.";
        root.status = "error";
    }

    function dismiss() {
        autoDismiss.stop();
        root.status = "idle";
    }

    Rectangle {
        anchors.fill: parent
        radius: 23
        color: {
            switch (root.status) {
            case "saving":
                return "#1A263D";
            case "success":
                return "#0E3025";
            case "conflict":
                return "#38240D";
            case "error":
                return "#381219";
            default:
                return NeonTheme.panel;
            }
        }
        border.width: 1
        border.color: {
            switch (root.status) {
            case "saving":
                return NeonTheme.cyan;
            case "success":
                return NeonTheme.emerald;
            case "conflict":
                return NeonTheme.warning;
            case "error":
                return NeonTheme.danger;
            default:
                return NeonTheme.border;
            }
        }

        Row {
            id: contentRow
            anchors.centerIn: parent
            spacing: 12

            Rectangle {
                width: 8
                height: 8
                radius: 4
                anchors.verticalCenter: parent.verticalCenter
                color: {
                    switch (root.status) {
                    case "saving":
                        return NeonTheme.cyan;
                    case "success":
                        return NeonTheme.emerald;
                    case "conflict":
                        return NeonTheme.warning;
                    case "error":
                        return NeonTheme.danger;
                    default:
                        return NeonTheme.muted;
                    }
                }
            }

            NeonText {
                text: root.message
                font.pixelSize: 13
                font.weight: Font.DemiBold
                color: {
                    switch (root.status) {
                    case "saving":
                        return NeonTheme.cyan;
                    case "success":
                        return NeonTheme.emerald;
                    case "conflict":
                        return NeonTheme.warning;
                    case "error":
                        return NeonTheme.danger;
                    default:
                        return NeonTheme.text;
                    }
                }
                anchors.verticalCenter: parent.verticalCenter
            }

            NeonButton {
                visible: root.status === "conflict" && root.onRefreshCallback !== null
                text: "Refresh & Review"
                compact: true
                highlighted: true
                accent: NeonTheme.warning
                anchors.verticalCenter: parent.verticalCenter
                onClicked: {
                    if (root.onRefreshCallback)
                        root.onRefreshCallback();
                    root.dismiss();
                }
            }

            NeonButton {
                text: "Dismiss"
                compact: true
                anchors.verticalCenter: parent.verticalCenter
                onClicked: root.dismiss()
            }
        }
    }
}
