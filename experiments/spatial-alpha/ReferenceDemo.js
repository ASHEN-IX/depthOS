.pragma library

// A self-contained sample catalog for the approved Worlds composition.
const worlds = [
    ["projects", "Projects", [
        ["Development", ["Editor", "Build", "Review"]],
        ["Product Design", ["Canvas", "Prototype", "Library"]],
        ["Operations", ["Monitoring", "Release"]]
    ]],
    ["research", "Research", [
        ["AI Exploration", ["Notebook", "Models", "Evaluation"]],
        ["Papers & Reading", ["Reading List", "Notes"]],
        ["Experiments", ["Lab", "Runs", "Results", "Replication"]],
        ["Analysis", ["Data", "Reports", "Review"]]
    ]],
    ["social", "Social", [
        ["Community", ["Forum", "Events", "Moderation"]],
        ["Connections", ["Messages", "Groups", "Calendar"]]
    ]],
    ["focus", "Focus", [
        ["Writing", ["Draft", "Sources", "Edit", "Publish"]],
        ["Planning", ["Today", "Week", "Roadmap"]],
        ["Studio", ["Sketch", "Build", "Archive"]]
    ]]
];

function catalog() {
    const data = { worlds: [], workspaces: [], desktops: [], windows: [] };
    for (const [category, name, workspaces] of worlds) {
        const worldId = "reference-" + category;
        data.worlds.push({ id: worldId, name: name, artCategory: category });
        workspaces.forEach(([workspace, desktops], index) => {
            const workspaceId = worldId + "-" + index;
            data.workspaces.push({ id: workspaceId, name: workspace, worldId: worldId });
            desktops.forEach((name, index) => {
                data.desktops.push({ id: workspaceId + "-" + index, name: name, workspaceId: workspaceId });
            });
        });
    }
    return data;
}
