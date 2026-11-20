import QtQuick
import "theme"
import "Presentation.js" as Display

Rectangle {
    id: header
    required property var scene
    property bool compact: width < 960
    property real margin: compact ? NeonTheme.compactMargin : NeonTheme.margin
    height: NeonTheme.hudHeight
    color: scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.shell

    Rectangle {
        anchors.bottom: parent.bottom
        width: parent.width
        height: 1
        color: header.scene.highContrast ? NeonTheme.muted : NeonTheme.border
    }

    Row {
        id: path
        anchors.left: parent.left
        anchors.leftMargin: header.margin
        anchors.right: actions.left
        anchors.rightMargin: 10
        anchors.verticalCenter: parent.verticalCenter
        spacing: header.compact ? 5 : 12
        clip: true

        Row {
            id: brand
            anchors.verticalCenter: parent.verticalCenter
            spacing: 8

            Rectangle {
                width: header.compact ? 26 : 32
                height: width
                radius: 8
                border.color: NeonTheme.emerald
                color: NeonTheme.tint(NeonTheme.emerald, 0.09)
                NeonText {
                    anchors.centerIn: parent
                    text: "◈"
                    color: NeonTheme.emerald
                    font.pixelSize: 21
                }
            }

            NeonText {
                anchors.verticalCenter: parent.verticalCenter
                visible: header.width > 560
                text: "DepthOS"
                font.pixelSize: 17
                font.weight: Font.Bold
            }
        }

        Rectangle {
            anchors.verticalCenter: parent.verticalCenter
            width: 1
            height: 23
            color: NeonTheme.border
        }

        NeonButton {
            id: worlds
            anchors.verticalCenter: parent.verticalCenter
            compact: true
            text: "Worlds"
            highlighted: header.scene.currentLevel === 1
            onClicked: header.scene.navigateBreadcrumb(1)
        }

        NeonText {
            anchors.verticalCenter: parent.verticalCenter
            visible: header.scene.currentLevel >= 2
            text: "›"
            color: NeonTheme.muted
        }

        NeonButton {
            id: workspaces
            anchors.verticalCenter: parent.verticalCenter
            compact: true
            visible: header.scene.currentLevel >= 2
            text: header.scene.backend && header.scene.backend.currentWorld
                ? Display.safeText(header.scene.backend.currentWorld.name) : "Workspaces"
            width: Math.min(implicitWidth, Math.max(70, (path.width - brand.width - worlds.width - 48)
                / (header.scene.currentLevel === 3 ? 2 : 1)))
            highlighted: header.scene.currentLevel === 2
            onClicked: header.scene.navigateBreadcrumb(2)
        }

        NeonText {
            anchors.verticalCenter: parent.verticalCenter
            visible: header.scene.currentLevel === 3 && header.width > 650
            text: "›"
            color: NeonTheme.muted
        }

        NeonButton {
            anchors.verticalCenter: parent.verticalCenter
            compact: true
            visible: header.scene.currentLevel === 3 && header.width > 650
            text: header.scene.backend && header.scene.backend.currentWorkspace
                ? Display.safeText(header.scene.backend.currentWorkspace.name) : "Logical desktops"
            width: Math.max(0, Math.min(implicitWidth, path.width - brand.width - worlds.width - workspaces.width - 72))
            highlighted: true
            onClicked: header.scene.navigateBreadcrumb(3)
        }
    }

    Row {
        id: actions
        anchors.right: parent.right
        anchors.rightMargin: header.margin
        anchors.verticalCenter: parent.verticalCenter
        spacing: header.compact ? 6 : 12

        Column {
            anchors.verticalCenter: parent.verticalCenter
            visible: header.width >= 780
            spacing: 2

            NeonText {
                objectName: "sourceLabel"
                text: header.scene.backend ? header.scene.backend.sourceLabel : "No Source"
                color: header.scene.backend && header.scene.backend.isDemo ? NeonTheme.lavender : NeonTheme.text
                font.pixelSize: NeonTheme.labelSize
                font.weight: Font.DemiBold
            }

            NeonText {
                objectName: "observerLabel"
                text: header.scene.backend ? header.scene.backend.observerLabel : "Native Observer Unavailable"
                color: header.scene.backend && header.scene.backend.observerLive ? NeonTheme.emerald : NeonTheme.muted
                font.pixelSize: NeonTheme.labelSize
            }
        }

        NeonButton {
            objectName: "searchButton"
            anchors.verticalCenter: parent.verticalCenter
            text: header.width < 620 ? "Search" : "Search  /"
            onClicked: header.scene.openSearch()
        }

        NeonButton {
            objectName: "controlCenterButton"
            anchors.verticalCenter: parent.verticalCenter
            text: header.width < 700 ? "Status" : "Control center"
            Accessible.name: "Spatial Control Center"
            onClicked: header.scene.openControlCenter()
        }
    }
}
