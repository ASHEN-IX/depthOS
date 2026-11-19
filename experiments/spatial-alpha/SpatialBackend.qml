import QtQuick
import "MockData.js" as MockData

Item {
    id: backend

    property bool mockMode: true
    property string state: "Demo Mode" // "Connected" | "Disconnected" | "Demo Mode" | "Loading" | "Degraded"
    property string health: "Synchronized"
    property string observationHealth: "Live"

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
        selectedDesktopId = desktopId;
        const d = desktops.find(item => item.id === desktopId);
        if (d && d.workspaceId !== selectedWorkspaceId) {
            selectWorkspace(d.workspaceId);
        }
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
        selectedWindowId = windowId;
        const w = windows.find(item => item.id === windowId);
        if (w && w.desktopId !== selectedDesktopId) {
            selectDesktop(w.desktopId);
        }
    }

    // Lightweight deterministic local search across Worlds, Workspaces, and Desktops
    function search(query) {
        if (!query || query.trim() === "")
            return [];
        const q = query.trim().toLowerCase();
        const results = [];

        // 1. Search Worlds
        for (let i = 0; i < worlds.length; ++i) {
            const w = worlds[i];
            if (w.name.toLowerCase().includes(q) || (w.description && w.description.toLowerCase().includes(q))) {
                results.push({
                    kind: "world",
                    id: w.id,
                    name: w.name,
                    detail: "World · " + (w.description || "Workspace group"),
                    worldId: w.id,
                    workspaceId: "",
                    desktopId: ""
                });
            }
        }

        // 2. Search Workspaces
        for (let i = 0; i < workspaces.length; ++i) {
            const ws = workspaces[i];
            const parentWorld = worlds.find(w => w.id === ws.worldId);
            const parentName = parentWorld ? parentWorld.name : "World";
            if (ws.name.toLowerCase().includes(q) || (ws.description && ws.description.toLowerCase().includes(q)) || (ws.rootDirectory && ws.rootDirectory.toLowerCase().includes(q)) || parentName.toLowerCase().includes(q)) {
                results.push({
                    kind: "workspace",
                    id: ws.id,
                    name: ws.name,
                    detail: "Workspace in " + parentName + (ws.description ? " · " + ws.description : ""),
                    worldId: ws.worldId,
                    workspaceId: ws.id,
                    desktopId: ""
                });
            }
        }

        // 3. Search Desktops
        for (let i = 0; i < desktops.length; ++i) {
            const d = desktops[i];
            const parentWs = workspaces.find(ws => ws.id === d.workspaceId);
            const wsName = parentWs ? parentWs.name : "Workspace";
            const worldId = parentWs ? parentWs.worldId : "";
            if (d.name.toLowerCase().includes(q) || (d.role && d.role.toLowerCase().includes(q)) || wsName.toLowerCase().includes(q)) {
                results.push({
                    kind: "desktop",
                    id: d.id,
                    name: d.name,
                    detail: "Logical Desktop in " + wsName + (d.role ? " (" + d.role + ")" : ""),
                    worldId: worldId,
                    workspaceId: d.workspaceId,
                    desktopId: d.id
                });
            }
        }

        return results;
    }

    // Direct context navigation
    function navigateTo(target) {
        if (!target)
            return;
        if (target.worldId) {
            selectedWorldId = target.worldId;
        }
        if (target.workspaceId) {
            selectedWorkspaceId = target.workspaceId;
        }
        if (target.desktopId) {
            selectedDesktopId = target.desktopId;
        }
        if (target.windowId) {
            selectedWindowId = target.windowId;
        }
    }

    function loadMock() {
        const mock = MockData.getMockData();
        worlds = mock.worlds;
        workspaces = mock.workspaces;
        desktops = mock.desktops;
        windows = mock.windows;
        state = "Demo Mode";
        health = "Synchronized";
        observationHealth = "Live";
        selectedWorldId = "";
        selectedWorkspaceId = "";
        selectedDesktopId = "";
        selectedWindowId = "";

        if (worlds.length > 0) {
            selectWorld(worlds[0].id);
        }
    }

    Component.onCompleted: {
        loadMock();
    }
}
