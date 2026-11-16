// Boundary fixture only. Does not establish real KWin filtering correctness.
import QtQuick
ListModel {
    id: proxy
    enum Types { Normal = 1, Dialog = 2, Dock = 4, Desktop = 8 }
    dynamicRoles: true
    property var windowModel: null
    property var desktop: null
    property string activity: ""
    property string screenName: ""
    property int windowType: 3
    property bool minimizedWindows: true
    property bool initialized: false
    Component.onCompleted: { initialized = true; rebuild(); }
    function rebuild() {
        if (!initialized) return;
        clear();
        if (!windowModel) return;
        for (let i = 0; i < windowModel.count; ++i) {
            const w = windowModel.get(i).window;
            if (!w) continue;
            const type = w.normalWindow ? 1 : w.dialog ? 2 : w.dock ? 4 : w.desktopWindow ? 8 : 0;
            if (!(type & windowType)) continue;
            if (desktop && !w.onAllDesktops && w.desktops.indexOf(desktop) < 0) continue;
            if (activity && w.activities.length && w.activities.indexOf(activity) < 0) continue;
            if (screenName && (!w.output || w.output.name !== screenName)) continue;
            if (!minimizedWindows && w.minimized) continue;
            append({window: w});
        }
    }
    onWindowModelChanged: rebuild()
    onDesktopChanged: rebuild()
    onActivityChanged: rebuild()
    onScreenNameChanged: rebuild()
    property QtObject watcher: Connections {
        target: proxy.windowModel
        function onCountChanged() { proxy.rebuild(); }
        function onRevisionChanged() { proxy.rebuild(); }
    }
}
