// Pure presentation policy shared by production QML and offscreen tests.
function read(object, name) {
    try { const v = object ? object[name] : null; return v === undefined ? null : v; }
    catch (_) { return null; }
}
function displayText(value) {
    return typeof value === "string" ? value.replace(/[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069]/g, " ").slice(0, 4096) : "";
}
function label(object) {
    return displayText(read(object, "desktopFileName")) || displayText(read(object, "resourceClass")) || "Application identifier unavailable";
}
function title(object, showTitles) {
    if (!showTitles || read(object, "excludeFromCapture") !== false) return label(object);
    return displayText(read(object, "caption")) || label(object);
}
function hasDesktop(window, desktop) {
    if (!window || !desktop) return false;
    if (read(window, "onAllDesktops") === true) return true;
    const members = read(window, "desktops");
    if (!members) return false;
    for (let i = 0; i < members.length; ++i) if (members[i] === desktop) return true;
    return false;
}
function hasActivity(window, activity) {
    const members = read(window, "activities");
    if (!members || !activity) return false;
    if (members.length === 0) return true;
    for (let i = 0; i < members.length; ++i) if (members[i] === activity) return true;
    return false;
}
function currentDesktopForOutput(system, output) {
    // Reading this NOTIFY property also invalidates bindings when KWin emits
    // currentDesktopChanged for any output; the invokable alone has no NOTIFY.
    const fallback = read(system, "currentDesktop");
    try {
        if (system && output && typeof system.currentDesktopForScreen === "function")
            return system.currentDesktopForScreen(output) || fallback;
    } catch (_) { /* Output may have disappeared. */ }
    return fallback;
}
function previewReason(window, system, enabled, index, budget) {
    if (!window || !system) return "Native state unavailable";
    if (!enabled) return "Previews disabled";
    if (read(window, "excludeFromCapture") !== false) return "Preview protected or permission unknown";
    if (read(window, "minimized") !== false) return "Minimized or state unknown — metadata only";
    if (!read(window, "output")) return "Output unavailable — metadata only";
    if (!hasDesktop(window, currentDesktopForOutput(system, read(window, "output")))) return "Inactive desktop — metadata only";
    if (!hasActivity(window, read(system, "currentActivity"))) return "Inactive Activity — metadata only";
    if (index >= budget) return "Preview budget reached — metadata only";
    return "";
}
function membership(window) {
    if (read(window, "onAllDesktops") === true) return "All desktops";
    const members = read(window, "desktops");
    if (!members || !members.length) return "Desktop membership unavailable";
    const names = [];
    for (let i = 0; i < members.length; ++i) names.push(displayText(read(members[i], "name")) || displayText(read(members[i], "id")) || "Unnamed desktop");
    return names.join(" · ");
}
