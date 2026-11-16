import QtQuick

Rectangle {
    id: panel
    property var backend: null
    objectName: "depthos-logical-panel"
    color: "#141D32"; radius: 8
    readonly property var worlds: backend ? backend.rows.filter(r => r.kind === "world") : []
    readonly property var workspaces: backend ? backend.rows.filter(r => r.kind === "workspace" && r.parentId === backend.selectedWorldId) : []
    readonly property var desktops: backend ? backend.rows.filter(r => r.kind === "desktop" && r.parentId === backend.selectedWorkspaceId) : []
    Text {
        objectName: "logical-backend-health"; textFormat: Text.PlainText
        x: 10; y: 3; width: parent.width - 20; elide: Text.ElideRight; color: "#B8C6EC"; font.pixelSize: 11
        text: !backend ? "Logical backend unavailable · native overview remains available" : "Logical: " + backend.state + " / " + backend.health +
              " · " + backend.associations.filter(r => r.status === "unknown" || r.status === "stale").length + " unassigned · " + backend.associations.filter(r => r.status === "conflicted").length + " conflicts · " + backend.associations.filter(r => r.status === "suggested").length + " suggestions · assignments read-only"
    }
    Row {
        x: 10; y: 22; spacing: 6
        Repeater {
            model: [panel.worlds, panel.workspaces, panel.desktops]
            delegate: Item {
                required property var modelData
                required property int index
                width: (panel.width - 40) / 3; height: 25
                Text { id: heading; textFormat: Text.PlainText; text: ["Worlds:","Workspaces:","Logical desktops:"][index]; color: "#94A3B8"; font.pixelSize: 11 }
                ListView {
                    x: heading.width + 6; width: parent.width - x; height: 25
                    orientation: ListView.Horizontal; spacing: 4; clip: true; model: modelData
                    delegate: Rectangle {
                        required property var modelData
                        objectName: "logical-row-" + modelData.kind
                        width: Math.min(170,label.implicitWidth + 16); height: 23; radius: 4
                        color: panel.backend && [panel.backend.selectedWorldId,panel.backend.selectedWorkspaceId,panel.backend.selectedDesktopId].includes(modelData.id) ? "#4C4685" : "#263451"
                        Text { id: label; objectName: "logical-label-" + parent.modelData.kind; textFormat: Text.PlainText; anchors.centerIn: parent; width: parent.width - 10; elide: Text.ElideRight; text: parent.modelData.name; color: "#F8FAFC"; font.pixelSize: 11 }
                        MouseArea { anchors.fill: parent; enabled: !!panel.backend && (panel.backend.state === "Connected" || panel.backend.state === "Degraded"); onClicked: panel.backend.choose(parent.modelData) }
                    }
                }
            }
        }
    }
    Text {
        objectName: "logical-selection"; textFormat: Text.PlainText; x: 10; y: 49; color: "#94A3B8"; font.pixelSize: 10
        text: !backend || backend.rows.length === 0 ? "Empty hierarchy or backend unavailable" : "Inspecting logical context only · selected workspace: " + (backend.rows.find(r => r.id === backend.selectedWorkspaceId) || {name:"none"}).name + " · no KDE desktop/Activity switch"
    }
}
