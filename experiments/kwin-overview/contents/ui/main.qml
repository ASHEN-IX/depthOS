// KWin 6.8.1 creates output delegates in a child context.
pragma ComponentBehavior: Unbound
import QtQuick
import org.kde.kwin
import org.kde.kirigami as Kirigami
import "Policy.js" as Policy

SceneEffect {
    id: effect
    // Title-free lifecycle diagnostics for native acceptance; no window metadata.
    property bool diagnosticsEnabled: true
    function trace(message) { if (diagnosticsEnabled) console.info("[depthos-overview] " + message); }
    Component.onCompleted: trace("initialized")
    property bool backendEnabled: true
    readonly property var logicalBackend: backendLoader.item
    Loader { id: backendLoader; active: effect.backendEnabled; asynchronous: true; source: "ControllerClient.qml" }
    property var system: Workspace
    property var windowSource: windows
    property bool requestedOpen: false
    // SceneEffect.visible is a request, not confirmation that KWin started views.
    property int constructedOutputs: 0
    Timer {
        id: startupGuard
        interval: 1000; repeat: false
        onTriggered: {
            if (effect.visible && effect.requestedOpen && effect.constructedOutputs === 0) {
                effect.trace("activation unavailable; no output delegate; request reset");
                effect.requestedOpen = false;
                effect.visible = false;
            }
        }
    }
    property real reveal: 0
    property int level: 1
    property var selectedDesktop: null
    property string selectedActivity: ""
    property string intendedDesktopId: ""
    property string intendedActivityId: ""
    property string selectedWindowId: ""
    property bool showTitles: false
    property bool previewsEnabled: true
    property bool reducedMotion: false
    readonly property bool motionOff: reducedMotion || Kirigami.Units.longDuration === 0
    readonly property var desktops: system ? system.desktops : []
    readonly property var activities: system ? system.activities : []
    readonly property int maxLevel: activities && activities.length ? 3 : 2

    WindowModel { id: windows }
    function reconcile() {
        const available = desktops || [];
        let found = false;
        for (let i = 0; i < available.length; ++i) if (available[i] === selectedDesktop) found = true;
        if (!found) selectedDesktop = system && available.indexOf(system.currentDesktop) >= 0 ? system.currentDesktop : (available.length ? available[0] : null);
        if (!activities || activities.indexOf(selectedActivity) < 0) {
            const current = system ? (system.currentActivity || "") : "";
            selectedActivity = activities && activities.indexOf(current) >= 0 ? current : (activities && activities.length ? activities[0] : "");
        }
        level = Math.min(level, maxLevel);
        // Intent is presentation-only and is cleared when the destination disappears.
        if (intendedDesktopId && !available.some(d => Policy.read(d, "id") === intendedDesktopId)) intendedDesktopId = "";
        if (intendedActivityId && (!activities || activities.indexOf(intendedActivityId) < 0)) intendedActivityId = "";
    }
    function open() {
        transition.stop();
        if (!visible) {
            reveal = 0; level = 1; selectedWindowId = "";
            selectedDesktop = system ? system.currentDesktop : null;
            selectedActivity = system ? (system.currentActivity || "") : "";
            intendedDesktopId = ""; intendedActivityId = "";
        }
        reconcile(); requestedOpen = true; visible = true;
        transition.to = 1; transition.duration = motionOff ? 0 : 300; transition.start();
    }
    function close() {
        if (!visible) return;
        requestedOpen = false; transition.stop();
        transition.to = 0; transition.duration = motionOff ? 0 : 230; transition.start();
    }
    function toggle() { if (visible && requestedOpen) close(); else open(); }
    function zoomOut() { level = Math.min(maxLevel, level + 1); }
    function zoomIn() { if (level > 1) level--; else close(); }
    function inspectDesktop(desktop) {
        if (!desktop || !desktops || desktops.indexOf(desktop) < 0) return;
        selectedDesktop = desktop; intendedDesktopId = Policy.read(desktop, "id") || ""; selectedWindowId = ""; level = 1;
    }
    function inspectActivity(activity) {
        if (!activities || activities.indexOf(activity) < 0) return;
        selectedActivity = activity; intendedActivityId = activity; selectedWindowId = ""; level = 2;
    }
    onLevelChanged: trace("level=" + level)
    onMotionOffChanged: trace("motionOff=" + motionOff)
    onSelectedWindowIdChanged: trace("window selection present=" + (selectedWindowId !== ""))
    onSystemChanged: { selectedDesktop = null; selectedActivity = ""; reconcile(); }
    onDesktopsChanged: reconcile()
    onActivitiesChanged: reconcile()
    onVisibleChanged: {
        trace("visible request=" + visible);
        if (visible) startupGuard.restart();
        else startupGuard.stop();
        if (!visible) { requestedOpen = false; transition.stop(); reveal = 0; selectedWindowId = ""; }
    }
    Connections {
        target: effect.system
        ignoreUnknownSignals: true
        function onWindowRemoved() { effect.selectedWindowId = ""; }
    }
    NumberAnimation {
        id: transition
        target: effect; property: "reveal"; easing.type: Easing.OutCubic
        onFinished: { effect.trace("transition finished requestedOpen=" + effect.requestedOpen); if (!effect.requestedOpen) effect.visible = false; }
    }
    ShortcutHandler {
        objectName: "depthos-overview-shortcut"
        name: "depthos-spatial-overview-toggle"
        text: "Toggle DepthOS Native Overview"
        // Empty by default: preserve POC-01 and every existing global binding.
        sequence: ""
        onActivated: { effect.trace("shortcut callback"); effect.toggle(); }
    }
    delegate: FocusScope {
        id: outputRoot
        Component.onCompleted: {
            effect.constructedOutputs++;
            effect.trace("screen delegate created " + width + "x" + height);
        }
        Component.onDestruction: effect.constructedOutputs = Math.max(0, effect.constructedOutputs - 1)
        readonly property var nativeOutput: SceneView.screen
        focus: true
        Keys.onEscapePressed: effect.close()
        // Keep a working dismissal path even if the inner scene fails to load.
        Rectangle { anchors.fill: parent; color: "#0B1020" }
        Text {
            textFormat: Text.PlainText
            anchors.centerIn: parent; color: "#F8FAFC"
            text: scene.status === Loader.Error ? "DepthOS scene unavailable — Escape to close" : "Loading native state…"
        }
        Loader {
            id: scene
            anchors.fill: parent
            sourceComponent: OverviewScene {
                controller: effect
                system: effect.system
                windowSource: effect.windowSource
                output: outputRoot.nativeOutput
            }
            onLoaded: item.forceActiveFocus()
        }
    }
}
