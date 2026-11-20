import QtQuick
import QtQuick.Controls.Basic as Basic
import "theme"
import "Presentation.js" as Display

Basic.AbstractButton {
    id: preview
    property var workspace: null
    property int ordinal: 0
    property int desktopCount: 0
    property string subtitle: Display.countLabel(desktopCount, "desktop", "desktops")
    property bool highContrast: false
    property bool lowEffects: false
    property bool highlighted: false
    readonly property color tone: ordinal % 2 ? NeonTheme.lavender : NeonTheme.cyan

    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.name: Display.safeText(workspace && workspace.name)
    Accessible.description: subtitle

    background: Rectangle {
        radius: 13
        color: preview.highContrast ? NeonTheme.contrastSurface : NeonTheme.panel
        border.width: preview.activeFocus || preview.highlighted ? 2 : 1
        border.color: preview.activeFocus ? NeonTheme.focus : preview.highlighted ? NeonTheme.emerald : NeonTheme.strongBorder
    }
    contentItem: Item {
        Rectangle {
            visible: !preview.lowEffects && !preview.highContrast
            x: 3
            y: 3
            width: parent.width - 6
            height: parent.height - 6
            radius: 11
            gradient: Gradient {
                GradientStop { position: 0; color: NeonTheme.tint(preview.tone, preview.hovered ? 0.30 : 0.18) }
                GradientStop { position: 0.86; color: NeonTheme.transparent }
            }
        }
        Image {
            visible: preview.height >= 92 && !preview.lowEffects && !preview.highContrast
            source: visible ? "art/tile-" + (preview.ordinal % 4) + ".png" : ""
            anchors.top: parent.top
            anchors.right: parent.right
            width: Math.min(parent.width * 0.94, 385)
            height: parent.height * 0.73
            fillMode: Image.PreserveAspectFit
            horizontalAlignment: Image.AlignRight
            opacity: 0.70
        }
        Column {
            x: 13
            width: parent.width - 26
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 9
            spacing: 3
            NeonText {
                width: parent.width
                text: Display.safeText(preview.workspace && preview.workspace.name)
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            NeonText {
                width: parent.width
                text: preview.subtitle
                font.pixelSize: NeonTheme.detailSize
                color: NeonTheme.muted
                elide: Text.ElideRight
            }
        }
    }
}
