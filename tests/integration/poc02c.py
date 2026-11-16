#!/usr/bin/env python3
"""Scoped Phase-1 interaction/performance/stability run using live-acceptance.py.

No virtual input, private API, compositor restart or global preference mutation.
Begin with the existing harness baseline and independent guard. The operator stage
accepts real local keyboard/mouse actions; it never injects them.
"""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import select
import statistics
import subprocess
import sys
import time

ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('acceptance',Path(__file__).with_name('live-acceptance.py'))
a=importlib.util.module_from_spec(spec);spec.loader.exec_module(a)


def cfg():return json.loads((a.directory()/'poc02c.json').read_text())
def renew():a.require_guard();a.renew()
def save(name,value):a.atomic(a.directory()/name,value)
def toggle():a.command([sys.executable,'tools/overview-dev.py','toggle'])
def ensure_open():
    renew();c=cfg()
    if not a.active(c['id']):toggle();time.sleep(.5)
    assert a.active(c['id'])
def ensure_closed():
    renew();c=cfg()
    if a.active(c['id']):toggle();time.sleep(.4)
    assert not a.active(c['id'])

def lease_record():
    m=a.manifest();m['phase']='POC-02C';a.save(m)


def launch_probe():
    renew();m=a.manifest();n=1+sum(r['role']=='terminal' for r in m['processes'])
    pidfile=a.directory()/f'probe-child-{n}.pid';ack=a.directory()/f'probe-{n}-input-ack.json'
    payload='''import os,time,sys,json,select
from pathlib import Path
Path(PIDFILE).write_text(str(os.getpid()))
print("DEPTHOS POC-02C — DISPOSABLE TERMINAL N",flush=True)
print("Type depthos-02c-ok then Enter to verify input restoration. This is not a shell.",flush=True)
start=time.monotonic()
while time.monotonic()-start<900:
    ready,_,_=select.select([sys.stdin],[],[],0.2)
    if ready:
        value=sys.stdin.readline().strip()
        if value=="depthos-02c-ok":
            Path(ACKFILE).write_text(json.dumps({"matched":True,"monotonic":time.monotonic()}))
            print("INPUT VERIFIED",flush=True)
    print("\\rNative terminal repaint sample: %07.1f seconds"%(time.monotonic()-start),end="",flush=True)
'''.replace('PIDFILE',repr(str(pidfile))).replace('ACKFILE',repr(str(ack))).replace('TERMINAL N',f'TERMINAL {n}')
    log=(a.directory()/f'probe-{n}.log').open('w');proc=subprocess.Popen(['konsole','--separate','--workdir','/tmp','-e',sys.executable,'-c',payload],stdout=log,stderr=subprocess.STDOUT,start_new_session=True);a.process_record(proc,'terminal')
    def discover():
        for w in a.replica()['state']['windows']:
            if w['applicationId']=='org.kde.konsole' and w['nativeId'] and int(a.test_info(w['nativeId']).get('pid',-1))==proc.pid:return w
    window=a.wait_for(discover);a.wait_for(pidfile.exists);child=int(pidfile.read_text());ticks=Path(f'/proc/{child}/stat').read_text().rsplit(')',1)[1].split()[19]
    target={'pid':proc.pid,'native':window['nativeId']};m=a.manifest();m['processes'].append({'pid':child,'startTicks':ticks,'role':'testchild'});m.setdefault('testWindows',[]).append(target);a.save(m)
    return {**target,'ackFile':str(ack)}


