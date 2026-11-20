import QtQuick
import "theme"
import "Presentation.js" as Display

FocusScope {
    id: root
    property bool isOpen: false
    property string mode: "create" // "create" | "edit"
    property string worldId: ""
    property string initialName: ""
    property string initialDescription: ""
    property string initialVisualIdentity: "neon-cyan"
    property string initialArtwork: "cyber-grid"
    property int version: 1
    property var hostBridge: null
    property string errorMessage: ""
    property bool isConflict: false
    property bool motionOff: false

    signal saved(var worldData)
    signal archiveRequested(string id, string name, int version)
    signal reorderRequested(string id, int delta)
    signal dismissed()

    visible: opacity > 0 || isOpen
    enabled: isOpen
    opacity: isOpen ? 1 : 0
    Behavior on opacity {
        NumberAnimation { duration: NeonTheme.ms(root.motionOff, NeonTheme.modalMs) }
    }

    readonly property var visualIdentities: [
        { id: "neon-cyan", label: "Cyan", color: NeonTheme.cyan },
        { id: "neon-magenta", label: "Magenta", color: NeonTheme.lavender },
        { id: "neon-amber", label: "Amber", color: NeonTheme.warning },
        { id: "neon-emerald", label: "Emerald", color: NeonTheme.emerald },
        { id: "matrix-green", label: "Matrix", color: NeonTheme.mint }
    ]

    readonly property var artworkOptions: [
        "cyber-grid", "cyber-city", "neural-net", "deep-space",
        "projects", "research", "social", "focus"
    ]

    property string selectedIdentity: initialVisualIdentity
    property string selectedArtwork: initialArtwork

    onIsOpenChanged: {
        if (isOpen) {
            root.errorMessage = "";
            root.isConflict = false;
            nameField.text = root.initialName;
            descField.text = root.initialDescription;
            root.selectedIdentity = root.initialVisualIdentity || "neon-cyan";
            root.selectedArtwork = root.initialArtwork || "cyber-grid";
            nameField.forceActiveFocus();
        }
    }

    Keys.onEscapePressed: root.dismissed()

    function save() {
        const name = nameField.text.trim();
        if (!name) {
            root.errorMessage = "World name cannot be empty";
            return;
        }
        root.errorMessage = "";
        root.isConflict = false;

        if (!root.hostBridge) {
            root.saved({ id: root.worldId, name: name });
            root.dismissed();
            return;
        }

        const req = {
            action: root.mode === "create" ? "create" : "update",
            entity: "world",
            id: root.worldId || undefined,
            expectedVersion: root.mode === "edit" ? root.version : undefined,
            payload: {
                name: name,
                description: descField.text.trim(),
                visualIdentity: root.selectedIdentity,
                artwork: root.selectedArtwork
            }
        };

        try {
            const respRaw = root.hostBridge.executeManagement(JSON.stringify(req));
            const resp = JSON.parse(respRaw);
            if (resp.success) {
                root.saved(resp);
                root.dismissed();
            } else {
                root.errorMessage = resp.error || "Save failed";
                if (resp.code === "conflict") {
                    root.isConflict = true;
                }
            }
        } catch (e) {
            root.errorMessage = "Execution error: " + e;
        }
    }

    Rectangle {
        anchors.fill: parent
        color: NeonTheme.scrim
        MouseArea {
            anchors.fill: parent
            onClicked: root.dismissed()
        }
    }

    SpatialGlass {
        width: Math.min(580, parent.width - 40)
        height: Math.min(contentColumn.implicitHeight + 48, parent.height - 40)
        anchors.centerIn: parent
        radius: 18
        rim: NeonTheme.cyan
        border.color: NeonTheme.cyan
        border.width: 1

        MouseArea {
            anchors.fill: parent
            onClicked: mouse => mouse.accepted = true
        }

        Column {
            id: contentColumn
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 24
            spacing: 14

            Row {
                width: parent.width
                spacing: 10

                Column {
                    width: parent.width - closeBtn.width - 10
                    spacing: 2
                    NeonLabel {
                        text: root.mode === "create" ? "NEW WORLD" : "WORLD SETTINGS"
                        color: NeonTheme.cyan
                    }
                    NeonText {
                        text: root.mode === "create" ? "Create Spatial World" : "Edit World: " + root.initialName
                        font.pixelSize: 20
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                        width: parent.width
                    }
                }

                NeonButton {
                    id: closeBtn
                    text: "Cancel"
                    compact: true
                    onClicked: root.dismissed()
                }
            }

            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

            Rectangle {
                visible: root.isConflict
                width: parent.width
                height: 40
                radius: 6
                color: "#38240D"
                border.color: NeonTheme.warning
                border.width: 1

                Row {
                    anchors.centerIn: parent
                    spacing: 10
                    NeonText {
                        text: "Conflict detected: World was modified concurrently."
                        color: NeonTheme.warning
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                        anchors.verticalCenter: parent.verticalCenter
                    }
                    NeonButton {
                        text: "Reload Data"
                        compact: true
                        highlighted: true
                        accent: NeonTheme.warning
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked: {
                            if (root.hostBridge) root.hostBridge.reloadHierarchy();
                            root.dismissed();
                        }
                    }
                }
            }

            NeonText {
                visible: root.errorMessage !== "" && !root.isConflict
                text: root.errorMessage
                color: NeonTheme.danger
                font.pixelSize: 12
            }

            CyberTextField {
                id: nameField
                objectName: "worldNameInput"
                width: parent.width
                label: "WORLD NAME"
                placeholderText: "e.g. Core Engineering"
                onAccepted: root.save()
            }

            CyberTextField {
                id: descField
                objectName: "worldDescriptionInput"
                width: parent.width
                label: "DESCRIPTION (OPTIONAL)"
                placeholderText: "e.g. Systems development, kernel, and tooling"
                onAccepted: root.save()
            }

            Column {
                width: parent.width
                spacing: 6
                NeonLabel { text: "VISUAL IDENTITY & THEME TONE" }
                Row {
                    spacing: 8
                    Repeater {
                        model: root.visualIdentities
                        delegate: Rectangle {
                            id: toneChip
                            required property var modelData
                            width: 80
                            height: 28
                            radius: 14
                            color: root.selectedIdentity === modelData.id ? NeonTheme.tint(modelData.color, 0.3) : NeonTheme.panel
                            border.color: root.selectedIdentity === modelData.id ? modelData.color : NeonTheme.border
                            border.width: 1

                            Row {
                                anchors.centerIn: parent
                                spacing: 6
                                Rectangle {
                                    width: 8
                                    height: 8
                                    radius: 4
                                    color: toneChip.modelData.color
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                                NeonText {
                                    text: toneChip.modelData.label
                                    font.pixelSize: 11
                                    color: toneChip.modelData.color
                                    anchors.verticalCenter: parent.verticalCenter
                                }
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.selectedIdentity = toneChip.modelData.id
                            }
                        }
                    }
                }
            }

            Column {
                width: parent.width
                spacing: 6
                NeonLabel { text: "ARTWORK IDENTITY" }
                Flow {
                    width: parent.width
                    spacing: 6
                    Repeater {
                        model: root.artworkOptions
                        delegate: Rectangle {
                            id: artChip
                            required property var modelData
                            width: 100
                            height: 26
                            radius: 6
                            color: root.selectedArtwork === modelData ? NeonTheme.tint(NeonTheme.cyan, 0.25) : NeonTheme.panel
                            border.color: root.selectedArtwork === modelData ? NeonTheme.cyan : NeonTheme.border
                            border.width: 1

                            NeonText {
                                anchors.centerIn: parent
                                text: artChip.modelData
                                font.pixelSize: 11
                                color: root.selectedArtwork === artChip.modelData ? NeonTheme.cyan : NeonTheme.precision
                            }

                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.selectedArtwork = artChip.modelData
                            }
                        }
                    }
                }
            }

            Rectangle { width: parent.width; height: 1; color: NeonTheme.border }

            Item {
                width: parent.width
                height: 36

                Row {
                    spacing: 8
                    visible: root.mode === "edit"
                    anchors.left: parent.left
                    anchors.verticalCenter: parent.verticalCenter

                    NeonButton {
                        text: "↑ Move Earlier"
                        compact: true
                        onClicked: root.reorderRequested(root.worldId, -1)
                    }
                    NeonButton {
                        text: "↓ Move Later"
                        compact: true
                        onClicked: root.reorderRequested(root.worldId, 1)
                    }
                    NeonButton {
                        objectName: "archiveWorldButton"
                        text: "Archive…"
                        compact: true
                        highlighted: true
                        accent: NeonTheme.danger
                        onClicked: {
                            root.archiveRequested(root.worldId, root.initialName, root.version);
                            root.dismissed();
                        }
                    }
                }

                NeonButton {
                    id: saveBtn
                    objectName: "saveWorldButton"
                    text: root.mode === "create" ? "Create World" : "Save Changes"
                    highlighted: true
                    accent: NeonTheme.cyan
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    enabled: nameField.text.trim().length > 0
                    onClicked: root.save()
                }
            }
        }
    }
}
