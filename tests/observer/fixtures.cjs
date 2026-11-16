const {createObserver} = require('../../experiments/kwin-observer/contents/code/main.js');
class Signal {
    constructor() { this.callbacks = new Set(); }
    connect(f) { this.callbacks.add(f); }
    disconnect(f) { this.callbacks.delete(f); }
    emit(...args) { for (const f of [...this.callbacks]) f(...args); }
}
function signals(object, names) {
    for (const name of names.split(' ')) object[name] = new Signal();
    return object;
}
function window(overrides = {}) {
    return signals(Object.assign({caption: 'Example editor', desktopFileName: 'org.example.Editor',
        resourceClass: 'example', resourceName: 'editor', internalId: '{12345678-1234-1234-1234-123456789abc}',
        desktops: [], activities: [], onAllDesktops: true, frameGeometry: {x: 10, y: 20, width: 800, height: 600},
        output: null, minimized: false, fullScreen: false, normalWindow: true, specialWindow: false,
        skipTaskbar: false, excludeFromCapture: false, transientFor: null, deleted: false,
        dialog: false, utility: false}, overrides),
        'captionChanged desktopFileNameChanged windowClassChanged desktopsChanged activitiesChanged frameGeometryChanged outputChanged minimizedChanged fullScreenChanged transientChanged skipTaskbarChanged excludeFromCaptureChanged closed');
}
function timer() { return {active: false, start() { this.active = true; }, stop() { this.active = false; }}; }
function fixture(windows = []) {
    const desktop = signals({id: 'desktop-native-1', name: 'Desktop 1'}, 'nameChanged');
    const screen = signals({name: 'eDP-example', geometry: {x: 0, y: 0, width: 1920, height: 1080}, devicePixelRatio: 1}, 'geometryChanged scaleChanged');
    for (const win of windows) if (win.output === null) win.output = screen;
    const workspace = signals({stackingOrder: windows, desktops: [desktop], screens: [screen], activities: ['activity-1'],
        currentDesktop: desktop, currentActivity: 'activity-1', activeWindow: windows[0] || null,
        currentDesktopForScreen() { return this.currentDesktop; }},
        'windowAdded windowRemoved windowActivated desktopsChanged currentDesktopChanged currentActivityChanged activitiesChanged screensChanged');
    const observer = createObserver({workspace, makeTimer: timer, sessionId: 'test-session'});
    observer.start();
    const initial = observer.take()[0];
    return {workspace, desktop, screen, observer, initial, events() { observer.flush(); return observer.take(); }};
}
module.exports = {Signal, signals, window, timer, fixture};
