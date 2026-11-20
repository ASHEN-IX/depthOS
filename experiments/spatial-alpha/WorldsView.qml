pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

LevelTransition {
    id: view
    required property var scene
    readonly property var selectedWorld: scene.backend ? scene.backend.currentWorld : null
    readonly property real margin: width < NeonTheme.narrowWidth ? NeonTheme.compactMargin : NeonTheme.margin
    readonly property real cardWidth: Math.min(420, Math.max(205, width * 0.33))
    readonly property real cardHeight: Math.min(332, Math.max(212, height * 0.49))
    readonly property real spacing: Math.min(width * 0.285, cardWidth * 0.99)
    readonly property real heroCenter: height * 0.5

    SceneHeading {
        x: view.margin
        y: Math.max(10, view.height * 0.043)
        width: view.width - 2 * x
        label: "DEPTH 01  /  HIERARCHY"
        title: "Worlds"
        detail: "Select a world to explore its workspaces and logical desktops."
    }

    NeonText {
        objectName: "emptyHierarchyHint"
        anchors.centerIn: parent
        visible: view.scene.currentWorlds.length === 0
        text: "No worlds in this workspace yet."
        color: NeonTheme.muted
    }

    Repeater {
        model: view.scene.currentWorlds
        delegate: Loader {
            id: slot
            required property var modelData
            required property int index
            readonly property int distance: Math.abs(index - view.scene.carouselIndex)
            active: distance <= 4
            width: view.cardWidth
            height: view.cardHeight
            z: 8 - distance

            sourceComponent: WorldCard {
                id: worldCard
                readonly property int index: slot.index
                readonly property var modelData: slot.modelData
                readonly property int offset: index - view.scene.carouselIndex
                readonly property int depth: Math.abs(offset)
                readonly property real arc: Math.sign(offset) * (depth === 0 ? 0 : 1 + (Math.min(depth, 5) - 1) * 0.71)
                objectName: "worldCard-" + modelData.id
                world: modelData
                workspaces: view.scene.backend ? view.scene.backend.getWorkspacesForWorld(modelData.id) : []
                desktopCount: workspaces.reduce((sum, ws) => sum + view.scene.backend.getDesktopsForWorkspace(ws.id).length, 0)
                identityIndex: index
                identityTone: [NeonTheme.cyan, NeonTheme.lavender, NeonTheme.cyan, NeonTheme.lavender][index % 4]
                width: view.cardWidth
                height: view.cardHeight
                cameraFocused: offset === 0
                isSelected: view.scene.worldIndex === index
                isFocused: isSelected && view.activeLevel
                motionOff: view.motionOff
                lowEffects: view.scene.lowEffects
                highContrast: view.scene.highContrast
                pointerEnabled: view.activeLevel && !view.scene.navigationBlocked() && depth <= 3
                visible: depth <= 3 || opacity > 0

                x: (view.width - width) / 2 + arc * view.spacing + view.scene.cameraX
                y: view.heroCenter - height / 2 + Math.min(3, depth) * 29 + view.scene.cameraY * 0.15
                z: 8 - depth
                opacity: depth === 0 ? 1 : depth === 1 ? 0.88 : depth === 2 ? 0.72 : depth === 3 ? 0.5 : 0
                scale: offset === 0 ? NeonTheme.selectedScale : Math.max(0.67, 1 - depth * NeonTheme.depthScale)
                transform: Rotation {
                    origin.x: worldCard.width / 2
                    origin.y: worldCard.height / 2
                    axis { x: 0; y: 1; z: 0 }
                    angle: worldCard.motionOff ? 0 : -Math.sign(worldCard.offset)
                        * Math.min(2, worldCard.depth) * NeonTheme.maxTilt
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

    Rectangle {
        id: summary
        anchors.bottom: parent.bottom
        anchors.bottomMargin: Math.max(10, view.height * 0.035)
        x: view.margin
        width: view.width - 2 * x
        height: view.height >= 570 ? 98 : 82
        radius: NeonTheme.cardRadius
        color: view.scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.shell
        border.color: view.scene.highContrast ? NeonTheme.muted : NeonTheme.border
        visible: !!view.selectedWorld && view.height >= 430
        z: 10

        Rectangle {
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.margins: 14
            width: 2
            color: NeonTheme.emerald
            radius: 1
        }

        Column {
            x: 29
            width: Math.max(0, parent.width - 60 - controls.width)
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4

            NeonLabel { text: "SELECTED WORLD"; color: NeonTheme.emerald }
            NeonText {
                width: parent.width
                text: Display.safeText(view.selectedWorld ? view.selectedWorld.name : "")
                font.pixelSize: 19
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }
            NeonText {
                width: parent.width
                visible: text.length > 0 && summary.height >= 90
                text: view.scene.currentWorkspaces.slice(0, 3).map(ws => Display.safeText(ws.name)).join("  ·  ")
                color: NeonTheme.precision
                font.pixelSize: NeonTheme.detailSize
                elide: Text.ElideRight
            }
        }

        Row {
            id: controls
            anchors.right: parent.right
            anchors.rightMargin: 15
            anchors.verticalCenter: parent.verticalCenter
            spacing: 6

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
                font.pixelSize: NeonTheme.labelSize
                color: NeonTheme.precision
            }
            NeonButton {
                compact: true
                text: "›"
                Accessible.name: "Next world"
                enabled: view.scene.worldIndex < view.scene.currentWorlds.length - 1
                onClicked: view.scene.moveHorizontal(1)
            }
            NeonButton {
                compact: summary.width < 600
                text: summary.width < 480 ? "Open" : "Open world  ↵"
                highlighted: true
                accent: NeonTheme.emerald
                enabled: !!view.selectedWorld && !view.selectedWorld.unavailable
                onClicked: {
                    view.scene.enterDeeper();
                    view.scene.forceActiveFocus();
                }
            }
        }
    }
}
