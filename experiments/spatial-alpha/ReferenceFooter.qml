pragma ComponentBehavior: Bound
import QtQuick
import "theme"

Item {
    id: footer
    required property var scene

    NeonLabel {
        objectName: "demoWatermark"
        text: "DEMO DATA"
        visible: !!footer.scene.backend?.isDemo
        anchors.right: parent.right
        anchors.rightMargin: 38
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 15
        color: NeonTheme.precision
    }

    Row {
        anchors.centerIn: parent
        anchors.verticalCenterOffset: 1
        spacing: footer.width >= 1400 ? 26 : 10

        Repeater {
            model: [
                { key: "↵", name: "Enter", tone: NeonTheme.cyan },
                { key: "Esc", name: "Back", tone: NeonTheme.danger },
                { key: "/", name: "Search", tone: NeonTheme.mint },
                { key: "← →", name: "Switch World", tone: NeonTheme.precision },
                { key: "≡", name: "Menu", tone: NeonTheme.precision }
            ]
            delegate: NeonButton {
                id: hint
                required property var modelData
                required property int index
                text: modelData.name
                padding: 8
                enabled: index !== 1 || footer.scene.currentLevel > 1
                contentItem: Row {
                    readonly property bool truncated: false
                    spacing: 10
                    Rectangle {
                        width: keyLabel.implicitWidth + 14
                        height: 28; radius: width > 39 ? 5 : 15
                        color: NeonTheme.background
                        border.color: NeonTheme.tint(hint.modelData.tone, 0.75)
                        anchors.verticalCenter: parent.verticalCenter
                        NeonText {
                            id: keyLabel
                            anchors.centerIn: parent
                            text: hint.modelData.key
                            color: hint.modelData.tone
                            font.pixelSize: text.length > 2 ? 12 : 17
                        }
                    }
                    NeonText {
                        anchors.verticalCenter: parent.verticalCenter
                        text: hint.index === 3 && footer.scene.currentLevel !== 1 ? "Browse" : hint.modelData.name
                        font.pixelSize: 15
                    }
                }
                background: Rectangle {
                    radius: 10
                    color: hint.hovered ? NeonTheme.glass : NeonTheme.transparent
                    border.color: hint.activeFocus ? NeonTheme.focus : NeonTheme.transparent
                }
                onClicked: {
                    if (index === 0)
                        footer.scene.enterDeeper();
                    else if (index === 1)
                        footer.scene.drillUp(false);
                    else if (index === 2)
                        footer.scene.openSearch();
                    else if (index === 3)
                        footer.scene.moveHorizontal(1);
                    else
                        footer.scene.openControlCenter();
                }
            }
        }
    }
}
