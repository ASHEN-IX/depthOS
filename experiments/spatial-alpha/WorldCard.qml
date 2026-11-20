pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display
import "WorldAppearance.js" as Appearance

CyberCard {
    id: card
    property var world: null
    property var workspaces: []
    property int desktopCount: 0
    property int identityIndex: 0
    property bool cameraFocused: false
    property color identityTone: NeonTheme.cyan
    property int referenceTier: 0
    property bool referenceStyle: false
    property var customCategories: []
    readonly property string category: Appearance.category(world, identityIndex)
    readonly property string worldName: Display.safeText(world ? world.name : "World")

    implicitWidth: 420
    implicitHeight: 260
    radius: 20
    available: !(world && world.unavailable)
    strongGlow: referenceStyle
    openTarget: card.isSelected && indicator.visible ? indicator : null
    accessibleLabel: worldName
    accessibleDescription: Display.countLabel(workspaces.length, "workspace", "workspaces") + ", "
        + Display.countLabel(desktopCount, "logical desktop", "logical desktops")

    Item {
        id: art
        anchors.fill: parent
        anchors.margins: card.referenceStyle ? 3 : 6
        Image {
            id: custom
            anchors.fill: parent
            fillMode: Image.PreserveAspectCrop
            source: card.customCategories.includes(card.category) && !card.highContrast && !card.lowEffects ? "image://world-art/" + card.category : ""
            sourceSize.width: 640
            sourceSize.height: 384
            visible: status === Image.Ready
        }
        Image {
            id: artwork
            anchors.fill: parent
            fillMode: Image.PreserveAspectCrop
            source: card.lowEffects || card.highContrast || custom.status === Image.Ready
                ? "" : Qt.resolvedUrl("art/" + card.category + ".jpg")
            asynchronous: true
        }
        NeonText {
            anchors.horizontalCenter: parent.horizontalCenter
            y: parent.height * 0.20
            visible: artwork.status !== Image.Ready && custom.status !== Image.Ready
            text: Appearance.symbol(card.category)
            color: card.isSelected ? NeonTheme.emerald : card.identityTone
            font.pixelSize: Math.min(66, parent.height * 0.32)
        }
        Rectangle {
            anchors.fill: parent
            visible: artwork.status === Image.Ready || custom.status === Image.Ready
            gradient: Gradient {
                GradientStop { position: 0; color: NeonTheme.tint(NeonTheme.background, 0.01) }
                GradientStop { position: 0.43; color: NeonTheme.tint(NeonTheme.background, 0.06) }
                GradientStop { position: 0.72; color: NeonTheme.tint(NeonTheme.background, 0.89) }
                GradientStop { position: 1; color: NeonTheme.background }
            }
        }
    }

    NeonRim {
        anchors.fill: parent
        visible: card.referenceStyle
        radius: card.radius
        tone: card.isSelected ? NeonTheme.emerald : card.identityTone
        bright: card.isSelected
        lowEffects: card.lowEffects || card.highContrast
    }

    Row {
        x: card.referenceStyle ? (card.referenceTier >= 2 ? 48 : 29) : 17
        anchors.bottom: parent.bottom
        anchors.bottomMargin: card.referenceStyle ? 23 : 15
        spacing: 11
        width: parent.width - (card.referenceStyle ? 54 : 34)

        SpatialIcon {
            width: card.referenceStyle ? 42 : 30
            height: width
            anchors.verticalCenter: parent.verticalCenter
            name: Appearance.icon(card.category)
            ink: NeonTheme.text
        }
        Column {
            spacing: 4
            width: parent.width - (card.referenceStyle ? 53 : 41) - (indicator.visible ? indicator.width + 11 : 0)
            NeonText {
                width: parent.width
                text: card.worldName
                font.pixelSize: card.width >= 370 ? 26 : 21
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            NeonText {
                objectName: "worldCardCounts"
                width: parent.width
                text: Display.countLabel(card.workspaces.length, "workspace", "workspaces") + "  ·  "
                    + Display.countLabel(card.desktopCount, "desktop", "desktops")
                color: NeonTheme.precision
                font.pixelSize: card.referenceStyle ? 14 : 12
                elide: Text.ElideRight
            }
        }
        Rectangle {
            id: indicator
            width: card.referenceStyle ? 42 : 32
            height: width
            anchors.verticalCenter: parent.verticalCenter
            radius: width / 2
            visible: (card.isSelected && card.width >= 350) || !card.available
            color: card.referenceStyle ? NeonTheme.deepTeal : NeonTheme.background
            border.width: card.referenceStyle ? 2 : 1
            border.color: card.available ? NeonTheme.emerald : NeonTheme.danger
            NeonText {
                anchors.centerIn: parent
                text: card.available ? "›" : "–"
                font.pixelSize: 24
                color: card.available ? NeonTheme.emerald : NeonTheme.danger
            }
        }
    }
}
