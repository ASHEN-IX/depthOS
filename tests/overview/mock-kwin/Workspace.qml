pragma Singleton
import QtQuick
QtObject {
    property string previewPath: ""
    property QtObject firstDesktop: QtObject { property string id: "desk-a"; property string name: "Desktop A" }
    property QtObject secondDesktop: QtObject { property string id: "desk-b"; property string name: "Desktop B" }
    property QtObject firstOutput: QtObject {
        property string name: "output-a"
        property var geometry: Qt.rect(0, 0, 1920, 1080)
        property real devicePixelRatio: 1
    }
    property var desktops: [firstDesktop, secondDesktop]
    property var activities: ["activity-a", "activity-b"]
    property var screens: [firstOutput]
    property var currentDesktop: firstDesktop
    property string currentActivity: "activity-a"
    property var activeWindow: null
    signal windowRemoved(var window)
}
