import QtQuick

Rectangle {
    id: badge
    property string status: "Unknown" // "Live" | "Stale" | "Reconnecting" | "Unavailable" | "Unknown" | "Demo"
    property bool compact: false
    property string detail: ""

    implicitWidth: compact ? dot.width + 12 : contentRow.implicitWidth + 16
    implicitHeight: compact ? 18 : 22
    radius: height / 2

    // Deterministic palette mapping
    readonly property color statusColor: {
        switch (status) {
        case "Live":
            return "#22C55E";          // Emerald green
        case "Stale":
            return "#F59E0B";         // Amber warning
        case "Reconnecting":
            return "#38BDF8";  // Cyan / Sky blue
        case "Unavailable":
            return "#64748B";   // Slate gray
        case "Demo":
            return "#C4B5FD";   // Sample data, never a live observation
        case "Unknown":
        default:
            return "#94A3B8";              // Muted slate
        }
    }

    readonly property color statusBg: {
        switch (status) {
        case "Live":
            return "#052e16";
        case "Stale":
            return "#451a03";
        case "Reconnecting":
            return "#082f49";
        case "Unavailable":
            return "#0f172a";
        case "Demo":
            return "#2e1064";
        case "Unknown":
        default:
            return "#1e293b";
        }
    }

    readonly property color statusBorder: {
        switch (status) {
        case "Live":
            return "#16a34a";
        case "Stale":
            return "#d97706";
        case "Reconnecting":
            return "#0284c7";
        case "Unavailable":
            return "#334155";
        case "Demo":
            return "#6D28D9";
        case "Unknown":
        default:
            return "#475569";
        }
    }

    color: statusBg
    border.color: statusBorder
    border.width: 1

    Row {
        id: contentRow
        anchors.centerIn: parent
        spacing: 6

        Rectangle {
            id: dot
            width: 7
            height: 7
            radius: 3.5
            color: badge.statusColor
            anchors.verticalCenter: parent.verticalCenter

            // Pulse animation for Live and Reconnecting states
            SequentialAnimation on opacity {
                running: badge.status === "Live" || badge.status === "Reconnecting"
                loops: Animation.Infinite
                NumberAnimation {
                    to: 0.4
                    duration: 900
                    easing.type: Easing.InOutQuad
                }
                NumberAnimation {
                    to: 1.0
                    duration: 900
                    easing.type: Easing.InOutQuad
                }
            }
        }

        Text {
            id: labelText
            visible: !badge.compact
            textFormat: Text.PlainText
            text: badge.status
            color: badge.statusColor
            font.pixelSize: 10
            font.weight: Font.DemiBold
            anchors.verticalCenter: parent.verticalCenter
        }
    }
}
