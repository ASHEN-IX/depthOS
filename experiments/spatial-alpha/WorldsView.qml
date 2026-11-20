pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

LevelTransition {
    id: view
    required property var scene
    readonly property var selectedWorld: scene.backend ? scene.backend.currentWorld : null
    readonly property real margin: width < NeonTheme.narrowWidth ? NeonTheme.compactMargin : NeonTheme.margin
    readonly property bool compact: width < 740
    readonly property bool previews: width >= 630 && height >= 515
    readonly property int peripheral: compact ? 0 : width >= 1160 ? 2 : 1
    readonly property real cardWidth: compact ? Math.min(440, width - 2 * margin - 16)
        : Math.min(450, Math.max(286, width * 0.325))
    readonly property real deckTop: heading.y + heading.height + (compact ? 12 : 19)
    readonly property real deckBottom: summary.y - (compact ? 48 : 52)
    readonly property real cardHeight: Math.min(310, Math.max(120,
        (deckBottom - deckTop - (compact ? 5 : 20)) / NeonTheme.selectedScale))
    readonly property real spacing: cardWidth * 0.96
    readonly property real heroCenter: (deckTop + deckBottom) / 2
    readonly property int selectedDesktops: scene.currentWorkspaces.reduce(
        (sum, ws) => sum + scene.backend.getDesktopsForWorkspace(ws.id).length, 0)

    SceneHeading {
        id: heading
        objectName: "worldsHeading"
        x: view.margin
        y: view.compact ? 10 : 17
        width: view.width - 2 * x
        label: ""
        title: "Worlds"
        detail: "Your spatial spaces for work, life, and exploration."
    }

    NeonText {
        objectName: "emptyHierarchyHint"
        anchors.centerIn: parent
        visible: view.scene.currentWorlds.length === 0
        text: "No worlds in this workspace yet."
        color: NeonTheme.muted
    }

    Rectangle {
        visible: view.width >= 960 && !!view.selectedWorld
        anchors.right: parent.right
        anchors.rightMargin: view.margin
        y: 22
        width: totals.implicitWidth + 32
        height: 40
        radius: 13
        color: view.scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.glass
        border.color: NeonTheme.strongBorder
        NeonText {
            id: totals
            anchors.centerIn: parent
            text: Display.countLabel(view.scene.currentWorlds.length, "World", "Worlds") + "     │     "
                + Display.countLabel(view.scene.backend.workspaces.length, "Workspace", "Workspaces") + "     │     "
                + Display.countLabel(view.scene.backend.desktops.length, "Desktop", "Desktops")
        }
    }

    Image {
        anchors.horizontalCenter: parent.horizontalCenter
        y: view.deckBottom - height * 0.65
        width: Math.min(parent.width, 1340)
        height: view.compact ? 44 : 103
        source: visible ? "art/orbit.png" : ""
        visible: !!view.selectedWorld && (view.activeLevel || view.opacity > 0)
            && !view.scene.lowEffects && !view.scene.highContrast
    }

    Repeater {
        model: view.scene.currentWorlds
        delegate: Loader {
            id: slot
            required property var modelData
            required property int index
            readonly property int distance: Math.abs(index - view.scene.carouselIndex)
            active: (view.activeLevel || view.opacity > 0) && distance <= view.peripheral + 1
            width: view.cardWidth
            height: view.cardHeight
            z: 8 - distance

            sourceComponent: WorldCard {
                id: worldCard
                readonly property int index: slot.index
                readonly property var modelData: slot.modelData
                readonly property int offset: index - view.scene.carouselIndex
                readonly property int depth: Math.abs(offset)
                readonly property real arc: Math.sign(offset) * (depth === 0 ? 0 : 1 + (Math.min(depth, 5) - 1) * 0.53)
                objectName: "worldCard-" + modelData.id
                world: modelData
                workspaces: view.scene.backend ? view.scene.backend.getWorkspacesForWorld(modelData.id) : []
                desktopCount: workspaces.reduce((sum, ws) => sum + view.scene.backend.getDesktopsForWorkspace(ws.id).length, 0)
                identityIndex: index
                identityTone: index % 2 ? NeonTheme.cyan : NeonTheme.lavender
                customCategories: view.scene.backend.customWorldArtwork
                width: view.cardWidth
                height: view.cardHeight
                cameraFocused: offset === 0
                isSelected: view.scene.worldIndex === index
                isFocused: isSelected && view.activeLevel
                motionOff: view.motionOff
                lowEffects: view.scene.lowEffects
                highContrast: view.scene.highContrast
                pointerEnabled: view.activeLevel && !view.scene.navigationBlocked() && depth <= view.peripheral
                visible: opacity > 0

                x: (view.width - width) / 2 + arc * view.spacing + view.scene.cameraX
                y: view.heroCenter - height / 2 + Math.min(3, depth) * 17 + view.scene.cameraY * 0.15
                z: 8 - depth
                opacity: depth > view.peripheral ? 0 : depth === 0 ? 1 : depth === 1 ? 0.95 : 0.75
                scale: offset === 0 ? NeonTheme.selectedScale : Math.max(0.67, 1 - depth * NeonTheme.depthScale)
                transform: Rotation {
                    origin.x: worldCard.width / 2
                    origin.y: worldCard.height / 2
                    axis { x: 0; y: 1; z: 0 }
                    angle: worldCard.motionOff ? 0 : -Math.sign(worldCard.offset)
                        * Math.min(2, worldCard.depth) * NeonTheme.maxTilt * 1.6
                    Behavior on angle {
                        NumberAnimation {
                            duration: NeonTheme.ms(worldCard.motionOff, NeonTheme.slideMs)
                            easing.type: NeonTheme.easing
                        }
                    }
                }

                Behavior on x {
                    NumberAnimation { duration: NeonTheme.ms(worldCard.motionOff, NeonTheme.slideMs); easing.type: NeonTheme.easing }
                }
                Behavior on y {
                    NumberAnimation { duration: NeonTheme.ms(worldCard.motionOff, NeonTheme.slideMs); easing.type: NeonTheme.easing }
                }
                Behavior on scale {
                    NumberAnimation { duration: NeonTheme.ms(worldCard.motionOff, NeonTheme.slideMs); easing.type: NeonTheme.easing }
                }
                Behavior on opacity {
                    NumberAnimation { duration: NeonTheme.ms(worldCard.motionOff, NeonTheme.slideMs); easing.type: NeonTheme.easing }
                }

                onChosen: {
                    view.scene.selectWorldFromPointer(index);
                    view.scene.forceActiveFocus();
                }

                onActivated: {
                    view.scene.selectWorldFromPointer(index);
                    view.scene.pointerDrillDown();
                    view.scene.forceActiveFocus();
                }
            }
        }
    }

    Row {
        objectName: "worldPaging"
        visible: !!view.selectedWorld
        anchors.horizontalCenter: parent.horizontalCenter
        y: summary.y - height - 8
        spacing: 8
        NeonButton {
            compact: true
            text: "‹"
            Accessible.name: "Previous world"
            enabled: view.scene.worldIndex > 0
            onClicked: view.scene.moveHorizontal(-1)
        }
        NeonText {
            anchors.verticalCenter: parent.verticalCenter
            text: (view.scene.worldIndex + 1) + " / " + view.scene.currentWorlds.length
            color: NeonTheme.precision
        }
        NeonButton {
            compact: true
            text: "›"
            Accessible.name: "Next world"
            enabled: view.scene.worldIndex + 1 < view.scene.currentWorlds.length
            onClicked: view.scene.moveHorizontal(1)
        }
    }

    SpatialGlass {
        id: summary
        objectName: "worldSummary"
        anchors.bottom: parent.bottom
        anchors.bottomMargin: view.compact ? 8 : 19
        x: view.margin
        width: view.width - 2 * x
        height: view.previews && view.scene.currentWorkspaces.length > 0 ? (view.compact ? 174 : 222) : 64
        visible: !!view.selectedWorld
        radius: NeonTheme.cardRadius + 6
        highContrast: view.scene.highContrast

        Item {
            id: summaryHeader
            x: 15
            y: 9
            width: parent.width - 30
            height: 44
            Column {
                width: parent.width - openWorld.width - 12
                spacing: 2
                NeonText {
                    width: parent.width
                    text: Display.safeText(view.selectedWorld && view.selectedWorld.name)
                    font.pixelSize: 19
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                NeonText {
                    width: parent.width
                    text: Display.countLabel(view.scene.currentWorkspaces.length, "workspace", "workspaces")
                        + "  ·  " + Display.countLabel(view.selectedDesktops, "desktop", "desktops")
                    font.pixelSize: NeonTheme.detailSize
                    color: NeonTheme.precision
                    elide: Text.ElideRight
                }
            }

            NeonButton {
                id: openWorld
                objectName: "openWorldButton"
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                text: "Open World  ↵"
                compact: view.compact
                highlighted: true
                accent: NeonTheme.emerald
                enabled: !!view.selectedWorld && !view.selectedWorld.unavailable
                onClicked: {
                    view.scene.enterDeeper();
                    view.scene.forceActiveFocus();
                }
            }
        }

        Rectangle {
            x: 15
            y: 59
            width: parent.width - 30
            height: 1
            color: NeonTheme.border
            visible: previewLoader.active
        }

        Loader {
            id: previewLoader
            x: 15
            y: 70
            width: parent.width - 30
            height: parent.height - y - 13
            active: view.previews && summary.height > 80 && (view.activeLevel || view.opacity > 0)
            sourceComponent: Column {
                spacing: 8
                NeonLabel { text: "WORKSPACES IN THIS WORLD"; color: NeonTheme.precision }
                Row {
                    id: previewsRow
                    width: parent.width
                    spacing: 10
                    readonly property int slots: view.width >= 1080 ? 4 : view.width >= 820 ? 3 : 2
                    readonly property int count: Math.min(slots, view.scene.currentWorkspaces.length)
                    Repeater {
                        model: view.scene.currentWorkspaces.slice(0, previewsRow.count)
                        delegate: WorkspacePreview {
                            required property var modelData
                            required property int index
                            width: (previewLoader.width - (previewsRow.count - 1) * 10) / Math.max(1, previewsRow.count)
                            height: Math.max(54, previewLoader.height - 24)
                            objectName: "worldPreview-" + modelData.id
                            workspace: modelData
                            ordinal: index
                            desktopCount: view.scene.backend.getDesktopsForWorkspace(modelData.id).length
                            highContrast: view.scene.highContrast
                            lowEffects: view.scene.lowEffects
                            highlighted: modelData.id === view.scene.backend.selectedWorkspaceId
                            onClicked: {
                                view.scene.backend.selectWorkspace(modelData.id);
                                view.scene.enterDeeper();
                                view.scene.forceActiveFocus();
                            }
                        }
                    }
                }
            }
        }
    }
}
