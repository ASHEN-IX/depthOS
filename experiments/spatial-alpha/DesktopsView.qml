pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

LevelTransition {
    id: view
    required property var scene
    readonly property var backend: scene.backend
    readonly property var desktop: scene.backend ? scene.backend.currentDesktop : null
    readonly property real margin: width < NeonTheme.narrowWidth ? NeonTheme.compactMargin : NeonTheme.margin
    readonly property real gap: NeonTheme.gap
    readonly property int columns: width >= NeonTheme.threeColumnsWidth ? 3 : width >= NeonTheme.twoColumnsWidth ? 2 : 1
    readonly property real cardWidth: Math.min(350, (width - 2 * margin - (columns - 1) * gap - 28) / columns)
    readonly property var panContent: (body.item as Body)?.panContent ?? null

    Loader {
        id: body
        anchors.fill: parent
        active: view.activeLevel || view.opacity > 0
        sourceComponent: Body {}
    }

    component Body: Item {
        readonly property alias panContent: content
        readonly property var rows: view.scene.currentWindows
        onRowsChanged: syncWindows()
        Component.onCompleted: syncWindows()

        ListModel { id: windowModel }

        function syncWindows() {
            const wanted = new Set(rows.map(row => row.id));
            for (let index = windowModel.count - 1; index >= 0; --index) {
                if (!wanted.has(windowModel.get(index).recordId))
                    windowModel.remove(index);
            }
            for (let index = 0; index < rows.length; ++index) {
                const row = rows[index];
                const json = JSON.stringify(row);
                let found = index;
                while (found < windowModel.count && windowModel.get(found).recordId !== row.id)
                    ++found;
                if (found === windowModel.count) {
                    windowModel.insert(index, { recordId: row.id, recordJson: json });
                    continue;
                }
                if (found !== index)
                    windowModel.move(found, index, 1);
                if (windowModel.get(index).recordJson !== json)
                    windowModel.setProperty(index, "recordJson", json);
            }
        }

        Column {
            id: content
            x: view.margin + view.scene.cameraX
            y: Math.max(view.margin, Math.min((view.height - height) * 0.32, view.height * 0.10)) + view.scene.cameraY
            width: parent.width - 2 * view.margin
            spacing: 15

            SceneHeading {
                width: parent.width
                label: "DEPTH 03  /  " + Display.safeText(view.scene.backend && view.scene.backend.currentWorkspace ? view.scene.backend.currentWorkspace.name : "WORKSPACE").toUpperCase()
                title: "Logical desktops"
                detail: "Read-only application metadata for the selected desktop"
            }

            Flow {
                id: selector
                width: parent.width
                spacing: view.gap
                readonly property int slots: Math.min(view.columns, Math.max(1, view.scene.currentDesktops.length))

                Repeater {
                    model: view.scene.currentDesktops
                    delegate: WorkspacePreview {
                        required property var modelData
                        required property int index
                        objectName: "desktopTab-" + modelData.id
                        workspace: modelData
                        ordinal: index
                        subtitle: Display.safeText(modelData.role)
                        highlighted: view.scene.desktopIndex === index
                        highContrast: view.scene.highContrast
                        lowEffects: view.scene.lowEffects
                        width: (selector.width - (selector.slots - 1) * selector.spacing) / selector.slots
                        height: 67
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
                color: view.scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.glass
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
                        height: desktopName.implicitHeight + (desktopDetail.text ? desktopDetail.implicitHeight + 8 : 0)
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
                            status: view.backend && !view.backend.isDemo
                                ? (view.backend.shelfFreshness === "live" ? "Live"
                                   : view.backend.shelfFreshness === "stale" ? "Stale" : "Unavailable")
                                : view.scene.workspaceSourceBadge()
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
                                const role = Display.safeText(view.desktop.role);
                                if (view.backend && !view.backend.isDemo) {
                                    return view.backend.shelfFreshness === "live"
                                        ? Display.countLabel(view.scene.currentWindows.length, "verified window", "verified windows") + "  ·  Titles withheld"
                                        : role;
                                }
                                const native = Display.safeText(view.desktop.nativeDesktopName || view.desktop.nativeDesktopId);
                                const observed = Display.safeText(view.desktop.observedAt);
                                return [role, native ? "Native desktop · " + native : "No native association available", observed ? "Observed " + observed : ""].filter(Boolean).join("   ·   ");
                            }
                        }
                    }

                    Grid {
                        columns: view.columns
                        spacing: view.gap

                        Repeater {
                            model: windowModel
                            delegate: WindowCard {
                                id: windowCard
                                required property string recordId
                                required property string recordJson
                                required property int index
                                objectName: "windowCard-" + recordId
                                width: view.cardWidth
                                window: JSON.parse(recordJson)
                                demoSource: !!(view.scene.backend && (view.scene.backend.isDemo || view.scene.backend.mockMode))
                                isSelected: view.scene.windowIndex === index
                                isFocused: isSelected && view.activeLevel
                                showTitles: view.scene.showTitles
                                motionOff: view.motionOff
                                lowEffects: view.scene.lowEffects
                                highContrast: view.scene.highContrast
                                pointerEnabled: view.activeLevel && !view.scene.navigationBlocked()

                                onChosen: {
                                    view.scene.backend.selectWindow(recordId);
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
                            if (!view.desktop)
                                return "Create a logical desktop to organize this workspace.";
                            const b = view.scene.backend;
                            if (b && b.isDemo)
                                return "This sample desktop has no window records.";
                            if (b && b.shelfFreshness === "live")
                                return "No verified windows on this logical desktop.";
                            if (b && b.shelfFreshness === "stale")
                                return "Window observations are stale. Waiting for a fresh snapshot.";
                            if (b && b.observerLive)
                                return "Window associations are unavailable for this logical desktop.";
                            return "Native observer unavailable. Window metadata cannot be shown.";
                        }
                    }
                    NeonText {
                        width: parent.width
                        visible: view.backend && view.backend.shelfFreshness === "live"
                        color: NeonTheme.muted
                        wrapMode: Text.WordWrap
                        text: "Protected windows and windows outside the verified workspace are hidden."
                    }
                    NeonText {
                        width: parent.width
                        visible: text !== ""
                        color: NeonTheme.muted
                        wrapMode: Text.WordWrap
                        text: view.backend ? view.backend.workspaceWindowSummary(view.backend.selectedWorkspaceId) : ""
                    }
                }
            }
        }
    }
}
