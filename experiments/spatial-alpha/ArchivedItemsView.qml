import QtQuick
import "theme"
import "Presentation.js" as Display

FocusScope {
    id: root
    property bool isOpen: false
    property var hostBridge: null
    property var archivedItems: []
    property string activeTab: "all" // "all" | "world" | "workspace" | "desktop"
    property string statusMessage: ""
    property string errorMessage: ""
    property bool motionOff: false

    signal dismissed()
    signal itemRestored(string kind, string id)

    visible: opacity > 0 || isOpen
    enabled: isOpen
    opacity: isOpen ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: NeonTheme.ms(root.motionOff, NeonTheme.modalMs) }
    }

    onIsOpenChanged: {
        if (isOpen) {
            refresh();
            closeBtn.forceActiveFocus();
        }
    }

    function refresh() {
        root.errorMessage = "";
        root.statusMessage = "";
        if (!root.hostBridge) {
            root.archivedItems = [];
            return;
        }
        try {
            const raw = root.hostBridge.getArchivedItems();
            root.archivedItems = JSON.parse(raw);
        } catch (e) {
            root.archivedItems = [];
        }
    }

    function restoreItem(item) {
        root.errorMessage = "";
        if (!root.hostBridge) return;
        const req = {
            action: "restore",
            entity: item.kind,
            id: item.id,
            expectedVersion: item.version
        };
        try {
            const respRaw = root.hostBridge.executeManagement(JSON.stringify(req));
            const resp = JSON.parse(respRaw);
            if (resp.success) {
                root.statusMessage = "Restored " + item.name + " successfully";
                root.refresh();
                root.itemRestored(item.kind, item.id);
            } else {
                root.errorMessage = resp.error || "Could not restore item";
            }
        } catch (err) {
            root.errorMessage = "Restore failed: " + err;
        }
    }

    Keys.onEscapePressed: root.dismissed()

    Rectangle {
        anchors.fill: parent
        color: NeonTheme.scrim
        MouseArea {
            anchors.fill: parent
            onClicked: root.dismissed()
        }
    }

    SpatialGlass {
        width: Math.min(780, parent.width - 40)
        height: Math.min(580, parent.height - 50)
        anchors.centerIn: parent
        radius: 18
        rim: NeonTheme.cyan
        border.color: NeonTheme.border
        border.width: 1

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Column {
            anchors.fill: parent
            anchors.margins: 22
            spacing: 14

            Row {
                width: parent.width
                spacing: 12

                Column {
                    width: parent.width - closeBtn.width - 12
                    spacing: 2
                    NeonLabel { text: "SAFE STORAGE & RECOVERY"; color: NeonTheme.cyan }
                    NeonText {
                        text: "Archived Items"
                        font.pixelSize: 22
                        font.weight: Font.DemiBold
                    }
                }

                NeonButton {
                    id: closeBtn
                    text: "Close"
                    compact: true
                    onClicked: root.dismissed()
                }
            }

            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

            Row {
                spacing: 8
                NeonButton {
                    text: "All (" + root.archivedItems.length + ")"
                    compact: true
                    highlighted: root.activeTab === "all"
                    onClicked: root.activeTab = "all"
                }
                NeonButton {
                    text: "Worlds"
                    compact: true
                    highlighted: root.activeTab === "world"
                    onClicked: root.activeTab = "world"
                }
                NeonButton {
                    text: "Workspaces"
                    compact: true
                    highlighted: root.activeTab === "workspace"
                    onClicked: root.activeTab = "workspace"
                }
                NeonButton {
                    text: "Desktops"
                    compact: true
                    highlighted: root.activeTab === "desktop"
                    onClicked: root.activeTab = "desktop"
                }
            }

            NeonText {
                visible: root.errorMessage !== ""
                text: root.errorMessage
                color: NeonTheme.danger
                font.pixelSize: 12
            }

            NeonText {
                visible: root.statusMessage !== ""
                text: root.statusMessage
                color: NeonTheme.emerald
                font.pixelSize: 12
            }

            ListView {
                id: itemsList
                width: parent.width
                height: parent.height - 150
                clip: true
                spacing: 8

                model: root.archivedItems.filter(item => root.activeTab === "all" || item.kind === root.activeTab)

                delegate: Rectangle {
                    id: rowCard
                    required property var modelData
                    required property int index
                    width: itemsList.width
                    height: 52
                    radius: 8
                    color: NeonTheme.panel
                    border.color: NeonTheme.border
                    border.width: 1

                    NeonBadge {
                        id: kindBadge
                        anchors.left: parent.left
                        anchors.leftMargin: 14
                        anchors.verticalCenter: parent.verticalCenter
                        tone: {
                            switch (rowCard.modelData.kind) {
                            case "world": return NeonTheme.cyan;
                            case "workspace": return NeonTheme.lavender;
                            case "desktop": return NeonTheme.emerald;
                            default: return NeonTheme.precision;
                            }
                        }
                        label: rowCard.modelData.kind.toUpperCase()
                    }

                    Column {
                        anchors.left: kindBadge.right
                        anchors.leftMargin: 12
                        anchors.right: restoreBtn.left
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2

                        NeonText {
                            text: Display.safeText(rowCard.modelData.name)
                            font.pixelSize: 14
                            font.weight: Font.DemiBold
                            elide: Text.ElideRight
                            width: parent.width
                        }

                        NeonText {
                            text: {
                                let info = "Archived";
                                if (rowCard.modelData.child_count > 0)
                                    info += "  ·  " + rowCard.modelData.child_count + " children";
                                return info;
                            }
                            font.pixelSize: 11
                            color: NeonTheme.muted
                        }
                    }

                    NeonButton {
                        id: restoreBtn
                        text: "Restore"
                        compact: true
                        highlighted: true
                        accent: NeonTheme.emerald
                        anchors.right: parent.right
                        anchors.rightMargin: 14
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: root.restoreItem(rowCard.modelData)
                    }
                }

                Text {
                    anchors.centerIn: parent
                    visible: itemsList.count === 0
                    text: "No archived items in this category."
                    color: NeonTheme.muted
                    font.pixelSize: 13
                }
            }
        }
    }
}
