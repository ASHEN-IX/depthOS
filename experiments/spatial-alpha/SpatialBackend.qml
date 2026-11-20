import QtQuick
import "MockData.js" as MockData
import "ReferenceDemo.js" as ReferenceDemo
import "Presentation.js" as Display

Item {
    id: backend

    // True ONLY while built-in sample data is loaded (see loadMock()).
    property bool mockMode: false
    property string state: "Detecting source" // "Connected" | "Database Mode" | "Demo Mode" | "Detecting source" | "Degraded"
    property string health: "Unknown"
    // Observation channel status: "Live" | "Stale" | "Reconnecting" | "Unavailable".
    // It never defaults to Live; only a connected controller reporting a
    // Synchronized observer may set it.
    property string observationHealth: "Unavailable"

    // --- Source provenance ---------------------------------------------------
    // Where the displayed hierarchy comes from. Exactly one of:
    //   "none" | "demo" | "sqlite" | "controller"
    // A controller connection says nothing about native observation: that is
    // the separate, independent observerLive flag.
    property string sourceKind: "none"
    property var customWorldArtwork: []
    property string sourceDetail: ""
    property string sourceLoadedAt: ""
    readonly property string controllerStatus: state === "Connecting" || state === "Connected" ? state : "Unavailable"
    property bool observerLive: false
    property string observerDetail: ""
    readonly property bool isDemo: sourceKind === "demo"
    readonly property string sourceLabel: {
        switch (sourceKind) {
        case "demo":
            return "Demo";
        case "sqlite":
            return "Local SQLite";
        case "controller":
            return state === "Unavailable" ? "Last controller snapshot" : "Connected Controller";
        default:
            return "No Source";
        }
    }
    readonly property string observerLabel: observerLive ? "Native Observer Live" : "Native Observer Unavailable"

    // --- First-run / onboarding state ---------------------------------------
    // "hidden" | "offer" | "blocked" | "creating" | "success" | "error"
    property string databasePath: ""
    property string databaseState: ""
    property string onboardingState: "hidden"
    property string onboardingMessage: ""
    property string onboardingDetail: ""

    signal createWorkspaceRequested
    signal continueDemoRequested
    signal retryRequested

    function requestCreateWorkspace() {
        if (onboardingState !== "offer" && onboardingState !== "error")
            return;
        onboardingState = "creating";
        createWorkspaceRequested();
    }

    function requestContinueDemo() {
        if (onboardingState === "creating")
            return;
        loadMock();
        onboardingState = "hidden";
        continueDemoRequested();
    }

    function requestRetry() {
        if (onboardingState === "creating")
            return;
        retryRequested();
    }

    function dismissOnboarding() {
        onboardingState = "hidden";
    }

    property var worlds: []
    property var workspaces: []
    property var desktops: []
    property var windows: []

    property string selectedWorldId: ""
    property string selectedWorkspaceId: ""
    property string selectedDesktopId: ""
    property string selectedWindowId: ""

    readonly property var currentWorld: worlds.find(w => w.id === selectedWorldId) || null
    readonly property var currentWorkspace: workspaces.find(ws => ws.id === selectedWorkspaceId) || null
    readonly property var currentDesktop: desktops.find(d => d.id === selectedDesktopId) || null
    readonly property var currentWindow: windows.find(w => w.id === selectedWindowId) || null

    function getWorkspacesForWorld(worldId) {
        return workspaces.filter(ws => ws.worldId === worldId);
    }

    function getDesktopsForWorkspace(workspaceId) {
        return desktops.filter(d => d.workspaceId === workspaceId);
    }

    function getWindowsForDesktop(desktopId) {
        return windows.filter(w => w.desktopId === desktopId);
    }

    function selectWorld(worldId) {
        selectedWorldId = worldId;
        const subWorkspaces = getWorkspacesForWorld(worldId);
        if (subWorkspaces.length > 0) {
            // Pick first workspace in world if current selection does not belong to this world
            if (!subWorkspaces.some(ws => ws.id === selectedWorkspaceId)) {
                selectWorkspace(subWorkspaces[0].id);
            }
        } else {
            selectedWorkspaceId = "";
            selectedDesktopId = "";
            selectedWindowId = "";
        }
    }

    function selectWorkspace(workspaceId) {
        selectedWorkspaceId = workspaceId;
        const ws = workspaces.find(w => w.id === workspaceId);
        if (ws && ws.worldId !== selectedWorldId) {
            selectedWorldId = ws.worldId;
        }
        const subDesktops = getDesktopsForWorkspace(workspaceId);
        if (subDesktops.length > 0) {
            if (!subDesktops.some(d => d.id === selectedDesktopId)) {
                selectDesktop(subDesktops[0].id);
            }
        } else {
            selectedDesktopId = "";
            selectedWindowId = "";
        }
    }

    function selectDesktop(desktopId) {
        const d = desktops.find(item => item.id === desktopId);
        if (d && d.workspaceId !== selectedWorkspaceId)
            selectWorkspace(d.workspaceId);
        selectedDesktopId = desktopId;
        const subWindows = getWindowsForDesktop(desktopId);
        if (subWindows.length > 0) {
            if (!subWindows.some(w => w.id === selectedWindowId)) {
                selectedWindowId = subWindows[0].id;
            }
        } else {
            selectedWindowId = "";
        }
    }

    function selectWindow(windowId) {
        const w = windows.find(item => item.id === windowId);
        if (w && w.desktopId !== selectedDesktopId)
            selectDesktop(w.desktopId);
        selectedWindowId = windowId;
    }

    // Bind the local index to hierarchy revisions, never to window metadata.
    // Parent maps avoid repeatedly walking the hierarchy on each keystroke.
    readonly property var searchIndex: {
        const worldNames = new Map();
        const workspacePaths = new Map();
        const indexed = [];

        for (const world of worlds) {
            const name = Display.safeText(world.name);
            worldNames.set(world.id, name);
            if (!world.unavailable)
                indexed.push({
                    kind: "world", id: world.id, name: name,
                    detail: "World · " + Display.safeText(world.description),
                    breadcrumb: "Worlds",
                    worldId: world.id, workspaceId: "", desktopId: "",
                    terms: [name, Display.safeText(world.description)]
                });
        }
        for (const workspace of workspaces) {
            const name = Display.safeText(workspace.name);
            const parent = worldNames.get(workspace.worldId);
            if (!parent)
                continue;
            workspacePaths.set(workspace.id, { name: name, world: parent, worldId: workspace.worldId });
            if (!workspace.unavailable)
                indexed.push({
                    kind: "workspace", id: workspace.id, name: name,
                    detail: "Workspace in " + parent,
                    breadcrumb: parent,
                    worldId: workspace.worldId, workspaceId: workspace.id, desktopId: "",
                    terms: [name, Display.safeText(workspace.description), Display.safeText(workspace.rootDirectory), parent]
                });
        }
        for (const desktop of desktops) {
            const path = workspacePaths.get(desktop.workspaceId);
            if (!path || desktop.unavailable)
                continue;
            const name = Display.safeText(desktop.name);
            indexed.push({
                kind: "desktop", id: desktop.id, name: name,
                detail: "Logical Desktop in " + path.name,
                breadcrumb: path.world + " › " + path.name,
                worldId: path.worldId, workspaceId: desktop.workspaceId, desktopId: desktop.id,
                terms: [name, Display.safeText(desktop.role), path.name, path.world]
            });
        }
        return indexed.map(item => Object.assign(item, { terms: item.terms.map(term => term.toLowerCase()) }));
    }

    function search(query) {
        let q = Display.safeText(query).toLowerCase();
        if (!q)
            return [];
        const kind = ({ "@": "world", "#": "workspace", "!": "desktop" })[q[0]];
        if (kind)
            q = q.slice(1).trim();
        const results = [];
        for (const item of searchIndex) {
            if ((!kind || item.kind === kind) && item.terms.some(term => term.includes(q))) {
                results.push(item);
                if (results.length === 150)
                    break;
            }
        }
        return results;
    }

    // Resolve IDs against the current hierarchy before applying a result.
    function navigateTo(target) {
        if (!target)
            return false;
        if (target.kind === "world" && worlds.some(w => w.id === target.id && !w.unavailable)) {
            selectWorld(target.id);
            return true;
        }
        if (target.kind === "workspace" && workspaces.some(w => w.id === target.id && !w.unavailable)) {
            selectWorkspace(target.id);
            return true;
        }
        if (target.kind === "desktop" && desktops.some(d => d.id === target.id && !d.unavailable)) {
            selectDesktop(target.id);
            return true;
        }
        return false;
    }

    function loadMock() {
        applyDemo(MockData.getMockData(), "");
    }

    function loadReferenceDemo() {
        applyDemo(ReferenceDemo.catalog(), "reference-research");
    }

    function applyDemo(mock, initialWorldId) {
        worlds = mock.worlds;
        workspaces = mock.workspaces;
        desktops = mock.desktops;
        windows = mock.windows;
        mockMode = true;
        sourceKind = "demo";
        sourceLoadedAt = "";
        sourceDetail = "Built-in sample data, not your workspace";
        observerLive = false;
        observerDetail = "Demo data is simulated; nothing is observed from KWin";
        state = "Demo Mode";
        health = "Simulated";
        observationHealth = "Unavailable";
        selectedWorldId = "";
        selectedWorkspaceId = "";
        selectedDesktopId = "";
        selectedWindowId = "";

        if (worlds.length > 0) {
            selectWorld(initialWorldId || worlds[0].id);
        }
    }
}
