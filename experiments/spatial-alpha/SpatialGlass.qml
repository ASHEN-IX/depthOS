import QtQuick
import "theme"

Rectangle {
    id: surface
    property bool highContrast: false
    property color rim: NeonTheme.strongBorder

    color: highContrast ? NeonTheme.contrastPanel : NeonTheme.glass
    radius: NeonTheme.cardRadius + 6
    border.color: highContrast ? NeonTheme.focus : rim
    gradient: highContrast ? null : wash

    Gradient {
        id: wash
        GradientStop { position: 0; color: NeonTheme.glassHighlight }
        GradientStop { position: 0.45; color: NeonTheme.glass }
        GradientStop { position: 1; color: NeonTheme.panel }
    }
}
