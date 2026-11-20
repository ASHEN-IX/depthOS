pragma ComponentBehavior: Bound
import QtQuick
import "theme"

Item {
    id: bar
    required property var scene
    property date now: new Date()

    Timer {
        interval: Math.max(500, 60000 - (bar.now.getSeconds() * 1000 + bar.now.getMilliseconds()))
        running: bar.visible
        onTriggered: {
            bar.now = new Date();
            restart();
        }
    }

    Row {
        x: bar.width * 0.035
        anchors.verticalCenter: parent.verticalCenter
        spacing: 19
        SpatialIcon {
            name: "logo"
            ink: NeonTheme.mint
            width: 55; height: 55
            anchors.verticalCenter: parent.verticalCenter
        }
        Column {
            spacing: 0
            NeonText {
                text: "DepthOS™"
                font.pixelSize: 30
                font.weight: Font.Bold
            }
            NeonText {
                text: "Linux Spatial Workspace Manager"
                color: NeonTheme.precision
                font.pixelSize: 14
            }
        }
    }

    Row {
        id: levels
        x: bar.width * (bar.width >= 1450 ? 0.295 : 0.318)
        anchors.verticalCenter: parent.verticalCenter
        spacing: bar.width >= 1450 ? 43 : 5

        Repeater {
            model: [
                { label: "Worlds", icon: "world" },
                { label: "Workspaces", icon: "folder" },
                { label: "Desktops", icon: "desktop" }
            ]
            delegate: ChromeAction {
                required property var modelData
                required property int index
                objectName: "topLevel" + (index + 1)
                text: modelData.label
                iconName: modelData.icon
                lowEffects: bar.scene.lowEffects || bar.scene.highContrast
                iconSize: bar.width >= 1450 ? 30 : 22
                width: Math.floor(bar.width * (bar.width >= 1450 ? 0.108 : 0.109))
                height: 49
                highlighted: bar.scene.currentLevel === index + 1
                enabled: bar.scene.canNavigateLevel(index + 1)
                onClicked: bar.scene.openLevel(index + 1)
            }
        }
    }

    Repeater {
        model: bar.width >= 1450 ? [1, 2] : []
        delegate: Rectangle {
            required property var modelData
            x: levels.x + modelData * (Math.floor(bar.width * 0.108) + levels.spacing) - levels.spacing / 2
            anchors.verticalCenter: parent.verticalCenter
            width: 1; height: 21
            color: NeonTheme.strongBorder
            opacity: 0.65
        }
    }

    Row {
        anchors.right: parent.right
        anchors.rightMargin: bar.width * 0.03
        anchors.verticalCenter: parent.verticalCenter
        spacing: bar.width >= 1450 ? 15 : 8

        ChromeAction {
            objectName: "searchButton"
            text: "Search"
            iconName: "search"
            showText: false
            width: 43; height: 46
            anchors.verticalCenter: parent.verticalCenter
            onClicked: bar.scene.openSearch()
        }
        ChromeAction {
            objectName: "controlCenterButton"
            text: "Status and settings"
            iconName: "bell"
            showText: false
            width: 43; height: 46
            anchors.verticalCenter: parent.verticalCenter
            onClicked: bar.scene.openControlCenter()
        }
        SpatialGlass {
            width: bar.width >= 1430 ? 141 : 112
            height: 53
            radius: 17
            highContrast: bar.scene.highContrast
            anchors.verticalCenter: parent.verticalCenter
            Row {
                anchors.centerIn: parent
                spacing: 7
                Rectangle {
                    width: 11; height: 11; radius: width / 2
                    anchors.verticalCenter: parent.verticalCenter
                    color: bar.scene.backend?.controllerStatus === "Connected"
                        ? NeonTheme.emerald : bar.scene.backend?.isDemo ? NeonTheme.lavender : NeonTheme.muted
                }
                Column {
                    spacing: 4
                    NeonText {
                        objectName: "sourceLabel"
                        text: bar.scene.backend ? bar.scene.backend.sourceLabel : "No source"
                        font.pixelSize: 13
                        width: bar.width >= 1430 ? 109 : 83
                        elide: Text.ElideRight
                    }
                    NeonText {
                        objectName: "observerLabel"
                        text: bar.scene.backend ? bar.scene.backend.observerLabel : "Native Observer Unavailable"
                        width: bar.width >= 1430 ? 109 : 83
                        wrapMode: Text.WordWrap
                        maximumLineCount: 2
                        elide: Text.ElideRight
                        color: bar.scene.backend?.observerLive ? NeonTheme.emerald : NeonTheme.precision
                        font.pixelSize: 11
                    }
                }
            }
        }
        Column {
            visible: bar.width >= 1490
            width: 96
            anchors.verticalCenter: parent.verticalCenter
            spacing: 3
            NeonText {
                width: parent.width
                horizontalAlignment: Text.AlignRight
                text: Qt.formatTime(bar.now, "h:mm AP")
                font.pixelSize: 19
                font.weight: Font.DemiBold
            }
            NeonText {
                width: parent.width
                horizontalAlignment: Text.AlignRight
                text: Qt.formatDate(bar.now, "ddd, MMM d")
                color: NeonTheme.precision
                font.pixelSize: 15
            }
        }
    }
}
