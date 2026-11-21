pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

CyberCard {
    id: card
    property var desktop: null
    property var windows: []
    property int ordinal: 0
    property bool demoSource: false
    readonly property color categoryTone: ordinal % 2 ? NeonTheme.cyan : NeonTheme.lavender
    readonly property string name: Display.safeText(desktop ? desktop.name : "Desktop")
    readonly property int windowCount: windows.length

    implicitWidth: 340
    implicitHeight: 200
    radius: 12
    available: !(desktop && desktop.unavailable)
    accessibleLabel: name
    accessibleDescription: Display.countLabel(windowCount, "window", "windows")

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 8

        Row {
            width: parent.width
            spacing: 8

            NeonText {
                width: Math.max(0, parent.width - badge.width - 8)
                anchors.verticalCenter: parent.verticalCenter
                text: card.name
                font.pixelSize: 16
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            StatusBadge {
                id: badge
                status: card.demoSource ? "Demo" : (card.available ? "Active" : "Unavailable")
            }
        }

        NeonText {
            width: parent.width
            text: Display.countLabel(card.windowCount, "window", "windows")
            color: NeonTheme.muted
            font.pixelSize: 12
        }
    }
}
