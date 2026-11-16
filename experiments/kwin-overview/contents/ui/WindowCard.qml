import QtQuick
import org.kde.kwin
import org.kde.kirigami as Kirigami
import "Policy.js" as Policy

Rectangle {
    id: card
    required property var client
    required property var system
    required property var controller
    property int previewIndex: 0
    property bool selected: false
    signal chosen()
    readonly property string previewReason: Policy.previewReason(client, system, controller.previewsEnabled && controller.level === 1, previewIndex, 6)
    onPreviewReasonChanged: if (controller && typeof controller.trace === "function") controller.trace("preview policy=" + previewReason)
    readonly property bool focusedWindow: !!client && !!system && system.activeWindow === client
    objectName: "native-window-card"
    radius: 14
    color: "#1E293B"
    border.color: selected || pointer.containsMouse ? "#818CF8" : (focusedWindow ? "#94A3B8" : "#354158")
    border.width: selected || focusedWindow ? 2 : 1
    clip: true

    Rectangle {
        x: 14; y: 12; width: 24; height: 24; radius: 5
        visible: !icon.valid
        color: "#354158"
        Text { textFormat: Text.PlainText; anchors.centerIn: parent; text: "□"; color: "#F8FAFC"; font.pixelSize: 18 }
    }
    Kirigami.Icon {
        id: icon
        x: 14; y: 12; width: 24; height: 24
        source: client ? client.icon : "application-x-executable"
        fallback: "application-x-executable"
    }
    Text {
        textFormat: Text.PlainText
        objectName: "window-title"
        anchors.left: icon.right; anchors.leftMargin: 9; anchors.right: parent.right; anchors.rightMargin: 14
        y: 13; color: "#F8FAFC"; font.pixelSize: 14; elide: Text.ElideRight
        text: Policy.title(card.client, controller.showTitles)
    }
    Item {
        x: 12; y: 44; width: parent.width - 24; height: Math.max(30, parent.height - 108)
        clip: true
        Text {
            objectName: "native-preview-fallback"
            visible: card.previewReason !== ""
            textFormat: Text.PlainText
            anchors.centerIn: parent; width: parent.width - 16; horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap; color: "#94A3B8"; font.pixelSize: 12
            text: card.previewReason
        }
        Text {
            id: previewStatus
            objectName: "native-preview-status"
            visible: card.previewReason === ""
            textFormat: Text.PlainText
            anchors.left: parent.left; anchors.right: parent.right; anchors.bottom: parent.bottom
            height: 30; horizontalAlignment: Text.AlignHCenter; verticalAlignment: Text.AlignVCenter
            wrapMode: Text.WordWrap; color: "#94A3B8"; font.pixelSize: 10
            text: "Native preview · frame availability depends on KWin"
        }
        Loader {
            objectName: "native-preview-loader"
            anchors.left: parent.left; anchors.right: parent.right; anchors.top: parent.top
            anchors.bottom: previewStatus.top
            active: card.previewReason === ""
            sourceComponent: WindowThumbnail { client: card.client }
        }
    }
    Text {
        textFormat: Text.PlainText
        x: 14; y: parent.height - 56; width: parent.width - 28; elide: Text.ElideRight
        text: Policy.membership(card.client); color: "#94A3B8"; font.pixelSize: 11
    }
    Text {
        textFormat: Text.PlainText
        objectName: "window-state"
        x: 14; y: parent.height - 33; width: parent.width - 28; elide: Text.ElideRight
        text: [(controller.logicalBackend ? controller.logicalBackend.badge(Policy.read(client,"internalId")) : "backend unavailable"), card.focusedWindow ? "Focused" : "", Policy.read(client, "minimized") === true ? "Minimized" : "", Policy.read(client, "fullScreen") === true ? "Fullscreen" : ""].filter(s => s).join(" · ")
        color: "#818CF8"; font.pixelSize: 12
    }
    MouseArea {
        id: pointer
        anchors.fill: parent; hoverEnabled: true
        onContainsMouseChanged: if (controller && typeof controller.trace === "function") controller.trace("card hover=" + containsMouse)
        onClicked: { if (controller && typeof controller.trace === "function") controller.trace("card clicked"); card.chosen(); }
    }
}
