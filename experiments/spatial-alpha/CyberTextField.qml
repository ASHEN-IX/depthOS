import QtQuick
import QtQuick.Controls.Basic as Basic
import "theme"

Item {
    id: root
    property string label: ""
    property alias text: input.text
    property alias placeholderText: input.placeholderText
    property alias readOnly: input.readOnly
    property string errorText: ""
    property alias echoMode: input.echoMode
    property bool hasFocus: input.activeFocus

    signal accepted()
    signal textModified()

    implicitWidth: 320
    implicitHeight: (label !== "" ? 22 : 0) + 38 + (errorText !== "" ? 20 : 0)

    Column {
        anchors.fill: parent
        spacing: 4

        NeonLabel {
            text: root.label
            visible: root.label !== ""
            color: root.errorText !== "" ? NeonTheme.danger : (root.hasFocus ? NeonTheme.cyan : NeonTheme.precision)
        }

        Rectangle {
            id: box
            width: parent.width
            height: 38
            radius: 8
            color: root.readOnly ? NeonTheme.panel : "#0B1424"
            border.width: root.hasFocus ? 2 : 1
            border.color: root.errorText !== "" ? NeonTheme.danger : (root.hasFocus ? NeonTheme.cyan : NeonTheme.border)

            Basic.TextField {
                id: input
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 12
                verticalAlignment: Text.AlignVCenter
                color: NeonTheme.text
                placeholderTextColor: NeonTheme.muted
                font.pixelSize: NeonTheme.bodySize
                font.family: "monospace"
                selectByMouse: true
                background: null

                onAccepted: root.accepted()
                onTextEdited: root.textModified()
            }
        }

        NeonText {
            visible: root.errorText !== ""
            text: root.errorText
            color: NeonTheme.danger
            font.pixelSize: 11
        }
    }

    function forceActiveFocus() {
        input.forceActiveFocus();
    }
}
