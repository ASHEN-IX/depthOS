import QtQuick
import "theme"

Row {
    id: metric
    property string glyph
    property string label
    property int count
    spacing: 10
    SpatialIcon {
        name: metric.glyph
        ink: NeonTheme.mint
        width: 29; height: 29
        anchors.verticalCenter: parent.verticalCenter
    }
    Column {
        spacing: 1
        NeonText {
            text: metric.count
            font.pixelSize: 19
            font.weight: Font.DemiBold
        }
        NeonText {
            text: metric.label
            color: NeonTheme.precision
            font.pixelSize: 13
        }
    }
}
