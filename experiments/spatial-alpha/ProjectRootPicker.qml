import QtQuick
import "theme"

Item {
    id: root
    property string label: "PROJECT DIRECTORY"
    property string path: ""
    property var hostBridge: null
    property var inspection: null
    property bool isValid: inspection ? (inspection.exists && inspection.isDirectory && inspection.isAbsolute) : false

    implicitWidth: 380
    implicitHeight: 110

    onPathChanged: {
        inspectTimer.restart();
    }

    Timer {
        id: inspectTimer
        interval: 200
        repeat: false
        onTriggered: {
            if (!root.hostBridge || !root.path.trim()) {
                root.inspection = null;
                return;
            }
            try {
                const res = root.hostBridge.inspectProject(root.path.trim());
                root.inspection = JSON.parse(res);
            } catch (e) {
                root.inspection = null;
            }
        }
    }

    Column {
        anchors.fill: parent
        spacing: 6

        CyberTextField {
            id: pathInput
            width: parent.width
            label: root.label
            text: root.path
            placeholderText: "/home/user/projects/my-app"
            onTextModified: root.path = pathInput.text
        }

        Row {
            spacing: 8
            width: parent.width
            visible: root.path.trim() !== ""

            NeonBadge {
                visible: root.inspection !== null
                tone: root.isValid ? NeonTheme.emerald : NeonTheme.danger
                label: {
                    if (!root.inspection) return "Checking…";
                    if (!root.inspection.isAbsolute) return "Relative path (needs absolute)";
                    if (!root.inspection.exists) return "Directory does not exist";
                    if (!root.inspection.isDirectory) return "Not a directory";
                    if (root.inspection.git && root.inspection.git.isRepository) {
                        const b = root.inspection.git.branch || "detached";
                        const c = root.inspection.git.commit || "";
                        return "Git: " + b + (c ? " @" + c : "");
                    }
                    return "Local directory (no git)";
                }
            }

            NeonBadge {
                visible: !!(root.inspection && root.inspection.git && root.inspection.git.isRepository)
                tone: NeonTheme.cyan
                label: "Version Controlled"
            }
        }
    }
}
