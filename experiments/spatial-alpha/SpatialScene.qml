pragma ComponentBehavior: Bound
import QtQuick
import "theme"
import "Presentation.js" as Display

Item {
    id: scene
    // Item, not FocusScope: activeFocus must mean "the canvas owns the keys".
    // A FocusScope would stay activeFocus while the search field or a modal
    // inside it is focused, and restoration would look like a focus steal.
    focus: true
    clip: true

    property var backend: null
    property bool reducedMotion: false
    property bool lowEffects: false
    property bool highContrast: false
    property int transitionDirection: 1
    property int pointerSelectionDelay: 430
    property int heldCarouselIndex: 0
    readonly property int carouselIndex: pointerSettle.running ? heldCarouselIndex : worldIndex
    property real carouselWheelDelta: 0
    property var modalReturnFocus: null
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
    readonly property real currentScale: 1

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

    // A first click updates selection immediately. Keep the card at the same
    // coordinates until Qt's pairing interval ends so a Wayland double-click
    // can still be delivered to the same MouseArea.
    Timer {
        id: pointerSettle
        interval: scene.pointerSelectionDelay
        repeat: false
    }

    function selectWorldFromPointer(index) {
        if (!pointerSettle.running)
            scene.heldCarouselIndex = scene.carouselIndex;
        pointerSettle.restart();
        scene.backend.selectWorld(scene.currentWorlds[index].id);
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
            text += "  ›  " + Display.safeText(scene.backend.currentWorld.name);
        }
        if (scene.currentLevel >= 2 && scene.backend.currentWorkspace) {
            text += "  ›  " + Display.safeText(scene.backend.currentWorkspace.name);
        }
        if (scene.currentLevel >= 3 && scene.backend.currentDesktop) {
            text += "  ›  " + Display.safeText(scene.backend.currentDesktop.name);
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

    onCurrentWorldsChanged: syncIndices()
    onCurrentWorkspacesChanged: syncIndices()
    onCurrentDesktopsChanged: syncIndices()
    onCurrentWindowsChanged: syncIndices()

    function onboardingOpen() {
        return !!(scene.backend && scene.backend.onboardingState && scene.backend.onboardingState !== "hidden");
    }

    function navigationBlocked() {
        return searchPalette.isOpen || controlCenter.isOpen || scene.onboardingOpen();
    }

    function setLevel(next) {
        if (next === scene.currentLevel)
            return false;
        if (next < 1 || next > 3)
            return false;
        scene.transitionDirection = next > scene.currentLevel ? 1 : -1;
        pointerSettle.stop();
        scene.carouselWheelDelta = 0;
        scene.currentLevel = next;
        scene.cameraX = 0;
        scene.cameraY = 0;
        scene.levelTransitions += 1;
        scene.syncIndices();
        return true;
    }

    function resetInteractionClocks() {
        scene.lastLevelStepMs = -1e15;
        scene.lastPointerDrillMs = -1e15;
        scene.pinchActive = false;
        scene.pinchLatched = false;
        pointerSettle.stop();
        scene.carouselWheelDelta = 0;
    }

    function activeColumn() {
        if (scene.currentLevel === 2)
            return levelBView.panContent;
        if (scene.currentLevel === 3)
            return levelCView.panContent;
        return null;
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
        if (!item || item.height + 56 <= canvasViewport.height)
            return 0;
        return item.height + 56 - canvasViewport.height;
    }

    function clampPan(value, limit) {
        return Math.max(-limit, Math.min(limit, value));
    }

    function panBy(dx, dy) {
        scene.cameraX = scene.clampPan(scene.cameraX + dx, scene.panLimitX());
        scene.cameraY = scene.clampPan(scene.cameraY + dy, scene.panLimitY());
    }

    // angle* are in eighths of a degree (120 = one notch). Each event uses
    // either its wheel steps or its pixel deltas.
    function handleWheel(angleX, angleY, pixelX, pixelY, modifiers, phase, deviceType) {
        if (scene.navigationBlocked())
            return;
        scene.wheelEventsHandled += 1;
        const mod = modifiers || 0;
        const angle = Math.max(Math.abs(angleX), Math.abs(angleY));
        const pixel = Math.max(Math.abs(pixelX), Math.abs(pixelY));
        // Wayland can deliver an unphased wheel notch with both angular units
        // and a pixel fallback (120 and 15 in the virtual KWin session).
        const preferSteps = scene.currentLevel === 1 && phase === Qt.NoScrollPhase && angle > 0
            && (deviceType === PointerDevice.Mouse || pixel === Math.round(angle / 8));
        const usePixels = pixel !== 0 && !preferSteps;
        let dx = usePixels ? pixelX : (angleX / 120) * scene.wheelStepPx;
        let dy = usePixels ? pixelY : (angleY / 120) * scene.wheelStepPx;
        if (mod & Qt.ControlModifier) {
            const delta = usePixels ? (pixelY !== 0 ? pixelY : pixelX) : (angleY !== 0 ? angleY : angleX);
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
        if (scene.currentLevel === 1) {
            const delta = Math.abs(dx) > Math.abs(dy) ? dx : dy;
            if (scene.carouselWheelDelta * delta < 0)
                scene.carouselWheelDelta = 0;
            scene.carouselWheelDelta += delta;
            if (Math.abs(scene.carouselWheelDelta) >= scene.wheelStepPx * 0.7) {
                const direction = scene.carouselWheelDelta > 0 ? -1 : 1;
                scene.carouselWheelDelta = 0;
                scene.moveHorizontal(direction);
            }
            return;
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
            if (scene.currentLevel < 3) {
                scene.focusCardAt(scene.pinchX, scene.pinchY);
                scene.drillDown(false);
            }
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
        let targetIndex = -1;
        let targetZ = -Infinity;
        for (let i = 0; i < list.length; ++i) {
            const card = scene.findNamed(prefix + list[i].id);
            if (!card || !card.visible || card.available === false || card.z < targetZ)
                continue;
            const p = card.mapFromItem(scene, x, y);
            if (p.x < 0 || p.y < 0 || p.x > card.width || p.y > card.height)
                continue;
            targetIndex = i;
            targetZ = card.z;
        }
        const i = targetIndex;
        if (i >= 0) {
            if (scene.currentLevel === 1) {
                pointerSettle.stop();
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
        if (scene.currentLevel === 1)
            return;
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
        if (controlCenter.isOpen) {
            if (!scene.itemInside(item, controlCenter))
                controlCenter.forceActiveFocus();
            return;
        }
        if (scene.isTextEntry(item) || (scene.itemInside(item, topBar) && item.enabled))
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
            pointerSettle.stop();
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
            scene.moveHorizontal(delta * levelBView.columns);
        } else if (scene.currentLevel === 3) {
            // In Level C, vertical moves between logical desktops
            if (scene.currentDesktops.length > 0) {
                scene.selectDesktopIndex(scene.desktopIndex + delta);
            }
        }
    }

    function selectDesktopIndex(index) {
        if (scene.navigationBlocked() || scene.currentDesktops.length === 0)
            return;
        scene.desktopIndex = Math.max(0, Math.min(scene.currentDesktops.length - 1, index));
        scene.backend.selectDesktop(scene.currentDesktops[scene.desktopIndex].id);
        scene.cameraX = 0;
        scene.cameraY = 0;
    }

    property real lastPointerDrillMs: 0

    function enterDeeper() {
        scene.drillDown(true);
    }

    function pointerDrillDown() {
        const now = Date.now();
        if (now - scene.lastPointerDrillMs < scene.levelStepCooldownMs)
            return;
        scene.lastPointerDrillMs = now;
        scene.drillDown(true);
    }

    function drillDown(cycleWindows) {
        if (scene.navigationBlocked() || !scene.backend)
            return;

        if (scene.currentLevel === 1) {
            if (!scene.backend.currentWorld || scene.backend.currentWorld.unavailable)
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
            if (!scene.backend.currentWorkspace || scene.backend.currentWorkspace.unavailable || scene.currentDesktops.length === 0)
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
        pointerSettle.stop();
        scene.carouselWheelDelta = 0;
    }

    function rememberModalFocus() {
        const win = scene.Window.window;
        scene.modalReturnFocus = win ? win.activeFocusItem : scene;
    }

    function restoreModalFocus() {
        const item = scene.modalReturnFocus;
        scene.modalReturnFocus = null;
        if (scene.onboardingOpen())
            return;
        if (item && item.visible && item.enabled && scene.itemInside(item, scene))
            item.forceActiveFocus();
        else
            scene.forceActiveFocus();
    }

    function canNavigateLevel(level) {
        if (!backend)
            return level === 1;
        if (level === 2)
            return !!backend.currentWorld && !backend.currentWorld.unavailable;
        if (level === 3)
            return canNavigateLevel(2) && !!backend.currentWorkspace && !backend.currentWorkspace.unavailable
                && currentDesktops.length > 0;
        return level === 1;
    }

    function openLevel(level) {
        if (navigationBlocked() || !canNavigateLevel(level))
            return;
        while (currentLevel < level) {
            const before = currentLevel;
            drillDown(false);
            if (currentLevel === before)
                break;
        }
        if (currentLevel > level)
            navigateBreadcrumb(level);
        forceActiveFocus();
    }

    function navigateBreadcrumb(level) {
        if (scene.navigationBlocked() || level > scene.currentLevel)
            return;
        scene.setLevel(level);
        scene.forceActiveFocus();
    }

    function openControlCenter() {
        if (scene.navigationBlocked())
            return;
        scene.rememberModalFocus();
        controlCenter.isOpen = true;
    }

    function openSearch() {
        if (scene.navigationBlocked())
            return;
        scene.rememberModalFocus();
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
        } else if (event.key === Qt.Key_F2) {
            scene.openControlCenter();
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
                scene.selectDesktopIndex(scene.desktopIndex - 1);
            }
            event.accepted = true;
        } else if (event.key === Qt.Key_BracketRight) {
            if (scene.currentDesktops.length > 0) {
                scene.selectDesktopIndex(scene.desktopIndex + 1);
            }
            event.accepted = true;
        }
    }

    SceneAtmosphere {
        anchors.fill: parent
        lowEffects: scene.lowEffects
        highContrast: scene.highContrast
        level: scene.currentLevel
        wide: scene.wideChrome
        drift: scene.motionOff ? 0 : scene.cameraX * 0.13 - scene.carouselIndex * 12
        Behavior on drift {
            NumberAnimation {
                duration: NeonTheme.ms(scene.motionOff, NeonTheme.slideMs)
                easing.type: NeonTheme.easing
            }
        }
    }

    readonly property bool wideChrome: width >= 1160 && height >= 640

    Loader {
        id: topBar
        width: parent.width
        height: scene.wideChrome ? Math.round(scene.height * 0.09) : NeonTheme.hudHeight
        sourceComponent: scene.wideChrome ? wideHeader : compactHeader
        enabled: !scene.navigationBlocked()
        z: 10
    }
    Component {
        id: wideHeader
        ReferenceHeader { scene: scene }
    }
    Component {
        id: compactHeader
        TelemetryHeader { scene: scene }
    }

    SpatialRail {
        id: navigationRail
        scene: scene
        visible: scene.wideChrome
        enabled: !scene.navigationBlocked()
        x: Math.round(scene.width * 0.018)
        y: Math.round(scene.height * 0.134)
        width: Math.round(scene.width * 0.078)
        height: Math.round(scene.height * 0.607)
        z: 3
    }

    Item {
        id: canvasViewport
        objectName: "canvasViewport"
        anchors.fill: parent
        anchors.topMargin: scene.wideChrome && scene.currentLevel === 1 ? Math.round(scene.height * 0.09) : NeonTheme.hudHeight
        anchors.leftMargin: navigationRail.visible ? navigationRail.x + navigationRail.width : 0
        anchors.bottomMargin: scene.wideChrome && scene.currentLevel === 1 ? Math.round(scene.height * 0.104) : NeonTheme.footerHeight
        enabled: !scene.navigationBlocked()
        clip: true

        WorldsView {
            id: levelAView
            objectName: "worldsView"
            anchors.fill: parent
            scene: scene
            activeLevel: scene.currentLevel === 1
            motionOff: scene.motionOff
            direction: scene.transitionDirection
            z: activeLevel ? 1 : 0
        }

        WorkspacesView {
            id: levelBView
            objectName: "workspacesView"
            anchors.fill: parent
            scene: scene
            activeLevel: scene.currentLevel === 2
            motionOff: scene.motionOff
            direction: scene.transitionDirection
            z: activeLevel ? 1 : 0
        }

        DesktopsView {
            id: levelCView
            objectName: "desktopsView"
            anchors.fill: parent
            scene: scene
            activeLevel: scene.currentLevel === 3
            motionOff: scene.motionOff
            direction: scene.transitionDirection
            z: activeLevel ? 1 : 0
        }
    }

    NeonLabel {
        objectName: scene.wideChrome ? "demoWatermark" : ""
        visible: scene.wideChrome && !!scene.backend?.isDemo
        anchors.right: parent.right
        anchors.rightMargin: 38
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 15
        text: "DEMO DATA"
        color: NeonTheme.precision
        z: 9
    }

    Rectangle {
        id: bottomBar
        visible: !scene.wideChrome
        anchors.bottom: parent.bottom
        width: parent.width
        height: NeonTheme.footerHeight
        color: scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.shell
        z: 10

        Rectangle {
            width: parent.width
            height: 1
            color: scene.highContrast ? NeonTheme.muted : NeonTheme.border
        }

        NeonText {
            anchors.left: parent.left
            anchors.leftMargin: 18
            anchors.right: demoMark.visible ? demoMark.left : readOnlyLabel.left
            anchors.rightMargin: 12
            anchors.verticalCenter: parent.verticalCenter
            elide: Text.ElideRight
            text: scene.currentLevel === 1
                ? "← / →  Select   ·   ↵  Open   ·   Wheel  Browse   ·   /  Search   ·   F2  Control center"
                : scene.currentLevel === 2
                    ? "H J K L  Select   ·   ↵  Open   ·   Esc  Worlds   ·   Ctrl+wheel  Depth   ·   /  Search"
                    : "↑ / ↓  Desktops   ·   ← / →  Windows   ·   Esc  Workspaces   ·   T  Titles"
            color: NeonTheme.precision
            font.pixelSize: NeonTheme.detailSize
        }

        NeonLabel {
            id: demoMark
            objectName: scene.wideChrome ? "" : "demoWatermark"
            anchors.right: readOnlyLabel.left
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            visible: !!(scene.backend && scene.backend.isDemo) && scene.width >= 660
            text: "DEMO DATA"
            color: NeonTheme.lavender
        }

        NeonText {
            id: readOnlyLabel
            anchors.right: parent.right
            anchors.rightMargin: 18
            anchors.verticalCenter: parent.verticalCenter
            text: "Read-only"
            color: NeonTheme.emerald
            font.pixelSize: NeonTheme.detailSize
            font.weight: Font.DemiBold
        }
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
                Qt.callLater(() => {
                    if (onboardingPanel.visible)
                        onboardingPanel.forceActiveFocus();
                });
            else if (!scene.navigationBlocked())
                scene.forceActiveFocus();
        }

        Rectangle {
            anchors.fill: parent
            color: NeonTheme.scrim
            MouseArea {
                anchors.fill: parent
            }
        }

        Rectangle {
            width: Math.min(640, parent.width - 80)
            height: onboardingColumn.implicitHeight + 48
            anchors.centerIn: parent
            radius: 16
            color: scene.highContrast ? NeonTheme.contrastPanel : NeonTheme.panel
            border.color: NeonTheme.cyan
            border.width: 1

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
                    color: NeonTheme.text
                    font.pixelSize: 16
                    font.weight: Font.DemiBold
                }

                Text {
                    width: parent.width
                    visible: onboardingPanel.detailText !== ""
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    text: onboardingPanel.detailText
                    color: NeonTheme.danger
                    font.pixelSize: 13
                }

                Flow {
                    width: parent.width
                    spacing: 10

                    NeonButton {
                        objectName: "createWorkspaceButton"
                        text: "Create my workspace"
                        visible: {
                            const state = scene.backend ? scene.backend.onboardingState : "";
                            return state === "offer" || state === "creating" || state === "error";
                        }
                        enabled: !scene.backend || scene.backend.onboardingState !== "creating"
                        highlighted: true
                        onClicked: scene.backend.requestCreateWorkspace()
                    }

                    NeonButton {
                        objectName: "continueDemoButton"
                        text: "Continue in Demo Mode"
                        visible: {
                            const state = scene.backend ? scene.backend.onboardingState : "";
                            return state === "offer" || state === "error" || state === "blocked";
                        }
                        onClicked: scene.backend.requestContinueDemo()
                    }

                    NeonButton {
                        objectName: "retryButton"
                        text: "Try again"
                        visible: {
                            const state = scene.backend ? scene.backend.onboardingState : "";
                            return state === "error" || state === "blocked";
                        }
                        onClicked: scene.backend.requestRetry()
                    }

                    NeonButton {
                        objectName: "openWorkspaceButton"
                        text: "Open workspace"
                        visible: !!(scene.backend && scene.backend.onboardingState === "success")
                        highlighted: true
                        accent: NeonTheme.emerald
                        onClicked: scene.backend.dismissOnboarding()
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
        highContrast: scene.highContrast

        onNavigate: item => {
            if (!scene.backend || !scene.backend.navigateTo(item))
                return;
            searchPalette.isOpen = false;
            const depth = item.kind === "world" ? 1 : item.kind === "workspace" ? 2 : 3;
            scene.setLevel(depth);
            scene.returnHome();
            Qt.callLater(scene.revealFocusedCard);
            scene.modalReturnFocus = null;
            scene.forceActiveFocus();
        }

        onClosed: scene.restoreModalFocus()
    }

    ControlCenter {
        id: controlCenter
        objectName: "controlCenter"
        anchors.fill: parent
        z: 95
        scene: scene
        onDismissed: {
            controlCenter.isOpen = false;
            scene.restoreModalFocus();
        }
    }

    // One wheel area consumes both axes in the same event. Button presses
    // continue to the cards, including the native double-click sequence.
    MouseArea {
        anchors.fill: parent
        z: 11
        acceptedButtons: Qt.NoButton
        scrollGestureEnabled: true
        enabled: !searchPalette.isOpen && !controlCenter.isOpen && !scene.onboardingOpen()
        onWheel: event => {
            scene.handleWheel(event.angleDelta.x, event.angleDelta.y, event.pixelDelta.x, event.pixelDelta.y,
                event.modifiers, event.phase, event.device ? event.device.type : PointerDevice.Unknown);
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