def setup():
    renew();lease_record();b=json.loads((a.directory()/'baseline.json').read_text())
    old=a.replica()['sessionId'];a.command(['fish','scripts/observer-session.fish','--start']);a.wait_for(lambda:a.replica()['sessionId']!=old)
    def empty_desktop():
        state=a.replica()['state'];normal=[w for w in state['windows'] if w['classification'] in ('application','dialog')]
        saved=a.tiling_preferences((Path.home()/'.config/kwinrc').read_text())
        return next((str(d[1]) for d in b['desktops'] if a.empty_saved_test_layout(str(d[1]),saved) and not any(w['onAllDesktops'] or w['desktopIds'] is None or str(d[1]) in w['desktopIds'] for w in normal)),None)
    empty=a.wait_for(empty_desktop)
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(empty,variant_level=1));a.wait_for(lambda:a.replica()['state']['activeDesktopId']==empty);time.sleep(1)
    a.command([sys.executable,'tools/overview-dev.py','prepare']);current=json.loads((ROOT/'artifacts/overview-dev/current.json').read_text());identifier=current['id'];assert not (a.DATA/'kwin/effects'/identifier).exists()
    m=a.manifest();m['packages'].append({'kind':'effects','id':identifier});m['effects'].append(identifier)
    if 'depthos-spatial-overview-toggle' not in b['shortcutNames']:m['newActions'].append('depthos-spatial-overview-toggle')
    a.save(m)
    # Verify the emergency unload path before the first load.
    a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',identifier);assert not a.loaded(identifier)
    a.command([sys.executable,'tools/overview-dev.py','install']);a.command([sys.executable,'tools/overview-dev.py','load'])
    save('poc02c.json',{'id':identifier,'hash':current['sourceHash'],'desktop':empty,'probes':[]})
    ensure_open();a.capture('02c-empty',identifier)
    c=cfg();c['probes']=[launch_probe(),launch_probe()];save('poc02c.json',c)
    ensure_open();a.capture('02c-operator-start',identifier)
    print(json.dumps(c,indent=2))


def operator():
    renew();c=cfg()
    # Only focus a recorded test-owned target before presenting the overview.
    ensure_closed()
    target={k:c['probes'][0][k] for k in ('pid','native')};a.drive(target,'focus')
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1))
    a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1)
    ensure_open();a.capture('02c-operator-ready',c['id'])
    save('operator-start.json',{'wallTime':time.time(),'desktop':str(a.prop('/VirtualDesktopManager',a.VDM,'current')),'activity':a.replica()['state']['activeActivityId']})
    print('Operator stage ready. No virtual keyboard/mouse is created.')


def operator_finish():
    renew();c=cfg();start=json.loads((a.directory()/'operator-start.json').read_text());state=a.replica()['state']
    result={'originalNativeDesktop':start['desktop'],'currentNativeDesktop':state['activeDesktopId'],'nativeDesktopUnchanged':state['activeDesktopId']==start['desktop'],'nativeActivityUnchanged':state['activeActivityId']==start['activity'],'testTerminalInputAcknowledged':any(Path(p['ackFile']).exists() for p in c['probes']),'activeAtObservation':a.active(c['id']),'observedAt':time.time()}
    save('operator-observation.json',result);print(json.dumps(result,indent=2))


def profiles():
    renew();c=cfg();ensure_closed();a.resources('02c-before-profiles',samples=5,seconds=2)
    # New clients inherit the current native desktop. Establish isolation first.
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1))
    a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1.2)
    # Retain two operator windows and add up to six; never touch unrelated clients.
    while len(c['probes'])<6:c['probes'].append(launch_probe());save('poc02c.json',c)
    for probe in c['probes']:
        a.drive({k:probe[k] for k in ('pid','native')},'desktops',[c['desktop']])
    a.wait_for(lambda:all(a.window_record(probe['native'])['desktopIds']==[c['desktop']] for probe in c['probes']))
    results=[]
    for count in (0,1,2,4,6):
        renew();ensure_closed()
        for i,p in enumerate(c['probes']):
            target={k:p[k] for k in ('pid','native')};a.drive(target,'minimized',i>=count)
        # Focus fallback can switch desktop. Return to the test desktop explicitly.
        a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1));a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop'])
        a.wait_for(lambda:sum(a.window_record(p['native'])['minimized'] is False for p in c['probes'])==count)
        time.sleep(1.2)  # Let the existing desktop-slide effect finish before requesting another fullscreen scene.
        native=[a.window_record(probe['native']) for probe in c['probes']]
        assert len(native)==6 and all(w['desktopIds']==[c['desktop']] and w['excludeFromCapture'] is False for w in native)
        assert sum(w['minimized'] is False for w in native)==count
        ensure_open();a.capture(f'02c-profile-{count}',c['id']);a.resources(f'02c-eligible-previews-{count}',samples=5,seconds=2)
        results.append({'eligiblePreviews':count,'nativeWindows':6,'minimized':6-count,'screenshot':f'02c-profile-{count}.png','dynamicTerminalRepaintHz':5})
    ensure_closed()
    for p in c['probes'][2:]:
        a.drive({k:p[k] for k in ('pid','native')},'close');a.wait_for(lambda p=p:a.window_record(p['native']) is None)
    c['probes']=c['probes'][:2];save('poc02c.json',c);save('profiles.json',results);print('Profiles 0,1,2,4,6 completed.')


