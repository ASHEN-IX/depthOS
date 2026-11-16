#!/usr/bin/env python3
"""Explicitly authorized native acceptance; only recorded DepthOS/test-owned resources.

Usage: baseline, guard, poc01, observer, overview, cleanup. Never restarts KWin.
The separate guard process calls the same idempotent cleanup after lease expiry.
"""
import argparse
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import time
import uuid
import dbus

ROOT = Path(__file__).resolve().parents[2]
ART = ROOT / 'artifacts/live-acceptance'
DATA = Path(os.environ.get('XDG_DATA_HOME', str(Path.home()/'.local/share')))
BUS = dbus.SessionBus()
PROPS = 'org.freedesktop.DBus.Properties'
DBUS = 'org.freedesktop.DBus'
EFFECTS = 'org.kde.kwin.Effects'
SCRIPTING = 'org.kde.kwin.Scripting'
VDM = 'org.kde.KWin.VirtualDesktopManager'
SERVICE = 'org.depthos.StateInspector'
# Host preparation remains limited to the historical baseline. Isolated evidence
# does not authorize a host session or replace its observer/portal prerequisites.
VERIFIED_KWIN_VERSIONS = frozenset({'6.8.1'})
KWIN_QUALIFICATIONS = {
    '6.8.1': {'classification': 'HISTORICALLY_VALIDATED', 'evidence': 'docs/poc-02e.md'},
    '6.8.3': {'classification': 'ISOLATED_QUALIFIED', 'evidence': 'docs/poc-02g.md'},
}


class CompatibilityError(RuntimeError):
    """Live preparation is forbidden; read-only evidence and cleanup remain safe."""


def kwin_compatibility(installed_output, support_information):
    """Fail closed on unknown output, differing versions, or unaudited releases."""
    binary = re.fullmatch(r'\s*kwin(?:_wayland)?\s+(\d+\.\d+\.\d+)\s*', installed_output)
    lines = re.findall(r'^\s*KWin version:\s*([^\r\n]+)', support_information, re.MULTILINE)
    running = lines[0].strip() if len(lines) == 1 else None
    if running is not None and not re.fullmatch(r'\d+\.\d+\.\d+', running):
        running = None
    installed = binary.group(1) if binary else None
    if installed is None or running is None:
        reason = 'Unrecognized or ambiguous installed/running KWin version output'
    elif installed != running:
        reason = f'Installed/running mismatch: installed {installed}, running {running}'
    elif running in KWIN_QUALIFICATIONS and running not in VERIFIED_KWIN_VERSIONS:
        reason = f'KWin {running} is source-audited and isolated-qualified only; host-session qualification is pending'
    elif running not in VERIFIED_KWIN_VERSIONS:
        reason = f'KWin {running} is not source-audited and verified for live acceptance'
    else:
        reason = 'Installed and running KWin match the verified 6.8.1 baseline'
    approved = installed is not None and installed == running and running in VERIFIED_KWIN_VERSIONS
    return {'installed': installed, 'running': running,
            'installedParts': [int(x) for x in installed.split('.')] if installed else None,
            'runningParts': [int(x) for x in running.split('.')] if running else None,
            'installedOutput': installed_output, 'runningVersionLines': lines,
            'verifiedVersions': sorted(VERIFIED_KWIN_VERSIONS),
            'releaseQualifications': {key: dict(value) for key, value in KWIN_QUALIFICATIONS.items()},
            'installedQualification': KWIN_QUALIFICATIONS.get(installed, {}).get('classification', 'UNTESTED'),
            'runningQualification': KWIN_QUALIFICATIONS.get(running, {}).get('classification', 'UNTESTED'),
            'hostSessionQualified': False,
            'hostRequirements': ['matching installed/running qualified release', 'verified compositor identity',
                                 'same-session native observer acceptance', 'operator-authorized OS portal flow'],
            'decision': 'APPROVED' if approved else 'BLOCKED', 'approved': approved,
            'reason': reason, 'policy': 'Exact release and explicit scope only; isolated qualification never grants host acceptance; no range or mismatch waiver'}


def detect_kwin_compatibility():
    # Do not use command(): preflight must never append to a previous cleaned run.
    identity = {'owner': owner(), 'busId': str(call(DBUS, '/org/freedesktop/DBus', DBUS, 'GetId')),
                'kwinPid': int(call(DBUS, '/org/freedesktop/DBus', DBUS,
                                    'GetConnectionUnixProcessID', 'org.kde.KWin'))}
    binary = subprocess.run(['kwin_wayland', '--version'], text=True, capture_output=True, timeout=10)
    info = str(call('org.kde.KWin', '/KWin', 'org.kde.KWin', 'supportInformation'))
    result = kwin_compatibility(binary.stdout, info)
    result.update(identity, installedExitCode=binary.returncode,
                  runningSource='org.kde.KWin /KWin org.kde.KWin.supportInformation')
    if binary.returncode != 0:
        result.update(approved=False, decision='BLOCKED', reason='Installed KWin version command failed')
    try:
        require_session(identity)
    except AssertionError:
        result.update(approved=False, decision='BLOCKED', reason='Compositor identity changed during preflight')
    return result


def compatibility_gate():
    result = detect_kwin_compatibility()
    if not result['approved']:
        raise CompatibilityError(result['reason'])
    return result


def call(service, path, interface, method, *args):
    # KWin exports one- and two-argument overloads. dbus-python introspection
    # can select the one-argument signature even when two arguments are supplied.
    if method == 'loadScript' and interface == SCRIPTING and len(args) == 2:
        return BUS.call_blocking(service, path, interface, method, 'ss', args, timeout=10)
    return dbus.Interface(BUS.get_object(service, path), interface).get_dbus_method(method)(*args, timeout=10)


