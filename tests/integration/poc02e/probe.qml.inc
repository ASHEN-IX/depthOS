    // Acceptance-only READ probe. Never calls a controller mutation or input handler.
    property var acceptanceViews: []
    function acceptanceSnapshot() {
        let items = [];
        function walk(item, root) {
            if (!item) return;
            const n = item.objectName || "";
            const isAction = typeof item.triggered === "function" && typeof item.text === "string";
            if (n === "native-window-card" || n === "native-window-grid" || n === "native-context-grid" || n === "depthos-overview-scene" || isAction || n === "native-context-card") {
                let p = item.mapToItem(root,0,0), q = item.mapToItem(root,item.width,item.height);
                let v = {name:n, x:p.x,y:p.y,w:q.x-p.x,h:q.y-p.y,visible:item.visible};
                if (isAction) v.action = item.text;
                if (item.currentIndex !== undefined) v.index=item.currentIndex;
                if (item.count !== undefined) v.count=item.count;
                if (n === "native-context-card") v.contextIndex=item.index;
                if (item.selected !== undefined) {v.selected=item.selected;v.hovered=(item.children || []).some(c=>c.containsMouse===true);}
                if (item.client !== undefined) v.uuid=String(item.client.internalId);
                if (item.previewReason !== undefined) v.previewReason=item.previewReason;
                if (item.desktopIndex !== undefined) {v.desktopIndex=item.desktopIndex;v.activityIndex=item.activityIndex;}
                items.push(v);
            }
            for (let child of item.children || []) walk(child,root);
        }
        for (let v of acceptanceViews) walk(v,v);
        let ws=[];
        for (let w of Workspace.stackingOrder) if(w.normalWindow || w.dialog) ws.push({uuid:String(w.internalId),pid:w.pid,desktops:w.desktops.map(d=>d.id),activities:w.activities,geometry:{x:w.frameGeometry.x,y:w.frameGeometry.y,w:w.frameGeometry.width,h:w.frameGeometry.height}});
        console.info("DEPTHOS02E_STATE " + JSON.stringify({visible:effect.visible,requested:effect.requestedOpen,outputs:effect.constructedOutputs,level:effect.level,reveal:effect.reveal,motionOff:effect.motionOff,selected:effect.selectedWindowId,desktop:effect.selectedDesktop ? effect.selectedDesktop.id : "",activity:effect.selectedActivity,desktops:effect.desktops.map(d=>d.id),activities:effect.activities,active:Workspace.activeWindow ? String(Workspace.activeWindow.internalId):"",windows:ws,items:items}));
    }
    ShortcutHandler { name: "DEPTHOS_PROBE_ID"; text: "DepthOS isolated read probe"; sequence: ""; onActivated: effect.acceptanceSnapshot() }