def memory():
    pid=a.manifest()['kwinPid'];lines=Path(f'/proc/{pid}/status').read_text().splitlines();return next(int(s.split()[1]) for s in lines if s.startswith('VmRSS:'))


def stress():
    renew();c=cfg();ensure_closed();began=time.monotonic();cycles=[];cohorts=[]
    attempts=[]
    while len(cycles)<120 and len(attempts)<140 and time.monotonic()-began<240:
        renew();t=time.monotonic();toggle()
        try:a.wait_for(lambda:a.active(c['id']),3)
        except TimeoutError:
            attempts.append({'attempt':len(attempts)+1,'outcome':'activation-declined-or-interrupted','desktop':str(a.prop('/VirtualDesktopManager',a.VDM,'current')),'activeEffects':list(a.prop('/Effects',a.EFFECTS,'activeEffects'))})
            save('stability-attempts.json',attempts);continue
        time.sleep(.34);toggle();time.sleep(.29);closed=not a.active(c['id'])
        attempts.append({'attempt':len(attempts)+1,'outcome':'completed' if closed else 'unexpected-active-after-close'})
        if not closed:
            ensure_closed();save('stability-attempts.json',attempts);continue
        cycles.append({'cycle':len(cycles)+1,'elapsedSeconds':time.monotonic()-t,'inactiveAfterClose':True})
        save('stability-attempts.json',attempts)
        if len(cycles)%20==0:
            cohorts.append({'cycles':len(cycles),'elapsedSeconds':time.monotonic()-began,'rssKiB':memory()});save('stability-progress.json',{'cycles':cycles,'memory':cohorts});print('Completed native cycles',len(cycles),flush=True)
    assert len(cycles)==120, 'Bounded cycle target not reached; inspect attempt evidence'
    for group in range(10):
        renew()
        for _ in range(6):toggle();time.sleep(.035)
        time.sleep(.4);assert not a.active(c['id'])
    for _ in range(10):
        renew();toggle();time.sleep(.07);toggle();time.sleep(.06);toggle();time.sleep(.36);assert a.active(c['id']);toggle();time.sleep(.3);assert not a.active(c['id'])
    # Five creation/removal/focus cases while a real preview-bearing scene is active.
    for _ in range(5):
        renew();p=launch_probe();target={k:p[k] for k in ('pid','native')};a.drive(target,'focus');ensure_open();a.drive(target,'close');a.wait_for(lambda:a.window_record(p['native']) is None);ensure_closed()
    ensure_open();old=a.replica()['sessionId'];a.command(['fish','scripts/observer-session.fish','--stop']);assert a.active(c['id']);a.command(['fish','scripts/observer-session.fish','--start']);a.wait_for(lambda:a.replica()['sessionId']!=old);assert a.active(c['id'])
    rec=next(r for r in reversed(a.manifest()['processes']) if r['role']=='receiver');a.terminate_owned(rec);a.wait_for(lambda:not a.call(a.DBUS,'/org/freedesktop/DBus',a.DBUS,'NameHasOwner',a.SERVICE));a.start_receiver();a.wait_for(lambda:a.summary()['snapshots']>=1,45);assert a.active(c['id']);ensure_closed()
    save('stability.json',{'cycles':120,'attempts':len(attempts),'interruptedOrDeclined':sum(r['outcome']!='completed' for r in attempts),'rapidGroups':10,'rapidToggles':60,'doubleReversals':10,'disposableCreateFocusRemove':5,'observerRestartActive':True,'receiverRestartActive':True,'durationSeconds':time.monotonic()-began,'memoryCohorts':cohorts,'rssEndKiB':memory(),'finalInactive':not a.active(c['id'])})
    a.resources('02c-after-stress',samples=5,seconds=2);print('Bounded stability complete.',flush=True)


