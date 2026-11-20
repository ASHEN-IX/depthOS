pragma ComponentBehavior: Bound
import QtQuick

Item {
    id: scene
    // Item, not FocusScope: activeFocus must mean "the canvas owns the keys".
    // A FocusScope would stay activeFocus while the search field or a modal
    // inside it is focused, and restoration would look like a focus steal.
    focus: true
    clip: true

    property var backend: null
    property bool reducedMotion: false
    readonly property bool motionOff: scene.reducedMotion

    // Three navigable depth levels:
    // 1 = Level A: World Overview
    // 2 = Level B: Workspace Overview
    // 3 = Level C: Logical Desktop Overview
    property int currentLevel: 1
    readonly property string currentLevelName: {
        switch (scene.currentLevel) {
        case 1:
            return "Level A: World Overview";
        case 2:
            return "Level B: Workspace Overview";
        case 3:
            return "Level C: Logical Desktop Overview";
        default:
            return "Level " + scene.currentLevel;
        }
    }

    // Indices for keyboard navigation
    property int worldIndex: 0
    property int workspaceIndex: 0
    property int desktopIndex: 0
    property int windowIndex: 0

    // Privacy redaction toggle for window titles
    property bool showTitles: false

    // Camera viewport coordinates for spatial panning & zoom.
    // Wheel and keyboard pans write these directly. A Behavior here would
    // queue one wheel notch behind an animation and double-count with the
    // next event, so camera motion is immediate. Level changes still animate
    // opacity and scale, and those honor reduced motion.
    property real cameraX: 0
    property real cameraY: 0
    property real currentScale: scene.currentLevel === 1 ? 0.90 : (scene.currentLevel === 2 ? 1.0 : 1.08)

    // One counted hierarchy transition per Enter, double-click, ctrl-wheel
    // step, or latched pinch. Window cycling at level C does not count.
    property int levelTransitions: 0

    // 120 angle-delta units (one traditional notch, or eight 15-unit hi-res
    // steps) move the camera by this many pixels. pixelDelta is used as-is
    // and is never added on top of angleDelta from the same event.
    readonly property real wheelStepPx: 96
    readonly property int levelStepCooldownMs: 320
    property real lastLevelStepMs: -1e15
    property int wheelEventsHandled: 0

    property bool pinchActive: false
    property bool pinchLatched: false
    property real pinchX: 0
    property real pinchY: 0

    signal closed

    Behavior on currentScale {
        NumberAnimation {
            duration: scene.motionOff ? 0 : 320
            easing.type: Easing.OutCubic
        }
    }

    // Helper collections from backend
    readonly property var currentWorlds: scene.backend ? scene.backend.worlds : []
    readonly property var currentWorkspaces: (scene.backend && scene.backend.selectedWorldId) ? scene.backend.getWorkspacesForWorld(scene.backend.selectedWorldId) : []
    readonly property var currentDesktops: (scene.backend && scene.backend.selectedWorkspaceId) ? scene.backend.getDesktopsForWorkspace(scene.backend.selectedWorkspaceId) : []
    readonly property var currentWindows: (scene.backend && scene.backend.selectedDesktopId) ? scene.backend.getWindowsForDesktop(scene.backend.selectedDesktopId) : []

    // Breadcrumb text
    readonly property string breadcrumbText: {
        if (!scene.backend)
            return "DepthOS Spatial Workspace";
        let text = "DepthOS";
        if (scene.backend.currentWorld) {
            text += "  ›  " + scene.backend.currentWorld.name;
        }
        if (scene.currentLevel >= 2 && scene.backend.currentWorkspace) {
            text += "  ›  " + scene.backend.currentWorkspace.name;
        }
        if (scene.currentLevel >= 3 && scene.backend.currentDesktop) {
            text += "  ›  " + scene.backend.currentDesktop.name;
        }
        return text;
    }

    function syncIndices() {
        if (!scene.backend)
            return;
        if (scene.backend.worlds && scene.backend.worlds.length > 0) {
            scene.worldIndex = Math.max(0, scene.backend.worlds.findIndex(w => w.id === scene.backend.selectedWorldId));
        } else {
            scene.worldIndex = 0;
        }
        if (scene.currentWorkspaces.length > 0) {
            scene.workspaceIndex = Math.max(0, scene.currentWorkspaces.findIndex(ws => ws.id === scene.backend.selectedWorkspaceId));
        } else {
            scene.workspaceIndex = 0;
        }
        if (scene.currentDesktops.length > 0) {
            scene.desktopIndex = Math.max(0, scene.currentDesktops.findIndex(d => d.id === scene.backend.selectedDesktopId));
        } else {
            scene.desktopIndex = 0;
        }
        if (scene.currentWindows.length > 0) {
            scene.windowIndex = Math.max(0, scene.currentWindows.findIndex(w => w.id === scene.backend.selectedWindowId));
        } else {
            scene.windowIndex = 0;
        }
    }

    Connections {
        target: scene.backend
        function onSelectedWorldIdChanged() {
            scene.syncIndices();
        }
        function onSelectedWorkspaceIdChanged() {
            scene.syncIndices();
        }
        function onSelectedDesktopIdChanged() {
            scene.syncIndices();
        }
        function onSelectedWindowIdChanged() {
            scene.syncIndices();
        }
    }

    function onboardingOpen() {
        return !!(scene.backend && scene.backend.onboardingState && scene.backend.onboardingState !== "hidden");
    }

    function navigationBlocked() {
        return searchPalette.isOpen || scene.onboardingOpen();
    }

    function setLevel(next) {
        if (next === scene.currentLevel)
            return false;
        scene.currentLevel = next;
        scene.cameraX = 0;
        scene.cameraY = 0;
        scene.levelTransitions += 1;
        scene.syncIndices();
        return true;
    }

    function resetInteractionClocks() {
        scene.lastLevelStepMs = -1e15;
        scene.pinchActive = false;
        scene.pinchLatched = false;
    }

    function activeColumn() {
        if (scene.currentLevel === 2)
            return levelBColumn;
        if (scene.currentLevel === 3)
            return levelCColumn;
        return levelAColumn;
    }

    function panLimitFor(length, viewport) {
        if (!length || !viewport)
            return 0;
        return Math.max(0, (length * scene.currentScale - viewport) / 2);
    }

    function panLimitX() {
        const item = scene.activeColumn();
        return item ? scene.panLimitFor(item.width, canvasViewport.width) : 0;
    }

    function panLimitY() {
        const item = scene.activeColumn();
        return item ? scene.panLimitFor(item.height, canvasViewport.height) : 0;
    }

    function clampPan(value, limit) {
        return Math.max(-limit, Math.min(limit, value));
    }

    function panBy(dx, dy) {
        scene.cameraX = scene.clampPan(scene.cameraX + dx, scene.panLimitX());
        scene.cameraY = scene.clampPan(scene.cameraY + dy, scene.panLimitY());
    }

    // angle* are in eighths of a degree (120 = one notch). pixel* are device
    // pixels from the same Qt wheel event. When pixelDelta is non-zero it
    // already describes that event, so angleDelta is ignored.
    function handleWheel(angleX, angleY, pixelX, pixelY, modifiers) {
        if (scene.navigationBlocked())
            return;
        scene.wheelEventsHandled += 1;
        const mod = modifiers || 0;
        const hasPixel = pixelX !== 0 || pixelY !== 0;
        let dx = hasPixel ? pixelX : (angleX / 120) * scene.wheelStepPx;
        let dy = hasPixel ? pixelY : (angleY / 120) * scene.wheelStepPx;
        if (mod & Qt.ControlModifier) {
            const delta = hasPixel ? (pixelY !== 0 ? pixelY : pixelX) : (angleY !== 0 ? angleY : angleX);
            if (delta > 0)
                scene.stepHierarchy(1);
            else if (delta < 0)
                scene.stepHierarchy(-1);
            return;
        }
        if (mod & Qt.ShiftModifier) {
            dx += dy;
            dy = 0;
        }
        scene.panBy(dx, dy);
    }

    function stepHierarchy(direction) {
        if (scene.navigationBlocked())
            return;
        const now = Date.now();
        if (now - scene.lastLevelStepMs < scene.levelStepCooldownMs)
            return;
        scene.lastLevelStepMs = now;
        if (direction > 0)
            scene.drillDown(false);
        else
            scene.drillUp(false);
    }

    // Synthetic pinch entry points. A real PinchHandler calls the same
    // functions when the compositor delivers a gesture. Delivery on this
    // KWin session is not assumed; see docs/phase-4-1-1-ux-corrections.md.
    function pinchBegin(x, y) {
        scene.pinchActive = false;
        scene.pinchLatched = false;
        if (scene.navigationBlocked())
            return;
        scene.pinchActive = true;
        scene.pinchX = x;
        scene.pinchY = y;
    }

    function pinchUpdate(scale) {
        if (!scene.pinchActive || scene.pinchLatched)
            return;
        if (scale >= 1.25) {
            scene.pinchLatched = true;
            scene.focusCardAt(scene.pinchX, scene.pinchY);
            scene.drillDown(false);
        } else if (scale <= 0.75) {
            scene.pinchLatched = true;
            scene.focusCardAt(scene.pinchX, scene.pinchY);
            scene.drillUp(false);
        }
    }

    function pinchEnd(cancelled) {
        if (cancelled)
            scene.pinchLatched = true;
        scene.pinchActive = false;
    }

    function cancelGestures() {
        scene.pinchActive = false;
        scene.pinchLatched = true;
    }

    function findNamed(name) {
        function walk(node) {
            if (!node)
                return null;
            if (node.objectName === name)
                return node;
            const kids = node.children;
            for (let i = 0; i < kids.length; ++i) {
                const found = walk(kids[i]);
                if (found)
                    return found;
            }
            return null;
        }
        return walk(scene);
    }

    function focusCardAt(x, y) {
        if (!scene.backend)
            return;
        let prefix = "windowCard-";
        let list = scene.currentWindows;
        if (scene.currentLevel === 1) {
            prefix = "worldCard-";
            list = scene.currentWorlds;
        } else if (scene.currentLevel === 2) {
            prefix = "workspaceCard-";
            list = scene.currentWorkspaces;
        }
        for (let i = 0; i < list.length; ++i) {
            const card = scene.findNamed(prefix + list[i].id);
            if (!card || card.available === false)
                continue;
            const p = card.mapFromItem(scene, x, y);
            if (p.x < 0 || p.y < 0 || p.x > card.width || p.y > card.height)
                continue;
            if (scene.currentLevel === 1) {
                scene.worldIndex = i;
                scene.backend.selectWorld(list[i].id);
            } else if (scene.currentLevel === 2) {
                scene.workspaceIndex = i;
                scene.backend.selectWorkspace(list[i].id);
            } else {
                scene.windowIndex = i;
                scene.backend.selectWindow(list[i].id);
            }
            return;
        }
    }

    function focusedCard() {
        if (!scene.backend)
            return null;
        if (scene.currentLevel === 1 && scene.currentWorlds.length > 0)
            return scene.findNamed("worldCard-" + scene.currentWorlds[scene.worldIndex].id);
        if (scene.currentLevel === 2 && scene.currentWorkspaces.length > 0)
            return scene.findNamed("workspaceCard-" + scene.currentWorkspaces[scene.workspaceIndex].id);
        if (scene.currentLevel === 3 && scene.currentWindows.length > 0)
            return scene.findNamed("windowCard-" + scene.currentWindows[scene.windowIndex].id);
        return null;
    }

    function revealFocusedCard() {
        const card = scene.focusedCard();
        if (!card || card.width <= 0)
            return;
        const p = card.mapToItem(canvasViewport, 0, 0);
        let dx = 0;
        let dy = 0;
        if (p.x < 0)
            dx = -p.x;
        else if (p.x + card.width > canvasViewport.width)
            dx = canvasViewport.width - (p.x + card.width);
        if (p.y < 0)
            dy = -p.y;
        else if (p.y + card.height > canvasViewport.height)
            dy = canvasViewport.height - (p.y + card.height);
        if (dx !== 0 || dy !== 0)
            scene.panBy(dx, dy);
    }

    function isTextEntry(item) {
        return !!(item && item.hasOwnProperty("inputMethodComposing") && item.hasOwnProperty("cursorPosition"));
    }

    function itemInside(item, ancestor) {
        let node = item;
        while (node) {
            if (node === ancestor)
                return true;
            node = node.parent;
        }
        return false;
    }

    // Called after the window becomes active again. Restores canvas keys when
    // focus has drifted to a non-editing item, and leaves search, rename, and
    // modal controls alone.
    function restoreFocusAfterActivation() {
        const win = scene.Window.window;
        const item = win ? win.activeFocusItem : null;
        if (scene.onboardingOpen()) {
            if (!scene.itemInside(item, onboardingPanel))
                onboardingPanel.forceActiveFocus();
            return;
        }
        if (searchPalette.isOpen) {
            if (!scene.itemInside(item, searchPalette))
                searchPalette.focusInput();
            return;
        }
        if (scene.isTextEntry(item))
            return;
        if (!scene.activeFocus)
            scene.forceActiveFocus();
    }

    function workspaceSourceBadge() {
        const b = scene.backend;
        if (!b)
            return "Unavailable";
        if (b.sourceKind === "demo" || b.mockMode)
            return "Demo";
        if (!b.observerLive) {
            if (b.observationHealth === "Stale" || b.observationHealth === "Reconnecting")
                return b.observationHealth;
            return "Unavailable";
        }
        const health = b.observationHealth || "Unavailable";
        if (health === "Live" || health === "Stale" || health === "Reconnecting" || health === "Unavailable" || health === "Unknown")
            return health;
        return "Unavailable";
    }

    // Keyboard-First Interaction Actions:
    function moveHorizontal(delta) {
        if (scene.navigationBlocked())
            return;

        if (scene.currentLevel === 1) {
            // Level A: Worlds
            if (scene.currentWorlds.length > 0) {
                scene.worldIndex = Math.max(0, Math.min(scene.currentWorlds.length - 1, scene.worldIndex + delta));
                scene.backend.selectWorld(scene.currentWorlds[scene.worldIndex].id);
            }
        } else if (scene.currentLevel === 2) {
            // Level B: Workspaces
            if (scene.currentWorkspaces.length > 0) {
                scene.workspaceIndex = Math.max(0, Math.min(scene.currentWorkspaces.length - 1, scene.workspaceIndex + delta));
                scene.backend.selectWorkspace(scene.currentWorkspaces[scene.workspaceIndex].id);
            }
        } else if (scene.currentLevel === 3) {
            // Level C: Windows
            if (scene.currentWindows.length > 0) {
                scene.windowIndex = Math.max(0, Math.min(scene.currentWindows.length - 1, scene.windowIndex + delta));
                scene.backend.selectWindow(scene.currentWindows[scene.windowIndex].id);
            }
        }
        scene.revealFocusedCard();
    }

    function moveVertical(delta) {
        if (scene.navigationBlocked())
            return;

        if (scene.currentLevel === 1) {
            scene.moveHorizontal(delta * 2);
        } else if (scene.currentLevel === 2) {
            const cols = Math.min(3, Math.max(1, scene.currentWorkspaces.length));
            scene.moveHorizontal(delta * cols);
        } else if (scene.currentLevel === 3) {
            // In Level C, vertical moves between logical desktops
            if (scene.currentDesktops.length > 0) {
                scene.desktopIndex = Math.max(0, Math.min(scene.currentDesktops.length - 1, scene.desktopIndex + delta));
                scene.backend.selectDesktop(scene.currentDesktops[scene.desktopIndex].id);
            }
        }
    }

    function enterDeeper() {
        scene.drillDown(true);
    }

    function drillDown(cycleWindows) {
        if (scene.navigationBlocked() || !scene.backend)
            return;

        if (scene.currentLevel === 1) {
            if (scene.currentWorlds.length === 0)
                return;
            scene.setLevel(2);
            if (scene.currentWorkspaces.length > 0) {
                const existingIdx = scene.currentWorkspaces.findIndex(ws => ws.id === scene.backend.selectedWorkspaceId);
                if (existingIdx >= 0) {
                    scene.workspaceIndex = existingIdx;
                } else {
                    scene.workspaceIndex = 0;
                    scene.backend.selectWorkspace(scene.currentWorkspaces[0].id);
                }
            } else {
                scene.workspaceIndex = 0;
            }
        } else if (scene.currentLevel === 2) {
            if (scene.currentDesktops.length === 0)
                return;
            scene.setLevel(3);
            const existingIdx = scene.currentDesktops.findIndex(d => d.id === scene.backend.selectedDesktopId);
            if (existingIdx >= 0) {
                scene.desktopIndex = existingIdx;
            } else {
                scene.desktopIndex = 0;
                scene.backend.selectDesktop(scene.currentDesktops[0].id);
            }
        } else if (scene.currentLevel === 3 && cycleWindows) {
            if (scene.currentWindows.length > 0) {
                scene.windowIndex = (scene.windowIndex + 1) % scene.currentWindows.length;
                scene.backend.selectWindow(scene.currentWindows[scene.windowIndex].id);
            }
        }
    }

    function drillUp(allowClose) {
        if (scene.navigationBlocked())
            return;
        if (scene.currentLevel > 1)
            scene.setLevel(scene.currentLevel - 1);
        else if (allowClose)
            scene.closed();
    }

    function returnToParent() {
        if (searchPalette.isOpen) {
            searchPalette.dismiss();
            return;
        }
        scene.drillUp(true);
    }

    function returnHome() {
        if (scene.navigationBlocked())
            return;
        scene.cameraX = 0;
        scene.cameraY = 0;
        scene.syncIndices();
    }

    function openSearch() {
        if (scene.onboardingOpen())
            return;
        searchPalette.isOpen = true;
    }

    Keys.onPressed: event => {
        if (scene.navigationBlocked())
            return;

        if (event.key === Qt.Key_Escape) {
            scene.returnToParent();
            event.accepted = true;
        } else if (event.key === Qt.Key_Backspace) {
            scene.returnToParent();
            event.accepted = true;
        } else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
            scene.enterDeeper();
            event.accepted = true;
        } else if (event.key === Qt.Key_H || event.key === Qt.Key_Left) {
            scene.moveHorizontal(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_L || event.key === Qt.Key_Right) {
            scene.moveHorizontal(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_J || event.key === Qt.Key_Down) {
            scene.moveVertical(1);
            event.accepted = true;
        } else if (event.key === Qt.Key_K || event.key === Qt.Key_Up) {
            scene.moveVertical(-1);
            event.accepted = true;
        } else if (event.key === Qt.Key_Slash) {
            scene.openSearch();
            event.accepted = true;
        } else if (event.key === Qt.Key_Home) {
            scene.returnHome();
            event.accepted = true;
        } else if (event.key === Qt.Key_T) {
            scene.showTitles = !scene.showTitles;
            event.accepted = true;
        } else if (event.key === Qt.Key_M) {
            scene.reducedMotion = !scene.reducedMotion;
            event.accepted = true;
        } else if (event.key === Qt.Key_BracketLeft) {
            if (scene.currentDesktops.length > 0) {
                scene.desktopIndex = Math.max(0, scene.desktopIndex - 1);
                scene.backend.selectDesktop(scene.currentDesktops[scene.desktopIndex].id);
            }
            event.accepted = true;
        } else if (event.key === Qt.Key_BracketRight) {
            if (scene.currentDesktops.length > 0) {
                scene.desktopIndex = Math.min(scene.currentDesktops.length - 1, scene.desktopIndex + 1);
                scene.backend.selectDesktop(scene.currentDesktops[scene.desktopIndex].id);
            }
            event.accepted = true;
        }
    }

    Rectangle {
        anchors.fill: parent
        color: "#080C14"

        Item {
            anchors.fill: parent
            x: scene.cameraX * 0.15
            y: scene.cameraY * 0.15

            Repeater {
                model: Math.ceil(scene.width / 120) + 2
                delegate: Rectangle {
                    required property int index
                    x: (index - 1) * 120
                    y: 0
                    width: 1
                    height: scene.height
                    color: "#1E293B18"
                }
            }

            Repeater {
                model: Math.ceil(scene.height / 120) + 2
                delegate: Rectangle {
                    required property int index
                    x: 0
                    y: (index - 1) * 120
                    width: scene.width
                    height: 1
                    color: "#1E293B18"
                }
            }
        }
    }

    Rectangle {
        id: topBar
        x: 0
        y: 0
        width: parent.width
        height: 60
        color: "#0B1020EE"
        border.color: "#1E293B"
        border.width: 1
        z: 10

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 24
            anchors.verticalCenter: parent.verticalCenter
            spacing: 12

            Text {
                textFormat: Text.PlainText
                text: "DEPTHOS"
                color: "#818CF8"
                font.pixelSize: 15
                font.weight: Font.Black
                anchors.verticalCenter: parent.verticalCenter
            }

            Rectangle {
                width: 1
                height: 18
                color: "#334155"
                anchors.verticalCenter: parent.verticalCenter
            }

            Text {
                textFormat: Text.PlainText
                text: scene.breadcrumbText
                color: "#E2E8F0"
                font.pixelSize: 13
                font.weight: Font.Medium
                anchors.verticalCenter: parent.verticalCenter
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 24
            anchors.verticalCenter: parent.verticalCenter
            spacing: 12

            Rectangle {
                height: 26
                width: levelLabel.implicitWidth + 16
                radius: 13
                color: "#1E293B"
                border.color: "#38BDF8"
                border.width: 1

                Text {
                    id: levelLabel
                    anchors.centerIn: parent
                    textFormat: Text.PlainText
                    text: scene.currentLevelName
                    color: "#38BDF8"
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }
            }

            Rectangle {
                height: 26
                width: searchBtnText.implicitWidth + 18
                radius: 13
                color: "#1E293B"
                border.color: "#475569"
                border.width: 1

                Row {
                    anchors.centerIn: parent
                    spacing: 5
                    Text {
                        text: "🔍"
                        font.pixelSize: 11
                    }
                    Text {
                        id: searchBtnText
                        textFormat: Text.PlainText
                        text: "Search (/)"
                        color: "#CBD5E1"
                        font.pixelSize: 11
                    }
                }

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: scene.openSearch()
                }
            }

            Column {
                anchors.verticalCenter: parent.verticalCenter
                spacing: 0

                Text {
                    objectName: "sourceLabel"
                    textFormat: Text.PlainText
                    text: scene.backend ? scene.backend.sourceLabel : "No Source"
                    color: scene.backend && scene.backend.isDemo ? "#C4B5FD" : "#E2E8F0"
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }

                Text {
                    objectName: "observerLabel"
                    textFormat: Text.PlainText
                    text: scene.backend ? scene.backend.observerLabel : "Native Observer Unavailable"
                    color: scene.backend && scene.backend.observerLive ? "#22C55E" : "#94A3B8"
                    font.pixelSize: 10
                }
            }

            StatusBadge {
                anchors.verticalCenter: parent.verticalCenter
                status: scene.backend ? scene.backend.observationHealth : "Unavailable"
                detail: scene.backend ? (scene.backend.observerDetail || scene.backend.state) : "Offline"
            }
        }
    }

    Item {
        id: canvasViewport
        objectName: "canvasViewport"
        anchors.fill: parent
        anchors.topMargin: 70
        anchors.bottomMargin: 50
        clip: true

        Item {
            id: canvasContainer
            anchors.fill: parent
            transform: [
                Scale {
                    origin.x: canvasContainer.width / 2
                    origin.y: canvasContainer.height / 2
                    xScale: scene.currentScale
                    yScale: scene.currentScale
                },
                Translate {
                    x: scene.cameraX
                    y: scene.cameraY
                }
            ]

            Item {
                id: levelAView
                anchors.fill: parent
                visible: scene.currentLevel === 1
                opacity: scene.currentLevel === 1 ? 1 : 0
                Behavior on opacity {
                    NumberAnimation {
                        duration: scene.motionOff ? 0 : 220
                        easing.type: Easing.OutCubic
                    }
                }

                Column {
                    id: levelAColumn
                    anchors.centerIn: parent
                    spacing: 24

                    Text {
                        textFormat: Text.PlainText
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "World Overview — Spatial Topography"
                        color: "#F8FAFC"
                        font.pixelSize: 22
                        font.weight: Font.DemiBold
                    }

                    Grid {
                        id: worldsGrid
                        anchors.horizontalCenter: parent.horizontalCenter
                        columns: 2
                        spacing: 24

                        Repeater {
                            model: scene.currentWorlds
                            delegate: WorldCard {
                                id: worldDelegate
                                required property var modelData
                                required property int index
                                objectName: "worldCard-" + worldDelegate.modelData.id

                                world: worldDelegate.modelData
                                workspaces: scene.backend ? scene.backend.getWorkspacesForWorld(worldDelegate.modelData.id) : []
                                isSelected: scene.backend && scene.backend.selectedWorldId === worldDelegate.modelData.id
                                isFocused: scene.worldIndex === worldDelegate.index && scene.currentLevel === 1
                                motionOff: scene.motionOff

                                onChosen: {
                                    if (!worldDelegate.available)
                                        return;
                                    scene.worldIndex = worldDelegate.index;
                                    scene.backend.selectWorld(worldDelegate.modelData.id);
                                }
                                onActivated: {
                                    if (!worldDelegate.available)
                                        return;
                                    scene.worldIndex = worldDelegate.index;
                                    scene.backend.selectWorld(worldDelegate.modelData.id);
                                    scene.enterDeeper();
                                }
                            }
                        }
                    }

                    Text {
                        objectName: "emptyHierarchyHint"
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: scene.currentWorlds.length === 0
                        textFormat: Text.PlainText
                        text: "No worlds in this workspace yet."
                        color: "#94A3B8"
                        font.pixelSize: 15
                    }
                }
            }

            Item {
                id: levelBView
                anchors.fill: parent
                visible: scene.currentLevel === 2
                opacity: scene.currentLevel === 2 ? 1 : 0
                Behavior on opacity {
                    NumberAnimation {
                        duration: scene.motionOff ? 0 : 220
                        easing.type: Easing.OutCubic
                    }
                }

                Column {
                    id: levelBColumn
                    anchors.centerIn: parent
                    spacing: 20

                    Row {
                        anchors.horizontalCenter: parent.horizontalCenter
                        spacing: 12

                        Text {
                            textFormat: Text.PlainText
                            text: (scene.backend && scene.backend.currentWorld) ? scene.backend.currentWorld.icon : "🌐"
                            font.pixelSize: 24
                        }

                        Text {
                            textFormat: Text.PlainText
                            text: ((scene.backend && scene.backend.currentWorld) ? scene.backend.currentWorld.name : "World") + " — Workspaces"
                            color: "#F8FAFC"
                            font.pixelSize: 22
                            font.weight: Font.DemiBold
                        }

                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            height: 22
                            width: returnWorldHint.implicitWidth + 12
                            radius: 4
                            color: "#1E293B"
                            border.color: "#334155"
                            border.width: 1

                            Text {
                                id: returnWorldHint
                                anchors.centerIn: parent
                                textFormat: Text.PlainText
                                text: "Esc: Return to Worlds"
                                color: "#94A3B8"
                                font.pixelSize: 10
                            }
                        }
                    }

                    Grid {
                        id: workspacesGrid
                        anchors.horizontalCenter: parent.horizontalCenter
                        columns: Math.min(3, Math.max(1, scene.currentWorkspaces.length))
                        spacing: 20

                        Repeater {
                            model: scene.currentWorkspaces
                            delegate: WorkspaceCard {
                                id: wsDelegate
                                required property var modelData
                                required property int index
                                objectName: "workspaceCard-" + wsDelegate.modelData.id

                                workspace: wsDelegate.modelData
                                desktops: scene.backend ? scene.backend.getDesktopsForWorkspace(wsDelegate.modelData.id) : []
                                observedWindowCount: {
                                    if (!scene.backend)
                                        return 0;
                                    const ds = scene.backend.getDesktopsForWorkspace(wsDelegate.modelData.id);
                                    let count = 0;
                                    for (let i = 0; i < ds.length; ++i) {
                                        count += scene.backend.getWindowsForDesktop(ds[i].id).length;
                                    }
                                    return count;
                                }
                                statusBadge: scene.workspaceSourceBadge()
                                isSelected: scene.backend && scene.backend.selectedWorkspaceId === wsDelegate.modelData.id
                                isFocused: scene.workspaceIndex === wsDelegate.index && scene.currentLevel === 2
                                motionOff: scene.motionOff

                                onChosen: {
                                    if (!wsDelegate.available)
                                        return;
                                    scene.workspaceIndex = wsDelegate.index;
                                    scene.backend.selectWorkspace(wsDelegate.modelData.id);
                                }
                                onActivated: {
                                    if (!wsDelegate.available)
                                        return;
                                    scene.workspaceIndex = wsDelegate.index;
                                    scene.backend.selectWorkspace(wsDelegate.modelData.id);
                                    scene.enterDeeper();
                                }
                            }
                        }
                    }

                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: scene.currentWorkspaces.length === 0
                        textFormat: Text.PlainText
                        text: "No workspaces created in this world yet."
                        color: "#94A3B8"
                        font.pixelSize: 15
                    }
                }
            }

            Item {
                id: levelCView
                anchors.fill: parent
                visible: scene.currentLevel === 3
                opacity: scene.currentLevel === 3 ? 1 : 0
                Behavior on opacity {
                    NumberAnimation {
                        duration: scene.motionOff ? 0 : 220
                        easing.type: Easing.OutCubic
                    }
                }

                Column {
                    id: levelCColumn
                    anchors.centerIn: parent
                    width: parent.width - 64
                    spacing: 16

                    Row {
                        anchors.horizontalCenter: parent.horizontalCenter
                        spacing: 10

                        Repeater {
                            model: scene.currentDesktops
                            delegate: Rectangle {
                                id: deskTabDelegate
                                required property var modelData
                                required property int index

                                height: 36
                                width: Math.max(120, tabLabel.implicitWidth + 32)
                                radius: 8
                                color: (scene.backend && scene.backend.selectedDesktopId === deskTabDelegate.modelData.id) ? "#312E81" : "#1E293B"
                                border.width: (scene.backend && scene.backend.selectedDesktopId === deskTabDelegate.modelData.id) ? 2 : 1
                                border.color: (scene.backend && scene.backend.selectedDesktopId === deskTabDelegate.modelData.id) ? "#818CF8" : "#334155"

                                Row {
                                    anchors.centerIn: parent
                                    spacing: 8

                                    Text {
                                        id: tabLabel
                                        textFormat: Text.PlainText
                                        text: deskTabDelegate.modelData.name
                                        color: (scene.backend && scene.backend.selectedDesktopId === deskTabDelegate.modelData.id) ? "#F8FAFC" : "#94A3B8"
                                        font.pixelSize: 12
                                        font.weight: Font.Medium
                                    }

                                    Rectangle {
                                        height: 16
                                        width: winCountLabel.implicitWidth + 8
                                        radius: 8
                                        color: "#0F172A"

                                        Text {
                                            id: winCountLabel
                                            anchors.centerIn: parent
                                            textFormat: Text.PlainText
                                            text: scene.backend ? String(scene.backend.getWindowsForDesktop(deskTabDelegate.modelData.id).length) : "0"
                                            color: "#A5B4FC"
                                            font.pixelSize: 9
                                        }
                                    }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: {
                                        scene.desktopIndex = deskTabDelegate.index;
                                        scene.backend.selectDesktop(deskTabDelegate.modelData.id);
                                    }
                                }
                            }
                        }

                        Rectangle {
                            anchors.verticalCenter: parent.verticalCenter
                            height: 26
                            width: escWsHint.implicitWidth + 12
                            radius: 4
                            color: "#1E293B"
                            border.color: "#334155"
                            border.width: 1

                            Text {
                                id: escWsHint
                                anchors.centerIn: parent
                                textFormat: Text.PlainText
                                text: "Esc: Return to Workspaces"
                                color: "#94A3B8"
                                font.pixelSize: 10
                            }
                        }
                    }

                    Grid {
                        id: windowsGrid
                        anchors.horizontalCenter: parent.horizontalCenter
                        columns: Math.min(3, Math.max(1, scene.currentWindows.length))
                        spacing: 20

                        Repeater {
                            model: scene.currentWindows
                            delegate: WindowCard {
                                id: winDelegate
                                required property var modelData
                                required property int index
                                objectName: "windowCard-" + winDelegate.modelData.id

                                window: winDelegate.modelData
                                isSelected: scene.backend && scene.backend.selectedWindowId === winDelegate.modelData.id
                                isFocused: scene.windowIndex === winDelegate.index && scene.currentLevel === 3
                                showTitles: scene.showTitles
                                motionOff: scene.motionOff

                                onChosen: {
                                    if (!winDelegate.available)
                                        return;
                                    scene.windowIndex = winDelegate.index;
                                    scene.backend.selectWindow(winDelegate.modelData.id);
                                }
                                onActivated: {
                                    if (!winDelegate.available)
                                        return;
                                    scene.windowIndex = winDelegate.index;
                                    scene.backend.selectWindow(winDelegate.modelData.id);
                                }
                            }
                        }
                    }

                    Rectangle {
                        anchors.horizontalCenter: parent.horizontalCenter
                        visible: scene.currentWindows.length === 0
                        width: 480
                        height: 140
                        radius: 12
                        color: "#131C2E"
                        border.color: "#26354D"
                        border.width: 1

                        Text {
                            id: emptyWindowsHint
                            objectName: "emptyWindowsHint"
                            anchors.centerIn: parent
                            width: parent.width - 48
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                            textFormat: Text.PlainText
                            text: (scene.backend && scene.backend.observerLive) ? "No native windows are associated with this logical desktop." : "Native observer unavailable. This desktop cannot list windows until a read-only observer is connected."
                            color: "#94A3B8"
                            font.pixelSize: 14
                            font.weight: Font.Medium
                        }
                    }
                }
            }
        }
    }

    Rectangle {
        id: bottomBar
        x: 0
        y: parent.height - 42
        width: parent.width
        height: 42
        color: "#0B1020EE"
        border.color: "#1E293B"
        border.width: 1
        z: 10

        Text {
            id: hintText
            anchors.left: parent.left
            anchors.leftMargin: 24
            anchors.right: readOnlyLabel.left
            anchors.rightMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            textFormat: Text.PlainText
            elide: Text.ElideRight
            text: "H/L: Siblings · J/K: Rows · Click: select · Double-click or Enter: open · Esc: back · Wheel: pan · Ctrl+wheel: depth · /: Search · Home · T: Titles · M: Motion"
            color: "#94A3B8"
            font.pixelSize: 11
        }

        Text {
            id: readOnlyLabel
            anchors.right: parent.right
            anchors.rightMargin: 24
            anchors.verticalCenter: parent.verticalCenter
            textFormat: Text.PlainText
            text: "Read-only · no window setters"
            color: "#64748B"
            font.pixelSize: 11
        }
    }

    Text {
        objectName: "demoWatermark"
        z: 8
        anchors.centerIn: parent
        rotation: -18
        textFormat: Text.PlainText
        text: "DEMO DATA"
        color: "#F8FAFC"
        opacity: 0.16
        font.pixelSize: 92
        font.weight: Font.Black
        visible: !!(scene.backend && scene.backend.isDemo)
        enabled: false
    }

    FocusScope {
        id: onboardingPanel
        objectName: "onboardingPanel"
        anchors.fill: parent
        z: 90
        visible: scene.onboardingOpen()
        enabled: visible

        readonly property string messageText: {
            const b = scene.backend;
            if (!b)
                return "";
            if (b.onboardingState === "offer") {
                const lead = b.onboardingMessage ? b.onboardingMessage : "No workspace database was found.";
                return lead + " " + b.databasePath + " Create a new persistent workspace or continue in Demo Mode. Nothing is written until you choose.";
            }
            if (b.onboardingState === "creating")
                return b.onboardingMessage ? b.onboardingMessage : "Creating the workspace…";
            if (b.onboardingState === "blocked")
                return b.onboardingMessage ? b.onboardingMessage : "This database cannot be opened.";
            return b.onboardingMessage ? b.onboardingMessage : "";
        }
        readonly property string detailText: scene.backend ? (scene.backend.onboardingDetail || "") : ""

        onVisibleChanged: {
            if (visible)
                Qt.callLater(onboardingPanel.forceActiveFocus);
        }

        Rectangle {
            anchors.fill: parent
            color: "#000000B0"
            MouseArea {
                anchors.fill: parent
            }
        }

        Rectangle {
            width: Math.min(640, parent.width - 80)
            height: onboardingColumn.implicitHeight + 48
            anchors.centerIn: parent
            radius: 16
            color: "#0F172A"
            border.color: "#38BDF8"
            border.width: 1.5

            Column {
                id: onboardingColumn
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 24
                spacing: 14

                Text {
                    width: parent.width
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    text: onboardingPanel.messageText
                    color: "#F8FAFC"
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    width: parent.width
                    visible: onboardingPanel.detailText !== ""
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    text: onboardingPanel.detailText
                    color: "#FCA5A5"
                    font.pixelSize: 13
                }

                Flow {
                    width: parent.width
                    spacing: 10

                    Rectangle {
                        objectName: "createWorkspaceButton"
                        property string text: "Create my workspace"
                        visible: {
                            const s = scene.backend ? scene.backend.onboardingState : "";
                            return s === "offer" || s === "creating" || s === "error";
                        }
                        implicitWidth: createLabel.implicitWidth + 28
                        implicitHeight: 36
                        radius: 8
                        color: "#1E293B"
                        border.color: "#38BDF8"
                        Text {
                            id: createLabel
                            anchors.centerIn: parent
                            textFormat: Text.PlainText
                            text: parent.text
                            color: "#E2E8F0"
                            font.pixelSize: 13
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: scene.backend.requestCreateWorkspace()
                        }
                    }

                    Rectangle {
                        objectName: "continueDemoButton"
                        property string text: "Continue in Demo Mode"
                        visible: {
                            const s = scene.backend ? scene.backend.onboardingState : "";
                            return s === "offer" || s === "error" || s === "blocked";
                        }
                        implicitWidth: demoLabel.implicitWidth + 28
                        implicitHeight: 36
                        radius: 8
                        color: "#1E293B"
                        border.color: "#6D28D9"
                        Text {
                            id: demoLabel
                            anchors.centerIn: parent
                            textFormat: Text.PlainText
                            text: parent.text
                            color: "#E2E8F0"
                            font.pixelSize: 13
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: scene.backend.requestContinueDemo()
                        }
                    }

                    Rectangle {
                        objectName: "retryButton"
                        property string text: "Try again"
                        visible: {
                            const s = scene.backend ? scene.backend.onboardingState : "";
                            return s === "error" || s === "blocked";
                        }
                        implicitWidth: retryLabel.implicitWidth + 28
                        implicitHeight: 36
                        radius: 8
                        color: "#1E293B"
                        border.color: "#475569"
                        Text {
                            id: retryLabel
                            anchors.centerIn: parent
                            textFormat: Text.PlainText
                            text: parent.text
                            color: "#E2E8F0"
                            font.pixelSize: 13
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: scene.backend.requestRetry()
                        }
                    }

                    Rectangle {
                        objectName: "openWorkspaceButton"
                        property string text: "Open workspace"
                        visible: scene.backend && scene.backend.onboardingState === "success"
                        implicitWidth: openLabel.implicitWidth + 28
                        implicitHeight: 36
                        radius: 8
                        color: "#1E293B"
                        border.color: "#22C55E"
                        Text {
                            id: openLabel
                            anchors.centerIn: parent
                            textFormat: Text.PlainText
                            text: parent.text
                            color: "#E2E8F0"
                            font.pixelSize: 13
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: scene.backend.dismissOnboarding()
                        }
                    }
                }
            }
        }
    }

    SearchPalette {
        id: searchPalette
        objectName: "searchPalette"
        anchors.fill: parent
        z: 100
        backend: scene.backend
        motionOff: scene.motionOff

        onNavigate: item => {
            searchPalette.isOpen = false;
            scene.backend.navigateTo(item);

            if (item.kind === "world") {
                scene.currentLevel = 1;
            } else if (item.kind === "workspace") {
                scene.currentLevel = 2;
            } else if (item.kind === "desktop") {
                scene.currentLevel = 3;
            }
            scene.syncIndices();
            scene.forceActiveFocus();
        }

        onClosed: {
            searchPalette.isOpen = false;
            if (!scene.onboardingOpen())
                scene.forceActiveFocus();
        }
    }

    // One handler, so a single Qt wheel event cannot be applied twice.
    WheelHandler {
        id: wheelHandler
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
        onWheel: event => {
            scene.handleWheel(event.angleDelta.x, event.angleDelta.y, event.pixelDelta.x, event.pixelDelta.y, event.modifiers);
        }
    }

    // Active only if KWin delivers a touchpad pinch to an ordinary Qt Quick
    // window. Unattended runs cannot prove that; the probe is tools/depthos-gesture-probe.
    PinchHandler {
        id: pinchHandler
        target: null
        acceptedDevices: PointerDevice.TouchPad
        onActiveChanged: {
            if (active) {
                scene.pinchBegin(pinchHandler.centroid.position.x, pinchHandler.centroid.position.y);
            } else {
                scene.pinchEnd(false);
            }
        }
        onActiveScaleChanged: {
            if (pinchHandler.active)
                scene.pinchUpdate(pinchHandler.activeScale);
        }
    }

    Connections {
        target: scene.Window.window
        function onActiveChanged() {
            const win = scene.Window.window;
            if (!win)
                return;
            if (!win.active) {
                scene.cancelGestures();
                return;
            }
            Qt.callLater(scene.restoreFocusAfterActivation);
        }
    }

    Component.onCompleted: {
        syncIndices();
        forceActiveFocus();
    }
}
