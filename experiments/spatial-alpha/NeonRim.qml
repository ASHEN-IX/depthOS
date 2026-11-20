pragma ComponentBehavior: Bound
import QtQuick
import "theme"

Item {
    id: rim
    property color tone: NeonTheme.emerald
    property real radius: 16
    property bool bright: false
    property bool lowEffects: false

    Repeater {
        model: rim.bright && !rim.lowEffects ? [1, 2, 3] : []
        delegate: Rectangle {
            required property var modelData
            anchors.fill: parent
            anchors.margins: -modelData * 2
            radius: rim.radius + modelData * 2
            color: NeonTheme.transparent
            border.color: NeonTheme.tint(rim.tone, 0.33 / modelData)
            border.width: 3 + modelData * 2
        }
    }

    Rectangle {
        anchors.fill: parent
        color: NeonTheme.transparent
        radius: rim.radius
        border.color: NeonTheme.tint(rim.tone, rim.bright ? 1 : 0.53)
        border.width: rim.bright ? 3 : 1
    }
}
