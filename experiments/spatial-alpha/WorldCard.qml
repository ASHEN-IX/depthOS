pragma ComponentBehavior: Bound
import QtQuick

Rectangle {
    id: card

    property var world: null
    property var workspaces: []
    property bool isSelected: false
    property bool isFocused: false
    property bool motionOff: false
    readonly property bool available: !(card.world && card.world.unavailable)

    signal chosen
    signal activated

    implicitWidth: 360
    implicitHeight: 260
    radius: 16

    opacity: card.available ? 1 : 0.45
    color: card.isSelected ? "#1E293B" : "#131C2E"
    border.width: card.isSelected || card.isFocused ? 2.5 : 1
    border.color: card.isFocused ? "#38BDF8" : (card.isSelected ? "#818CF8" : "#2A3756")

    scale: card.isSelected ? 1.03 : (hoverArea.containsMouse ? 1.015 : 1.0)
    Behavior on scale {
        NumberAnimation {
            duration: card.motionOff ? 0 : 200
            easing.type: Easing.OutCubic
        }
    }
    Behavior on border.color {
        ColorAnimation {
            duration: card.motionOff ? 0 : 180
        }
    }

    Rectangle {
        anchors.fill: parent
        anchors.margins: -4
        radius: card.radius + 4
        color: "transparent"
        border.width: card.isFocused ? 2 : (card.isSelected ? 1.5 : 0)
        border.color: card.isFocused ? "#38BDF8" : "#818CF8"
        opacity: card.isFocused ? 0.6 : (card.isSelected ? 0.35 : 0)
        z: -1
        Behavior on opacity {
            NumberAnimation {
                duration: card.motionOff ? 0 : 200
            }
        }
    }

    Column {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 12

        Row {
            width: parent.width
            spacing: 12

            Rectangle {
                width: 44
                height: 44
                radius: 12
                color: (card.world && card.world.color) ? card.world.color : "#6366F1"
                opacity: 0.9

                Text {
                    anchors.centerIn: parent
                    text: (card.world && card.world.icon) ? card.world.icon : "🌐"
                    font.pixelSize: 22
                }
            }

            Column {
                width: parent.width - 56 - countBadge.width
                anchors.verticalCenter: parent.verticalCenter
                spacing: 3

                Text {
                    id: worldTitle
                    textFormat: Text.PlainText
                    text: card.world ? card.world.name : "World"
                    color: "#F8FAFC"
                    font.pixelSize: 18
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                    width: parent.width
                }

                Text {
                    textFormat: Text.PlainText
                    text: "Level A: World Overview"
                    color: "#94A3B8"
                    font.pixelSize: 11
                }
            }

            Rectangle {
                id: countBadge
                anchors.verticalCenter: parent.verticalCenter
                height: 24
                width: countText.implicitWidth + 14
                radius: 12
                color: "#1E293B"
                border.color: "#334155"
                border.width: 1

                Text {
                    id: countText
                    anchors.centerIn: parent
                    textFormat: Text.PlainText
                    text: (card.workspaces ? card.workspaces.length : 0) + " Workspaces"
                    color: "#94A3B8"
                    font.pixelSize: 11
                    font.weight: Font.Medium
                }
            }
        }

        Text {
            textFormat: Text.PlainText
            text: (card.world && card.world.description) ? card.world.description : "No context summary provided"
            color: "#CBD5E1"
            font.pixelSize: 12
            width: parent.width
            elide: Text.ElideRight
            maximumLineCount: 2
            wrapMode: Text.WordWrap
        }

        Rectangle {
            width: parent.width
            height: 1
            color: "#27354F"
        }

        Text {
            textFormat: Text.PlainText
            text: "WORKSPACES IN THIS WORLD"
            color: "#64748B"
            font.pixelSize: 10
            font.weight: Font.Bold
        }

        Flow {
            width: parent.width
            spacing: 6
            clip: true

            Repeater {
                model: card.workspaces ? card.workspaces.slice(0, 4) : []
                delegate: Rectangle {
                    id: chipDelegate
                    required property var modelData
                    height: 24
                    width: Math.min(card.width - 40, chipLabel.implicitWidth + 16)
                    radius: 6
                    color: "#0F172A"
                    border.color: "#334155"
                    border.width: 1

                    Text {
                        id: chipLabel
                        anchors.centerIn: parent
                        textFormat: Text.PlainText
                        text: chipDelegate.modelData.name
                        color: "#E2E8F0"
                        font.pixelSize: 11
                        elide: Text.ElideRight
                        width: parent.width - 12
                        horizontalAlignment: Text.AlignHCenter
                    }
                }
            }

            Rectangle {
                visible: card.workspaces && card.workspaces.length > 4
                height: 24
                width: moreText.implicitWidth + 12
                radius: 6
                color: "#0F172A"
                border.color: "#334155"

                Text {
                    id: moreText
                    anchors.centerIn: parent
                    textFormat: Text.PlainText
                    text: "+" + (card.workspaces.length - 4) + " more"
                    color: "#94A3B8"
                    font.pixelSize: 11
                }
            }
        }

        Item {
            width: 1
            height: 1
        }

        Row {
            anchors.right: parent.right
            spacing: 6

            Text {
                textFormat: Text.PlainText
                text: !card.available ? "Unavailable" : (card.isSelected ? "Double-click or Enter to open →" : "Click to select · double-click to open")
                color: card.isSelected ? "#818CF8" : "#64748B"
                font.pixelSize: 11
                font.weight: Font.Medium
            }
        }
    }

    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: card.available ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: if (card.available)
            card.chosen()
        onDoubleClicked: if (card.available)
            card.activated()
    }
}
