pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

CyberCard {
    id: card
    property var world: null
    property var workspaces: []
    property int desktopCount: 0
    property int identityIndex: 0
    property bool cameraFocused: false
    property color identityTone: identityIndex % 2 ? NeonTheme.lavender : NeonTheme.cyan
    readonly property string worldName: Display.safeText(world ? world.name : "World")

    implicitWidth: 420
    implicitHeight: 316
    available: !(world && world.unavailable)
    accessibleLabel: worldName
    accessibleDescription: Display.countLabel(workspaces.length, "workspace", "workspaces") + ", "
        + Display.countLabel(desktopCount, "logical desktop", "logical desktops")

    Item {
        id: art
        x: 2
        y: 2
        width: card.width - 4
        height: card.height * 0.58

        Rectangle {
            anchors.fill: parent
            radius: card.radius - 1
            visible: !card.highContrast
            gradient: Gradient {
                GradientStop {
                    position: 0
                    color: NeonTheme.tint(card.cameraFocused ? NeonTheme.emerald : card.identityTone,
                        card.cameraFocused ? 0.24 : 0.17)
                }
                GradientStop { position: 1; color: NeonTheme.transparent }
            }
        }

        WorldInsignia {
            anchors.centerIn: parent
            width: parent.width - 20
            height: parent.height - 2
            ink: card.cameraFocused ? NeonTheme.emerald : card.identityTone
            variant: card.identityIndex
            simple: card.lowEffects || card.highContrast
        }

        NeonLabel {
            x: 17
            y: 15
            text: "WORLD"
            color: card.cameraFocused ? NeonTheme.emerald : NeonTheme.precision
        }

        NeonBadge {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 14
            visible: card.isSelected || !card.available
            label: card.available ? "Selected" : "Unavailable"
            tone: card.available ? NeonTheme.emerald : NeonTheme.danger
            dot: true
        }
    }

    Rectangle {
        width: card.width - 36
        x: 18
        y: art.height + 2
        height: 1
        color: card.isSelected ? NeonTheme.tint(NeonTheme.emerald, 0.42) : NeonTheme.border
    }

    Column {
        x: 18
        y: art.height + 14
        width: card.width - 36
        spacing: 6

        NeonText {
            width: parent.width
            text: card.worldName
            font.pixelSize: card.width > 340 ? 25 : 20
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }

        NeonText {
            width: parent.width
            visible: text.length > 0
            text: Display.safeText(card.world && card.world.description)
            color: NeonTheme.precision
            font.pixelSize: NeonTheme.detailSize
            maximumLineCount: 1
            elide: Text.ElideRight
        }

        Flow {
            width: parent.width
            spacing: 6
            topPadding: 2
            NeonBadge {
                tone: card.cameraFocused ? NeonTheme.emerald : NeonTheme.precision
                label: Display.countLabel(card.workspaces.length, "workspace", "workspaces")
            }
            NeonBadge {
                tone: NeonTheme.precision
                label: Display.countLabel(card.desktopCount, "desktop", "desktops")
            }
        }
    }
}
