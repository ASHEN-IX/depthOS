import QtQuick
import "theme"

NeonButton {
    id: action
    property string iconName
    property bool showText: true
    property bool lowEffects: false
    property int iconSize: 26

    contentItem: Item {
        // Icon-only actions still retain an accessible name through button.text.
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight
        readonly property bool truncated: label.truncated
        Row {
            id: row
            anchors.centerIn: parent
            spacing: action.showText ? 12 : 0
            SpatialIcon {
                anchors.verticalCenter: parent.verticalCenter
                name: action.iconName
                width: action.iconSize
                height: width
                ink: action.highlighted ? NeonTheme.mint : action.enabled ? NeonTheme.text : NeonTheme.muted
            }
            NeonText {
                id: label
                visible: action.showText
                anchors.verticalCenter: parent.verticalCenter
                text: action.text
                color: action.enabled ? NeonTheme.text : NeonTheme.muted
                font.pixelSize: 16
                font.weight: action.highlighted ? Font.DemiBold : Font.Normal
            }
        }
    }

    background: Item {
        Rectangle {
            anchors.fill: parent
            radius: 16
            color: action.hovered || action.activeFocus ? NeonTheme.glassHighlight : NeonTheme.transparent
            border.color: action.activeFocus ? NeonTheme.focus : NeonTheme.transparent
        }
        Rectangle {
            anchors.fill: parent
            radius: 16
            visible: action.highlighted
            gradient: Gradient {
                GradientStop { position: 0; color: NeonTheme.deepTeal }
                GradientStop { position: 0.6; color: NeonTheme.glass }
                GradientStop { position: 1; color: NeonTheme.activeTeal }
            }
        }
        NeonRim {
            anchors.fill: parent
            visible: action.highlighted
            radius: 16
            tone: NeonTheme.mint
            bright: true
            lowEffects: action.lowEffects
        }
    }
}
