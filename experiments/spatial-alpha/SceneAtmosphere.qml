import QtQuick
import "theme"

Rectangle {
    id: sky
    property bool lowEffects: false
    property bool highContrast: false
    property int level: 1
    property real drift: 0

    color: highContrast ? NeonTheme.contrastPanel : NeonTheme.background

    Image {
        anchors.fill: parent
        anchors.margins: -8
        source: sky.lowEffects || sky.highContrast ? "" : "art/horizon.jpg"
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        horizontalAlignment: Image.AlignHCenter
        verticalAlignment: Image.AlignTop
        opacity: sky.level === 1 ? 1 : 0.70
        transform: Translate { x: Math.max(-8, Math.min(8, sky.drift * 0.16)) }
    }
}
