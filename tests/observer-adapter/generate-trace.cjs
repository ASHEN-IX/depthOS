// Sanitized trace made exclusively by the unchanged production observer.
const {fixture,window,signals}=require('../observer/fixtures.cjs');
const a=window({caption:'Sanitized editor'}), f=fixture([a]);
const frames=[f.initial];
function flush(){frames.push(...f.events());}
a.caption='Sanitized changed'; a.captionChanged.emit(); flush();
a.desktops=[f.desktop];a.onAllDesktops=false;a.desktopsChanged.emit();flush();
a.activities=['activity-1'];a.activitiesChanged.emit();flush();
a.minimized=true;a.fullScreen=true;a.minimizedChanged.emit();a.fullScreenChanged.emit();flush();
a.frameGeometry={x:30,y:40,width:640,height:480};a.frameGeometryChanged.emit();flush();
const b=window({caption:null,desktopFileName:null}); f.workspace.windowAdded.emit(b);flush();
f.workspace.activeWindow=b;f.workspace.windowActivated.emit(b);flush();
const desktop=signals({id:'desktop-native-2',name:'Desktop 2'},'nameChanged');
f.workspace.desktops.push(desktop);f.workspace.desktopsChanged.emit();flush();
f.workspace.currentDesktop=desktop;f.workspace.currentDesktopChanged.emit();flush();
f.workspace.activities.push('activity-2');f.workspace.activitiesChanged.emit();flush();
f.workspace.currentActivity='activity-2';f.workspace.currentActivityChanged.emit();flush();
f.screen.devicePixelRatio=1.5;f.screen.scaleChanged.emit();flush();
a.closed.emit();flush();
const final=f.observer.snapshot('parity');
console.log(JSON.stringify({frames,final},null,2));
