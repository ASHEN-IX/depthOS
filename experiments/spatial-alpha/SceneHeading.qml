import QtQuick
import "theme"

Column {
    id: heading
    property int titleSize: 0
    property int detailSize: 0
    property alias label: eyebrow.text
    property alias title: name.text
    property alias detail: subtitle.text
    spacing: 5

    NeonLabel {
        id: eyebrow
        visible: text.length > 0
        width: parent.width
        color: NeonTheme.cyan
        elide: Text.ElideRight
    }

    NeonText {
        id: name
        width: parent.width
        font.pixelSize: heading.titleSize || (heading.width >= 700 ? NeonTheme.displaySize : 34)
        font.weight: Font.DemiBold
        elide: Text.ElideRight
    }

    NeonText {
        id: subtitle
        width: parent.width
        visible: text.length > 0
        color: NeonTheme.precision
        font.pixelSize: heading.detailSize || NeonTheme.bodySize
        elide: Text.ElideRight
    }
}
