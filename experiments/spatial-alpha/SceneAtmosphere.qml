import QtQuick
import "theme"

Rectangle {
    id: sky
    property bool lowEffects: false
    property bool highContrast: false
    property int level: 1
    property bool wide: false
    property real drift: 0

    color: highContrast ? NeonTheme.contrastPanel : NeonTheme.background

    Image {
        anchors.fill: parent
        anchors.margins: sky.wide ? 0 : -8
        source: sky.lowEffects || sky.highContrast ? "" : sky.wide ? "art/reference-stage.jpg" : "art/horizon.jpg"
        fillMode: sky.wide ? Image.Stretch : Image.PreserveAspectCrop
        asynchronous: true
        horizontalAlignment: Image.AlignHCenter
        verticalAlignment: Image.AlignTop
        opacity: sky.level === 1 ? 1 : 0.70
        transform: Translate { x: sky.wide ? 0 : Math.max(-8, Math.min(8, sky.drift * 0.16)) }
    }
}
