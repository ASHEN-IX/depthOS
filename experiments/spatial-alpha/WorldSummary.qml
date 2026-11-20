pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display
import "WorldAppearance.js" as Appearance

SpatialGlass {
    id: panel
    required property var scene
    required property var selectedWorld
    property int desktopCount
    property bool wide
    property bool previews
    readonly property real inset: wide ? width * 0.028 : 15
    highContrast: scene.highContrast
    radius: wide ? 36 : 22

    Item {
        id: summaryHeader
        x: panel.inset
        y: panel.wide ? panel.height * 0.06 : 10
        width: parent.width - 2 * x
        height: panel.wide ? panel.height * 0.29 : 44

        Rectangle {
            anchors.fill: parent
            visible: panel.wide
            radius: 25
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: NeonTheme.deepTeal }
                GradientStop { position: 0.45; color: NeonTheme.glass }
                GradientStop { position: 1; color: NeonTheme.panel }
            }
            border.color: NeonTheme.tint(NeonTheme.cyan, 0.24)
        }

        Image {
            id: summaryArt
            anchors.right: parent.right
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            width: parent.width * 0.36
            height: parent.height - 6
            fillMode: Image.PreserveAspectCrop
            opacity: 0.51
            source: panel.wide && !panel.scene.lowEffects && !panel.highContrast ? "art/reference-preview-0.jpg" : ""
        }

        Rectangle {
            anchors.fill: summaryArt
            visible: summaryArt.status === Image.Ready
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0; color: NeonTheme.panel }
                GradientStop { position: 0.36; color: NeonTheme.transparent }
                GradientStop { position: 0.90; color: NeonTheme.transparent }
                GradientStop { position: 1; color: NeonTheme.panel }
            }
        }

        Row {
            x: panel.wide ? 35 : 0
            anchors.verticalCenter: parent.verticalCenter
            width: panel.wide ? parent.width * 0.40 : parent.width - openWorld.width - 12
            spacing: panel.wide ? 22 : 0
            SpatialIcon {
                name: Appearance.icon(Appearance.category(panel.selectedWorld, panel.scene.worldIndex))
                ink: NeonTheme.text
                anchors.verticalCenter: parent.verticalCenter
                width: 51; height: 51
                visible: panel.wide
            }
            Column {
                width: panel.wide ? Math.min(150, Math.max(0, parent.width - 156)) : parent.width
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2
                NeonText {
                    visible: panel.wide
                    text: "Selected World"
                    font.pixelSize: 13
                    color: NeonTheme.precision
                }
                NeonText {
                    width: parent.width
                    text: Display.safeText(panel.selectedWorld && panel.selectedWorld.name)
                    font.pixelSize: panel.wide ? 28 : 19
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                NeonText {
                    width: parent.width
                    visible: !panel.wide
                    text: Display.countLabel(panel.scene.currentWorkspaces.length, "workspace", "workspaces")
                        + "  ·  " + Display.countLabel(panel.desktopCount, "desktop", "desktops")
                    font.pixelSize: NeonTheme.detailSize
                    color: NeonTheme.precision
                    elide: Text.ElideRight
                }
            }
            NeonBadge {
                label: "Active"
                tone: NeonTheme.emerald
                visible: panel.wide
                anchors.verticalCenter: parent.verticalCenter
            }
        }

        Row {
            visible: panel.wide
            x: parent.width * 0.337
            anchors.verticalCenter: parent.verticalCenter
            spacing: panel.width >= 1210 ? 51 : 16
            Rectangle {
                width: 1
                height: 43
                color: NeonTheme.strongBorder
            }
            HierarchyMetric {
                glyph: "layers"
                label: "Workspaces"
                count: panel.scene.currentWorkspaces.length
            }
            HierarchyMetric {
                glyph: "tiles"
                label: "Desktops"
                count: panel.desktopCount
            }
        }
    }

    NeonButton {
        id: openWorld
        objectName: "openWorldButton"
        anchors.right: parent.right
        anchors.rightMargin: panel.inset + (panel.wide ? 10 : 0)
        y: panel.wide ? (panel.previews ? panel.height * 0.376 : panel.height - height - 7) : 17
        text: panel.wide ? "View All  →" : "Open World  ↵"
        compact: true
        highlighted: !panel.wide
        accent: NeonTheme.emerald
        enabled: !!panel.selectedWorld && !panel.selectedWorld.unavailable
        background: Rectangle {
            radius: 8
            color: !panel.wide || openWorld.hovered ? NeonTheme.deepTeal : NeonTheme.transparent
            border.color: openWorld.activeFocus ? NeonTheme.focus
                : panel.wide ? NeonTheme.transparent : NeonTheme.emerald
        }
        z: 1
        onClicked: panel.scene.openLevel(2)
    }

    Loader {
        id: previews
        x: panel.inset + (panel.wide ? 6 : 0)
        y: panel.wide ? panel.height * 0.392 : 69
        width: parent.width - 2 * x
        height: parent.height - y - (panel.wide ? 16 : 9)
        active: panel.previews && panel.visible
        sourceComponent: Column {
            spacing: panel.wide ? 8 : 4
            NeonText {
                text: "Workspaces in " + Display.safeText(panel.selectedWorld && panel.selectedWorld.name)
                color: NeonTheme.text
                width: parent.width - openWorld.width - 24
                height: panel.wide ? 24 : 20
                font.pixelSize: panel.wide ? 16 : 13
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            Row {
                id: previewsRow
                width: parent.width
                spacing: panel.wide ? 14 : 10
                readonly property int slots: panel.width >= 960 ? 4 : panel.width >= 720 ? 3 : 2
                readonly property int count: Math.min(slots, panel.scene.currentWorkspaces.length)
                Repeater {
                    model: panel.scene.currentWorkspaces.slice(0, previewsRow.count)
                    delegate: WorkspacePreview {
                        required property var modelData
                        required property int index
                        width: (previews.width - (previewsRow.count - 1) * previewsRow.spacing) / Math.max(1, previewsRow.count)
                        height: Math.max(54, previews.height - (panel.wide ? 32 : 24))
                        objectName: "worldPreview-" + modelData.id
                        workspace: modelData
                        ordinal: index
                        desktopCount: panel.scene.backend.getDesktopsForWorkspace(modelData.id).length
                        highContrast: panel.scene.highContrast
                        lowEffects: panel.scene.lowEffects
                        highlighted: modelData.id === panel.scene.backend.selectedWorkspaceId
                        referenceStyle: panel.wide
                        onClicked: {
                            panel.scene.backend.selectWorkspace(modelData.id);
                            panel.scene.openLevel(2);
                        }
                        onOpenDesktopsRequested: {
                            panel.scene.backend.selectWorkspace(modelData.id);
                            panel.scene.openLevel(3);
                        }
                    }
                }
            }
        }
    }
}