def contention():
    renew();c=cfg();ensure_closed();time.sleep(1.2)
    b=json.loads((a.directory()/'baseline.json').read_text());identifier='depthos-spatial-poc'
    assert identifier not in b['loadedEffects'], 'Preserve an already loaded user POC-01'
    m=a.manifest()
    if identifier not in m['effects']:m['effects'].append(identifier);a.save(m)
    assert a.call('org.kde.KWin','/Effects',a.EFFECTS,'loadEffect',identifier)
    invoke=lambda:a.call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','invokeShortcut','depthos-spatial-poc-toggle')
    invoke();time.sleep(.5);assert a.active(identifier)
    before=time.time();toggle();time.sleep(.5)
    rejected=not a.active(c['id']) and a.active(identifier)
    time.sleep(1.0)
    invoke();time.sleep(.5);assert not a.active(identifier)
    # Exactly one explicit user-equivalent invocation must now open the overview.
    toggle();time.sleep(.5);recovered=a.active(c['id'])
    result={'controlledConflict':'original DepthOS POC-01 only','overviewRejectedWhileOtherDepthosActive':rejected,'firstExplicitInvocationAfterConflictOpens':recovered,'wallTime':before}
    save('contention.json',result);assert rejected and recovered,result
    ensure_closed();a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',identifier);assert not a.loaded(identifier)
    print(json.dumps(result,indent=2))


def anomaly():
    renew();c=cfg();ensure_closed();results=[];other=next(str(d[1]) for d in json.loads((a.directory()/'baseline.json').read_text())['desktops'] if str(d[1])!=c['desktop'])
    for n in range(12):
        renew();ensure_closed();a.command([sys.executable,'tools/overview-dev.py','unload']);assert not a.loaded(c['id']);a.command([sys.executable,'tools/overview-dev.py','load'])
        # Test immediate initialization and desktop-transition timing separately.
        context='immediate-load' if n<4 else ('settled-load' if n<8 else 'desktop-transition')
        if context=='settled-load':time.sleep(.5)
        if context=='desktop-transition':
            for desktop in (other,c['desktop']):a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(desktop,variant_level=1))
            time.sleep(.4)
        start=time.monotonic();toggle();time.sleep(.45);ok=a.active(c['id']);results.append({'attempt':n+1,'context':context,'active':ok,'elapsedMs':(time.monotonic()-start)*1000})
        save('activation-attempts.json',results)
        if not ok:
            print('Activation anomaly reproduced; stopping at first failed native activation.',flush=True);break
        ensure_closed()
    a.command([sys.executable,'tools/overview-dev.py','unload']);a.resources('02c-after-unload',samples=5,seconds=2);save('activation-summary.json',{'attempts':len(results),'passed':sum(r['active'] for r in results),'failed':sum(not r['active'] for r in results),'sourceEditedBetweenLoads':False,'sameImmutableRevision':c['id']});print(json.dumps(results,indent=2))


