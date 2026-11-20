pragma ComponentBehavior: Bound
import QtQuick
import "theme"

SpatialGlass {
    id: rail
    required property var scene
    radius: 25
    highContrast: scene.highContrast
    rim: NeonTheme.tint(NeonTheme.cyan, 0.41)

    Column {
        id: buttons
        anchors.fill: parent
        anchors.topMargin: rail.height * 0.08
        anchors.bottomMargin: rail.height * 0.05
        anchors.leftMargin: 3
        anchors.rightMargin: 8
        spacing: 3

        Repeater {
            model: [
                { label: "Worlds", icon: "home" },
                { label: "Workspaces", icon: "layers" },
                { label: "Desktops", icon: "desktop" },
                { label: "Search", icon: "search" },
                { label: "Settings", icon: "settings" }
            ]
            delegate: ChromeAction {
                id: action
                required property var modelData
                required property int index
                objectName: index === 3 ? "railSearch" : index === 4 ? "railSettings" : "railLevel" + (index + 1)
                text: modelData.label
                iconName: modelData.icon
                lowEffects: rail.scene.lowEffects || rail.scene.highContrast
                width: buttons.width
                height: (buttons.height - 4 * buttons.spacing) / 5
                highlighted: rail.scene.currentLevel === index + 1
                enabled: index > 2 || rail.scene.canNavigateLevel(index + 1)
                contentItem: Item {
                    Column {
                        anchors.centerIn: parent
                        spacing: 8
                        SpatialIcon {
                            anchors.horizontalCenter: parent.horizontalCenter
                            name: action.iconName
                            width: Math.min(30, action.height * 0.36); height: width
                        }
                        NeonText {
                            anchors.horizontalCenter: parent.horizontalCenter
                            text: action.text
                            color: action.enabled ? NeonTheme.text : NeonTheme.muted
                            font.pixelSize: rail.width >= 111 ? 16 : 13
                        }
                    }
                }
                onClicked: {
                    if (index === 3)
                        rail.scene.openSearch();
                    else if (index === 4)
                        rail.scene.openControlCenter();
                    else
                        rail.scene.openLevel(index + 1);
                }
            }
        }
    }
}
