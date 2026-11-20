import QtQuick
import "theme"

Rectangle {
    id: badge
    property alias label: caption.text
    property color tone: NeonTheme.cyan
    property bool dot: false
    implicitWidth: caption.implicitWidth + (dot ? 28 : 16)
    implicitHeight: 23
    radius: NeonTheme.badgeRadius
    color: NeonTheme.tint(tone, 0.1)
    border.color: NeonTheme.tint(tone, 0.42)
    border.width: 1

    Rectangle {
        visible: badge.dot
        x: 8
        anchors.verticalCenter: parent.verticalCenter
        width: 5
        height: width
        radius: width / 2
        color: badge.tone
    }

    NeonText {
        id: caption
        anchors.fill: parent
        anchors.leftMargin: badge.dot ? 20 : 8
        anchors.rightMargin: 8
        verticalAlignment: Text.AlignVCenter
        font.pixelSize: NeonTheme.labelSize
        font.weight: Font.DemiBold
        color: badge.tone
        elide: Text.ElideRight
    }
}
