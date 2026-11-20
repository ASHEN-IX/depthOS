import QtQuick
import QtQuick.Controls.Basic as Basic
import "theme"

Basic.Button {
    id: button
    property color accent: NeonTheme.cyan
    property bool compact: false

    implicitHeight: compact ? 28 : 36
    implicitWidth: Math.ceil(contentItem.implicitWidth) + leftPadding + rightPadding + 2
    horizontalPadding: compact ? 10 : 14
    verticalPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    background: Rectangle {
        color: button.highlighted || button.hovered || button.down ? NeonTheme.hover : NeonTheme.panel
        radius: NeonTheme.buttonRadius
        border.width: button.activeFocus ? 2 : 1
        border.color: button.activeFocus ? NeonTheme.focus : button.highlighted ? button.accent : NeonTheme.border
    }
    contentItem: NeonText {
        text: button.text
        color: button.enabled ? (button.highlighted ? button.accent : NeonTheme.text) : NeonTheme.muted
        font.pixelSize: NeonTheme.detailSize
        font.weight: Font.DemiBold
        elide: Text.ElideRight
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }
}
