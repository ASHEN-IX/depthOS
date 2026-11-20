pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

CyberCard {
    id: card
    property var workspace: null
    property var desktops: []
    property int observedWindowCount: -1
    property string statusBadge: "Unavailable"
    property string freshness: ""
    property int ordinal: 0
    readonly property color categoryTone: ordinal % 2 ? NeonTheme.cyan : NeonTheme.lavender
    property bool demoSource: false
    readonly property string name: Display.safeText(workspace ? workspace.name : "Workspace")
    readonly property string directory: Display.safeText(workspace ? workspace.rootDirectory : "")
    readonly property string branch: Display.safeText(workspace ? workspace.gitBranch : "")

    implicitWidth: 380
    implicitHeight: 224
    radius: 14
    available: !(workspace && workspace.unavailable)
    accessibleLabel: name
    accessibleDescription: Display.countLabel(desktops.length, "logical desktop", "logical desktops")

    Rectangle {
        x: 4
        y: 4
        width: parent.width - 8
        height: 96
        radius: card.radius - 3
        visible: !card.lowEffects && !card.highContrast
        gradient: Gradient {
            GradientStop { position: 0; color: NeonTheme.tint(card.categoryTone, 0.18) }
            GradientStop { position: 1; color: NeonTheme.transparent }
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 17
        spacing: 8

        Row {
            width: parent.width
            spacing: 10

            Rectangle {
                width: 36
                height: width
                radius: NeonTheme.buttonRadius
                color: NeonTheme.tint(card.categoryTone, 0.18)
                border.color: NeonTheme.tint(card.categoryTone, 0.58)
                NeonText {
                    anchors.centerIn: parent
                    color: card.categoryTone
                    font.pixelSize: 20
                    text: ["▤", "◫", "◇", "▥"][card.ordinal % 4]
                }
            }

            NeonText {
                width: parent.width - 56 - status.width
                anchors.verticalCenter: parent.verticalCenter
                text: card.name
                font.pixelSize: 18
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            StatusBadge {
                id: status
                anchors.verticalCenter: parent.verticalCenter
                status: card.statusBadge
            }
        }

        NeonText {
            width: parent.width
            visible: text.length > 0
            text: Display.safeText(card.workspace ? card.workspace.description : "")
            color: NeonTheme.precision
            font.pixelSize: NeonTheme.detailSize
            elide: Text.ElideRight
        }

        NeonText {
            width: parent.width
            visible: text.length > 0
            text: card.directory
            color: NeonTheme.muted
            font.family: "monospace"
            font.pixelSize: NeonTheme.detailSize
            elide: Text.ElideMiddle
        }

        NeonText {
            width: parent.width
            visible: text.length > 0
            text: card.branch ? "git · " + card.branch : ""
            color: NeonTheme.lavender
            font.family: "monospace"
            font.pixelSize: NeonTheme.detailSize
            elide: Text.ElideRight
        }

        Rectangle {
            width: parent.width
            height: 1
            color: NeonTheme.border
        }

        Row {
            width: parent.width
            spacing: 8

            NeonLabel {
                text: "LINKED DESKTOPS · " + card.desktops.length
            }

            NeonText {
                visible: card.observedWindowCount >= 0
                text: card.demoSource ? Display.countLabel(card.observedWindowCount, "sample window", "sample windows")
                    : Display.countLabel(card.observedWindowCount, "observed window", "observed windows")
                color: NeonTheme.precision
                font.pixelSize: NeonTheme.labelSize
            }
        }

        Row {
            id: desktopsRow
            width: parent.width
            spacing: 6

            Repeater {
                model: card.desktops.slice(0, 3)

                NeonBadge {
                    required property var modelData
                    width: Math.min(implicitWidth, (desktopsRow.width - 12) / Math.max(1, Math.min(3, card.desktops.length)))
                    label: Display.safeText(modelData.name)
                    tone: NeonTheme.precision
                }
            }
        }

        NeonText {
            visible: text.length > 0
            width: parent.width
            text: card.freshness
            color: NeonTheme.muted
            font.pixelSize: NeonTheme.labelSize
            elide: Text.ElideRight
        }
    }
}
