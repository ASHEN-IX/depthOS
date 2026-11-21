pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

LevelTransition {
    id: view
    required property var scene
    readonly property var selectedWorld: scene.backend ? scene.backend.currentWorld : null
    readonly property real margin: width < NeonTheme.narrowWidth ? NeonTheme.compactMargin : NeonTheme.margin
    readonly property bool wide: scene.wideChrome
    readonly property bool compact: width < 740
    readonly property bool previews: width >= 630 && height >= 515
    readonly property int peripheral: compact ? 0 : width >= 1160 ? 2 : 1
    readonly property real cardWidth: wide ? scene.width * 0.266 : compact ? Math.min(440, width - 2 * margin - 16)
        : Math.min(475, Math.max(286, width * 0.325))
    readonly property real deckTop: heading.y + heading.height + (compact ? 12 : 19)
    readonly property real deckBottom: summary.y - (compact ? 48 : 52)
    readonly property real cardHeight: wide ? height * 0.307 : Math.min(310, Math.max(120,
        (deckBottom - deckTop - (compact ? 5 : 20)) / NeonTheme.selectedScale))
    readonly property real spacing: cardWidth * (wide ? 0.884 : 0.96)
    readonly property real deckCenter: width * (wide ? 0.419 : 0.5)
    readonly property real heroCenter: wide ? height * 0.315 : Math.max((deckTop + deckBottom) / 2,
        deckBottom - cardHeight * NeonTheme.selectedScale / 2 - (compact ? 5 : 13))
    readonly property int selectedDesktops: scene.currentWorkspaces.reduce(
        (sum, ws) => sum + scene.backend.getDesktopsForWorkspace(ws.id).length, 0)

    SceneHeading {
        id: heading
        objectName: "worldsHeading"
        x: view.wide ? view.width * 0.036 : view.margin
        y: view.wide ? view.height * 0.028 : view.compact ? 10 : 17
        width: view.wide ? view.width * 0.51 : view.width - 2 * x
        titleSize: view.wide ? 38 : 0
        detailSize: view.wide ? 16 : 0
        label: ""
        title: "Worlds"
        detail: "Your spatial spaces for work, life, and exploration."
    }

    Row {
        id: worldActionsRow
        x: heading.x
        y: heading.y + heading.height + (view.compact ? 4 : 8)
        spacing: 8
        z: 10

        NeonButton {
            objectName: "createWorldButton"
            text: "+ New World"
            compact: true
            highlighted: true
            accent: NeonTheme.cyan
            onClicked: view.scene.openCreateWorldDialog()
        }

        NeonButton {
            objectName: "editWorldButton"
            text: "Edit World"
            compact: true
            visible: !!view.selectedWorld
            onClicked: view.scene.openEditWorldDialog(view.selectedWorld)
        }

        NeonButton {
            objectName: "archivedItemsButton"
            text: "Archived Items"
            compact: true
            onClicked: view.scene.openArchivedView()
        }
    }

    NeonText {
        objectName: "emptyHierarchyHint"
        anchors.centerIn: parent
        visible: view.scene.currentWorlds.length === 0
        text: "No worlds in this workspace yet."
        color: NeonTheme.muted
    }

    SpatialGlass {
        visible: view.width >= 960 && !!view.selectedWorld
        anchors.right: parent.right
        anchors.rightMargin: view.wide ? view.width * 0.028 : view.margin
        y: view.wide ? view.height * 0.028 : 15
        width: view.width * (view.wide ? 0.330 : 0.355)
        height: view.wide ? Math.min(67, view.height * 0.092) : 47
        radius: view.wide ? 24 : 14
        highContrast: view.scene.highContrast
        Row {
            anchors.centerIn: parent
            spacing: view.width >= 1250 ? 21 : 9
            HierarchyMetric { label: "Worlds"; glyph: "world"; count: view.scene.currentWorlds.length }
            Rectangle { width: 1; height: 34; color: NeonTheme.strongBorder; anchors.verticalCenter: parent.verticalCenter }
            HierarchyMetric { label: "Workspaces"; glyph: "layers"; count: view.scene.backend ? view.scene.backend.workspaces.length : 0 }
            Rectangle { width: 1; height: 34; color: NeonTheme.strongBorder; anchors.verticalCenter: parent.verticalCenter }
            HierarchyMetric { label: "Desktops"; glyph: "tiles"; count: view.scene.backend ? view.scene.backend.desktops.length : 0 }
        }
    }

    Image {
        anchors.horizontalCenter: parent.horizontalCenter
        y: view.deckBottom - height * 0.65
        width: Math.min(parent.width, 1340)
        height: view.compact ? 44 : 103
        source: visible ? "art/orbit.png" : ""
        visible: !view.wide && !!view.selectedWorld && (view.activeLevel || view.opacity > 0)
            && !view.scene.lowEffects && !view.scene.highContrast
    }

    Repeater {
        model: view.wide && view.selectedWorld && !view.scene.lowEffects && !view.scene.highContrast ? [2, 1] : []
        delegate: SpatialGlass {
            required property var modelData
            x: view.deckCenter - width / 2 + modelData * 24
            y: view.heroCenter - height / 2 - modelData * 14
            width: view.cardWidth * 0.94
            height: view.cardHeight
            rotation: 7
            rim: NeonTheme.tint(NeonTheme.mint, 0.66)
            opacity: modelData === 1 ? 0.85 : 0.55
            radius: 25
        }
    }

    readonly property var visibleWorldSlots: {
        const worlds = view.scene.currentWorlds;
        if (!worlds || worlds.length === 0)
            return [];
        if (worlds.length <= 9)
            return worlds.map((w, idx) => ({ world: w, worldIndex: idx }));
        const center = Math.round(view.scene.carouselIndex);
        const radius = view.peripheral + 2;
        const start = Math.max(0, center - radius);
        const end = Math.min(worlds.length - 1, center + radius);
        const slots = [];
        for (let i = start; i <= end; ++i) {
            slots.push({
                world: worlds[i],
                worldIndex: i
            });
        }
        return slots;
    }

    Repeater {
        model: view.visibleWorldSlots
        delegate: Loader {
            id: slot
            required property var modelData
            readonly property int itemWorldIndex: modelData.worldIndex
            readonly property var itemWorld: modelData.world
            readonly property int distance: Math.abs(itemWorldIndex - view.scene.carouselIndex)
            active: (view.activeLevel || view.opacity > 0) && distance <= view.peripheral + 1
            width: view.cardWidth * (!view.wide || distance === 0 ? 1 : distance === 1 ? 0.79 : 0.74)
            height: view.cardHeight
            z: 8 - distance

            sourceComponent: WorldCard {
                id: worldCard
                readonly property int index: slot.itemWorldIndex
                readonly property var modelData: slot.itemWorld
                readonly property int offset: index - view.scene.carouselIndex
                readonly property int depth: Math.abs(offset)
                readonly property real arc: Math.sign(offset) * (depth === 0 ? 0 : 1 + (Math.min(depth, 5) - 1) * (view.wide ? 0.63 : 0.53))
                objectName: "worldCard-" + modelData.id
                world: modelData
                workspaces: view.scene.backend ? view.scene.backend.getWorkspacesForWorld(modelData.id) : []
                desktopCount: workspaces.reduce((sum, ws) => sum + view.scene.backend.getDesktopsForWorkspace(ws.id).length, 0)
                identityIndex: index
                identityTone: index % 2 ? NeonTheme.cyan : NeonTheme.lavender
                customCategories: view.scene.backend.customWorldArtwork
                width: slot.width
                height: view.cardHeight
                referenceStyle: view.wide
                referenceTier: depth
                cameraFocused: offset === 0
                isSelected: view.scene.worldIndex === index
                isFocused: isSelected && view.activeLevel
                motionOff: view.motionOff
                lowEffects: view.scene.lowEffects
                highContrast: view.scene.highContrast
                pointerEnabled: view.activeLevel && !view.scene.navigationBlocked() && depth <= view.peripheral
                visible: opacity > 0

                x: view.deckCenter - width / 2 + arc * view.spacing * (view.wide && offset < 0 ? 0.93 : 1) + view.scene.cameraX
                y: view.heroCenter - height / 2 + Math.min(3, depth) * (view.wide ? 6 : 17) + view.scene.cameraY * 0.15
                z: 8 - depth
                opacity: depth > view.peripheral ? 0 : depth === 0 ? 1 : depth === 1 ? 0.95 : 0.75
                scale: offset === 0 ? NeonTheme.selectedScale : (view.wide ? (depth === 1 ? 0.89 : 0.87) : Math.max(0.67, 1 - depth * NeonTheme.depthScale))
                transform: [
                    Rotation {
                        origin.x: worldCard.width / 2
                        origin.y: worldCard.height / 2
                        axis { x: 0; y: 1; z: 0 }
                        angle: worldCard.motionOff ? 0 : -Math.sign(worldCard.offset)
                            * Math.min(2, worldCard.depth) * NeonTheme.maxTilt * (view.wide ? 0.92 : 1.6)
                        Behavior on angle { NumberAnimation { duration: NeonTheme.ms(worldCard.motionOff, NeonTheme.slideMs); easing.type: NeonTheme.easing } }
                    },
                    Rotation {
                        origin.x: worldCard.width / 2
                        origin.y: worldCard.height / 2
                        angle: view.wide && !worldCard.motionOff ? -Math.sign(worldCard.offset) * 4.5 : 0
                        Behavior on angle { NumberAnimation { duration: NeonTheme.ms(worldCard.motionOff, NeonTheme.slideMs); easing.type: NeonTheme.easing } }
                    }
                ]

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
        x: (view.wide ? view.width * 0.446 : parent.width / 2) - width / 2
        y: view.wide ? view.height * 0.501 : summary.y - height - 8
        spacing: 8
        NeonButton {
            visible: !view.wide
            compact: true
            text: "‹"
            Accessible.name: "Previous world"
            enabled: view.scene.worldIndex > 0
            onClicked: view.scene.moveHorizontal(-1)
        }
        NeonText {
            visible: !view.wide
            anchors.verticalCenter: parent.verticalCenter
            text: (view.scene.worldIndex + 1) + " / " + view.scene.currentWorlds.length
            color: NeonTheme.precision
        }
        NeonButton {
            visible: !view.wide
            compact: true
            text: "›"
            Accessible.name: "Next world"
            enabled: view.scene.worldIndex + 1 < view.scene.currentWorlds.length
            onClicked: view.scene.moveHorizontal(1)
        }
        Repeater {
            model: view.wide ? Math.min(5, view.scene.currentWorlds.length) : 0
            delegate: Rectangle {
                id: dot
                required property int index
                readonly property int target: index + Math.min(Math.max(0, view.scene.worldIndex - 2),
                    Math.max(0, view.scene.currentWorlds.length - 5))
                readonly property bool selected: target === view.scene.worldIndex
                width: selected ? 48 : 14
                height: 24
                color: NeonTheme.transparent
                Accessible.role: Accessible.Button
                Accessible.name: Display.safeText(view.scene.currentWorlds[target]?.name)
                Accessible.onPressAction: choose()
                function choose() { view.scene.moveHorizontal(target - view.scene.worldIndex); view.scene.forceActiveFocus(); }
                Rectangle {
                    anchors.centerIn: parent
                    width: dot.selected ? 47 : 7
                    height: dot.selected ? 8 : 7
                    radius: 6
                    color: dot.selected ? NeonTheme.focus : NeonTheme.strongBorder
                }
                MouseArea { anchors.fill: parent; cursorShape: Qt.PointingHandCursor; onClicked: dot.choose() }
            }
        }
    }

    WorldSummary {
        id: summary
        objectName: "worldSummary"
        scene: view.scene
        selectedWorld: view.selectedWorld
        desktopCount: view.selectedDesktops
        wide: view.wide
        previews: view.previews && view.scene.currentWorkspaces.length > 0 && (view.activeLevel || view.opacity > 0)
        anchors.bottom: parent.bottom
        anchors.bottomMargin: view.wide ? view.height * 0.024 : view.compact ? 8 : 19
        x: view.wide ? view.width * 0.019 : view.margin
        width: view.wide ? view.width * 0.941 : view.width - 2 * x
        height: view.wide ? (previews ? view.height * 0.399 : 112) : previews ? (view.compact ? 174 : 222) : 64
        visible: !!view.selectedWorld
    }
}