def render_probe():
    """Native scene controls via isolated diagnostic actions, never physical input proof."""
    import hashlib
    import shutil
    renew();c=cfg();ensure_closed();a.command([sys.executable,'tools/overview-dev.py','unload'])
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1))
    a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1.2)
    # Fresh fixed-lifetime clients keep the later profile interval bounded and comparable.
    for probe in c['probes']:
        if a.test_info(probe['native']):
            a.drive({k:probe[k] for k in ('pid','native')},'close')
            a.wait_for(lambda probe=probe:a.window_record(probe['native']) is None)
    c['probes']=[];save('poc02c.json',c)
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1))
    a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1.2)
    while len(c['probes'])<6:c['probes'].append(launch_probe());save('poc02c.json',c)
    for probe in c['probes']:a.drive({k:probe[k] for k in ('pid','native')},'desktops',[c['desktop']])
    a.wait_for(lambda:all(a.window_record(probe['native'])['desktopIds']==[c['desktop']] for probe in c['probes']))
    tag=a.manifest()['run'].split('-')[-1];identifier='depthos-02c-render-probe-'+tag+'-'+a.uuid.uuid4().hex[:6]
    path=a.directory()/identifier
    assert not path.exists() and not (a.DATA/'kwin/effects'/identifier).exists()
    shutil.copytree(ROOT/'experiments/kwin-overview',path)
    metadata=json.loads((path/'metadata.json').read_text());metadata['KPlugin']['Id']=identifier;metadata['KPlugin']['Name']='DepthOS isolated POC-02C render probe';metadata['X-DepthOS-AcceptanceRun']=a.manifest()['run'];a.atomic(path/'metadata.json',metadata)
    other=next(str(d[1]) for d in reversed(json.loads((a.directory()/'baseline.json').read_text())['desktops']) if str(d[1])!=c['desktop'])
    qml=path/'contents/ui/main.qml';source=qml.read_text().replace('name: "depthos-spatial-overview-toggle"',f'name: "{identifier}-toggle"').replace('[depthos-overview]','[depthos-02c-probe]')
    actions={'open':'effect.open()','close':'effect.close()','out':'effect.zoomOut()','in':'effect.zoomIn()', 'other':f'effect.inspectDesktop(effect.desktops.find(d => d.id === {json.dumps(other)}))','activity':'effect.inspectActivity(effect.activities[0])','motion':'effect.reducedMotion = !effect.reducedMotion','record':'effect.acceptanceRecord()'}
    injected='''
    // Test-only public scene calls; no keyboard/pointer event is synthesized.
    function acceptanceRecord() {
        console.info("[depthos-02c-probe-state] " + JSON.stringify({
            level: effect.level, motionOff: effect.motionOff, reducedMotion: effect.reducedMotion,
            visible: effect.visible, requestedOpen: effect.requestedOpen, reveal: effect.reveal,
            transitionDuration: transition.duration, desktopCount: effect.desktops.length,
            activityCount: effect.activities.length, selectedDesktop: effect.selectedDesktop ? effect.selectedDesktop.id : null,
            nativeDesktop: effect.system.currentDesktop.id, selectedActivity: effect.selectedActivity,
            nativeActivity: effect.system.currentActivity, kirigamiDuration: Kirigami.Units.longDuration
        }));
    }
'''
    for action,body in actions.items():injected+=f'    ShortcutHandler {{ name: "{identifier}-{action}"; text: "DepthOS 02C probe {action}"; sequence: ""; onActivated: {{ {body}; }} }}\n'
    source=source[:source.rfind('}')]+injected+'\n}\n';qml.write_text(source)
    # This copy has an immutable new URL. Record every byte and injected control.
    hashes={str(f.relative_to(path)):hashlib.sha256(f.read_bytes()).hexdigest() for f in path.rglob('*') if f.is_file()};save('render-probe-source.json',{'id':identifier,'baseSourceHash':c['hash'],'files':hashes,'testOnlyActions':list(actions)})
    m=a.manifest();m['packages'].append({'kind':'effects','id':identifier});m['effects'].append(identifier);m['newActions'] += [identifier+'-'+x for x in ['toggle',*actions]];a.save(m)
    a.command(['kpackagetool6','--type','KWin/Effect','--packageroot',str(a.DATA/'kwin/effects'),'--install',str(path)])
    assert all(hashlib.sha256((a.DATA/'kwin/effects'/identifier/name).read_bytes()).hexdigest()==h for name,h in hashes.items())
    a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',identifier);assert not a.loaded(identifier)
    assert a.call('org.kde.KWin','/Effects',a.EFFECTS,'loadEffect',identifier)
    invoke=lambda name:a.call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','invokeShortcut',identifier+'-'+name)
    originalDesktop=str(a.prop('/VirtualDesktopManager',a.VDM,'current'));originalActivity=a.replica()['state']['activeActivityId'];checks={}
    def record():invoke('record');time.sleep(.08)
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1))
    a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1.2)
    invoke('open');time.sleep(.5);assert a.active(identifier);record();a.capture('02c-native-z1',identifier)
    invoke('out');time.sleep(.3);record();a.capture('02c-native-z2',identifier)
    invoke('out');time.sleep(.3);record();a.capture('02c-native-z3',identifier)
    checks['nativeZ1Z2Z3Screenshots']=True
    for level in (3,2,1):
        if level==2:invoke('in');time.sleep(.25)
        if level==1:invoke('in');time.sleep(.25)
        invoke('close');time.sleep(.3);assert not a.active(identifier);invoke('open');time.sleep(.35)
        for _ in range(level-1):invoke('out');time.sleep(.2)
        checks['programmaticCloseFromZ'+str(level)]=True
    # Local inactive-context inspection must preserve native workspace state.
    target={k:c['probes'][1][k] for k in ('pid','native')};a.drive(target,'desktops',[other]);a.wait_for(lambda:a.window_record(target['native'])['desktopIds']==[other])
    invoke('other');time.sleep(.3);record();a.capture('02c-inactive-inspection',identifier)
    checks['inspectionPreservesNativeDesktop']=str(a.prop('/VirtualDesktopManager',a.VDM,'current'))==originalDesktop
    invoke('out');invoke('out');time.sleep(.3);invoke('activity');time.sleep(.3);record();checks['inspectionPreservesNativeActivity']=a.replica()['state']['activeActivityId']==originalActivity
    a.drive(target,'desktops',[c['desktop']]);a.wait_for(lambda:a.window_record(target['native'])['desktopIds']==[c['desktop']])
    invoke('motion');time.sleep(.1);record();a.capture('02c-reduced-motion',identifier)
    timings=[]
    for _ in range(10):
        renew();t=time.monotonic();invoke('close');time.sleep(.05);assert not a.active(identifier);closed=time.monotonic()-t
        t=time.monotonic();invoke('open');time.sleep(.05);assert a.active(identifier);opened=time.monotonic()-t
        timings.append({'closeObservedSeconds':closed,'openObservedSeconds':opened})
    record();checks['reducedMotionTenOpenCloseCycles']=True
    for _ in range(20):
        renew()
        for name in ('out','out','in','in'):invoke(name);time.sleep(.08)
    checks['twentyProgrammaticDepthRoundTrips']=True;record()
    invoke('close');time.sleep(.1);assert not a.active(identifier);checks['finalInactive']=True
    a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',identifier);checks['unloaded']=not a.loaded(identifier)
    save('render-probe.json',{'id':identifier,'checks':checks,'timingsInclude50msObservationWait':timings,'method':'Run-only diagnostic actions call production scene functions. Native rendering and state evidence; NOT keyboard/mouse proof.','inactiveInspectionDesktop':other});print(json.dumps(checks,indent=2))


