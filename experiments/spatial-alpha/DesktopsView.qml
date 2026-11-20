pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

LevelTransition {
    id: view
    required property var scene
    readonly property var desktop: scene.backend ? scene.backend.currentDesktop : null
    readonly property real margin: width < NeonTheme.narrowWidth ? NeonTheme.compactMargin : NeonTheme.margin
    readonly property real gap: NeonTheme.gap
    readonly property int columns: width >= NeonTheme.threeColumnsWidth ? 3 : width >= NeonTheme.twoColumnsWidth ? 2 : 1
    readonly property real cardWidth: Math.min(350, (width - 2 * margin - (columns - 1) * gap - 28) / columns)
    readonly property alias panContent: content

    Column {
        id: content
        x: view.margin + view.scene.cameraX
        y: Math.max(view.margin, Math.min((parent.height - height) / 2, parent.height * 0.12)) + view.scene.cameraY
        width: parent.width - 2 * view.margin
        spacing: 15

        SceneHeading {
            width: parent.width
            label: "DEPTH 03  /  " + Display.safeText(view.scene.backend && view.scene.backend.currentWorkspace
                ? view.scene.backend.currentWorkspace.name : "WORKSPACE").toUpperCase()
            title: "Logical desktops"
            detail: "Read-only application metadata for the selected desktop"
        }

        Flow {
            width: parent.width
            spacing: 7

            Repeater {
                model: view.scene.currentDesktops
                delegate: NeonButton {
                    required property var modelData
                    required property int index
                    objectName: "desktopTab-" + modelData.id
                    text: Display.safeText(modelData.name)
                    highlighted: view.scene.desktopIndex === index
                    accent: NeonTheme.emerald
                    width: Math.min(implicitWidth, view.width - 2 * view.margin)
                    onClicked: {
                        view.scene.selectDesktopIndex(index);
                        view.scene.forceActiveFocus();
                    }
                }
            }
        }

        Rectangle {
            id: shelf
            width: parent.width
            height: shelfContent.implicitHeight + 32
            radius: NeonTheme.cardRadius
            color: view.scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.shell
            border.color: view.scene.highContrast ? NeonTheme.muted : NeonTheme.border

            Rectangle {
                anchors.top: parent.top
                anchors.left: parent.left
                anchors.margins: 12
                width: 44
                height: 2
                color: NeonTheme.emerald
            }

            Column {
                id: shelfContent
                x: 14
                y: 16
                width: parent.width - 28
                spacing: 12

                Item {
                    width: parent.width
                    height: desktopName.implicitHeight + desktopDetail.implicitHeight + 8
                    NeonText {
                        id: desktopName
                        width: Math.max(0, parent.width - observer.width - 12)
                        text: view.desktop ? Display.safeText(view.desktop.name) : "No logical desktops"
                        font.pixelSize: NeonTheme.titleSize
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    StatusBadge {
                        id: observer
                        anchors.right: parent.right
                        anchors.verticalCenter: desktopName.verticalCenter
                        status: view.scene.workspaceSourceBadge()
                    }
                    NeonText {
                        id: desktopDetail
                        anchors.top: desktopName.bottom
                        anchors.topMargin: 4
                        width: parent.width
                        elide: Text.ElideRight
                        color: NeonTheme.muted
                        font.pixelSize: NeonTheme.detailSize
                        text: {
                            if (!view.desktop)
                                return "";
                            const native = Display.safeText(view.desktop.nativeDesktopName || view.desktop.nativeDesktopId);
                            const role = Display.safeText(view.desktop.role);
                            const observed = Display.safeText(view.desktop.observedAt);
                            return [role, native ? "Native desktop · " + native : "No native association available",
                                observed ? "Observed " + observed : ""].filter(Boolean).join("   ·   ");
                        }
                    }
                }

                Grid {
                    columns: view.columns
                    spacing: view.gap

                    Repeater {
                        model: view.scene.currentWindows
                        delegate: WindowCard {
                            id: windowCard
                            required property var modelData
                            required property int index
                            objectName: "windowCard-" + modelData.id
                            width: view.cardWidth
                            window: modelData
                            demoSource: !!(view.scene.backend && (view.scene.backend.isDemo || view.scene.backend.mockMode))
                            isSelected: view.scene.windowIndex === index
                            isFocused: isSelected && view.activeLevel
                            showTitles: view.scene.showTitles
                            motionOff: view.motionOff
                            lowEffects: view.scene.lowEffects
                            highContrast: view.scene.highContrast
                            pointerEnabled: view.activeLevel && !view.scene.navigationBlocked()

                            onChosen: {
                                view.scene.backend.selectWindow(modelData.id);
                                view.scene.forceActiveFocus();
                            }
                        }
                    }
                }

                NeonText {
                    objectName: "emptyWindowsHint"
                    width: parent.width
                    visible: view.scene.currentWindows.length === 0
                    color: NeonTheme.precision
                    wrapMode: Text.WordWrap
                    text: {
                        if (!view.desktop) return "Create a logical desktop to organize this workspace.";
                        const b = view.scene.backend;
                        if (b && b.isDemo) return "This sample desktop has no window records.";
                        if (b && b.observerLive) return "Window associations are unavailable for this logical desktop.";
                        return "Native observer unavailable. Window metadata cannot be shown.";
                    }
                }
            }
        }
    }
}
