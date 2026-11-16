import QtQuick
Rectangle {
    id: root
    property string text: ""
    signal triggered()
    width: label.implicitWidth + 24
    height: 36
    radius: 9
    color: pointer.containsMouse ? "#354158" : "#1E293B"
    border.color: "#475569"
    opacity: enabled ? 1 : 0.4
    Text { id: label; textFormat: Text.PlainText; anchors.centerIn: parent; text: root.text; color: "#F8FAFC"; font.pixelSize: 13 }
    MouseArea { id: pointer; anchors.fill: parent; hoverEnabled: true; onClicked: root.triggered() }
}
