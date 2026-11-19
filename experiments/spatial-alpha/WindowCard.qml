pragma ComponentBehavior: Bound
import QtQuick

Rectangle {
    id: card

    property var window: null
    property bool isSelected: false
    property bool isFocused: false
    property bool showTitles: false // Privacy redaction toggle
    property bool motionOff: false

    signal chosen

    implicitWidth: 320
    implicitHeight: 220
    radius: 14

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
        anchors.margins: 14
        spacing: 10

        Row {
            width: parent.width
            spacing: 10

            Rectangle {
                width: 32
                height: 32
                radius: 8
                color: "#1E293B"
                border.color: "#334155"
                border.width: 1

                Text {
                    anchors.centerIn: parent
                    text: {
                        if (!card.window)
                            return "🗔";
                        const id = String(card.window.appId || "").toLowerCase();
                        if (id.includes("kate") || id.includes("code"))
                            return "📝";
                        if (id.includes("alacritty") || id.includes("terminal"))
                            return "💻";
                        if (id.includes("firefox") || id.includes("chrome") || id.includes("browser"))
                            return "🌐";
                        if (id.includes("dolphin") || id.includes("file"))
                            return "📁";
                        if (id.includes("creator"))
                            return "⚙️";
                        return "🗔";
                    }
                    font.pixelSize: 16
                }
            }

            Column {
                width: parent.width - 42 - statusBadge.width - 6
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                Text {
                    id: appNameText
                    textFormat: Text.PlainText
                    text: (card.window && card.window.appName) ? card.window.appName : (card.window ? card.window.appId : "Window")
                    color: "#F8FAFC"
                    font.pixelSize: 13
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                    width: parent.width
                }

                Text {
                    id: safeIdentityText
                    textFormat: Text.PlainText
                    text: card.showTitles ? ((card.window && card.window.title) ? card.window.title : "No title") : ((card.window && card.window.appId) ? card.window.appId : "Protected Identity")
                    color: card.showTitles ? "#CBD5E1" : "#818CF8"
                    font.pixelSize: 11
                    elide: Text.ElideRight
                    width: parent.width
                }
            }

            StatusBadge {
                id: statusBadge
                anchors.verticalCenter: parent.verticalCenter
                status: (card.window && card.window.statusBadge) ? card.window.statusBadge : "Unknown"
            }
        }

        Rectangle {
            id: previewBox
            width: parent.width
            height: 100
            radius: 8
            color: "#0B1020"
            border.color: "#1E293B"
            border.width: 1
            clip: true

            Column {
                anchors.centerIn: parent
                spacing: 6
                visible: !(card.window && card.window.minimized)

                Repeater {
                    model: 4
                    delegate: Rectangle {
                        required property int index
                        width: index === 0 ? 140 : (index === 1 ? 200 : (index === 2 ? 160 : 100))
                        height: 5
                        radius: 2
                        color: index === 0 ? "#4338CA" : "#1E293B"
                    }
                }
            }

            Rectangle {
                anchors.fill: parent
                visible: card.window && (card.window.minimized || card.window.statusBadge === "Unavailable")
                color: "#0F172ACC"

                Column {
                    anchors.centerIn: parent
                    spacing: 4

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        textFormat: Text.PlainText
                        text: (card.window && card.window.minimized) ? "Window Minimized" : "Preview Unavailable"
                        color: "#94A3B8"
                        font.pixelSize: 11
                        font.weight: Font.Medium
                    }

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        textFormat: Text.PlainText
                        text: "Metadata preserved · no compositor texture"
                        color: "#64748B"
                        font.pixelSize: 9
                    }
                }
            }

            Rectangle {
                anchors.top: parent.top
                anchors.right: parent.right
                anchors.margins: 6
                height: 18
                width: focusLabel.implicitWidth + 12
                radius: 4
                color: "#1E3A8A"
                border.color: "#3B82F6"
                border.width: 1
                visible: card.window && card.window.focused

                Text {
                    id: focusLabel
                    anchors.centerIn: parent
                    textFormat: Text.PlainText
                    text: "Active Focus"
                    color: "#93C5FD"
                    font.pixelSize: 9
                    font.weight: Font.DemiBold
                }
            }
        }

        Row {
            width: parent.width
            spacing: 6

            Rectangle {
                visible: card.window && card.window.minimized
                height: 18
                width: minText.implicitWidth + 10
                radius: 4
                color: "#1E293B"

                Text {
                    id: minText
                    anchors.centerIn: parent
                    textFormat: Text.PlainText
                    text: "Minimized"
                    color: "#94A3B8"
                    font.pixelSize: 9
                }
            }

            Rectangle {
                visible: card.window && card.window.fullscreen
                height: 18
                width: fsText.implicitWidth + 10
                radius: 4
                color: "#1E293B"

                Text {
                    id: fsText
                    anchors.centerIn: parent
                    textFormat: Text.PlainText
                    text: "Fullscreen"
                    color: "#94A3B8"
                    font.pixelSize: 9
                }
            }

            Text {
                textFormat: Text.PlainText
                text: (card.window && card.window.statusReason) ? card.window.statusReason : "Read-only observation"
                color: "#64748B"
                font.pixelSize: 10
                elide: Text.ElideRight
                width: parent.width - (card.window && card.window.minimized ? 65 : 0)
                anchors.verticalCenter: parent.verticalCenter
            }
        }
    }

    MouseArea {
        id: hoverArea
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: card.chosen()
    }
}
