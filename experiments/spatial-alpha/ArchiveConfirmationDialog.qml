import QtQuick
import "theme"

FocusScope {
    id: root
    property bool isOpen: false
    property string entity: "world" // "world" | "workspace" | "desktop"
    property string entityId: ""
    property string entityName: ""
    property int expectedVersion: 1
    property int childCount: 0
    property int ruleCount: 0
    property var hostBridge: null
    property bool motionOff: false

    signal confirmed(string entity, string entityId, int expectedVersion)
    signal dismissed()

    visible: opacity > 0 || isOpen
    enabled: isOpen
    opacity: isOpen ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: NeonTheme.ms(root.motionOff, NeonTheme.modalMs) }
    }

    onIsOpenChanged: {
        if (isOpen) {
            cancelBtn.forceActiveFocus();
            if (root.hostBridge && root.entityId) {
                root.ruleCount = root.hostBridge.getAssociationImpact(root.entity, root.entityId);
            }
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
        width: Math.min(540, parent.width - 40)
        height: dialogColumn.implicitHeight + 48
        anchors.centerIn: parent
        radius: 18
        rim: NeonTheme.warning
        border.color: NeonTheme.warning
        border.width: 1

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Column {
            id: dialogColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 24
            spacing: 16

            Row {
                width: parent.width
                spacing: 10

                Rectangle {
                    width: 32
                    height: 32
                    radius: 16
                    color: NeonTheme.tint(NeonTheme.warning, 0.2)
                    border.color: NeonTheme.warning
                    border.width: 1
                    anchors.verticalCenter: parent.verticalCenter

                    NeonText {
                        anchors.centerIn: parent
                        text: "!"
                        font.pixelSize: 16
                        font.weight: Font.Bold
                        color: NeonTheme.warning
                    }
                }

                Column {
                    width: parent.width - 44
                    spacing: 2
                    anchors.verticalCenter: parent.verticalCenter

                    NeonLabel {
                        text: "ARCHIVE CONFIRMATION"
                        color: NeonTheme.warning
                    }
                    NeonText {
                        text: "Archive " + root.entity.toUpperCase() + ": " + root.entityName
                        font.pixelSize: 18
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        width: parent.width
                    }
                }
            }

            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

            NeonText {
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Archiving hides this " + root.entity + " from the active spatial overview. No files or records are deleted permanently."
                color: NeonTheme.text
                font.pixelSize: 14
            }

            Column {
                width: parent.width
                spacing: 6

                Row {
                    spacing: 8
                    visible: root.childCount > 0
                    NeonBadge {
                        tone: NeonTheme.warning
                        label: root.childCount + " Descendant Items"
                    }
                    NeonText {
                        text: "will be archived together with this parent."
                        color: NeonTheme.precision
                        font.pixelSize: 12
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }

                Row {
                    spacing: 8
                    visible: root.ruleCount > 0
                    NeonBadge {
                        tone: NeonTheme.warning
                        label: root.ruleCount + " Association Rules"
                    }
                    NeonText {
                        text: "point to this item and will safely become dormant."
                        color: NeonTheme.precision
                        font.pixelSize: 12
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }

                Row {
                    spacing: 8
                    NeonBadge {
                        tone: NeonTheme.emerald
                        label: "Safe & Reversible"
                    }
                    NeonText {
                        text: "You can restore this item at any time from Archived Items."
                        color: NeonTheme.precision
                        font.pixelSize: 12
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
            }

            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

            Row {
                anchors.right: parent.right
                spacing: 12

                NeonButton {
                    id: cancelBtn
                    text: "Cancel"
                    onClicked: root.dismissed()
                }

                NeonButton {
                    id: confirmBtn
                    objectName: "confirmArchiveButton"
                    text: "Confirm Archive"
                    highlighted: true
                    accent: NeonTheme.danger
                    onClicked: {
                        root.confirmed(root.entity, root.entityId, root.expectedVersion);
                        root.dismissed();
                    }
                }
            }
        }
    }
}
