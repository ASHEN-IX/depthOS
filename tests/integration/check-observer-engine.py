#!/usr/bin/env python3
"""Qt 6.12 QJSEngine smoke test of unchanged production JS; no KWin connection."""
import json
from pathlib import Path
from PyQt6.QtCore import QCoreApplication, QT_VERSION_STR
from PyQt6.QtQml import QJSEngine

ROOT = Path(__file__).resolve().parents[2]
app = QCoreApplication([])
engine = QJSEngine()
engine.evaluate("var module = {exports: {}};")
loaded = engine.evaluate((ROOT / "experiments/kwin-observer/contents/code/main.js").read_text())
assert not loaded.isError(), loaded.toString()
result = engine.evaluate("""
(function() {
    function signal() {
        const callbacks=[];
        return {connect: f => callbacks.push(f), disconnect: f => callbacks.splice(callbacks.indexOf(f), 1),
            emit: function() { for (const f of callbacks.slice()) f(); }};
    }
    const w={caption:'Qt engine fixture', normalWindow:true, desktops:[], activities:[], onAllDesktops:true};
    for (const key of ['captionChanged','desktopFileNameChanged','windowClassChanged','desktopsChanged','activitiesChanged',
        'frameGeometryChanged','outputChanged','minimizedChanged','fullScreenChanged','transientChanged','skipTaskbarChanged',
        'excludeFromCaptureChanged','closed']) w[key]=signal();
    const ws={stackingOrder:[w], desktops:[], screens:[], activities:[], activeWindow:w, currentActivity:null};
    for (const key of ['windowAdded','windowRemoved','windowActivated','desktopsChanged','currentDesktopChanged',
        'currentActivityChanged','activitiesChanged','screensChanged']) ws[key]=signal();
    const o=module.exports.createObserver({workspace:ws,sessionId:'qt-test',makeTimer:()=>({start(){},stop(){}})});
    o.start(); const first=o.take()[0];
    w.caption='Qt update'; w.captionChanged.emit(); const event=o.take()[0];
    o.stop();
    if(first.state.windows.length!==1 || event.type!=='window.metadata.changed' || event.window.title!=='Qt update')
        throw Error('QJSEngine runtime mismatch');
    return JSON.stringify({initialWindows:first.state.windows.length,event:event.type});
}())
""")
assert not result.isError(), result.toString()
print(f"Qt {QT_VERSION_STR} QJSEngine production-JS smoke: PASS", json.loads(result.toString()))
