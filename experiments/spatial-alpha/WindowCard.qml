pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

CyberCard {
    id: card
    property var window: null
    property bool showTitles: false
    property bool demoSource: false
    readonly property bool protectedIdentity: !!(window && (window.excludeFromCapture || window.protectedIdentity))
    readonly property string appIdentity: protectedIdentity ? "Protected window" :
        Display.safeText(window && (window.appName || window.appId)) || "Unknown application"
    readonly property string safeCaption: !protectedIdentity && showTitles && window && !window.titleRedacted
        ? Display.safeText(window.title) : ""
    readonly property string paintedStatus: demoSource ? "Demo" :
        ((window && window.statusBadge) ? window.statusBadge : "Unknown")

    implicitWidth: 320
    implicitHeight: 200
    radius: 12
    available: !(window && window.unavailable)
    accessibleLabel: appIdentity
    accessibleDescription: safeCaption || (protectedIdentity ? "Protected identity" : "Window title hidden")

    Column {
        anchors.fill: parent
        anchors.margins: 15
        spacing: 9

        Row {
            width: parent.width
            spacing: 8
            NeonText {
                width: Math.max(0, parent.width - status.width - 8)
                anchors.verticalCenter: parent.verticalCenter
                text: card.appIdentity
                font.pixelSize: 16
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            StatusBadge { id: status; status: card.paintedStatus }
        }

        NeonText {
            width: parent.width
            text: card.protectedIdentity ? "Protected identity" : card.safeCaption ||
                (!card.demoSource && card.window && card.window.titleRedacted ? "Title withheld" :
                 card.showTitles ? "Title unavailable" : "Title hidden")
            color: NeonTheme.muted
            font.pixelSize: NeonTheme.detailSize
            elide: Text.ElideRight
        }

        Rectangle {
            width: parent.width
            height: 76
            radius: NeonTheme.buttonRadius
            color: card.highContrast ? NeonTheme.contrastPanel : NeonTheme.background
            border.color: NeonTheme.border
            border.width: 1

            Rectangle {
                anchors.fill: parent
                anchors.margins: 2
                radius: NeonTheme.buttonRadius
                visible: !card.lowEffects && !card.highContrast && !card.protectedIdentity
                gradient: Gradient {
                    orientation: Gradient.Horizontal
                    GradientStop { position: 0; color: NeonTheme.tint(NeonTheme.cyan, .14) }
                    GradientStop { position: 1; color: NeonTheme.transparent }
                }
            }

            NeonText {
                anchors.centerIn: parent
                text: card.protectedIdentity ? "◇" : Array.from(card.appIdentity).slice(0, 2).join("").toUpperCase()
                color: card.protectedIdentity ? NeonTheme.muted : NeonTheme.cyan
                font.pixelSize: 35
            }

            NeonLabel {
                anchors.left: parent.left
                anchors.bottom: parent.bottom
                anchors.margins: 9
                text: card.protectedIdentity ? "PROTECTED" : "APPLICATION IDENTITY"
                font.pixelSize: 9
            }
        }

        Flow {
            width: parent.width
            spacing: 5

            NeonBadge {
                visible: !card.protectedIdentity && !!(card.window && card.window.focused)
                tone: NeonTheme.emerald
                label: card.demoSource ? "Sample focus" : "Focused"
            }

            NeonBadge {
                visible: !card.demoSource && !!(card.window && card.window.assignment)
                tone: NeonTheme.precision
                label: card.window && card.window.assignment === "explicit" ? "Explicit" : "Observed"
            }

            NeonBadge {
                visible: !card.protectedIdentity && !!(card.window && card.window.minimized)
                tone: NeonTheme.precision
                label: "Minimized"
            }

            NeonBadge {
                visible: !card.protectedIdentity && !!(card.window && card.window.fullscreen)
                tone: NeonTheme.precision
                label: "Fullscreen"
            }

            NeonText {
                visible: !card.protectedIdentity && !card.demoSource && !!(card.window && card.window.observedAt)
                width: parent.width
                text: "Observed " + Display.safeText(card.window ? card.window.observedAt : "")
                color: NeonTheme.muted
                font.pixelSize: NeonTheme.labelSize
                elide: Text.ElideRight
            }
        }
    }
}
