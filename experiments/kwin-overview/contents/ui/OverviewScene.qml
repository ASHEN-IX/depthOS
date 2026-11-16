import QtQuick
import org.kde.kwin
import "Policy.js" as Policy

FocusScope {
    id: scene
    required property var controller
    required property var system
    required property var windowSource
    required property var output
    readonly property bool available: !!system && !!windowSource && !!controller.selectedDesktop
    property int desktopIndex: 0
    property int activityIndex: 0
    property real layerReveal: 1
    readonly property string selectedDesktopName: Policy.displayText(Policy.read(controller.selectedDesktop, "name")) || Policy.displayText(Policy.read(controller.selectedDesktop, "id")) || "Unavailable"
    focus: true
    clip: true
    objectName: "depthos-overview-scene"

    function choose() {
        if (typeof controller.trace === "function") controller.trace("choose level=" + controller.level);
        if (controller.level === 3 && controller.activities && controller.activities.length)
            controller.inspectActivity(controller.activities[Math.min(activityIndex, controller.activities.length - 1)]);
        else if (controller.level === 2 && controller.desktops && controller.desktops.length)
            controller.inspectDesktop(controller.desktops[Math.min(desktopIndex, controller.desktops.length - 1)]);
        else if (windowGrid.currentItem) windowGrid.currentItem.chooseWindow();
    }
    function move(delta) {
        if (typeof controller.trace === "function") controller.trace("move delta=" + delta + " level=" + controller.level);
        if (controller.level === 3) activityIndex = Math.max(0, Math.min((controller.activities || []).length - 1, activityIndex + delta));
        else if (controller.level === 2) desktopIndex = Math.max(0, Math.min((controller.desktops || []).length - 1, desktopIndex + delta));
        else windowGrid.currentIndex = Math.max(0, Math.min(windowGrid.count - 1, windowGrid.currentIndex + delta));
    }
    Keys.onPressed: event => {
        if (event.key === Qt.Key_Escape) controller.close();
        else if (event.key === Qt.Key_Right) move(1);
        else if (event.key === Qt.Key_Left) move(-1);
        else if (event.key === Qt.Key_Down) move(controller.level === 1 ? Math.max(1, Math.floor(windowGrid.width / windowGrid.cellWidth)) : 1);
        else if (event.key === Qt.Key_Up) move(controller.level === 1 ? -Math.max(1, Math.floor(windowGrid.width / windowGrid.cellWidth)) : -1);
        else if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) choose();
        else if (event.key === Qt.Key_Plus || event.key === Qt.Key_Equal) controller.zoomIn();
        else if (event.key === Qt.Key_Minus || event.key === Qt.Key_Backspace) controller.zoomOut();
        else return;
        if (typeof controller.trace === "function") controller.trace("key=" + event.key + " level=" + controller.level);
        event.accepted = true;
    }
    Connections {
        target: controller
        function onLevelChanged() {
            scene.desktopIndex = Math.max(0, (controller.desktops || []).indexOf(controller.selectedDesktop));
            scene.activityIndex = Math.max(0, (controller.activities || []).indexOf(controller.selectedActivity));
            layerReveal = 0; depthAnimation.restart();
        }
        function onSelectedDesktopChanged() { scene.desktopIndex = Math.max(0, (controller.desktops || []).indexOf(controller.selectedDesktop)); }
        function onSelectedActivityChanged() { scene.activityIndex = Math.max(0, (controller.activities || []).indexOf(controller.selectedActivity)); }
    }
    NumberAnimation { id: depthAnimation; target: scene; property: "layerReveal"; to: 1; duration: controller.motionOff ? 0 : 180; easing.type: Easing.OutCubic }

    Rectangle { anchors.fill: parent; color: "#0B1020" }
    Loader {
        anchors.fill: parent
        active: controller.reveal < 1 && controller.previewsEnabled && scene.available
        scale: 1 - controller.reveal * 0.18
        opacity: 1 - controller.reveal
        sourceComponent: DesktopCanvas { system: scene.system; source: scene.windowSource; output: scene.output; previewsEnabled: controller.previewsEnabled }
    }
    MouseArea { anchors.fill: parent; onClicked: controller.close() }
    Item {
        id: content
        anchors.fill: parent
        opacity: controller.reveal
        scale: (0.85 + controller.reveal * 0.15) * (controller.level === 1 ? 1 : controller.level === 2 ? 0.96 : 0.92)
        Behavior on scale { NumberAnimation { duration: controller.motionOff ? 0 : 180; easing.type: Easing.OutCubic } }
        Text { textFormat: Text.PlainText; x: 32; y: 22; text: "DepthOS  /  Z" + controller.level; color: "#F8FAFC"; font.pixelSize: 26; font.weight: Font.DemiBold }
        Row {
            anchors.right: parent.right; anchors.rightMargin: 32; y: 22; spacing: 8
            Action { text: "− Zoom out"; enabled: controller.level < controller.maxLevel; onTriggered: controller.zoomOut() }
            Action { text: "+ Zoom in"; onTriggered: controller.zoomIn() }
            Action { text: "Close · Esc"; onTriggered: controller.close() }
        }
        Text {
            textFormat: Text.PlainText
            objectName: "native-context-status"
            x: 32; y: 72; width: parent.width - 64; elide: Text.ElideRight; color: "#94A3B8"; font.pixelSize: 14
            text: "Active desktop: " + (Policy.displayText(Policy.read(system ? system.currentDesktop : null, "name")) || "Unavailable") +
                "  |  Inspecting desktop: " + scene.selectedDesktopName + "  |  Active Activity: " + (system ? system.currentActivity : "Unavailable") +
                "  |  Inspecting Activity: " + (controller.selectedActivity || "Unavailable")
        }
        Text {
            textFormat: Text.PlainText
            x: 32; y: 97; width: parent.width - 64; elide: Text.ElideRight; color: "#818CF8"; font.pixelSize: 12
            text: "Source: native KWin models · observer independent · selection never switches desktops or activates windows"
        }
        LogicalPanel {
            x: 32; y: parent.height - 175; width: parent.width - 64; height: 72
            backend: controller.logicalBackend || null
        }
        WindowFilterModel {
            id: filtered
            windowModel: scene.windowSource
            desktop: controller.selectedDesktop
            activity: controller.selectedActivity
            windowType: WindowFilterModel.Normal | WindowFilterModel.Dialog
            minimizedWindows: true
        }
        GridView {
            id: windowGrid
            onCountChanged: if (controller && typeof controller.trace === "function") controller.trace("native filtered rows=" + count)
            objectName: "native-window-grid"
            x: 32; y: 142; width: parent.width - 64; height: Math.max(1, parent.height - 360)
            visible: controller.level === 1
            opacity: scene.layerReveal
            clip: true
            cellWidth: width / Math.max(1, Math.floor(width / 320))
            cellHeight: Math.max(160, Math.min(260, height))
            model: scene.available ? filtered : null
            currentIndex: count ? 0 : -1
            delegate: Item {
                id: windowDelegate
                required property var window
                required property int index
                width: windowGrid.cellWidth; height: windowGrid.cellHeight
                function chooseWindow() {
                    controller.selectedWindowId = String(Policy.read(window, "internalId") || "");
                    windowGrid.currentIndex = index;
                }
                WindowCard {
                    anchors.fill: parent; anchors.margins: 8
                    client: windowDelegate.window; system: scene.system; controller: scene.controller
                    previewIndex: windowDelegate.index
                    selected: windowGrid.currentIndex === windowDelegate.index
                    onChosen: windowDelegate.chooseWindow()
                }
            }
        }
        Text {
            textFormat: Text.PlainText
            anchors.centerIn: windowGrid; width: windowGrid.width - 32; horizontalAlignment: Text.AlignHCenter
            visible: controller.level === 1 && (!scene.available || windowGrid.count === 0)
            color: "#94A3B8"; font.pixelSize: 18
            text: !scene.available ? "Native state unavailable — no cached cards shown" : "No normal or dialog windows in this desktop/Activity"
        }
        GridView {
            id: contexts
            objectName: "native-context-grid"
            x: 32; y: 142; width: parent.width - 64; height: Math.max(1, parent.height - 360)
            visible: controller.level > 1; opacity: scene.layerReveal; clip: true
            cellWidth: width / Math.max(1, Math.floor(width / 320)); cellHeight: 190
            model: controller.level === 2 ? controller.desktops : controller.activities
            delegate: Item {
                objectName: "native-context-card"
                required property var modelData
                required property int index
                width: contexts.cellWidth; height: contexts.cellHeight
                Rectangle {
                    anchors.fill: parent; anchors.margins: 8; radius: 16; color: "#1E293B"
                    border.width: 2
                    border.color: (controller.level === 2 ? scene.desktopIndex : scene.activityIndex) === index || contextMouse.containsMouse ? "#818CF8" : "#354158"
                    Column {
                        anchors.centerIn: parent; width: parent.width - 28; spacing: 15
                        Text {
                            textFormat: Text.PlainText
                            width: parent.width; horizontalAlignment: Text.AlignHCenter; elide: Text.ElideRight
                            text: controller.level === 2 ? (Policy.displayText(Policy.read(modelData, "name")) || Policy.displayText(Policy.read(modelData, "id")) || "Unnamed desktop") : Policy.displayText(String(modelData))
                            color: "#F8FAFC"; font.pixelSize: 18
                        }
                        Text {
                            textFormat: Text.PlainText
                            width: parent.width; horizontalAlignment: Text.AlignHCenter
                            text: controller.level === 2 ? (modelData === (system ? system.currentDesktop : null) ? "Active KDE desktop" : "Inspect windows · no desktop switch") : (modelData === (system ? system.currentActivity : "") ? "Active KDE Activity" : "Inspect desktops · no Activity switch")
                            color: "#94A3B8"; font.pixelSize: 12
                        }
                    }
                    MouseArea {
                        id: contextMouse; anchors.fill: parent; hoverEnabled: true
                        onClicked: { if (controller.level === 2) controller.inspectDesktop(modelData); else controller.inspectActivity(modelData); }
                    }
                }
            }
        }
        Row {
            x: 32; y: parent.height - 94; spacing: 10
            Action { text: controller.showTitles ? "Hide titles" : "Show titles"; onTriggered: controller.showTitles = !controller.showTitles }
            Action { text: controller.previewsEnabled ? "Disable previews" : "Enable previews"; onTriggered: controller.previewsEnabled = !controller.previewsEnabled }
            Action { text: controller.reducedMotion ? "Motion: reduced" : "Reduce motion"; onTriggered: controller.reducedMotion = !controller.reducedMotion }
        }
        Text {
            textFormat: Text.PlainText
            x: 32; y: parent.height - 42; width: parent.width - 64; elide: Text.ElideRight; color: "#94A3B8"; font.pixelSize: 12
            text: "Outputs: " + (system && system.screens ? system.screens.map(s => Policy.displayText(Policy.read(s, "name")) || "Unavailable").join(" · ") : "Unavailable") +
                "  |  This output: " + (Policy.displayText(Policy.read(output, "name")) || "Unavailable") +
                "  |  Output desktop: " + (Policy.displayText(Policy.read(Policy.currentDesktopForOutput(system, output), "name")) || "Unavailable") +
                "  |  Arrows / Enter / + / − / Esc"
        }
    }
}
