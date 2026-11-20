pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

LevelTransition {
    id: view
    required property var scene
    readonly property real margin: width < NeonTheme.narrowWidth ? NeonTheme.compactMargin : NeonTheme.margin
    readonly property int columns: Math.min(Math.max(1, scene.currentWorkspaces.length),
        width >= NeonTheme.threeColumnsWidth ? 3 : width >= NeonTheme.twoColumnsWidth ? 2 : 1)
    readonly property real gap: NeonTheme.gap
    readonly property real cardWidth: Math.min(395, (width - 2 * margin - (columns - 1) * gap) / columns)
    readonly property alias panContent: content

    Column {
        id: content
        width: parent.width - 2 * view.margin
        x: view.margin + view.scene.cameraX
        y: Math.max(view.margin, (parent.height - height) / 2) + view.scene.cameraY
        spacing: 22

        SceneHeading {
            width: parent.width
            label: "DEPTH 02  /  " + Display.safeText(view.scene.backend && view.scene.backend.currentWorld
                ? view.scene.backend.currentWorld.name : "WORLD").toUpperCase()
            title: "Workspaces"
            detail: Display.countLabel(view.scene.currentWorkspaces.length, "workspace", "workspaces") + " in this world"
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

        NeonText {
            visible: view.scene.currentWorkspaces.length === 0
            text: "No workspaces created in this world yet."
            color: NeonTheme.muted
        }
    }
}
