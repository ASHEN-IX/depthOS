pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display
import "WorldAppearance.js" as Appearance

LevelTransition {
    id: view
    required property var scene
    readonly property real margin: width < NeonTheme.narrowWidth ? NeonTheme.compactMargin : NeonTheme.margin
    readonly property int columns: Math.min(Math.max(1, scene.currentWorkspaces.length), width >= NeonTheme.threeColumnsWidth ? 3 : width >= NeonTheme.twoColumnsWidth ? 2 : 1)
    readonly property real gap: NeonTheme.gap
    readonly property real cardWidth: Math.min(478, (width - 2 * margin - (columns - 1) * gap) / columns)
    readonly property var panContent: (body.item as Body)?.panContent ?? null
    readonly property var world: scene.backend && scene.backend.currentWorld
    readonly property var activeWorkspace: scene.backend && scene.backend.currentWorkspace

    Loader {
        id: body
        anchors.fill: parent
        // The source item sizes itself to its content so wheel bounds stay accurate.
        sourceComponent: Body {}
        active: view.activeLevel || view.opacity > 0
    }

    component Body: Item {
        readonly property alias panContent: content
        Column {
            id: content
            width: parent.width - 2 * view.margin
            x: view.margin + view.scene.cameraX
            y: Math.max(view.margin, (view.height - height) * 0.18) + view.scene.cameraY
            spacing: 22

            SceneHeading {
                width: parent.width
                label: "DEPTH 02  /  " + Display.safeText(view.scene.backend && view.scene.backend.currentWorld ? view.scene.backend.currentWorld.name : "WORLD").toUpperCase()
                title: "Workspaces"
                detail: Display.countLabel(view.scene.currentWorkspaces.length, "workspace", "workspaces") + " in this world"
            }

            Rectangle {
                width: parent.width
                height: view.height < 570 ? 74 : 135
                radius: NeonTheme.cardRadius
                color: view.scene.highContrast ? NeonTheme.contrastSurface : NeonTheme.glass
                border.color: NeonTheme.strongBorder
                clip: true

                Image {
                    anchors.right: parent.right
                    anchors.rightMargin: 4
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width * 0.56
                    height: parent.height - 8
                    fillMode: Image.PreserveAspectCrop
                    asynchronous: true
                    source: view.world && !view.scene.lowEffects && !view.scene.highContrast ? "art/" + Appearance.category(view.world, view.scene.worldIndex) + ".jpg" : ""
                    opacity: 0.63
                }
                Rectangle {
                    anchors.fill: parent
                    anchors.margins: 3
                    radius: NeonTheme.cardRadius - 2
                    gradient: Gradient {
                        orientation: Gradient.Horizontal
                        GradientStop {
                            position: .35
                            color: NeonTheme.panel
                        }
                        GradientStop {
                            position: 1
                            color: NeonTheme.transparent
                        }
                    }
                }
                Column {
                    anchors.left: parent.left
                    anchors.leftMargin: 18
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width * .7
                    spacing: 4
                    NeonText {
                        text: Display.safeText(view.world && view.world.name)
                        width: parent.width
                        font.pixelSize: view.height < 570 ? 20 : 27
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    NeonText {
                        text: Display.safeText(view.world && view.world.description)
                        visible: text.length > 0 && view.height >= 570
                        width: parent.width
                        color: NeonTheme.precision
                        elide: Text.ElideRight
                    }
                }
            }

            Grid {
                columns: view.columns
                spacing: view.gap

                Repeater {
                    model: view.scene.currentWorkspaces
                    delegate: WorkspaceCard {
                        id: workspaceCard
                        required property var modelData
                        required property int index
                        objectName: "workspaceCard-" + modelData.id
                        width: view.cardWidth
                        workspace: modelData
                        ordinal: index
                        desktops: view.scene.backend ? view.scene.backend.getDesktopsForWorkspace(modelData.id) : []
                        demoSource: view.scene.backend && view.scene.backend.isDemo
                        observedWindowCount: {
                            const b = view.scene.backend;
                            if (!b || (!b.isDemo && !b.observerLive))
                                return -1;
                            const count = desktops.reduce((sum, d) => sum + b.getWindowsForDesktop(d.id).length, 0);
                            return b.isDemo || count > 0 ? count : -1;
                        }
                        freshness: {
                            const b = view.scene.backend;
                            if (demoSource)
                                return "Sample hierarchy";
                            if (modelData.updatedAt)
                                return "Workspace updated " + Display.safeText(modelData.updatedAt);
                            if (b && b.sourceLoadedAt)
                                return "Hierarchy read " + Display.safeText(b.sourceLoadedAt);
                            return "";
                        }
                        statusBadge: view.scene.workspaceSourceBadge()
                        isSelected: view.scene.workspaceIndex === index
                        isFocused: isSelected && view.activeLevel
                        motionOff: view.motionOff
                        lowEffects: view.scene.lowEffects
                        highContrast: view.scene.highContrast
                        pointerEnabled: view.activeLevel && !view.scene.navigationBlocked()
                        scale: isFocused ? 1.025 : 1
                        z: isFocused ? 1 : 0

                        Behavior on scale {
                            NumberAnimation {
                                duration: NeonTheme.ms(workspaceCard.motionOff, NeonTheme.hoverMs)
                                easing.type: NeonTheme.easing
                            }
                        }

                        onChosen: {
                            view.scene.backend.selectWorkspace(modelData.id);
                            view.scene.forceActiveFocus();
                        }
                        onActivated: {
                            view.scene.backend.selectWorkspace(modelData.id);
                            view.scene.forceActiveFocus();
                            view.scene.pointerDrillDown();
                        }
                    }
                }
            }

            Rectangle {
                visible: !!view.activeWorkspace && view.height >= 590
                width: parent.width
                height: 64
                radius: NeonTheme.cardRadius
                color: view.scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.glass
                border.color: NeonTheme.border
                NeonText {
                    x: 18
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - desktopOpen.width - 46
                    text: Display.safeText(view.activeWorkspace && view.activeWorkspace.name) + "  ·  " + Display.countLabel(view.scene.currentDesktops.length, "logical desktop", "logical desktops")
                    elide: Text.ElideRight
                }
                NeonButton {
                    id: desktopOpen
                    anchors.right: parent.right
                    anchors.rightMargin: 12
                    anchors.verticalCenter: parent.verticalCenter
                    text: "Open Desktops  ↵"
                    enabled: view.scene.currentDesktops.length > 0
                    onClicked: {
                        view.scene.enterDeeper();
                        view.scene.forceActiveFocus();
                    }
                }
            }

            NeonText {
                objectName: "workspaceWindowSummary"
                width: parent.width
                wrapMode: Text.WordWrap
                visible: text !== ""
                color: NeonTheme.muted
                text: view.scene.backend ? view.scene.backend.workspaceWindowSummary(view.scene.backend.selectedWorkspaceId) : ""
            }

            NeonText {
                visible: view.scene.currentWorkspaces.length === 0
                text: "No workspaces created in this world yet."
                color: NeonTheme.muted
            }
        }
    }
}
