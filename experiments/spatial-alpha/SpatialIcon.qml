import QtQuick
import QtQuick.Shapes
import "theme"

Item {
    id: icon
    property string name: ""
    property color ink: NeonTheme.text
    implicitWidth: 24
    implicitHeight: 24

    function path(key) {
        switch (key) {
        case "logo": return "M12 2L21 7V12M21 15L12 21L3 16V11M3 8L12 2M4 11L12 16L20 11L12 6Z M8 11L12 8L16 11L12 14Z";
        case "world": return "M16.5 12a6.5 6.5 0 1 0-13 0a6.5 6.5 0 0 0 13 0M3 17C-1 14 5 9 11 6S23 4 22 8C21 13 9 20 4 18";
        case "home": return "M2 12L12 3L22 12M5 10V21H10V15H14V21H19V10";
        case "folder": return "M2 20V6H9L11 8H22V20ZM2 11H22";
        case "layers": return "M1 8L12 2L23 8L12 14ZM1 12L12 18L23 12M1 16L12 22L23 16";
        case "desktop": return "M2 3H22V17H2ZM12 17V21M7 22H17";
        case "book": return "M12 5Q7 2 2 5V20Q7 17 12 20Q17 17 22 20V5Q17 2 12 5ZM12 5V20";
        case "bars": return "M2 22V15H6V22ZM10 22V10H14V22ZM18 22V4H22V22Z";
        case "tiles": return "M3 3H9V9H3ZM15 3H21V9H15ZM3 15H9V21H3ZM15 15H21V21H15Z";
        case "search": return "M17 10a7 7 0 1 0-14 0a7 7 0 0 0 14 0M15 15L22 22";
        case "settings": return "M9 2H15L16 5L19 5L21 8L19 11L20 14L21 16L19 19L16 19L15 22H9L8 19L5 19L3 16L5 13L5 11L3 8L5 5H8ZM15 12a3 3 0 1 0-6 0a3 3 0 0 0 6 0";
        case "bell": return "M4 17L6 14V8a6 6 0 0 1 12 0v6l2 3H4ZM9 21Q12 24 15 21";
        case "research": return "M8 2H16M10 2V9L3 21Q2 23 5 23H19Q22 23 21 21L14 9V2M7 15H17M10 19h.1M14 17h.1";
        case "social": return "M15 7a4 4 0 1 0-8 0a4 4 0 0 0 8 0M2 21v-2a8 8 0 0 1 16 0v2ZM17 4a3 3 0 0 1 0 6M18 13q5 1 5 7";
        case "focus": return "M19 12a7 7 0 1 0-14 0a7 7 0 0 0 14 0M12 2v5M12 17v5M2 12h5M17 12h5";
        }
        return "";
    }

    Shape {
        width: 24
        height: 24
        anchors.centerIn: parent
        scale: Math.min(icon.width, icon.height) / 24
        preferredRendererType: Shape.CurveRenderer
        ShapePath {
            strokeColor: icon.ink
            strokeWidth: icon.name === "logo" ? 2.4 : 1.65
            fillColor: NeonTheme.transparent
            joinStyle: ShapePath.RoundJoin
            capStyle: ShapePath.RoundCap
            PathSvg { path: icon.path(icon.name) }
        }
    }
}
