import QtQuick
import "theme"
import "Presentation.js" as Display

FocusScope {
    id: overlay
    required property var scene
    property bool isOpen: false
    signal dismissed
    readonly property var backend: scene.backend

    visible: opacity > 0 || isOpen
    enabled: isOpen
    opacity: isOpen ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: NeonTheme.ms(overlay.scene.motionOff, NeonTheme.modalMs) }
    }

    onIsOpenChanged: if (isOpen) motion.forceActiveFocus()
    Keys.onEscapePressed: overlay.dismissed()
    Keys.onPressed: event => {
        if (event.key === Qt.Key_F2) {
            overlay.dismissed();
            event.accepted = true;
        }
    }

    Rectangle {
        anchors.fill: parent
        color: NeonTheme.scrim
        MouseArea {
            anchors.fill: parent
            onClicked: overlay.dismissed()
        }
    }

    Rectangle {
        id: panel
        width: Math.min(490, parent.width - 30)
        height: Math.min(content.implicitHeight + 42, parent.height - 40)
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.rightMargin: 15
        anchors.topMargin: Math.min(NeonTheme.hudHeight + 12, parent.height - height - 12)
        radius: NeonTheme.cardRadius
        color: overlay.scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.panel
        border.color: NeonTheme.cyan
        border.width: 1
        clip: true

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Flickable {
            anchors.fill: parent
            contentHeight: content.implicitHeight + 42
            boundsBehavior: Flickable.StopAtBounds

            Column {
                id: content
                x: 21
                y: 20
                width: panel.width - 42
                spacing: 11

                Row {
                    width: parent.width
                    spacing: 10
                    Column {
                        width: parent.width - close.implicitWidth - 10
                        spacing: 3
                        NeonLabel { text: "STATUS & SETTINGS"; color: NeonTheme.cyan }
                        NeonText {
                            text: "Spatial Control Center"
                            width: parent.width
                            elide: Text.ElideRight
                            font.pixelSize: NeonTheme.titleSize
                            font.weight: Font.DemiBold
                        }
                    }
                    NeonButton {
                        id: close
                        text: "Close"
                        compact: true
                        onClicked: overlay.dismissed()
                    }
                }

                Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

                NeonLabel { text: "DATA CHANNELS" }
                NeonText {
                    width: parent.width
                    elide: Text.ElideRight
                    text: "Controller · " + (overlay.backend ? overlay.backend.controllerStatus : "Unavailable")
                }
                NeonText {
                    width: parent.width
                    elide: Text.ElideRight
                    text: "Observer · " + (overlay.backend ? overlay.backend.observationHealth : "Unavailable")
                    color: overlay.backend && overlay.backend.observerLive ? NeonTheme.emerald : NeonTheme.precision
                }
                NeonText {
                    width: parent.width
                    elide: Text.ElideRight
                    text: "Source · " + (overlay.backend ? overlay.backend.sourceLabel : "No source")
                        + (overlay.backend && overlay.backend.isDemo ? " / Sample hierarchy" :
                            overlay.backend && overlay.backend.sourceKind === "sqlite" ? " / Persistent hierarchy" : "")
                    color: NeonTheme.precision
                }
                NeonText {
                    width: parent.width
                    elide: Text.ElideRight
                    visible: !!(overlay.backend && overlay.backend.sourceLoadedAt)
                    text: "Source last read · " + Display.safeText(overlay.backend && overlay.backend.sourceLoadedAt)
                    color: NeonTheme.muted
                    font.pixelSize: NeonTheme.detailSize
                }
                NeonText {
                    width: parent.width
                    elide: Text.ElideRight
                    text: "World · " + Display.safeText(overlay.backend && overlay.backend.currentWorld
                        ? overlay.backend.currentWorld.name : "None")
                }
                NeonText {
                    width: parent.width
                    elide: Text.ElideRight
                    text: "Workspace · " + Display.safeText(overlay.backend && overlay.backend.currentWorkspace
                        ? overlay.backend.currentWorkspace.name : "None")
                }

                Rectangle { width: parent.width; height: 1; color: NeonTheme.border }
                NeonLabel { text: "PRESENTATION" }

                NeonButton {
                    id: motion
                    objectName: "reducedMotionToggle"
                    width: parent.width
                    checkable: true
                    checked: overlay.scene.reducedMotion
                    highlighted: checked
                    text: "Reduced motion · " + (checked ? "On" : "Off")
                    onClicked: overlay.scene.reducedMotion = checked
                }
                NeonButton {
                    objectName: "lowEffectsToggle"
                    width: parent.width
                    checkable: true
                    checked: overlay.scene.lowEffects
                    highlighted: checked
                    text: "Effects quality · " + (checked ? "Low" : "Standard")
                    onClicked: overlay.scene.lowEffects = checked
                }
                NeonButton {
                    objectName: "highContrastToggle"
                    width: parent.width
                    checkable: true
                    checked: overlay.scene.highContrast
                    highlighted: checked
                    text: "High contrast · " + (checked ? "On" : "Off")
                    onClicked: overlay.scene.highContrast = checked
                }
                NeonButton {
                    objectName: "showTitlesToggle"
                    width: parent.width
                    checkable: true
                    checked: overlay.scene.showTitles
                    highlighted: checked
                    text: "Window titles · " + (checked ? "Revealed" : "Hidden")
                    onClicked: overlay.scene.showTitles = checked
                }

                Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

                NeonLabel { text: "UI DIAGNOSTICS" }
                NeonText {
                    width: parent.width
                    elide: Text.ElideRight
                    text: Math.round(overlay.scene.width) + " × " + Math.round(overlay.scene.height)
                        + " logical px  ·  " + overlay.scene.levelTransitions + " depth transitions"
                    color: NeonTheme.muted
                    font.pixelSize: NeonTheme.detailSize
                }
            }
        }
    }
}
