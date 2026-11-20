pragma Singleton
import QtQuick

QtObject {
    readonly property color background: "#080D17"
    readonly property color panel: "#0D1527"
    readonly property color surface: "#111C33"
    readonly property color hover: "#162642"
    readonly property color selected: "#112D31"
    readonly property color contrastPanel: "#000000"
    readonly property color contrastSurface: "#070D17"
    readonly property color emerald: "#12E6A0"
    readonly property color violet: "#8B5CF6"
    readonly property color lavender: "#C4ABFF"
    readonly property color cyan: "#22C7F2"
    readonly property color text: "#E8EFFA"
    readonly property color muted: "#95A5BB"
    readonly property color precision: "#B9C9DE"
    readonly property color border: "#273753"
    readonly property color strongBorder: "#526986"
    readonly property color warning: "#FFB85C"
    readonly property color danger: "#FF9AAB"
    readonly property color focus: "#FFFFFF"
    readonly property color scrim: "#C8080D17"
    readonly property color transparent: "transparent"

    readonly property color cyanHaze: "#3522C7F2"
    readonly property color violetHaze: "#538B5CF6"
    readonly property color emeraldHaze: "#3512E6A0"
    readonly property color grid: "#402D5081"
    readonly property color gridHorizon: "#7A3F5985"
    readonly property color mint: "#6FFFE9"
    readonly property color deepTeal: "#073238"
    readonly property color activeTeal: "#0D554C"
    readonly property color glassHighlight: "#F0172C3C"
    readonly property color glass: "#EC0C1925"
    readonly property color shell: "#EE0D1527"

    readonly property int margin: 28
    readonly property int compactMargin: 18
    readonly property int gap: 18
    readonly property int smallGap: 8
    readonly property int cardRadius: 16
    readonly property int buttonRadius: 8
    readonly property int badgeRadius: 5
    readonly property int hudHeight: 65
    readonly property int footerHeight: 45
    readonly property int narrowWidth: 850
    readonly property int threeColumnsWidth: 1040
    readonly property int twoColumnsWidth: 660
    readonly property int displaySize: 46
    readonly property int titleSize: 26
    readonly property int bodySize: 14
    readonly property int detailSize: 12
    readonly property int labelSize: 11
    readonly property real selectedScale: 1.05
    readonly property real depthScale: 0.13
    readonly property real maxTilt: 13
    readonly property real glowOpacity: 0.21
    readonly property int hoverMs: 160
    readonly property int slideMs: 220
    readonly property int zoomMs: 240
    readonly property int modalMs: 180
    readonly property int easing: Easing.OutCubic

    function ms(disabled, duration) {
        return disabled ? 0 : duration;
    }

    function tint(color, alpha) {
        return Qt.rgba(color.r, color.g, color.b, alpha);
    }
}
