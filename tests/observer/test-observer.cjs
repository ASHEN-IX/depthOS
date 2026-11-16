const test = require('node:test');
const assert = require('node:assert/strict');
const {fixture, window, signals, timer} = require('./fixtures.cjs');
const {createObserver, createTransport} = require('../../experiments/kwin-observer/contents/code/main.js');

test('deterministic initial snapshot, native metadata and all-membership semantics', () => {
    const f = fixture([window()]);
    assert.equal(f.initial.revision, 1);
    assert.equal(f.initial.state.windows[0].nativeId, '12345678-1234-1234-1234-123456789abc');
    assert.equal(f.initial.state.windows[0].onAllDesktops, true);
    assert.equal(f.initial.state.windows[0].onAllActivities, true);
    assert.equal(f.initial.state.windows[0].windowingSystem, null);
    assert.equal(f.initial.state.activeWindowId, f.initial.state.windows[0].runtimeId);
    assert.equal(JSON.parse(JSON.stringify(f.initial)).state.desktops[0].id, 'desktop-native-1');
});

test('subscriptions precede the single initial native window enumeration', () => {
    const f = fixture();
    f.observer.stop();
    let enumerations = 0;
    Object.defineProperty(f.workspace, 'stackingOrder', {get() {
        enumerations++;
        assert.equal(f.workspace.windowAdded.callbacks.size, 1);
        return [];
    }});
    const observer = createObserver({workspace: f.workspace, makeTimer: timer, sessionId: 'new'});
    observer.start(); observer.take();
    for (let i = 0; i < 50; ++i) observer.flush();
    assert.equal(enumerations, 1);
});

test('add/remove/duplicate notifications and fresh IDs after reopen', () => {
    const f = fixture(), a = window();
    f.workspace.windowAdded.emit(a); f.workspace.windowAdded.emit(a);
    const added = f.events();
    assert.equal(added.length, 1); assert.equal(added[0].type, 'window.added');
    a.closed.emit(); f.workspace.windowRemoved.emit(a);
    assert.deepEqual(f.events().map(e => e.type), ['window.removed']);
    f.workspace.windowAdded.emit(window());
    assert.notEqual(f.events()[0].windowId, added[0].windowId);
});

test('focus changes and null focus are event driven', () => {
    const a = window(), b = window(), f = fixture([a, b]);
    f.workspace.activeWindow = b; f.workspace.windowActivated.emit(b);
    assert.equal(f.events()[0].activeWindowId, 'window-2');
    f.workspace.activeWindow = null; f.workspace.windowActivated.emit(null);
    assert.equal(f.events()[0].activeWindowId, null);
});

test('focus across outputs also refreshes the active desktop context', () => {
    const a = window(), f = fixture([a]);
    const other = signals({id: 'desktop-focus', name: 'Other output desktop'}, 'nameChanged');
    f.workspace.currentDesktop = other;
    f.workspace.windowActivated.emit(a);
    assert.ok(f.events().some(e => e.type === 'desktop.active.changed' && e.activeDesktopId === 'desktop-focus'));
});

test('title, app ID and class changes, redundant signals suppressed', () => {
    const a = window(), f = fixture([a]);
    a.caption = 'New title'; a.desktopFileName = 'new.app'; a.resourceClass = 'newclass';
    a.captionChanged.emit(); a.desktopFileNameChanged.emit(); a.windowClassChanged.emit();
    const events = f.events(); assert.equal(events.length, 1);
    assert.equal(events[0].type, 'window.metadata.changed');
    assert.equal(events[0].window.title, 'New title');
    a.captionChanged.emit(); assert.equal(f.events().length, 0);
});

test('multiple desktops, sticky and activity membership changes', () => {
    const a = window(), f = fixture([a]);
    const other = signals({id: 'desktop-native-2', name: 'Desktop 2'}, 'nameChanged');
    f.workspace.desktops.push(other); f.workspace.desktopsChanged.emit();
    a.desktops = [other, f.desktop]; a.onAllDesktops = false;
    a.activities = ['activity-2', 'activity-1'];
    a.desktopsChanged.emit(); a.activitiesChanged.emit();
    const events = f.events();
    const desktop = events.find(e => e.type === 'window.desktop.changed');
    assert.deepEqual(desktop.window.desktopIds, ['desktop-native-1', 'desktop-native-2']);
    assert.equal(desktop.window.onAllDesktops, false);
    assert.equal(events.find(e => e.type === 'window.activity.changed').window.onAllActivities, false);
    a.desktops = []; a.onAllDesktops = true; a.desktopsChanged.emit();
    assert.equal(f.events()[0].window.onAllDesktops, true);
});

