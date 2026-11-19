pragma ComponentBehavior: Bound
import QtQuick

FocusScope {
    id: scene
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

    // Camera viewport coordinates for spatial panning & zoom
    property real cameraX: 0
    property real cameraY: 0
    property real currentScale: scene.currentLevel === 1 ? 0.90 : (scene.currentLevel === 2 ? 1.0 : 1.08)

    signal closed

    Behavior on cameraX {
        NumberAnimation {
            duration: scene.motionOff ? 0 : 280
            easing.type: Easing.OutCubic
        }
    }
    Behavior on cameraY {
        NumberAnimation {
            duration: scene.motionOff ? 0 : 280
            easing.type: Easing.OutCubic
        }
    }
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

    // Keyboard-First Interaction Actions:
    function moveHorizontal(delta) {
        if (searchPalette.isOpen)
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
    }

    function moveVertical(delta) {
        if (searchPalette.isOpen)
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
        if (searchPalette.isOpen)
            return;

        if (scene.currentLevel === 1) {
            // World -> Workspace Overview (preserve selection if already in world)
            scene.currentLevel = 2;
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
            scene.syncIndices();
        } else if (scene.currentLevel === 2) {
            // Workspace -> Logical Desktop Overview (preserve selection if already in workspace)
            if (scene.currentDesktops.length > 0) {
                scene.currentLevel = 3;
                const existingIdx = scene.currentDesktops.findIndex(d => d.id === scene.backend.selectedDesktopId);
                if (existingIdx >= 0) {
                    scene.desktopIndex = existingIdx;
                } else {
                    scene.desktopIndex = 0;
                    scene.backend.selectDesktop(scene.currentDesktops[0].id);
                }
                scene.syncIndices();
            }
        } else if (scene.currentLevel === 3) {
            // Desktop -> Window Focus cycle
            if (scene.currentWindows.length > 0) {
                scene.windowIndex = (scene.windowIndex + 1) % scene.currentWindows.length;
                scene.backend.selectWindow(scene.currentWindows[scene.windowIndex].id);
            }
        }
    }

    function returnToParent() {
        if (searchPalette.isOpen) {
            searchPalette.isOpen = false;
            scene.forceActiveFocus();
            return;
        }

        if (scene.currentLevel === 3) {
            scene.currentLevel = 2; // Desktop -> Workspace
            scene.syncIndices();
        } else if (scene.currentLevel === 2) {
            scene.currentLevel = 1; // Workspace -> World
            scene.syncIndices();
        } else if (scene.currentLevel === 1) {
            scene.closed();
        }
    }

    function returnHome() {
        if (searchPalette.isOpen)
            return;
        scene.cameraX = 0;
        scene.cameraY = 0;
        scene.syncIndices();
    }

    function openSearch() {
        searchPalette.isOpen = true;
    }

    Keys.onPressed: event => {
        if (searchPalette.isOpen)
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

            StatusBadge {
                anchors.verticalCenter: parent.verticalCenter
                status: scene.backend ? scene.backend.observationHealth : "Unavailable"
                detail: scene.backend ? scene.backend.state : "Offline"
            }
        }
    }

    Item {
        id: canvasViewport
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

                                world: worldDelegate.modelData
                                workspaces: scene.backend ? scene.backend.getWorkspacesForWorld(worldDelegate.modelData.id) : []
                                isSelected: scene.backend && scene.backend.selectedWorldId === worldDelegate.modelData.id
                                isFocused: scene.worldIndex === worldDelegate.index && scene.currentLevel === 1
                                motionOff: scene.motionOff

                                onChosen: {
                                    scene.worldIndex = worldDelegate.index;
                                    scene.backend.selectWorld(worldDelegate.modelData.id);
                                    scene.enterDeeper();
                                }
                            }
                        }
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
                                statusBadge: (wsDelegate.index === 1 ? "Stale" : "Live")
                                isSelected: scene.backend && scene.backend.selectedWorkspaceId === wsDelegate.modelData.id
                                isFocused: scene.workspaceIndex === wsDelegate.index && scene.currentLevel === 2
                                motionOff: scene.motionOff

                                onChosen: {
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

                                window: winDelegate.modelData
                                isSelected: scene.backend && scene.backend.selectedWindowId === winDelegate.modelData.id
                                isFocused: scene.windowIndex === winDelegate.index && scene.currentLevel === 3
                                showTitles: scene.showTitles
                                motionOff: scene.motionOff

                                onChosen: {
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

                        Column {
                            anchors.centerIn: parent
                            spacing: 8

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                textFormat: Text.PlainText
                                text: "No native windows observed in this logical desktop"
                                color: "#94A3B8"
                                font.pixelSize: 14
                                font.weight: Font.Medium
                            }

                            Text {
                                anchors.horizontalCenter: parent.horizontalCenter
                                textFormat: Text.PlainText
                                text: "Windows will appear here when launched or associated"
                                color: "#64748B"
                                font.pixelSize: 11
                            }
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

        Row {
            anchors.left: parent.left
            anchors.leftMargin: 24
            anchors.verticalCenter: parent.verticalCenter
            spacing: 16

            Text {
                textFormat: Text.PlainText
                text: "H/L: Siblings  ·  J/K: Rows/Desktops  ·  Enter: Zoom in  ·  Esc/Backspace: Zoom out  ·  /: Search  ·  Home: Reset  ·  T: Titles  ·  M: Motion"
                color: "#94A3B8"
                font.pixelSize: 11
            }
        }

        Row {
            anchors.right: parent.right
            anchors.rightMargin: 24
            anchors.verticalCenter: parent.verticalCenter
            spacing: 12

            Text {
                textFormat: Text.PlainText
                text: "Read-only native integration · Zero window setters"
                color: "#64748B"
                font.pixelSize: 11
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
            scene.forceActiveFocus();
        }
    }

    Component.onCompleted: {
        syncIndices();
        forceActiveFocus();
    }
}