def prop(path, interface, name):
    return call('org.kde.KWin', path, PROPS, 'Get', interface, name)


def owner():
    return str(call(DBUS, '/org/freedesktop/DBus', DBUS, 'GetNameOwner', 'org.kde.KWin'))


def atomic(path, value):
    temp = path.with_suffix(path.suffix+'.tmp')
    temp.write_text(json.dumps(value, indent=2)+'\n'); temp.replace(path)


def command(args, timeout=60):
    result = subprocess.run([str(a) for a in args], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
    if ART.joinpath('current.json').exists():
        with (directory()/'commands.jsonl').open('a') as stream:
            stream.write(json.dumps({'time':time.time(),'argv':[str(a) for a in args], 'returncode':result.returncode})+'\n')
    if result.returncode:
        raise RuntimeError(f'Command failed ({result.returncode}): {args}: {result.stdout[:1200]}')
    return result.stdout


def directory():
    pinned = os.environ.get('DEPTHOS_ACCEPTANCE_RUN_DIRECTORY')
    path = Path(pinned) if pinned else Path(json.loads((ART/'current.json').read_text())['directory'])
    assert path.parent == ART and path.is_dir() and not path.is_symlink()
    return path


def manifest(): return json.loads((directory()/'manifest.json').read_text())
def save(m): atomic(directory()/'manifest.json', m)
def renew(): (directory()/'lease').write_text(str(time.time()+180))
def loaded(identifier): return bool(call('org.kde.KWin','/Effects',EFFECTS,'isEffectLoaded',identifier))
def active(identifier): return identifier in prop('/Effects',EFFECTS,'activeEffects')
def scripting_loaded(identifier): return bool(call('org.kde.KWin','/Scripting',SCRIPTING,'isScriptLoaded',identifier))

def canonical(value):
    return json.loads(json.dumps(value))


def tiling_preferences(text):
    """Retain only non-content tiling values needed for scoped rollback evidence."""
    result={};section=None
    for line in text.splitlines():
        if line.startswith('['):section=line if line.startswith('[Tiling][') else None
        elif section and '=' in line:
            key,value=line.split('=',1)
            if key in ('tiles','padding'):result.setdefault(section,{})[key]=value
    return result


def empty_saved_test_layout(desktop, preferences):
    groups=[v for k,v in preferences.items() if '['+desktop+']' in k and 'tiles' in v]
    if not groups:return False
    try:return all(json.loads(v['tiles'])=={'layoutDirection':'horizontal','tiles':[]} for v in groups)
    except (ValueError,TypeError):return False


def state():
    outputs=json.loads(subprocess.check_output(['kscreen-doctor','-j'],text=True))['outputs']
    filtered=[]
    for out in outputs:
        mode=next((m for m in out.get('modes',[]) if m['id']==out.get('currentModeId')),None)
        filtered.append({**{k:out.get(k) for k in ('name','connected','enabled','pos','size','scale','currentModeId')},'currentMode':mode})
    shortcuts=list(call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','shortcutNames'))
    package_ids={kind:sorted(p.name for p in (DATA/'kwin'/kind).glob('depthos*') if p.is_dir()) for kind in ('effects','scripts')}
    hashes={}
    for identifier in ('codex-glass','wallpaper-style','wallpaper-style-transparency','depthos-spatial-poc'):
        path=DATA/'kwin/effects'/identifier
        for file in path.rglob('*'):
            if file.is_file(): hashes[str(file)]=hashlib.sha256(file.read_bytes()).hexdigest()
    configs={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in (Path.home()/'.config/kwinrc',Path.home()/'.config/kglobalshortcutsrc') if p.is_file()}
    result={'owner':owner(),'busId':str(call(DBUS,'/org/freedesktop/DBus',DBUS,'GetId')),
        'kwinPid':int(call(DBUS,'/org/freedesktop/DBus',DBUS,'GetConnectionUnixProcessID','org.kde.KWin')),
        'loadedEffects':list(prop('/Effects',EFFECTS,'loadedEffects')),'activeEffects':list(prop('/Effects',EFFECTS,'activeEffects')),
        'desktops':list(prop('/VirtualDesktopManager',VDM,'desktops')),'desktopCount':int(prop('/VirtualDesktopManager',VDM,'count')),
        'currentDesktop':str(prop('/VirtualDesktopManager',VDM,'current')),
        'activities':list(call('org.kde.ActivityManager','/ActivityManager/Activities','org.kde.ActivityManager.Activities','ListActivities')),
        'currentActivity':str(call('org.kde.ActivityManager','/ActivityManager/Activities','org.kde.ActivityManager.Activities','CurrentActivity')),
        'outputs':filtered,'shortcutNames':shortcuts,'packages':package_ids,'preservedHashes':hashes,'configHashes':configs,'tilingPreferences':tiling_preferences((Path.home()/'.config/kwinrc').read_text())}
    return canonical(result)


def baseline():
    assert os.getuid()!=0 and os.environ.get('XDG_SESSION_TYPE')=='wayland'
    compatibility = detect_kwin_compatibility()
    ART.mkdir(mode=0o700,parents=True,exist_ok=True);os.chmod(ART,0o700)
    if (ART/'current.json').exists():
        old=manifest()
        assert old.get('cleaned'), 'Previous acceptance run must be cleaned first'
    run=time.strftime('%Y%m%dT%H%M%SZ',time.gmtime())+'-'+uuid.uuid4().hex[:8]
    path=ART/run;path.mkdir(mode=0o700)
    b=state(); require_session(compatibility)
    b.update(run=run,utc=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),
             kwinVersion=compatibility['running'], kwinCompatibility=compatibility,
             gitStatus=subprocess.run(['git','status','--short'],cwd=ROOT,text=True,
                                      capture_output=True,check=True,timeout=10).stdout)
    if not compatibility['approved']:
        atomic(path/'baseline.json', b)
        atomic(path/'manifest.json', {'run':run, 'owner':b['owner'], 'busId':b['busId'],
            'kwinPid':b['kwinPid'], 'kwinCompatibility':compatibility, 'phase':'PREFLIGHT_BLOCKED',
            'packages':[], 'effects':[], 'scripts':[], 'processes':[], 'newActions':[],
            'cleaned':True, 'preparationStarted':False})
        atomic(ART/'current.json',{'directory':str(path),'run':run})
        raise CompatibilityError(compatibility['reason'])
    assert not any(x.startswith('depthos') for x in b['activeEffects']), 'User DepthOS overlay is active; do not disturb it'
    assert not any(scripting_loaded(i) for i in ('depthos-system-observer','depthos-system-observer-session'))
    assert not call(DBUS,'/org/freedesktop/DBus',DBUS,'NameHasOwner',SERVICE)
    atomic(path/'baseline.json',b);atomic(ART/'baseline.json',b)
    atomic(path/'manifest.json',{'run':run,'owner':b['owner'],'busId':b['busId'],'kwinPid':b['kwinPid'],'kwinCompatibility':compatibility,'packages':[],'effects':[],'scripts':[],'processes':[],'newActions':[],'cleaned':False})
    atomic(ART/'current.json',{'directory':str(path),'run':run})
    renew();print(str(path))


def process_record(proc, role):
    rec={'pid':proc.pid,'startTicks':Path(f'/proc/{proc.pid}/stat').read_text().rsplit(')',1)[1].split()[19], 'role':role}
    m=manifest();m['processes'].append(rec);save(m);return rec


def terminate_owned(rec):
    path=Path(f'/proc/{rec["pid"]}/stat')
    if path.exists() and path.read_text().rsplit(')',1)[1].split()[19]==rec['startTicks']:
        os.kill(rec['pid'],signal.SIGTERM)


def guard():
    # Pin this watchdog to its run; a later current.json cannot redirect cleanup.
    os.environ['DEPTHOS_ACCEPTANCE_RUN_DIRECTORY'] = str(directory())
    m=manifest()
    if m.get('cleaned'):return
    pid=os.getpid();ticks=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()[19]
    if not any(r['role']=='guard' and r['pid']==pid and r['startTicks']==ticks for r in m['processes']):
        m['processes'].append({'pid':pid,'startTicks':ticks,'role':'guard'});save(m)
    print('Independent DepthOS recovery guard ready.',flush=True)
    while True:
        if manifest().get('cleaned'):return
        if time.time()>float((directory()/'lease').read_text()):
            cleanup();return
        time.sleep(1)


def require_session(m):
    assert owner() == m['owner'], 'Compositor session changed'
    bus_id = str(call(DBUS,'/org/freedesktop/DBus',DBUS,'GetId'))
    pid = int(call(DBUS,'/org/freedesktop/DBus',DBUS,'GetConnectionUnixProcessID','org.kde.KWin'))
    assert bus_id == m['busId'] and pid == m['kwinPid'], 'Session bus or compositor PID changed'


def require_guard():
    m=manifest()
    assert not m['cleaned'], 'Start a fresh baseline and independent guard before live stages'
    assert time.time() < float((directory()/'lease').read_text()), 'Recovery lease has expired'
    require_session(m)
    for rec in m['processes']:
        path=Path(f'/proc/{rec["pid"]}/stat')
        if rec['role']=='guard' and path.exists() and path.read_text().rsplit(')',1)[1].split()[19]==rec['startTicks']:
            return
    raise RuntimeError('Independent recovery guard is not running')


def cleanup():
    m=manifest()
    if m.get('cleaned'):
        print('Run already cleaned; no stale resource mutations performed.')
        return
    b=json.loads((directory()/'baseline.json').read_text()); errors=[]
    require_session(m)
    def attempt(label,fn):
        try: fn()
        except Exception as error:errors.append({'operation':label,'error':str(error)[:600]})
    for identifier in reversed(m['effects']):
        attempt('unload '+identifier,lambda i=identifier:call('org.kde.KWin','/Effects',EFFECTS,'unloadEffect',i))
    for p in reversed(m['packages']):
        if p['kind']!='effects':continue
        path=DATA/'kwin'/p['kind']/p['id']
        if loaded(p['id']):
            errors.append({'operation':'remove '+p['id'],'error':'Still loaded; package retained'})
            continue
        if path.exists():
            attempt('remove '+p['id'],lambda p=p:command(['kpackagetool6','--type','KWin/Effect','--packageroot',str(DATA/'kwin/effects'),'--remove',p['id']]))
    for identifier in reversed(m['scripts']):
        attempt('unload script '+identifier,lambda i=identifier:call('org.kde.KWin','/Scripting',SCRIPTING,'unloadScript',i))
    for rec in reversed(m['processes']):
        if rec['role']!='guard':attempt('stop '+rec['role'],lambda r=rec:terminate_owned(r))
    time.sleep(.5)
    for p in reversed(m['packages']):
        if p['kind']!='scripts':continue
        if any(scripting_loaded(i) for i in m['scripts']):
            errors.append({'operation':'remove '+p['id'],'error':'Owned script still loaded; package retained'})
            continue
        if (DATA/'kwin/scripts'/p['id']).exists():
            attempt('remove '+p['id'],lambda p=p:command(['kpackagetool6','--type','KWin/Script','--packageroot',str(DATA/'kwin/scripts'),'--remove',p['id']]))
    for action in m['newActions']:
        attempt('unregister '+action,lambda a=action:call('org.kde.kglobalaccel','/kglobalaccel','org.kde.KGlobalAccel','unregister','kwin',a))
    attempt('restore current desktop',lambda:call('org.kde.KWin','/VirtualDesktopManager',PROPS,'Set',VDM,'current',dbus.String(b['currentDesktop'],variant_level=1)))
    time.sleep(.5); after=state();atomic(directory()/'after.json',after)
    checks={'ownerUnchanged':after['owner']==b['owner'],'loadedEffectsRestored':set(after['loadedEffects'])==set(b['loadedEffects']),
        'preservedFilesUnchanged':after['preservedHashes']==b['preservedHashes'],'desktopsUnchanged':after['desktops']==b['desktops'],
        'currentDesktopRestored':after['currentDesktop']==b['currentDesktop'],'activitiesUnchanged':after['activities']==b['activities'] and after['currentActivity']==b['currentActivity'],
        'outputsUnchanged':after['outputs']==b['outputs'],'shortcutNamesRestored':set(after['shortcutNames'])==set(b['shortcutNames']),
        'packagesRestored':after['packages']==b['packages'],'receiverStopped':not bool(call(DBUS,'/org/freedesktop/DBus',DBUS,'NameHasOwner',SERVICE)),
        'ownedScriptsUnloaded':all(not scripting_loaded(i) for i in m['scripts'])}
    checks['configurationHashesRestored'] = after['configHashes']==b['configHashes']
    m['cleaned']=not errors and all(checks.values());m['cleanupErrors']=errors;save(m)
    atomic(directory()/'cleanup.json',{'checks':checks,'errors':errors,'configFileHashesMatch':after['configHashes']==b['configHashes']})
    print(json.dumps({'cleanup':checks,'errors':errors},indent=2))
    if not m['cleaned']:raise RuntimeError('Cleanup comparison failed; inspect scoped remaining resources')


def stage_poc01():
    require_guard();renew();m=manifest();b=json.loads((directory()/'baseline.json').read_text())
    identifier='depthos-acceptance-poc01-'+m['run'].split('-')[-1];action=identifier+'-toggle'
    path=directory()/identifier;shutil.copytree(ROOT/'experiments/kwin-effect',path)
    metadata=json.loads((path/'metadata.json').read_text());metadata['KPlugin']['Id']=identifier;metadata['KPlugin']['Name']='DepthOS POC-01 isolated acceptance';metadata['X-DepthOS-AcceptanceRun']=m['run'];atomic(path/'metadata.json',metadata)
    qml=path/'contents/ui/main.qml';text=qml.read_text().replace('name: "depthos-spatial-poc-toggle"',f'name: "{action}"').replace('sequence: "Meta+Ctrl+Shift+F11"','sequence: ""').replace('[depthos-spatial-poc]',f'[{identifier}]');qml.write_text(text)
    assert not (DATA/'kwin/effects'/identifier).exists() and action not in b['shortcutNames']
    m['packages'].append({'kind':'effects','id':identifier});m['effects'].append(identifier);m['newActions'].append(action);save(m)
    command(['kpackagetool6','--type','KWin/Effect','--packageroot',str(DATA/'kwin/effects'),'--install',str(path)])
    assert not loaded(identifier)
    call('org.kde.KWin','/Effects',EFFECTS,'unloadEffect',identifier);assert not loaded(identifier)
    assert call('org.kde.KWin','/Effects',EFFECTS,'loadEffect',identifier)
    toggle=lambda:call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','invokeShortcut',action)
    results={'id':identifier,'action':action,'cycles':0,'loaded':loaded(identifier),'registered':action in call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','shortcutNames'),'cycleStates':[]}
    for i in range(30):
        toggle();time.sleep(.38);opened=active(identifier)
        toggle();time.sleep(.31);closed=not active(identifier)
        results['cycleStates'].append({'cycle':i+1,'activeAfterOpen':opened,'inactiveAfterClose':closed});results['cycles']+=1
        assert opened and closed, 'POC-01 cycle state failed'
    for _ in range(6):toggle();time.sleep(.045)
    time.sleep(.4);results['rapidSixEndsInactive']=not active(identifier)
    toggle();time.sleep(.08);toggle();time.sleep(.06);toggle();time.sleep(.4)
    results['reversalReopens']=active(identifier);toggle();time.sleep(.35);results['finalInactive']=not active(identifier)
    call('org.kde.KWin','/Effects',EFFECTS,'unloadEffect',identifier);results['cleanUnload']=not loaded(identifier)
    atomic(directory()/'poc01.json',results);print(json.dumps({k:v for k,v in results.items() if k!='cycleStates'},indent=2))


def wait_for(predicate, timeout=15):
    deadline=time.monotonic()+timeout
    while time.monotonic()<deadline:
        try:
            result=predicate()
            if result:return result
        except (FileNotFoundError,json.JSONDecodeError,KeyError):pass
        time.sleep(.15)
    raise TimeoutError('Native acceptance condition timed out')


def start_receiver():
    require_guard()
    m=manifest();index=1+sum(r['role']=='receiver' for r in m['processes'])
    out=directory()/f'receiver-{index}';out.mkdir(mode=0o700)
    log=(out/'receiver.log').open('w')
    proc=subprocess.Popen([sys.executable,str(ROOT/'tests/integration/live-observer.py'),'--output',str(out)],stdout=log,stderr=subprocess.STDOUT,cwd=ROOT,start_new_session=True,env={**os.environ,'PYTHONDONTWRITEBYTECODE':'1'})
    rec=process_record(proc,'receiver');m=manifest();m['receiverDirectory']=str(out);save(m)
    wait_for(lambda:call(DBUS,'/org/freedesktop/DBus',DBUS,'NameHasOwner',SERVICE))
    return rec


def receiver_path():return Path(manifest()['receiverDirectory'])
def replica():return json.loads((receiver_path()/'latest-snapshot.json').read_text())
def events():return json.loads((receiver_path()/'events.json').read_text())
def summary():return json.loads((receiver_path()/'summary.json').read_text())
def has_event(kind, native=None):return any(e['type']==kind and (native is None or e.get('window',{}).get('nativeId')==native) for e in events())

def test_info(native):
    values=call('org.kde.KWin','/KWin','org.kde.KWin','getWindowInfo',native)
    return {str(k):v for k,v in values.items() if k in ('uuid','pid','desktopFile','desktops','minimized','fullscreen','excludeFromCapture','x','y','width','height')}


def window_record(native):
    return next((w for w in replica()['state']['windows'] if w['nativeId']==native),None)


def launch_terminal():
    require_guard()
    m=manifest();n=1+sum(r['role']=='terminal' for r in m['processes']);pidfile=directory()/f'test-child-{n}.pid'
    payload=f'import os,time;from pathlib import Path;Path({str(pidfile)!r}).write_text(str(os.getpid()));print("DEPTHOS LIVE ACCEPTANCE — disposable test terminal {n}\\nNative KWin window; safe test content only",flush=True);time.sleep(300)'
    log=(directory()/f'terminal-{n}.log').open('w')
    proc=subprocess.Popen(['konsole','--separate','--workdir','/tmp','-e',sys.executable,'-c',payload],stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
    rec=process_record(proc,'terminal')
    def discover():
        for w in replica()['state']['windows']:
            if w['applicationId']=='org.kde.konsole' and w['nativeId']:
                info=test_info(w['nativeId'])
                if int(info.get('pid',-1))==proc.pid:return w
    window=wait_for(discover)
    wait_for(pidfile.exists)
    child=int(pidfile.read_text());childrec={'pid':child,'startTicks':Path(f'/proc/{child}/stat').read_text().rsplit(')',1)[1].split()[19],'role':'testchild'}
    m=manifest();m['processes'].append(childrec);m.setdefault('testWindows',[]).append({'pid':proc.pid,'native':window['nativeId']});save(m)
    return {'pid':proc.pid,'native':window['nativeId']}


def drive(target, action, value=None, allow_missing=False):
    require_guard()
    # Public Window writes, exclusively the UUID+PID+application triple of a test-owned Konsole.
    assert action in ('focus','desktops','sticky','minimized','fullscreen','protected','close')
    assert target in manifest()['testWindows']
    if not allow_missing:
        info=test_info(target['native']);assert int(info['pid'])==target['pid'] and info['desktopFile']=='org.kde.konsole'
    script_id='depthos-acceptance-driver-'+uuid.uuid4().hex[:12]
    params=json.dumps({'target':target,'action':action,'value':value})
    source='''(function(){const p=PARAMS;const candidates=workspace.stackingOrder.filter(w=>String(w.internalId).replace(/[{}]/g,"")===p.target.native && w.pid===p.target.pid && w.desktopFileName==="org.kde.konsole");
if(candidates.length!==1){print("[depthos-acceptance-driver] target missing; no action");return;}
const w=candidates[0];
if(p.action==="focus")workspace.activeWindow=w;
else if(p.action==="desktops"){const ds=workspace.desktops.filter(d=>p.value.indexOf(d.id)>=0);if(ds.length!==p.value.length)throw new Error("Test desktop missing");w.desktops=ds;}
else if(p.action==="sticky")w.onAllDesktops=p.value;
else if(p.action==="minimized")w.minimized=p.value;
else if(p.action==="fullscreen")w.fullScreen=p.value;
else if(p.action==="protected")w.excludeFromCapture=p.value;
else if(p.action==="close")w.closeWindow();
print("[depthos-acceptance-driver] completed "+p.action);
})();'''.replace('PARAMS',params)
    path=directory()/(script_id+'.js');path.write_text(source)
    m=manifest();m['scripts'].append(script_id);save(m)
    number=int(call('org.kde.KWin','/Scripting',SCRIPTING,'loadScript',str(path),script_id));assert number>=0
    try:call('org.kde.KWin',f'/Scripting/Script{number}','org.kde.kwin.Script','run')
    finally:call('org.kde.KWin','/Scripting',SCRIPTING,'unloadScript',script_id)


def unrelated_effects_match(before, after, owned):
    return set(before) - set(owned) == set(after) - set(owned)


def stage_observer():
    require_guard();renew();m=manifest();b=json.loads((directory()/'baseline.json').read_text())
    assert not (DATA/'kwin/scripts/depthos-system-observer').exists()
    m['packages'].append({'kind':'scripts','id':'depthos-system-observer'});m['scripts'].append('depthos-system-observer-session');save(m)
    command(['fish','scripts/install-observer.fish','--install']);first=start_receiver()
    assert not scripting_loaded('depthos-system-observer-session')
    command(['fish','scripts/observer-session.fish','--start']);wait_for(lambda:summary()['snapshots']>=1)
    initial=replica();atomic(directory()/'observer-initial.json',initial)
    s=initial['state'];desktop_ids=[str(d[1]) for d in b['desktops']]
    checks={'initialSnapshot':True,'existingWindows':len(s['windows'])>=1,
        'desktopEnumeration':set(d['id'] for d in s['desktops'])==set(desktop_ids),
        'activityEnumeration':set(s['activities'])==set(b['activities']),
        'outputEnumeration':{o['name'] for o in s['screens']}=={o['name'] for o in b['outputs'] if o['enabled']},
        'outputGeometry':all(o['geometry']['width']==b['outputs'][i]['size']['width'] and o['geometry']['height']==b['outputs'][i]['size']['height'] and o['scale']==b['outputs'][i]['scale'] for i,o in enumerate(s['screens']))}
    began=time.monotonic();a=launch_terminal();second=launch_terminal();checks['windowAdded']=has_event('window.added',a['native']) and has_event('window.added',second['native'])
    for target in (a,second,a):
        drive(target,'focus');wait_for(lambda t=target:next((w['nativeId'] for w in replica()['state']['windows'] if w['runtimeId']==replica()['state']['activeWindowId']),None)==t['native'])
    checks['focusChanged']=has_event('window.focus.changed')
    other=next(d for d in desktop_ids if d!=b['currentDesktop'])
    drive(a,'desktops',[other]);wait_for(lambda:window_record(a['native'])['desktopIds']==[other]);checks['membershipChanged']=has_event('window.desktop.changed',a['native'])
    call('org.kde.KWin','/VirtualDesktopManager',PROPS,'Set',VDM,'current',dbus.String(other,variant_level=1));wait_for(lambda:replica()['state']['activeDesktopId']==other)
    checks['desktopChanged']=has_event('desktop.active.changed')
    call('org.kde.KWin','/VirtualDesktopManager',PROPS,'Set',VDM,'current',dbus.String(b['currentDesktop'],variant_level=1));wait_for(lambda:replica()['state']['activeDesktopId']==b['currentDesktop'])
    drive(a,'sticky',True);wait_for(lambda:window_record(a['native'])['onAllDesktops'] is True);checks['stickyMembership']=True
    drive(a,'desktops',[b['currentDesktop'],other]);wait_for(lambda:len(window_record(a['native'])['desktopIds'])==2);checks['multipleDesktopMembership']=True
    drive(a,'desktops',[b['currentDesktop']]);wait_for(lambda:window_record(a['native'])['desktopIds']==[b['currentDesktop']])
    for action,key in [('minimized','minimized'),('fullscreen','fullscreen'),('protected','excludeFromCapture')]:
        for value in (True,False):
            drive(a,action,value);wait_for(lambda k=key,v=value:window_record(a['native'])[k] is v)
            assert bool(test_info(a['native'])[key]) is value
        checks[action+'State']=True
    old_id=window_record(a['native'])['runtimeId'];drive(a,'close');wait_for(lambda:window_record(a['native']) is None)
    checks['windowRemoved']=any(e['type']=='window.removed' and e.get('windowId')==old_id for e in events());drive(a,'focus',allow_missing=True);checks['staleIdGuard']=not test_info(a['native'])
    drive(second,'close');wait_for(lambda:window_record(second['native']) is None)
    elapsed=time.monotonic()-began;stats=summary();checks['sessionRevisionConsistent']=stats['errors']==0 and not stats['needSnapshot'] and stats['removalConsistent']
    old_count=stats['snapshots'];os.kill(first['pid'],signal.SIGUSR1);resync_start=time.monotonic();wait_for(lambda:summary()['snapshots']>old_count);resync=time.monotonic()-resync_start;checks['reconciliation']=True
    first_token=replica()['sessionId'];terminate_owned(first);wait_for(lambda:not call(DBUS,'/org/freedesktop/DBus',DBUS,'NameHasOwner',SERVICE));reconnect_start=time.monotonic();start_receiver();wait_for(lambda:summary()['snapshots']>=1,45);reconnect=time.monotonic()-reconnect_start;checks['receiverRestart']=replica()['sessionId']==first_token
    command(['fish','scripts/observer-session.fish','--stop']);checks['cleanUnload']=not scripting_loaded('depthos-system-observer-session')
    command(['fish','scripts/observer-session.fish','--start']);restart_start=time.monotonic();wait_for(lambda:replica()['sessionId']!=first_token);restart=time.monotonic()-restart_start;checks['observerRestart']=True
    command(['fish','scripts/observer-session.fish','--stop']);checks['cleanUnload'] &= not scripting_loaded('depthos-system-observer-session')
    now=state();checks['unrelatedStatePreserved']=unrelated_effects_match(b['loadedEffects'],now['loadedEffects'],manifest()['effects']) and now['preservedHashes']==b['preservedHashes'] and now['desktops']==b['desktops'] and now['activities']==b['activities']
    result={'checks':checks,'testWindows':[a,second],'eventCounts':stats['eventCounts'],'eventTestDurationSeconds':elapsed,'eventFramesPerSecond':sum(stats['eventCounts'].values())/elapsed,'resyncLatencySeconds':resync,'receiverReconnectSeconds':reconnect,'observerRestartSnapshotSeconds':restart,'collectorSaveCadenceSeconds':.5}
    atomic(directory()/'observer.json',result);assert all(checks.values()),checks
    template=json.loads((ROOT/'artifacts/observer-acceptance.template.json').read_text());template.update(status='PASS',kwinVersion='6.8.1',compositorOwner=owner(),sessionBusId=b['busId'],observerSourceHash=hashlib.sha256((ROOT/'experiments/kwin-observer/contents/code/main.js').read_bytes()).hexdigest(),evidenceDirectory=str(directory()))
    template['checks']={k:True for k in template['checks']};atomic(ROOT/'artifacts/observer-acceptance.json',template)
    print(json.dumps(result,indent=2))


def resources(label, samples=3, seconds=2):
    renew();m=manifest();b=json.loads((directory()/'baseline.json').read_text());pids={'kwin':b['kwinPid']}
    for rec in reversed(m['processes']):
        if rec['role']=='receiver' and Path(f'/proc/{rec["pid"]}').exists():pids['inspector']=rec['pid'];break
    def read(pid):
        stat=Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()
        rss=next(int(s.split()[1]) for s in Path(f'/proc/{pid}/status').read_text().splitlines() if s.startswith('VmRSS:'))
        return int(stat[11])+int(stat[12]),rss
    measurements=[]
    for _ in range(samples):
        before={name:read(pid) for name,pid in pids.items()};start=time.monotonic();time.sleep(seconds);elapsed=time.monotonic()-start
        for name,pid in pids.items():
            ticks,rss=read(pid);measurements.append({'process':name,'pid':pid,'sampleSeconds':elapsed,'cpuPercentOneCore':(ticks-before[name][0])/os.sysconf('SC_CLK_TCK')/elapsed*100,'rssKiB':rss})
    path=directory()/'performance.json';data=json.loads(path.read_text()) if path.exists() else {};data[label]=measurements;atomic(path,data)


def capture(name, identifier):
    require_guard()
    renew();time.sleep(.6);assert active(identifier),'Do not capture private desktop behind a closed overlay'
    current=str(prop('/VirtualDesktopManager',VDM,'current'))
    own_ids={w['native'] for w in manifest().get('testWindows',[])}
    for w in replica()['state']['windows']:
        if w['classification'] in ('application','dialog') and (w['onAllDesktops'] or current in (w['desktopIds'] or [])):
            assert w['nativeId'] in own_ids, 'Screenshot skipped: non-test application present on selected desktop'
    atomic(directory()/(name+'.state.json'),replica())
    log=(directory()/f'spectacle-{name}.log').open('w')
    proc=subprocess.Popen(['spectacle','--background','--nonotify','--fullscreen','--output',str(directory()/(name+'.png'))],stdout=log,stderr=subprocess.STDOUT,start_new_session=True);process_record(proc,'capture');assert proc.wait(timeout=25)==0
    assert (directory()/(name+'.png')).is_file()


def stage_overview():
    require_guard();renew();m=manifest();b=json.loads((directory()/'baseline.json').read_text())
    for own_id in set(m['effects']):
        if own_id != 'depthos-spatial-poc' and loaded(own_id):
            call('org.kde.KWin','/Effects',EFFECTS,'unloadEffect',own_id)
    if loaded('depthos-spatial-poc'):
        if active('depthos-spatial-poc'):
            call('org.kde.kglobalaccel','/component/kwin','org.kde.kglobalaccel.Component','invokeShortcut','depthos-spatial-poc-toggle');time.sleep(.4)
        call('org.kde.KWin','/Effects',EFFECTS,'unloadEffect','depthos-spatial-poc');assert not loaded('depthos-spatial-poc')
    resources('idleObserverUnloaded')
    command(['fish','scripts/observer-session.fish','--start']);wait_for(lambda:summary()['sessions'] and summary()['packetAgeSeconds'] is not None and summary()['packetAgeSeconds']<3)
    command(['python3','tools/overview-dev.py','prepare']);prepared=json.loads((ROOT/'artifacts/overview-dev/current.json').read_text());identifier=prepared['id']
    if (DATA/'kwin/effects'/identifier).exists():
        assert identifier in manifest()['effects']
        command(['python3','tools/overview-dev.py','unload','--id',identifier])
        command(['python3','tools/overview-dev.py','remove','--id',identifier])
    m=manifest();m['effects'].append(identifier);m['packages'].append({'kind':'effects','id':identifier})
    if 'depthos-spatial-overview-toggle' not in b['shortcutNames']:m['newActions'].append('depthos-spatial-overview-toggle')
    save(m)
    call('org.kde.KWin','/Effects',EFFECTS,'unloadEffect',identifier);assert not loaded(identifier)
    command(['python3','tools/overview-dev.py','install']);command(['python3','tools/overview-dev.py','load'])
    toggle=lambda:command(['python3','tools/overview-dev.py','toggle'])
    checks={'load':loaded(identifier)}
    for operation,key in [('install','duplicateInstallRejected'),('load','duplicateLoadRejected')]:
        result=subprocess.run([sys.executable,'tools/overview-dev.py',operation],text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,cwd=ROOT);checks[key]=result.returncode!=0
    resources('idleObserverLoaded')
    def choose_empty():
        snap=replica()['state'];normal=[w for w in snap['windows'] if w['classification'] in ('application','dialog')]
        def occupied(desktop):return any(w['onAllDesktops'] or w['desktopIds'] is None or desktop in w['desktopIds'] for w in normal)
        return next((str(d[1]) for d in b['desktops'] if not occupied(str(d[1]))),None)
    empty=wait_for(choose_empty,10)
    atomic(directory()/'overview-pre-capture.json',replica())
    other=next(str(d[1]) for d in b['desktops'] if str(d[1])!=empty)
    call('org.kde.KWin','/VirtualDesktopManager',PROPS,'Set',VDM,'current',dbus.String(empty,variant_level=1));wait_for(lambda:replica()['state']['activeDesktopId']==empty)
    def ensure_open():
        # Minimizing a test client can make KWin focus a window on another desktop.
        # Restore only the authorized test desktop, never alter that existing window.
        if str(prop('/VirtualDesktopManager',VDM,'current')) != empty:
            call('org.kde.KWin','/VirtualDesktopManager',PROPS,'Set',VDM,'current',dbus.String(empty,variant_level=1))
            wait_for(lambda:replica()['state']['activeDesktopId']==empty)
            time.sleep(.6)
        time.sleep(.4)
        for _ in range(3):
            if active(identifier):break
            toggle();time.sleep(.6)
        assert active(identifier)
    ensure_open();capture('overview-empty',identifier);checks['activeOnEmptyDesktop']=True
    a=launch_terminal();ensure_open();capture('overview-one-window',identifier)
    second=launch_terminal();ensure_open();capture('overview-two-windows',identifier);checks['twoNativeTestWindows']=all(window_record(w['native']) is not None for w in (a,second))
    drive(a,'focus');wait_for(lambda:replica()['state']['activeWindowId']==window_record(a['native'])['runtimeId']);ensure_open();capture('overview-focused',identifier)
    resources('overviewTwoThumbnails')
    drive(a,'minimized',True);wait_for(lambda:window_record(a['native'])['minimized']);ensure_open();capture('overview-minimized',identifier);checks['nativeMinimizedState']=bool(test_info(a['native'])['minimized']);drive(a,'minimized',False);wait_for(lambda:not window_record(a['native'])['minimized'])
    drive(a,'fullscreen',True);wait_for(lambda:window_record(a['native'])['fullscreen']);ensure_open();capture('overview-fullscreen',identifier);checks['nativeFullscreenState']=bool(test_info(a['native'])['fullscreen']);drive(a,'fullscreen',False);wait_for(lambda:not window_record(a['native'])['fullscreen'])
    drive(a,'desktops',[other]);wait_for(lambda:window_record(a['native'])['desktopIds']==[other]);ensure_open();capture('overview-membership-filter',identifier);checks['nativeMembershipChanged']=True
    drive(a,'desktops',[empty,other]);wait_for(lambda:len(window_record(a['native'])['desktopIds'])==2);ensure_open();capture('overview-multi-desktop',identifier);checks['nativeMultipleMembership']=True
    drive(a,'sticky',True);wait_for(lambda:window_record(a['native'])['onAllDesktops']);ensure_open();capture('overview-sticky',identifier);checks['nativeStickyState']=True
    drive(a,'desktops',[empty]);wait_for(lambda:window_record(a['native'])['desktopIds']==[empty])
    drive(a,'protected',True);wait_for(lambda:window_record(a['native'])['excludeFromCapture']);ensure_open();capture('overview-protected',identifier);checks['nativeCaptureRestriction']=bool(test_info(a['native'])['excludeFromCapture']);drive(a,'protected',False);wait_for(lambda:not window_record(a['native'])['excludeFromCapture'])
    old_token=replica()['sessionId'];command(['fish','scripts/observer-session.fish','--stop']);time.sleep(.5);checks['overviewActiveObserverDisconnected']=active(identifier);capture('overview-observer-disconnected',identifier)
    command(['fish','scripts/observer-session.fish','--start']);wait_for(lambda:replica()['sessionId']!=old_token);checks['overviewActiveObserverRestarted']=active(identifier);capture('overview-observer-restarted',identifier)
    drive(second,'close');wait_for(lambda:window_record(second['native']) is None);ensure_open();capture('overview-window-removed',identifier);checks['nativeRemovalDuringPreview']=True
    # Close the remaining disposable during a reversed transition.
    toggle();time.sleep(.05);toggle();time.sleep(.05);drive(a,'close');wait_for(lambda:window_record(a['native']) is None);ensure_open();capture('overview-all-removed',identifier);checks['nativeRemovalDuringTransition']=True
    toggle();time.sleep(.4)
    cycles=[];latencies=[]
    for n in range(30):
        start=time.monotonic();toggle();wait_for(lambda:active(identifier),3);latencies.append((time.monotonic()-start)*1000);time.sleep(.35)
        toggle();time.sleep(.3);cycles.append(not active(identifier));assert cycles[-1]
    checks['thirtyCycles']=len(cycles)==30 and all(cycles)
    for _ in range(6):toggle();time.sleep(.045)
    time.sleep(.5);checks['sixRapidTogglesInactive']=not active(identifier)
    ensure_open();toggle();time.sleep(.05);toggle();time.sleep(.4);checks['reversalReopens']=active(identifier)
    command(['python3','tools/overview-dev.py','unload']);checks['activeUnload']=not loaded(identifier) and not active(identifier)
    command(['python3','tools/overview-dev.py','load']);checks['reloadUnchangedRevision']=loaded(identifier)
    toggle();time.sleep(.5);toggle();time.sleep(.4);checks['finalInactive']=not active(identifier)
    command(['python3','tools/overview-dev.py','unload']);checks['unloadInactive']=not loaded(identifier)
    resources('afterOverviewUnload')
    result={'id':identifier,'checks':checks,'cycles':30,'activationToObservedActiveMs':latencies,'latencyMethod':'monotonic around helper subprocess and first successful native activeEffects read; includes process/IPC/poll overhead, not presented frame','screenshotDesktop':empty,'testWindows':[a,second]}
    atomic(directory()/'overview.json',result);assert all(checks.values()),checks
    print(json.dumps({k:v for k,v in result.items() if k not in ('activationToObservedActiveMs','testWindows')},indent=2))


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('operation',choices=['baseline','guard','poc01','observer','overview','cleanup']);args=parser.parse_args()
    {'baseline':baseline,'guard':guard,'poc01':stage_poc01,'observer':stage_observer,'overview':stage_overview,'cleanup':cleanup}[args.operation]()

if __name__=='__main__':main()
