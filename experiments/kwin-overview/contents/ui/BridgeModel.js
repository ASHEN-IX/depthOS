// Pure bounded decoder. No metadata inference, dispatch, eval or native actions.
function decode(raw) {
    if (typeof raw !== "string" || raw.length > 131072) throw new Error("limit");
    const p = JSON.parse(raw);
    if (p.schemaVersion !== 1 || typeof p.instanceId !== "string" || p.instanceId.length > 64 || typeof p.revision !== "string" || p.revision.length > 64 || typeof p.available !== "boolean" || !p.data || typeof p.data !== "object") throw new Error("schema");
    return p;
}
function hierarchyRows(items) {
    if (!Array.isArray(items) || items.length > 64) throw new Error("page");
    return items.map(r => {
        if (!["world", "workspace", "desktop"].includes(r.kind) || typeof r.id !== "string" || r.id.length > 64 || (r.parentId !== null && typeof r.parentId !== "string") || typeof r.name !== "string" || r.name.length > 480) throw new Error("row");
        return {kind:r.kind, id:r.id, parentId:r.parentId, name:r.name.replace(/[\u0000-\u001f\u007f-\u009f\u202a-\u202e\u2066-\u2069]/g, "")};
    });
}
function contextRows(items) {
    if (!Array.isArray(items) || items.length > 64) throw new Error("page");
    return items.map(r => {
        if (!r.window || typeof r.window.epoch !== "string" || typeof r.window.runtime_id !== "string" || r.window.runtime_id.length > 512 || !["confirmed", "suggested", "conflicted", "unknown", "stale"].includes(r.status) || !Array.isArray(r.targets) || r.targets.length > 4096 || r.targets.some(id => typeof id !== "string" || id.length > 64)) throw new Error("association");
        return {nativeId:typeof r.nativeId === "string" && /^[0-9a-f-]{36}$/i.test(r.nativeId) ? r.nativeId.toLowerCase() : "", status:r.status, targets:r.targets};
    });
}
function badges(rows) {
    const result = {}, duplicate = {};
    for (const r of rows) if (r.nativeId) { if (result[r.nativeId]) duplicate[r.nativeId] = true; else result[r.nativeId] = r; }
    for (const id of Object.keys(duplicate)) delete result[id];
    return result;
}
if (typeof module !== "undefined") module.exports = {decode, hierarchyRows, contextRows, badges};
