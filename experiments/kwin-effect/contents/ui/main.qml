// KWin 6.8.1 incubates each screen delegate in a new child QQmlContext.
// Bound components reject that context even though createObject() tests pass.
pragma ComponentBehavior: Unbound

import QtQuick
import org.kde.kwin

SceneEffect {
    id: effect

    // One lifecycle for all output delegates. Keep visible true through exit:
    // KWin releases input and destroys the views when it becomes false.
    property bool requestedOpen: false
    property bool selected: false
    property real cardScale: 0.75
    property real cardOpacity: 0
    property real cardOffset: 40
    property real backdropOpacity: 0
    // Temporary POC diagnostics; disable in source after live acceptance.
    property bool diagnosticsEnabled: true

    function trace(message) {
        if (diagnosticsEnabled)
            console.info("[depthos-spatial-poc] " + message);
    }

    Component.onCompleted: trace("initialized; shortcut handler constructed")

    function open() {
        leaving.stop();
        entering.stop();
        if (!visible) {
            cardScale = 0.75;
            cardOpacity = 0;
            cardOffset = 40;
            backdropOpacity = 0;
            selected = false;
        }
        requestedOpen = true;
        visible = true;
        entering.start();
    }

    function close() {
        if (!visible || !requestedOpen)
            return;
        requestedOpen = false;
        entering.stop();
        leaving.start();
    }

    function toggle() {
        if (requestedOpen && visible)
            close();
        else
            open();
    }

    onVisibleChanged: {
        trace("visible request=" + visible);
        if (!visible) {
            requestedOpen = false;
            entering.stop();
            leaving.stop();
        }
    }

    ShortcutHandler {
        objectName: "depthos-spatial-poc-shortcut"
        name: "depthos-spatial-poc-toggle"
        text: "Toggle DepthOS Spatial POC"
        sequence: "Meta+Ctrl+Shift+F11"
        onActivated: {
            effect.trace("shortcut callback");
            effect.toggle();
        }
    }

    // Omitting 'from' resumes from the current values on interrupted transitions.
    ParallelAnimation {
        id: entering
        NumberAnimation { target: effect; property: "cardScale"; to: 1; duration: 300; easing.type: Easing.OutCubic }
        NumberAnimation { target: effect; property: "cardOpacity"; to: 1; duration: 300; easing.type: Easing.OutCubic }
        NumberAnimation { target: effect; property: "cardOffset"; to: 0; duration: 300; easing.type: Easing.OutCubic }
        NumberAnimation { target: effect; property: "backdropOpacity"; to: 1; duration: 120; easing.type: Easing.OutCubic }
        onFinished: effect.trace("entrance finished")
    }

    ParallelAnimation {
        id: leaving
        NumberAnimation { target: effect; property: "cardScale"; to: 0.85; duration: 230; easing.type: Easing.InOutCubic }
        NumberAnimation { target: effect; property: "cardOpacity"; to: 0; duration: 230; easing.type: Easing.InOutCubic }
        NumberAnimation { target: effect; property: "cardOffset"; to: 24; duration: 230; easing.type: Easing.InOutCubic }
        NumberAnimation { target: effect; property: "backdropOpacity"; to: 0; duration: 230; easing.type: Easing.InOutCubic }
        onFinished: {
            effect.trace("exit finished");
            if (!effect.requestedOpen)
                effect.visible = false;
        }
    }

    delegate: FocusScope {
        id: scene
        focus: true
        clip: true
        readonly property real unit: Math.max(0.25, Math.min(1, width / 960, height / 640))
        Component.onCompleted: effect.trace("screen delegate created " + width + "x" + height)

        Keys.onEscapePressed: event => {
            effect.close();
            event.accepted = true;
        }
        Keys.onPressed: event => {
            if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter || event.key === Qt.Key_Space) {
                effect.selected = !effect.selected;
                event.accepted = true;
            }
        }

        Rectangle {
            anchors.fill: parent
            color: "#0B1020"
            opacity: effect.backdropOpacity
        }

        MouseArea {
            anchors.fill: parent
            acceptedButtons: Qt.AllButtons
            onClicked: effect.close()
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            y: parent.height / 2 - 245 * scene.unit
            text: "DepthOS"
            color: "#F8FAFC"
            font.pixelSize: 36 * scene.unit
            font.weight: Font.DemiBold
            opacity: effect.cardOpacity
        }

        Item {
            id: card
            width: 480 * scene.unit
            height: 280 * scene.unit
            anchors.centerIn: parent
            anchors.verticalCenterOffset: effect.cardOffset * scene.unit
            scale: effect.cardScale
            opacity: effect.cardOpacity

            Rectangle {
                x: -8 * scene.unit
                y: 8 * scene.unit
                width: parent.width + 16 * scene.unit
                height: parent.height + 16 * scene.unit
                radius: 30 * scene.unit
                color: "#060914"
                opacity: 0.6
            }

            Rectangle {
                anchors.fill: parent
                radius: 24 * scene.unit
                color: "#1E293B"
                border.width: effect.selected ? 2 : 1
                border.color: effect.selected || cardMouse.containsMouse ? "#818CF8" : "#354158"

                Column {
                    anchors.centerIn: parent
                    spacing: 18 * scene.unit
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "PROJECTS"
                        color: "#F8FAFC"
                        font.pixelSize: 25 * scene.unit
                        font.weight: Font.DemiBold
                        font.letterSpacing: 4 * scene.unit
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "Spatial Layer"
                        color: "#94A3B8"
                        font.pixelSize: 19 * scene.unit
                    }
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: effect.selected ? "Selected · preview only" : "Select to explore"
                        color: "#818CF8"
                        font.pixelSize: 14 * scene.unit
                    }
                }

                MouseArea {
                    id: cardMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    acceptedButtons: Qt.AllButtons
                    onClicked: mouse => {
                        if (mouse.button === Qt.LeftButton)
                            effect.selected = !effect.selected;
                        scene.forceActiveFocus();
                    }
                }
            }
        }

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            y: parent.height / 2 + 200 * scene.unit
            text: "ESC — Return to Desktop"
            color: "#94A3B8"
            font.pixelSize: 16 * scene.unit
            opacity: effect.cardOpacity
        }
    }
}
