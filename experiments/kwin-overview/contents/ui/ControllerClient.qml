import QtQuick
import org.kde.plasma.workspace.dbus as DBus
import "BridgeModel.js" as Model

Item {
    id: client
    visible: false
    objectName: "depthos-controller-client"
    property string state: "Disconnected"
    property string instance: ""
    property string owner: ""
    property string lastCall: ""
    property string lastError: ""
    property string health: "Unavailable"
    property var rows: []
    property var associations: []
    property var nativeBadges: ({})
    property bool contextAvailable: false
    property string selectedWorldId: ""
    property string selectedWorkspaceId: ""
    property string selectedDesktopId: ""
    property int serial: 0
    property var pending: null
    property bool busy: false
    property int pendingScopes: 3
    property string hierarchyRevision: ""
    property string contextRevision: ""
    property int fetchedPages: 0
    property string service: "org.depthos.Controller"
    readonly property string path: "/org/depthos/Controller"
    readonly property string iface: "org.depthos.Controller1"
    function reset() {
        serial++; if (pending) { pending.destroy(); pending = null; }
        busy = false; watchdog.stop(); state = "Disconnected"; instance = ""; owner = "";
        rows = []; associations = []; nativeBadges = ({}); contextAvailable = false; health = "Unavailable";
        pendingScopes = 3; hierarchyRevision = ""; contextRevision = "";
        retry.stop();
        selectedWorldId = ""; selectedWorkspaceId = ""; selectedDesktopId = "";
    }
    function fail() {
        serial++; if (pending) { pending.destroy(); pending = null; }
        busy = false; watchdog.stop(); instance = ""; owner = ""; rows = []; state = "Backend unavailable"; contextAvailable = false; nativeBadges = ({}); associations = [];
        pendingScopes = 3; hierarchyRevision = ""; contextRevision = "";
        retry.restart();
    }
    function call(target, objectPath, interfaceName, member, args, signature, done) {
        const ticket = serial; lastCall = member;
        const reply = DBus.SessionBus.asyncCall({service:target, path:objectPath, interface:interfaceName, iface:interfaceName, member:member, arguments:args, signature:"(" + signature + ")"});
        pending = reply; watchdog.restart();
        let handled = false;
        function finished() {
            if (handled) return; handled = true;
            if (ticket !== serial) { reply.destroy(); return; }
            watchdog.stop(); pending = null;
            if (reply.isError) { lastError = reply.error.name + ":" + reply.error.message; reply.destroy(); client.fail(); return; }
            const value = String(reply.value); reply.destroy();
            try { done(value); } catch (error) { lastError = String(error) + " type=" + typeof value; client.fail(); }
        }
        reply.finished.connect(finished);
        if (reply.isFinished) finished();
    }
    function packet(raw) {
        const p = Model.decode(raw);
        if (instance && p.instanceId !== instance) throw new Error("replaced");
        return p;
    }
    function refresh() {
        if (busy) return;
        if (!serviceWatch.registered) { reset(); return; }
        const scopes = pendingScopes || 3;
        pendingScopes = scopes;
        busy = true; state = "Loading"; fetchedPages = 0;
        retry.stop();
        if (instance) {
            refreshModels(scopes);
            return;
        }
        call("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus", "GetNameOwner", [service], "s", value => {
            owner = value;
            call(owner, path, iface, "GetCapabilities", [], "", raw => {
                const p = Model.decode(raw);
                if (p.data.readOnly !== true || p.data.assignmentMutations !== false) throw new Error("unsupported");
                instance = p.instanceId;
                refreshModels(scopes);
            });
        });
    }
    function refreshModels(scopes) {
        call(owner, path, iface, "GetControllerHealth", [], "", raw => {
            const h = packet(raw);
            health = h.data.health ? h.data.health.lifecycle : "Unavailable";
            // A hierarchy fetch can discover a context change before its signal arrives.
            if (h.revision !== contextRevision) {
                scopes |= 2;
                pendingScopes |= 2;
                contextAvailable = false;
                nativeBadges = ({});
            }
            function finish() {
                pendingScopes = 0;
                state = health === "Synchronized" ? "Connected" : "Degraded";
                busy = false;
            }
            function refreshContext() {
                if (!(scopes & 2)) { finish(); return; }
                fetchPages("GetContextReport", 0, "", [], (context, revision, available) => {
                    const badges = available ? Model.badges(context) : ({});
                    associations = context;
                    nativeBadges = badges;
                    contextRevision = revision;
                    contextAvailable = available;
                    finish();
                });
            }
            if (!(scopes & 1)) { refreshContext(); return; }
            fetchPages("GetHierarchy", 0, "", [], (hierarchy, revision) => {
                if (!hierarchyRevision || revision !== hierarchyRevision) {
                    rows = hierarchy;
                    hierarchyRevision = revision;
                    reconcile();
                }
                refreshContext();
            });
        });
    }
    function fetchPages(method, cursor, revision, items, done) {
        if (++fetchedPages > 130) throw new Error("pages");
        call(owner, path, iface, method, [cursor, revision], "us", raw => {
            const p = packet(raw);
            if (revision && p.revision !== revision) throw new Error("stale page");
            const page = method === "GetHierarchy" ? Model.hierarchyRows(p.data.items) : Model.contextRows(p.data.items);
            if (items.length + page.length > 4096) throw new Error("rows");
            for (const row of page) items.push(row);
            if (p.data.nextCursor !== null) {
                if (typeof p.data.nextCursor !== "number" || p.data.nextCursor <= cursor || p.data.nextCursor > 4096) throw new Error("cursor");
                fetchPages(method, p.data.nextCursor, p.revision, items, done);
            } else done(items, p.revision, p.available);
        });
    }
    function reconcile() {
        if (!rows.some(r => r.id === selectedWorldId && r.kind === "world")) { selectedWorldId = ""; selectedWorkspaceId = ""; selectedDesktopId = ""; }
        if (!rows.some(r => r.id === selectedWorkspaceId && r.parentId === selectedWorldId)) { selectedWorkspaceId = ""; selectedDesktopId = ""; }
        if (!rows.some(r => r.id === selectedDesktopId && r.parentId === selectedWorkspaceId)) selectedDesktopId = "";
    }
    function choose(row) {
        if (row.kind === "world") { selectedWorldId = row.id; selectedWorkspaceId = ""; selectedDesktopId = ""; }
        else if (row.kind === "workspace") { selectedWorldId = row.parentId; selectedWorkspaceId = row.id; selectedDesktopId = ""; }
        else if (row.kind === "desktop") { selectedWorkspaceId = row.parentId; selectedDesktopId = row.id; }
    }
    function badge(id) {
        const key = String(id || "").replace(/[{}]/g, "").toLowerCase();
        return contextAvailable && nativeBadges[key] ? nativeBadges[key].status : "unassigned / unavailable";
    }
    DBus.DBusServiceWatcher {
        id: serviceWatch; busType: DBus.BusType.Session; watchedService: client.service
        onRegisteredChanged: { client.reset(); if (registered) refreshTimer.restart(); }
    }
    DBus.SignalWatcher {
        service: client.service; path: client.path; iface: client.iface
        function dbusInvalidated(instance, scopes) {
            scopes = Number(scopes) & 3;
            if (!scopes) return;
            if (client.instance && String(instance) !== client.instance) {
                client.reset();
                scopes = 3;
            }
            // Keep scopes from an interrupted refresh until its replacement completes.
            client.pendingScopes |= scopes;
            client.serial++; if (client.pending) {client.pending.destroy();client.pending=null;}
            client.busy=false;client.watchdogStop();
            if (scopes & 2) {client.contextAvailable=false;client.nativeBadges=({});}
            refreshTimer.restart();
        }
    }
    DBus.SignalWatcher {
        service: "org.freedesktop.DBus"; path: "/org/freedesktop/DBus"; iface: "org.freedesktop.DBus"
        function dbusNameAcquired(name) {}
        function dbusNameLost(name) {}
        function dbusNameOwnerChanged(name, previous, current) {
            if (String(name) === client.service) {client.reset();if (String(current)) refreshTimer.restart();}
        }
    }
    function watchdogStop() { watchdog.stop(); }
    Timer { id: refreshTimer; interval: 100; onTriggered: client.refresh() }
    Timer { id: retry; interval: 2000; onTriggered: client.refresh() }
    Timer { id: watchdog; interval: 2500; onTriggered: client.fail() }
    Component.onCompleted: refreshTimer.start()
    Component.onDestruction: { serial++; if (pending) pending.destroy(); }
}