def quick_profiles():
    """Coarse /proc samples; no FPS, causal-overhead or physical-input claim."""
    renew();c=cfg();probe=json.loads((a.directory()/'render-probe-source.json').read_text());identifier=probe['id'];assert identifier in a.manifest()['effects']
    import hashlib
    assert all(hashlib.sha256((a.DATA/'kwin/effects'/identifier/name).read_bytes()).hexdigest()==digest for name,digest in probe['files'].items())
    if not a.loaded(identifier):assert a.call('org.kde.KWin','/Effects',a.EFFECTS,'loadEffect',identifier)
    invoke=lambda name:a.call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','invokeShortcut',identifier+'-'+name)
    results=[]
    for count in (0,1,2,4,6):
        renew();invoke('close');time.sleep(.3)
        for index,window in enumerate(c['probes']):a.drive({k:window[k] for k in ('pid','native')},'minimized',index>=count)
        a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1));a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1.5)
        a.wait_for(lambda:sum(a.window_record(w['native'])['minimized'] is False for w in c['probes'])==count)
        assert all(a.window_record(w['native'])['desktopIds']==[c['desktop']] and a.window_record(w['native'])['excludeFromCapture'] is False for w in c['probes'])
        invoke('open');time.sleep(.4)
        if not a.active(identifier):
            results.append({'eligiblePreviews':count,'valid':False,'reason':'activation declined'});save('quick-profiles.json',results);continue
        start=time.time();a.resources(f'02c-short-previews-{count}',samples=5,seconds=.2)
        valid=a.active(identifier) and str(a.prop('/VirtualDesktopManager',a.VDM,'current'))==c['desktop']
        results.append({'eligiblePreviews':count,'valid':valid,'nativeTestWindows':6,'startWallTime':start,'endWallTime':time.time(),'sampleCount':5,'sampleSeconds':.2,'cpuTickResolutionPercent':100/os.sysconf('SC_CLK_TCK')/.2})
        save('quick-profiles.json',results)
        if count==6 and valid:
            try:a.capture('02c-six-previews-verified',identifier)
            except AssertionError:pass
        invoke('close');time.sleep(.2)
    a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',identifier);assert not a.loaded(identifier)
    a.resources('02c-after-render-probe-unload',samples=5,seconds=2)
    print(json.dumps(results,indent=2))


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('operation',choices=['setup','operator','operator-finish','profiles','stress','contention','anomaly','render-probe','quick-profiles']);args=p.parse_args();globals()[args.operation.replace('-','_')]()
if __name__=='__main__':main()