test('desktop creation/removal/rename and active desktop/Activity changes', () => {
    const f = fixture();
    f.desktop.name = 'Renamed'; f.desktop.nameChanged.emit();
    assert.equal(f.events()[0].desktops[0].name, 'Renamed');
    f.workspace.activities.push('activity-2'); f.workspace.activitiesChanged.emit('activity-2');
    f.workspace.currentActivity = 'activity-2'; f.workspace.currentActivityChanged.emit('activity-2');
    assert.deepEqual(f.events().map(e => e.type), ['activity.list.changed', 'activity.active.changed']);
    const d = signals({id: 'new-desktop', name: 'Other'}, 'nameChanged');
    f.workspace.desktops.push(d); f.workspace.currentDesktop = d;
    f.workspace.desktopsChanged.emit(); f.workspace.currentDesktopChanged.emit(f.desktop, d, f.screen);
    assert.ok(f.events().some(e => e.type === 'desktop.active.changed' && e.activeDesktopId === 'new-desktop'));
    f.workspace.desktops = [f.desktop]; f.workspace.currentDesktop = f.desktop; f.workspace.desktopsChanged.emit();
    f.events(); assert.equal(d.nameChanged.callbacks.size, 0);
});

test('display topology, scale and geometry changes; stale outputs disconnected', () => {
    const f = fixture();
    f.screen.geometry.width = 1280; f.screen.devicePixelRatio = 1.5;
    f.screen.geometryChanged.emit(); f.screen.scaleChanged.emit();
    const event = f.events()[0]; assert.equal(event.type, 'screen.list.changed');
    assert.equal(event.screens[0].scale, 1.5); assert.equal(event.screens[0].geometry.width, 1280);
    f.workspace.screens = []; f.workspace.screensChanged.emit(); f.events();
    assert.equal(f.screen.geometryChanged.callbacks.size, 0);
});

test('geometry is coalesced and minimized/fullscreen state updates emitted', () => {
    const a = window(), f = fixture([a]);
    for (let i = 0; i < 200; ++i) { a.frameGeometry.x = i; a.frameGeometryChanged.emit(); }
    const events = f.events(); assert.equal(events.length, 1); assert.equal(events[0].window.geometry.x, 199);
    a.minimized = true; a.fullScreen = true; a.minimizedChanged.emit(); a.fullScreenChanged.emit();
    assert.equal(f.events()[0].type, 'window.state.changed');
});

test('classification filters system UI; dialogs retain parent relationship', () => {
    const a = window(), dialog = window({normalWindow: false, dialog: true, transientFor: a});
    const f = fixture([dialog, a, window({dock: true}), window({skipTaskbar: true}), window({excludeFromCapture: true})]);
    assert.equal(f.initial.state.windows.find(w => w.classification === 'dialog').parentRuntimeId, 'window-2');
    assert.deepEqual(f.initial.state.applicationWindowIds, ['window-2']);
});

test('removed objects are not dereferenced; queued callbacks become inert', () => {
    const a = window(), f = fixture([a]);
    const stale = [...a.captionChanged.callbacks];
    const id = f.initial.state.windows[0].runtimeId;
    a.closed.emit();
    Object.defineProperty(a, 'caption', {get() { throw new Error('destroyed QObject read'); }});
    for (const cb of stale) cb();
    assert.equal(f.events()[0].windowId, id);
    assert.equal(a.captionChanged.callbacks.size, 0);
    assert.equal(f.observer.stats().windows, 0);
});

test('missing properties, throwing getters and missing optional signals are explicit', () => {
    const a = window(); delete a.activities; delete a.minimized; delete a.fullScreenChanged;
    Object.defineProperty(a, 'caption', {get() { throw new Error('missing'); }});
    const f = fixture([a]), state = f.initial.state.windows[0];
    assert.equal(state.title, null); assert.equal(state.activityIds, null);
    assert.equal(state.onAllActivities, null); assert.equal(state.minimized, null);
    assert.ok(f.initial.state.capabilities.missingSignals.includes('fullScreenChanged'));
});

test('event sequence/revision ordering, overflow snapshot and consumer resync', () => {
    const f = fixture();
    for (let i = 0; i < 300; ++i) f.workspace.windowAdded.emit(window());
    assert.ok(f.observer.stats().queuedEvents <= 256);
    assert.equal(f.observer.stats().overflows, 1);
    const snapshot = f.observer.take()[0];
    assert.equal(snapshot.type, 'snapshot'); assert.equal(snapshot.state.windows.length, 300);
    assert.ok(snapshot.sequence > f.initial.sequence);
    f.observer.resync('requested'); assert.equal(f.observer.take()[0].reason, 'requested');
    const win = window(); f.workspace.windowAdded.emit(win); win.caption = 'changed'; win.captionChanged.emit();
    const batch = f.events();
    assert.ok(batch[1].sequence > batch[0].sequence); assert.ok(batch[1].revision > batch[0].revision);
});

