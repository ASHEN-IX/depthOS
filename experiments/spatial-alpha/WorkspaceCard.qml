pragma ComponentBehavior: Bound
import QtQuick

Rectangle {
    id: card

    property var workspace: null
    property var desktops: []
    property int observedWindowCount: 0
    property string statusBadge: "Unavailable"
    property bool isSelected: false
    property bool isFocused: false
    property bool motionOff: false
    readonly property bool available: !(card.workspace && card.workspace.unavailable)

    signal chosen
    signal activated

    implicitWidth: 380
    implicitHeight: 280
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
            spacing: 10

            Rectangle {
                width: 36
                height: 36
                radius: 10
                color: "#312E81"
                border.color: "#4338CA"
                border.width: 1

                Text {
                    anchors.centerIn: parent
                    text: "◫"
                    font.pixelSize: 18
                    color: "#A5B4FC"
                }
            }

            Column {
                width: parent.width - 46 - badgeItem.width - 10
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Text {
                    textFormat: Text.PlainText
                    text: card.workspace ? card.workspace.name : "Workspace"
                    color: "#F8FAFC"
                    font.pixelSize: 17
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                    width: parent.width
                }

                Text {
                    textFormat: Text.PlainText
                    text: (card.workspace && card.workspace.gitBranch) ? "git: " + card.workspace.gitBranch : "Root: " + (card.workspace ? card.workspace.rootDirectory : "~")
                    color: "#818CF8"
                    font.pixelSize: 11
                    elide: Text.ElideRight
                    width: parent.width
                }
            }

            StatusBadge {
                id: badgeItem
                anchors.verticalCenter: parent.verticalCenter
                status: card.statusBadge
            }
        }

        Text {
            textFormat: Text.PlainText
            text: (card.workspace && card.workspace.description) ? card.workspace.description : "No description provided"
            color: "#94A3B8"
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

        Row {
            width: parent.width
            spacing: 8

            Text {
                textFormat: Text.PlainText
                text: "LINKED DESKTOPS (" + (card.desktops ? card.desktops.length : 0) + ")"
                color: "#64748B"
                font.pixelSize: 10
                font.weight: Font.Bold
            }

            Item {
                width: 1
                height: 1
            }

            Text {
                textFormat: Text.PlainText
                text: card.observedWindowCount + " windows observed"
                color: "#A5B4FC"
                font.pixelSize: 10
            }
        }

        Row {
            width: parent.width
            height: 64
            spacing: 8

            Repeater {
                model: card.desktops ? card.desktops.slice(0, 3) : []
                delegate: Rectangle {
                    id: deskDelegate
                    required property var modelData
                    required property int index
                    width: (card.width - 40 - 16) / 3
                    height: 64
                    radius: 8
                    color: "#0F172A"
                    border.color: "#26354D"
                    border.width: 1

                    Column {
                        anchors.fill: parent
                        anchors.margins: 8
                        spacing: 4

                        Text {
                            textFormat: Text.PlainText
                            text: deskDelegate.modelData.name
                            color: "#E2E8F0"
                            font.pixelSize: 11
                            font.weight: Font.Medium
                            elide: Text.ElideRight
                            width: parent.width
                        }

                        Text {
                            textFormat: Text.PlainText
                            text: deskDelegate.modelData.role || "Logical Desktop"
                            color: "#64748B"
                            font.pixelSize: 9
                            elide: Text.ElideRight
                            width: parent.width
                        }

                        Row {
                            spacing: 3
                            Repeater {
                                model: Math.min(4, Math.max(1, deskDelegate.index + 1))
                                delegate: Rectangle {
                                    width: 8
                                    height: 6
                                    radius: 1
                                    color: "#475569"
                                }
                            }
                        }
                    }
                }
            }
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
        cursorShape: Qt.PointingHandCursor
        onClicked: if (card.available)
            card.chosen()
        onDoubleClicked: if (card.available)
            card.activated()
    }
}
