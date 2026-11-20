import QtQuick
import "theme"

Rectangle {
    id: rail
    required property var scene
    radius: 22
    color: scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.glass
    border.color: scene.highContrast ? NeonTheme.muted : NeonTheme.strongBorder

    component RailButton: NeonButton {
        id: control
        property string glyph
        contentItem: Item {
            Column {
                anchors.centerIn: parent
                spacing: 3
                NeonText {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: control.glyph
                    color: control.highlighted ? NeonTheme.emerald : NeonTheme.precision
                    font.pixelSize: 17
                }
                NeonText {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: control.text
                    color: control.enabled ? NeonTheme.text : NeonTheme.muted
                    font.pixelSize: NeonTheme.detailSize
                }
            }
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 9

        RailButton {
            width: parent.width
            height: 58
            glyph: "◉"
            text: "Worlds"
            highlighted: rail.scene.currentLevel === 1
            onClicked: rail.scene.navigateBreadcrumb(1)
        }
        RailButton {
            width: parent.width
            height: 58
            glyph: "▤"
            text: "Workspaces"
            highlighted: rail.scene.currentLevel === 2
            enabled: rail.scene.currentLevel > 1 || !!rail.scene.backend?.currentWorld
            onClicked: {
                if (rail.scene.currentLevel === 1)
                    rail.scene.enterDeeper();
                else
                    rail.scene.navigateBreadcrumb(2);
                rail.scene.forceActiveFocus();
            }
        }
        RailButton {
            width: parent.width
            height: 58
            glyph: "▣"
            text: "Desktops"
            highlighted: rail.scene.currentLevel === 3
            enabled: rail.scene.currentLevel >= 2 && rail.scene.currentDesktops.length > 0
            onClicked: {
                if (rail.scene.currentLevel === 2)
                    rail.scene.enterDeeper();
                rail.scene.forceActiveFocus();
            }
        }
        RailButton {
            width: parent.width
            height: 58
            glyph: "⌕"
            text: "Search"
            onClicked: rail.scene.openSearch()
        }
        RailButton {
            width: parent.width
            height: 58
            glyph: "⊙"
            text: "Settings"
            onClicked: rail.scene.openControlCenter()
        }
    }
}
