/* POC-02A. Reads KWin state only. No window/workspace setters or shortcuts.
 * KWin 6.8.1 JavaScript scripts get their own QJSEngine and a public QTimer.
 * The exports are solely for deterministic offscreen tests of this same code.
 */
var depthosRuntime;
(function () {
    "use strict";
    const SCHEMA = 1;
    const SERVICE = "org.depthos.StateInspector";
    const PATH = "/org/depthos/StateInspector";
    const INTERFACE = SERVICE + "1";

    function read(object, name) {
        try {
            const value = object === null || object === undefined ? null : object[name];
            return value === undefined ? null : value;
        } catch (_) { return null; }
    }
    function list(value) {
        if (value === null || typeof value.length !== "number") return null;
        const result = [];
        for (let i = 0; i < value.length; ++i) result.push(value[i]);
        return result;
    }
    function text(value, limit) {
        return typeof value === "string" ? value.slice(0, limit || 4096) : null;
    }
    function boolean(value) { return typeof value === "boolean" ? value : null; }
    function number(value) { return typeof value === "number" && isFinite(value) ? value : null; }
    function geometry(value) {
        if (value === null) return null;
        const result = {};
        for (const key of ["x", "y", "width", "height"]) {
            result[key] = number(read(value, key));
            if (result[key] === null) return null;
        }
        return result;
    }
    function strings(value) {
        const values = list(value);
        return values === null ? null : values.map(v => text(v, 512)).filter(v => v !== null).sort();
    }
    function uuid(value) {
        if (value === null) return null;
        try {
            const match = String(value).match(/^\{?([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})\}?$/i);
            return match ? match[1].toLowerCase() : null;
        } catch (_) { return null; }
    }
    function equal(a, b) { return JSON.stringify(a) === JSON.stringify(b); }
    function clone(value) { return JSON.parse(JSON.stringify(value)); }
    function connect(object, signalName, callback, bag, missing) {
        try {
            const signal = read(object, signalName);
            if (signal && typeof signal.connect === "function") {
                signal.connect(callback);
                bag.push([signal, callback]);
                return;
            }
        } catch (_) { /* Optional/removed QObject: never log metadata. */ }
        missing.add(signalName);
    }
    function disconnect(bag) {
        for (const item of bag) {
            try { item[0].disconnect(item[1]); } catch (_) { /* QObject may be gone. */ }
        }
        bag.length = 0;
    }

    function createObserver(env) {
        const workspace = env.workspace;
        const sessionId = env.sessionId || "kwin-" + Date.now().toString(36) + "-" +
            Math.random().toString(36).slice(2) + Math.random().toString(36).slice(2);
        const entries = new Map(), byId = new Map(), desktops = new Map(), screens = new Map();
        const dirty = new Set(), globalDirty = new Set(), missing = new Set(), subscriptions = [];
        const groups = {
            "window.metadata.changed": ["title", "applicationId", "resourceClass", "resourceName", "classification", "parentRuntimeId", "skipTaskbar", "excludeFromCapture", "truncatedFields"],
            "window.desktop.changed": ["desktopIds", "onAllDesktops"],
            "window.activity.changed": ["activityIds", "onAllActivities"],
            "window.geometry.changed": ["geometry", "screenId"],
            "window.state.changed": ["minimized", "fullscreen"]
        };
        let running = false, initializing = false, revision = 1, sequence = 0;
        let nextWindow = 0, nextDesktop = 0, nextScreen = 0;
        let queue = [], needsSnapshot = true, resyncReason = "initial", overflows = 0;
        const globals = {desktops: null, activities: null, screens: null,
            activeWindowId: null, activeDesktopId: null, activeActivityId: null, activeDesktopByScreen: null};

        function desktopId(object) {
            if (!object) return null;
            let record = desktops.get(object);
            if (!record) {
                record = {id: text(read(object, "id"), 512) || "desktop-local-" + (++nextDesktop), bag: []};
                desktops.set(object, record);
                connect(object, "nameChanged", () => globalDirty.add("desktops"), record.bag, missing);
            }
            return record.id;
        }
        function screenId(object) {
            if (!object) return null;
            let record = screens.get(object);
            if (!record) {
                record = {id: "screen-" + (++nextScreen), bag: []};
                screens.set(object, record);
                for (const signal of ["geometryChanged", "scaleChanged"])
                    connect(object, signal, () => globalDirty.add("screens"), record.bag, missing);
            }
            return record.id;
        }
        function runtimeId(object) {
            const record = entries.get(object);
            return record && record.live ? record.id : null;
        }
        function windowState(record) {
            const object = record.object;
            const flags = {};
            for (const name of ["desktopWindow", "dock", "toolbar", "menu", "splash", "dropdownMenu", "popupMenu", "tooltip", "notification", "criticalNotification", "appletPopup", "onScreenDisplay", "comboBox", "dndIcon", "outline", "inputMethod"])
                flags[name] = boolean(read(object, name));
            let classification = "unknown";
            if (Object.keys(flags).some(key => flags[key] === true)) classification = "system";
            else if (read(object, "dialog") === true) classification = "dialog";
            else if (read(object, "utility") === true) classification = "utility";
            else if (read(object, "normalWindow") === true && read(object, "specialWindow") !== true) classification = "application";
            const membership = list(read(object, "desktops"));
            const activityIds = strings(read(object, "activities"));
            const caption = read(object, "caption");
            return {
                runtimeId: record.id, nativeId: uuid(read(object, "internalId")), windowingSystem: null,
                title: text(caption), applicationId: text(read(object, "desktopFileName"), 512),
                resourceClass: text(read(object, "resourceClass"), 512), resourceName: text(read(object, "resourceName"), 512),
                classification: classification, typeFlags: flags,
                parentRuntimeId: runtimeId(read(object, "transientFor")),
                desktopIds: membership === null ? null : membership.map(desktopId).filter(v => v !== null).sort(),
                activityIds: activityIds,
                // Empty activities means all; this is verified in Window::isOnAllActivities().
                onAllActivities: activityIds === null ? null : activityIds.length === 0,
                onAllDesktops: boolean(read(object, "onAllDesktops")),
                geometry: geometry(read(object, "frameGeometry")), screenId: screenId(read(object, "output")),
                minimized: boolean(read(object, "minimized")), fullscreen: boolean(read(object, "fullScreen")),
                skipTaskbar: boolean(read(object, "skipTaskbar")),
                excludeFromCapture: boolean(read(object, "excludeFromCapture")),
                truncatedFields: typeof caption === "string" && caption.length > 4096 ? ["title"] : []
            };
        }
        function emit(type, payload) {
            if (initializing || !running) return;
            revision++;
            const frame = Object.assign({schemaVersion: SCHEMA, sessionId: sessionId,
                sequence: ++sequence, revision: revision, type: type}, payload);
            if (needsSnapshot) return; // Latest state will be included in the resync baseline.
            if (queue.length >= 256) {
                queue = []; needsSnapshot = true; resyncReason = "overflow"; overflows++;
            } else queue.push(frame);
        }
        function update(record) {
            if (!record.live) return;
            const current = windowState(record), previous = record.state;
            record.state = current;
            for (const type of Object.keys(groups)) {
                const changed = groups[type].filter(key => !equal(previous[key], current[key]));
                if (changed.length) emit(type, {windowId: record.id, changedFields: changed, window: current});
            }
        }
        function add(object) {
            if (!running || !object || entries.has(object) || read(object, "deleted") === true) return;
            const record = {id: "window-" + (++nextWindow), object: object, live: true, bag: [], state: null};
            entries.set(object, record); byId.set(record.id, record);
            // Subscribe before taking a window's first state. No Qt event-loop yield here.
            for (const name of ["captionChanged", "desktopFileNameChanged", "windowClassChanged", "desktopsChanged", "activitiesChanged", "frameGeometryChanged", "outputChanged", "minimizedChanged", "fullScreenChanged", "transientChanged", "skipTaskbarChanged", "excludeFromCaptureChanged"])
                connect(object, name, () => { if (running && record.live) dirty.add(record.id); }, record.bag, missing);
            connect(object, "closed", () => remove(object), record.bag, missing);
            record.state = windowState(record);
            emit("window.added", {windowId: record.id, window: record.state});
            // A child can be observed before its parent in stacking order.
            if (!initializing) for (const other of byId.values())
                if (other.live && other !== record && read(other.object, "transientFor") === object) dirty.add(other.id);
        }
        function remove(object) {
            if (!running) return;
            const record = entries.get(object);
            if (!record || !record.live) return; // closed + windowRemoved is one removal.
            record.live = false;
            dirty.delete(record.id); entries.delete(object); byId.delete(record.id);
            disconnect(record.bag);
            record.object = null; record.state = null; // Never read a removed QObject again.
            emit("window.removed", {windowId: record.id});
            if (globals.activeWindowId === record.id) {
                globals.activeWindowId = null;
                emit("window.focus.changed", {activeWindowId: null});
            }
            for (const other of byId.values()) if (other.state.parentRuntimeId === record.id) dirty.add(other.id);
        }
        function setGlobal(key, value, type, extra) {
            if (equal(globals[key], value)) return;
            globals[key] = value;
            emit(type, Object.assign({[key]: clone(value)}, extra || {}));
        }
        function refreshGlobals() {
            if (globalDirty.has("desktops")) {
                const values = list(read(workspace, "desktops"));
                const present = new Set(values || []);
                const result = values === null ? null : values.map(object => ({
                    id: desktopId(object), nativeId: text(read(object, "id"), 512), name: text(read(object, "name"), 512)
                }));
                for (const [object, record] of desktops) if (!present.has(object)) { disconnect(record.bag); desktops.delete(object); }
                setGlobal("desktops", result, "desktop.list.changed");
            }
            if (globalDirty.has("activities"))
                setGlobal("activities", strings(read(workspace, "activities")), "activity.list.changed");
            if (globalDirty.has("screens")) {
                const values = list(read(workspace, "screens")), present = new Set(values || []);
                const result = values === null ? null : values.map(object => ({id: screenId(object),
                    name: text(read(object, "name"), 512), geometry: geometry(read(object, "geometry")),
                    scale: number(read(object, "devicePixelRatio"))}));
                for (const [object, record] of screens) if (!present.has(object)) { disconnect(record.bag); screens.delete(object); }
                setGlobal("screens", result, "screen.list.changed");
            }
            if (globalDirty.has("desktop") || globalDirty.has("screens") || globalDirty.has("desktops") || globalDirty.has("focus")) {
                const byScreen = {};
                for (const [object, record] of screens) {
                    try { byScreen[record.id] = desktopId(workspace.currentDesktopForScreen(object)); }
                    catch (_) { byScreen[record.id] = null; }
                }
                const current = desktopId(read(workspace, "currentDesktop"));
                if (!equal(globals.activeDesktopId, current) || !equal(globals.activeDesktopByScreen, byScreen)) {
                    globals.activeDesktopId = current; globals.activeDesktopByScreen = byScreen;
                    emit("desktop.active.changed", {activeDesktopId: current, activeDesktopByScreen: byScreen});
                }
            }
            if (globalDirty.has("activity"))
                setGlobal("activeActivityId", text(read(workspace, "currentActivity"), 512), "activity.active.changed");
            if (globalDirty.has("focus")) {
                const object = read(workspace, "activeWindow");
                if (object && !entries.has(object) && read(object, "deleted") !== true) add(object);
                setGlobal("activeWindowId", runtimeId(object), "window.focus.changed");
            }
            globalDirty.clear();
        }
        function flush() {
            if (!running) return;
            refreshGlobals();
            const pending = Array.from(dirty); dirty.clear();
            for (const id of pending) { const record = byId.get(id); if (record) update(record); }
        }
        function snapshot(reason) {
            flush();
            const windows = Array.from(byId.values()).map(record => clone(record.state)).sort((a, b) => a.runtimeId.localeCompare(b.runtimeId));
            return {schemaVersion: SCHEMA, sessionId: sessionId, sequence: ++sequence, revision: revision, type: "snapshot",
                reason: reason || "requested", state: Object.assign(clone(globals), {
                    windows: windows,
                    applicationWindowIds: windows.filter(w => w.classification === "application" && w.skipTaskbar !== true && w.excludeFromCapture !== true).map(w => w.runtimeId),
                    capabilities: {missingSignals: Array.from(missing).sort(), windowingSystem: "not-exposed-by-public-window-properties"}
                })};
        }
        function resync(reason) { queue = []; needsSnapshot = true; resyncReason = reason || "requested"; }
        function take() {
            flush();
            if (needsSnapshot) {
                const frame = snapshot(resyncReason);
                needsSnapshot = false; queue = [];
                return [frame];
            }
            return queue.splice(0, 32);
        }
        const timer = env.makeTimer(50, flush);
        return {
            start: function () {
                if (running) return;
                running = true; initializing = true;
                connect(workspace, "windowAdded", add, subscriptions, missing);
                connect(workspace, "windowRemoved", remove, subscriptions, missing);
                const signals = {windowActivated: "focus", desktopsChanged: "desktops", currentDesktopChanged: "desktop",
                    currentActivityChanged: "activity", activitiesChanged: "activities", screensChanged: "screens"};
                for (const name of Object.keys(signals)) connect(workspace, name, () => globalDirty.add(signals[name]), subscriptions, missing);
                for (const key of ["desktops", "activities", "screens", "desktop", "activity", "focus"]) globalDirty.add(key);
                refreshGlobals();
                // Workspace subscriptions are installed before the only full enumeration.
                for (const object of list(read(workspace, "stackingOrder")) || []) add(object);
                for (const record of byId.values()) dirty.add(record.id);
                globalDirty.add("focus"); refreshGlobals();
                flush(); initializing = false;
                timer.start();
            },
            stop: function () {
                running = false; timer.stop(); disconnect(subscriptions);
                for (const record of byId.values()) { record.live = false; disconnect(record.bag); record.object = null; record.state = null; }
                for (const record of desktops.values()) disconnect(record.bag);
                for (const record of screens.values()) disconnect(record.bag);
                entries.clear(); byId.clear(); desktops.clear(); screens.clear(); dirty.clear(); globalDirty.clear(); queue = [];
            },
            flush: flush, snapshot: snapshot, take: take, resync: resync,
            sessionId: sessionId,
            position: () => ({sequence: sequence, revision: revision}),
            stats: () => ({windows: byId.size, queuedEvents: queue.length, overflows: overflows, running: running})
        };
    }

    // No title payload leaves KWin until the receiver's bus UID matches KWin's.
    // Use its unique name afterwards, so well-known-name takeover cannot redirect data.
    function createTransport(env, observer) {
        let owner = null, consumerId = null, busy = false, elapsed = 0, generation = 0, alive = false;
        let ticks = 0, oversized = false;
        function rpc(service, path, iface, method, args, done) {
            const token = generation;
            busy = true; elapsed = 0;
            env.callDBus.apply(null, [service, path, iface, method].concat(args, [function (value) {
                if (!alive || token !== generation) return;
                busy = false; done(value);
            }]));
        }
        function daemon(method, args, done) { rpc("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", method, args, done); }
        function reset() { generation++; owner = null; consumerId = null; busy = false; elapsed = 0; observer.resync("reconnect"); }
        function handshake() {
            daemon("NameHasOwner", [SERVICE], function (exists) {
                if (!exists) return;
                daemon("GetNameOwner", [SERVICE], function (unique) {
                    daemon("GetConnectionUnixUser", [unique], function (targetUid) {
                        daemon("GetConnectionUnixUser", ["org.kde.KWin"], function (kwinUid) {
                            if (Number(targetUid) !== Number(kwinUid)) return;
                            rpc(unique, PATH, INTERFACE, "Hello", [observer.sessionId], function (answer) {
                                let response;
                                try { response = JSON.parse(answer); } catch (_) { return; }
                                if (response.schemaVersion !== SCHEMA || typeof response.consumerId !== "string") return;
                                owner = unique; consumerId = response.consumerId;
                                observer.resync("consumer-connected");
                            });
                        });
                    });
                });
            });
        }
        function tick() {
            if (!alive) return;
            ticks++;
            if (busy) {
                elapsed += 50;
                // KWin 6.8.1 suppresses callbacks on D-Bus errors. Its default
                // async timeout is 25 s; allow it to finish before reconnecting.
                if (elapsed >= 30000) reset();
                return;
            }
            if (!owner) { if (ticks % 60 === 1) handshake(); return; }
            if (oversized) return;
            const frames = observer.take();
            if (!frames.length && ticks % 40 !== 0) return;
            let packet = JSON.stringify({schemaVersion: SCHEMA, sessionId: observer.sessionId,
                consumerId: consumerId, frames: frames, position: observer.position()});
            // Explicit fail-closed bound: no endless giant-snapshot retries.
            // One Mi UTF-16 code units fits inside the consumer's four MiB UTF-8 bound.
            if (packet.length > 1024 * 1024) {
                oversized = true; observer.resync("payload-too-large");
                packet = JSON.stringify({schemaVersion: SCHEMA, sessionId: observer.sessionId, consumerId: consumerId,
                    frames: [], position: observer.position(), status: "payload-too-large"});
            }
            rpc(owner, PATH, INTERFACE, "Push", [packet], function (answer) {
                let response;
                try { response = JSON.parse(answer); } catch (_) { reset(); return; }
                if (response.consumerId !== consumerId) { reset(); return; }
                if (response.needSnapshot) observer.resync("consumer-requested");
            });
        }
        const timer = env.makeTimer(50, tick);
        return {start: function () { alive = true; timer.start(); tick(); },
            stop: function () { alive = false; generation++; timer.stop(); },
            tick: tick, stats: () => ({connected: owner !== null, busy: busy, oversized: oversized})};
    }

    if (typeof module !== "undefined" && module.exports) {
        module.exports = {createObserver: createObserver, createTransport: createTransport};
    } else {
        const env = {workspace: workspace, callDBus: callDBus, makeTimer: function (interval, callback) {
            const timer = new QTimer(); timer.interval = interval; timer.singleShot = false;
            timer.timeout.connect(callback); return timer;
        }};
        const observer = createObserver(env);
        observer.start();
        const transport = createTransport(env, observer);
        transport.start();
        // Keep JS-owned timers/objects reachable for the script's lifetime.
        depthosRuntime = {observer: observer, transport: transport};
    }
}());
