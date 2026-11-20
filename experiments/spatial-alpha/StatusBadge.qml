import QtQuick
import "theme"

NeonBadge {
    id: badge
    property string status: "Unknown"
    property string detail: ""
    property bool compact: false
    label: compact ? "" : status
    dot: true
    tone: {
        switch (status) {
        case "Live": return NeonTheme.emerald;
        case "Stale": return NeonTheme.warning;
        case "Unavailable": return NeonTheme.danger;
        case "Reconnecting": return NeonTheme.cyan;
        case "Demo": return NeonTheme.lavender;
        default: return NeonTheme.muted;
        }
    }
    implicitWidth: compact ? 20 : labelWidth
    readonly property int labelWidth: Math.ceil(labelMetrics.advanceWidth) + 28

    TextMetrics {
        id: labelMetrics
        text: badge.label
        font.pixelSize: NeonTheme.labelSize
        font.weight: Font.DemiBold
    }
    Accessible.name: detail ? status + " — " + detail : status
}