test('stop removes subscriptions and invalidates already queued creation callbacks', () => {
    const a = window(), f = fixture([a]);
    const stale = [...f.workspace.windowAdded.callbacks];
    f.observer.stop();
    for (const cb of stale) cb(window());
    assert.equal(f.observer.stats().windows, 0);
    assert.equal(a.captionChanged.callbacks.size, 0);
    assert.equal(f.workspace.windowAdded.callbacks.size, 0);
});

function transportHarness(options = {}) {
    const f = fixture([window()]);
    const packets = [], calls = [];
    let consumer = 'consumer-1', drop = false;
    const transport = createTransport({makeTimer: timer, callDBus(service, path, iface, method, ...args) {
        const callback = args.pop(); calls.push({service, method});
        if (method === 'NameHasOwner') callback(options.absent !== true);
        else if (method === 'GetNameOwner') callback(':1.200');
        else if (method === 'GetConnectionUnixUser') callback(args[0] === 'org.kde.KWin' ? 1000 : (options.foreign ? 1001 : 1000));
        else if (method === 'Hello') callback(JSON.stringify({schemaVersion: 1, consumerId: consumer}));
        else if (method === 'Push') { packets.push(JSON.parse(args[0])); if (!drop) callback(JSON.stringify({consumerId: consumer, needSnapshot: options.needSnapshot === true})); }
    }}, f.observer);
    transport.start();
    return {f, packets, calls, transport, reconnect() { drop = true; }, restore() { drop = false; consumer = 'consumer-2'; }};
}

test('transport addresses the verified unique same-UID receiver, not the public name', () => {
    const h = transportHarness(); h.transport.tick();
    assert.equal(h.packets[0].frames[0].type, 'snapshot');
    assert.ok(h.calls.filter(c => c.method === 'Push').every(c => c.service === ':1.200'));
    const denied = transportHarness({foreign: true});
    for (let i = 0; i < 100; ++i) denied.transport.tick();
    assert.equal(denied.packets.length, 0);
});

test('consumer asks for a fresh snapshot and idle heartbeats serialize no title events', () => {
    const h = transportHarness({needSnapshot: true}); h.transport.tick(); h.transport.tick();
    assert.equal(h.packets[1].frames[0].reason, 'consumer-requested');
    const idle = transportHarness(); for (let i = 0; i < 40; ++i) idle.transport.tick();
    assert.equal(idle.packets.at(-1).frames.length, 0);
});

test('missing receiver and lost callback/reconnection stay bounded and resnapshot', () => {
    const absent = transportHarness({absent: true}); for (let i = 0; i < 100; ++i) absent.transport.tick();
    assert.equal(absent.packets.length, 0);
    const h = transportHarness(); h.transport.tick(); h.reconnect();
    for (let i = 0; i < 40; ++i) h.transport.tick(); // lose a heartbeat reply
    assert.equal(h.transport.stats().busy, true);
    for (let i = 0; i < 600; ++i) h.transport.tick();
    h.restore();
    for (let i = 0; i < 65; ++i) h.transport.tick();
    assert.ok(h.packets.some(p => p.consumerId === 'consumer-2' && p.frames.some(f => f.type === 'snapshot')));
    assert.ok(h.f.observer.stats().queuedEvents <= 256);
});

test('oversized snapshots produce one metadata-free status instead of unbounded retries', () => {
    const f = fixture(Array.from({length: 300}, () => window({caption: 'x'.repeat(4096)})));
    const packets = [];
    const transport = createTransport({makeTimer: timer, callDBus(service, path, iface, method, ...args) {
        const cb = args.pop();
        if (method === 'NameHasOwner') cb(true);
        else if (method === 'GetNameOwner') cb(':1.5');
        else if (method === 'GetConnectionUnixUser') cb(1000);
        else if (method === 'Hello') cb(JSON.stringify({schemaVersion: 1, consumerId: 'large'}));
        else if (method === 'Push') { packets.push(JSON.parse(args[0])); cb(JSON.stringify({consumerId: 'large', needSnapshot: true})); }
    }}, f.observer);
    transport.start(); for (let i = 0; i < 100; ++i) transport.tick();
    assert.equal(packets.length, 1);
    assert.equal(packets[0].status, 'payload-too-large');
    assert.deepEqual(packets[0].frames, []);
    assert.equal(transport.stats().oversized, true);
});
