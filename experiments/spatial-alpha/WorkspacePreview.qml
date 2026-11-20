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
    property bool referenceStyle: false
    property bool highlighted: false
    readonly property bool optionsOpen: choices.opened
    signal openDesktopsRequested
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
            id: previewArt
            visible: preview.height >= 92 && !preview.lowEffects && !preview.highContrast
            source: !visible ? "" : preview.referenceStyle
                ? "art/reference-preview-" + (preview.ordinal % 4) + ".jpg"
                : "art/tile-" + (preview.ordinal % 4) + ".png"
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.topMargin: preview.referenceStyle ? 4 : 0
            anchors.rightMargin: preview.referenceStyle ? 4 : 0
            width: preview.referenceStyle ? parent.width - 8 : Math.min(parent.width * 0.94, 385)
            height: preview.referenceStyle ? parent.height - 8 : parent.height * 0.73
            fillMode: preview.referenceStyle ? Image.PreserveAspectCrop : Image.PreserveAspectFit
            horizontalAlignment: Image.AlignRight
            opacity: preview.referenceStyle ? 0.89 : 0.7
        }
        Rectangle {
            visible: preview.referenceStyle && previewArt.visible
            anchors.fill: previewArt
            radius: 11
            gradient: Gradient {
                GradientStop { position: 0; color: NeonTheme.transparent }
                GradientStop { position: 0.43; color: NeonTheme.transparent }
                GradientStop { position: 0.79; color: NeonTheme.tint(NeonTheme.panel, 0.84) }
                GradientStop { position: 1; color: NeonTheme.panel }
            }
        }
        NeonRim {
            visible: preview.referenceStyle && preview.highlighted
            anchors.fill: parent
            radius: 13
            bright: true
            lowEffects: preview.lowEffects || preview.highContrast
        }
        NeonButton {
            id: more
            objectName: preview.objectName + "-options"
            text: "•••"
            Accessible.name: "Options for " + Display.safeText(preview.workspace && preview.workspace.name)
            visible: preview.referenceStyle
            compact: true
            width: 46; height: 25
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 5
            background: Rectangle {
                radius: 6
                color: more.hovered || more.activeFocus ? NeonTheme.panel : NeonTheme.transparent
                border.color: more.activeFocus ? NeonTheme.focus : NeonTheme.transparent
            }
            onClicked: choices.open()

            Basic.Menu {
                id: choices
                objectName: preview.objectName + "-menu"
                width: 196
                x: more.width - width
                y: more.height + 4
                popupType: Basic.Popup.Item
                background: SpatialGlass {}
                Basic.MenuItem {
                    id: workspaceAction
                    text: "Open workspace"
                    onTriggered: preview.clicked()
                    contentItem: NeonText { text: workspaceAction.text; verticalAlignment: Text.AlignVCenter; leftPadding: 8 }
                }
                Basic.MenuItem {
                    id: desktopsAction
                    objectName: preview.objectName + "-open-desktops"
                    text: "Open desktops"
                    enabled: preview.desktopCount > 0
                    onTriggered: preview.openDesktopsRequested()
                    contentItem: NeonText { text: desktopsAction.text; verticalAlignment: Text.AlignVCenter; leftPadding: 8 }
                }
            }
        }
        SpatialIcon {
            visible: preview.referenceStyle
            name: ["desktop", "book", "research", "bars"][preview.ordinal % 4]
            width: 30; height: 30
            x: 22
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 17
        }
        Column {
            x: preview.referenceStyle ? 80 : 13
            width: parent.width - x - 13
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 9
            spacing: 3
            NeonText {
                width: parent.width
                text: Display.safeText(preview.workspace && preview.workspace.name)
                font.weight: Font.DemiBold
                font.pixelSize: preview.referenceStyle ? 17 : 14
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
