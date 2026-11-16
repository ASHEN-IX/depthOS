import QtQuick
import QtTest

Item {
    width: 1920; height: 200
    QtObject {
        id: fixture
        property string state: "Disconnected"
        property string health: "Unavailable"
        property var rows: []
        property var associations: []
        property string selectedWorldId: ""
        property string selectedWorkspaceId: ""
        property string selectedDesktopId: ""
        function choose(row) {
            if (row.kind === "world") selectedWorldId = row.id;
            else if (row.kind === "workspace") selectedWorkspaceId = row.id;
            else selectedDesktopId = row.id;
        }
    }
    TestCase {
        name: "LogicalPanelFixture"
        when: windowShown
        property var panel
        function init() {
            fixture.rows=[]; fixture.associations=[]; fixture.selectedWorldId=""; fixture.selectedWorkspaceId=""; fixture.selectedDesktopId="";
            fixture.state="Disconnected"; fixture.health="Unavailable";
            const c=Qt.createComponent("../../experiments/kwin-overview/contents/ui/LogicalPanel.qml");
            compare(c.status,Component.Ready,c.errorString());
            panel=c.createObject(parent,{width:1856,height:72,backend:fixture});verify(panel);wait(20);
        }
        function cleanup() {panel.destroy();}
        function test_states() {
            const health=findChild(panel,"logical-backend-health");verify(health);
            for (const state of ["Disconnected","Loading","Connected","Degraded","Backend unavailable"]) {
                fixture.state=state;verify(health.text.includes(state));
            }
            verify(findChild(panel,"logical-selection").text.includes("Empty hierarchy"));
            fixture.associations=[{status:"unknown"},{status:"stale"},{status:"suggested"},{status:"conflicted"}];
            verify(health.text.includes("2 unassigned"));verify(health.text.includes("1 conflicts"));verify(health.text.includes("1 suggestions"));
            panel.backend=null;verify(health.text.includes("backend unavailable"));
        }
        function test_actual_fixture_hierarchy_and_mouse() {
            fixture.state="Connected";
            fixture.rows=[{kind:"world",id:"w",parentId:null,name:"Empty world"},{kind:"workspace",id:"p",parentId:"w",name:"Real workspace"},{kind:"desktop",id:"d",parentId:"p",name:"Code"}];
            tryVerify(()=>findChild(panel,"logical-row-world")!==null);
            const world=findChild(panel,"logical-row-world");compare(findChild(world,"logical-label-world").text,"Empty world");mouseClick(world,20,10);
            tryCompare(fixture,"selectedWorldId","w");
            tryVerify(()=>findChild(panel,"logical-row-workspace")!==null);
            const workspace=findChild(panel,"logical-row-workspace");compare(findChild(workspace,"logical-label-workspace").text,"Real workspace");mouseClick(workspace,20,10);
            tryCompare(fixture,"selectedWorkspaceId","p");
            tryVerify(()=>findChild(panel,"logical-row-desktop")!==null);
            const desktop=findChild(panel,"logical-row-desktop");compare(findChild(desktop,"logical-label-desktop").text,"Code");mouseClick(desktop,20,10);
            compare(fixture.selectedDesktopId,"d");verify(findChild(panel,"logical-selection").text.includes("Real workspace"));
        }
    }
}
