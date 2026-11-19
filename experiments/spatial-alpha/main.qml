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

    Component.onCompleted: {
        spatialScene.forceActiveFocus();
    }
}
