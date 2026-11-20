import QtQuick
import QtQuick.Window

Window {
    id: rootWindow
    width: 1440
    height: 900
    visible: true
    title: "DepthOS Spatial Workspace Alpha"
    color: "#080C14"

    property bool reducedMotion: false
    property alias backend: spatialBackend
    property alias scene: spatialScene
    // Set by the Python launcher. Null in qmltestrunner; QML must not require it.
    property var hostBridge: null

    SpatialBackend {
        id: spatialBackend
    }

    SpatialScene {
        id: spatialScene
        anchors.fill: parent
        backend: spatialBackend
        reducedMotion: rootWindow.reducedMotion

        onClosed: {
            rootWindow.close();
        }
    }

    Connections {
        target: spatialBackend
        function onCreateWorkspaceRequested() {
            if (rootWindow.hostBridge)
                rootWindow.hostBridge.createWorkspace();
        }
        function onRetryRequested() {
            if (rootWindow.hostBridge)
                rootWindow.hostBridge.retryProbe();
        }
    }

    Component.onCompleted: {
        spatialScene.forceActiveFocus();
    }
}
