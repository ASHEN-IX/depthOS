#!/usr/bin/env python3
"""Small operator groups on the existing guarded native acceptance harness."""
import argparse
import importlib.util
import json
import subprocess
import sys
import time
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('acceptance',Path(__file__).with_name('live-acceptance.py'))
a=importlib.util.module_from_spec(spec);spec.loader.exec_module(a)

def cfg():return json.loads((a.directory()/'poc02d.json').read_text())
def renew():a.require_guard();a.renew()
def target(window):return {k:window[k] for k in ('pid','native')}

def terminal(number):
    pidfile=a.directory()/f'02d-child-{number}.pid';ack=a.directory()/f'02d-input-{number}.json'
    log=(a.directory()/f'02d-terminal-{number}.log').open('w')
    proc=subprocess.Popen(['konsole','--separate','--workdir','/tmp','-e',sys.executable,str(ROOT/'tests/integration/marker-terminal.py'),'--pid-file',str(pidfile),'--ack-file',str(ack)],stdout=log,stderr=subprocess.STDOUT,start_new_session=True);a.process_record(proc,'terminal')
    def find():
        for w in a.replica()['state']['windows']:
            if w['applicationId']=='org.kde.konsole' and w['nativeId'] and int(a.test_info(w['nativeId']).get('pid',-1))==proc.pid:return w
    w=a.wait_for(find);a.wait_for(pidfile.exists);pid=int(pidfile.read_text());m=a.manifest();m['processes'].append({'pid':pid,'startTicks':Path(f'/proc/{pid}/stat').read_text().rsplit(')',1)[1].split()[19],'role':'testchild'});window={'pid':proc.pid,'native':w['nativeId']};m.setdefault('testWindows',[]).append(window);a.save(m)
    return {**window,'ackFile':str(ack)}

def setup():
    renew();b=json.loads((a.directory()/'baseline.json').read_text());m=a.manifest();m['phase']='POC-02D';m['packages'].append({'kind':'scripts','id':'depthos-system-observer'});m['scripts'].append('depthos-system-observer-session');a.save(m)
    a.command(['fish','scripts/install-observer.fish','--install']);a.start_receiver();a.command(['fish','scripts/observer-session.fish','--start']);a.wait_for(lambda:a.summary()['snapshots']>=1)
    a.atomic(a.directory()/'observer-initial.json',a.replica())
    state=a.replica()['state'];windows=[w for w in state['windows'] if w['classification'] in ('application','dialog')]
    desktop=next(str(d[1]) for d in b['desktops'] if a.empty_saved_test_layout(str(d[1]),b['tilingPreferences']) and not any(w['onAllDesktops'] or w['desktopIds'] is None or str(d[1]) in w['desktopIds'] for w in windows))
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(desktop,variant_level=1));a.wait_for(lambda:a.replica()['state']['activeDesktopId']==desktop);time.sleep(1.2)
    a.command([sys.executable,'tools/overview-dev.py','prepare']);prepared=json.loads((ROOT/'artifacts/overview-dev/current.json').read_text());identifier=prepared['id']
    m=a.manifest();m['effects'].append(identifier);m['packages'].append({'kind':'effects','id':identifier});m['testDesktop']=desktop
    if 'depthos-spatial-overview-toggle' not in b['shortcutNames']:m['newActions'].append('depthos-spatial-overview-toggle')
    a.save(m);a.call('org.kde.KWin','/Effects',a.EFFECTS,'unloadEffect',identifier);assert not a.loaded(identifier)
    a.command([sys.executable,'tools/overview-dev.py','install']);a.command([sys.executable,'tools/overview-dev.py','load'])
    # The existing gate checks the previously accepted observer source/session.
    windows=[terminal(1),terminal(2)]
    a.atomic(a.directory()/'poc02d.json',{'id':identifier,'hash':prepared['sourceHash'],'desktop':desktop,'windows':windows})
    print(json.dumps(cfg(),indent=2))

def open_group(name):
    renew();c=cfg()
    if a.active(c['id']):
        a.command([sys.executable,'tools/overview-dev.py','toggle']);time.sleep(.4)
    a.drive(target(c['windows'][0]),'focus')
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1));a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1.2)
    a.command([sys.executable,'tools/overview-dev.py','toggle']);a.wait_for(lambda:a.active(c['id']),3);time.sleep(.4)
    a.capture('02d-'+name+'-start',c['id'])
    a.atomic(a.directory()/('group-'+name+'.json'),{'startWallTime':time.time(),'nativeDesktop':str(a.prop('/VirtualDesktopManager',a.VDM,'current')),'nativeActivity':a.replica()['state']['activeActivityId'],'activeAtStart':True,'screenshot':'02d-'+name+'-start.png'})
    print('Native active state and actual screenshot confirmed; group',name,'ready.')

def observe(name):
    renew();c=cfg();f=a.directory()/('group-'+name+'.json');value=json.loads(f.read_text());value['observationWallTime']=time.time();value['activeAtObservation']=a.active(c['id']);value['currentNativeDesktop']=str(a.prop('/VirtualDesktopManager',a.VDM,'current'));value['currentNativeActivity']=a.replica()['state']['activeActivityId'];value['markerAcknowledgements']=[json.loads(Path(w['ackFile']).read_text()) for w in c['windows'] if Path(w['ackFile']).exists()];a.atomic(f,value);print(json.dumps(value,indent=2))

def empty_tiling_reconstruction(text, preferences, desktop, expected_hash):
    """One known-empty test entry only; require the exact original whole-file hash."""
    import hashlib
    lines=text.splitlines(keepends=True);section=None
    for index,line in enumerate(lines):
        if line.startswith('['):section=line.strip()
        if not line.startswith('tiles=') or '['+desktop+']' not in (section or ''):continue
        baseline=preferences.get(section,{}).get('tiles')
        if not baseline or json.loads(baseline)!={'layoutDirection':'horizontal','tiles':[]}:continue
        candidate=lines.copy();candidate[index]='tiles='+baseline+'\n';raw=''.join(candidate).encode()
        if hashlib.sha256(raw).hexdigest()==expected_hash:
            return {'section':section,'baseline':baseline,'current':line.strip()[6:]}
    raise ValueError('No exact-hash recovery for a known-empty test tile entry; no layout may be changed')


def restore_tiling():
    import hashlib
    m=a.manifest();a.require_session(m)
    if m.get('cleaned'):return
    b=json.loads((a.directory()/'baseline.json').read_text());file=Path.home()/'.config/kwinrc';expected=b['configHashes'][str(file)]
    if hashlib.sha256(file.read_bytes()).hexdigest()==expected:return
    c=cfg();desktop=c['desktop'];assert m['testDesktop']==desktop
    match=empty_tiling_reconstruction(file.read_text(),b['tilingPreferences'],desktop,expected)
    outputs=[out for out in b['outputs'] if out['enabled']];assert len(outputs)==1
    identifier='depthos-02d-empty-tiling-recovery-'+m['run'].split('-')[-1]
    parameters=json.dumps({'desktop':desktop,'output':outputs[0]['name'],'tree':json.loads(match['current'])})
    source="""(function(){const p=PARAMS;const d=workspace.desktops.find(d=>d.id===p.desktop);const o=workspace.screens.find(o=>o.name===p.output);
if(!d||!o)throw new Error('Recovery context unavailable');
if(workspace.stackingOrder.some(w=>(w.normalWindow||w.dialog)&&!w.deleted&&(w.onAllDesktops||w.desktops.indexOf(d)>=0)))throw new Error('Test desktop is not empty');
const root=workspace.rootTile(o,d);const dirs={floating:0,horizontal:1,vertical:2};
function check(t,s){if(t.windows.length!==0||t.tiles.length!==(s.tiles||[]).length)throw new Error('Unknown or occupied tile tree');if(s.layoutDirection!==undefined&&t.layoutDirection!==dirs[s.layoutDirection])throw new Error('Unexpected tile direction');for(let i=0;i<t.tiles.length;i++)check(t.tiles[i],s.tiles[i]);}
check(root,p.tree);let count=0;while(root.tiles.length){if(++count>64)throw new Error('Recovery bound exceeded');root.tiles[0].remove();}
if(root.tiles.length||root.windows.length||root.layoutDirection!==1)throw new Error('Empty baseline root not restored');
print('[depthos-02d-recovery] empty test tile tree restored');})();""".replace('PARAMS',parameters)
    path=a.directory()/(identifier+'.js');path.write_text(source);m['scripts'].append(identifier);a.save(m)
    number=int(a.call('org.kde.KWin','/Scripting',a.SCRIPTING,'loadScript',str(path),identifier));assert number>=0
    try:a.call('org.kde.KWin',f'/Scripting/Script{number}','org.kde.kwin.Script','run')
    finally:a.call('org.kde.KWin','/Scripting',a.SCRIPTING,'unloadScript',identifier)
    a.wait_for(lambda:hashlib.sha256(file.read_bytes()).hexdigest()==expected,12)
    a.atomic(a.directory()/'tiling-recovery.json',{'exactBaselineHashRestored':True,'section':match['section'],'method':'Public empty-tree removal; no configuration file overwrite'})


def cleanup():
    try:a.cleanup()
    except RuntimeError:
        path=a.directory()/'cleanup.json';initial=json.loads(path.read_text())
        a.atomic(a.directory()/'cleanup-first-attempt.json',initial)
        other={k:v for k,v in initial['checks'].items() if k!='configurationHashesRestored'}
        if not all(other.values()) or initial['errors']:raise
        restore_tiling();a.cleanup()


def guard():
    import os
    from types import SimpleNamespace
    os.environ['DEPTHOS_ACCEPTANCE_RUN_DIRECTORY']=str(a.directory())
    if a.manifest().get('cleaned'):return
    a.process_record(SimpleNamespace(pid=os.getpid()),'guard')
    print('POC-02D independent cleanup guard ready.',flush=True)
    while not a.manifest().get('cleaned'):
        if time.time()>float((a.directory()/'lease').read_text()):
            cleanup();return
        time.sleep(1)


def independent():
    """Only after the bounded operator attempt has ended; no physical-input claims."""
    renew();c=cfg();assert not any(a.test_info(w['native']) for w in c['windows']), 'Do not interrupt an operator terminal still in use'
    spec=importlib.util.spec_from_file_location('previous',Path(__file__).with_name('poc02c.py'));previous=importlib.util.module_from_spec(spec);spec.loader.exec_module(previous)
    a.atomic(a.directory()/'poc02c.json',{'id':c['id'],'hash':c['hash'],'desktop':c['desktop'],'probes':[]})
    previous.contention()
    a.atomic(a.directory()/'02d-contention.json',json.loads((a.directory()/'contention.json').read_text()))
    # Empty test desktop, then two new fixed-lifetime disposable preview clients.
    a.call('org.kde.KWin','/VirtualDesktopManager',a.PROPS,'Set',a.VDM,'current',a.dbus.String(c['desktop'],variant_level=1));a.wait_for(lambda:a.replica()['state']['activeDesktopId']==c['desktop']);time.sleep(1.5)
    a.resources('02d-empty-test-desktop-baseline',samples=5,seconds=2)
    windows=[a.launch_terminal(),a.launch_terminal()]
    a.wait_for(lambda:all(a.window_record(w['native'])['desktopIds']==[c['desktop']] for w in windows));time.sleep(.5)
    started=time.time();a.command([sys.executable,'tools/overview-dev.py','toggle']);a.wait_for(lambda:a.active(c['id']),3);time.sleep(.5)
    a.capture('02d-two-preview-measurement',c['id']);a.resources('02d-two-native-previews',samples=5,seconds=2)
    valid=a.active(c['id']) and str(a.prop('/VirtualDesktopManager',a.VDM,'current'))==c['desktop']
    a.atomic(a.directory()/'02d-performance-context.json',{'validAtEnd':valid,'started':started,'ended':time.time(),'nativePreviewClients':windows,'sampleCount':5,'sampleSeconds':2,'physicalIdleNotAsserted':True})
    if a.active(c['id']):a.command([sys.executable,'tools/overview-dev.py','toggle']);time.sleep(.4)
    for w in windows:
        a.drive(w,'close');a.wait_for(lambda w=w:a.window_record(w['native']) is None)
    a.command([sys.executable,'tools/overview-dev.py','unload']);assert not a.loaded(c['id'])
    a.resources('02d-after-overview-unload',samples=5,seconds=2)
    print('Independent refusal/recovery check passed. Preview sample endpoint validity:',valid)


def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('operation',choices=['setup','open','observe','cleanup','guard','independent']);parser.add_argument('--group',default='escape');args=parser.parse_args()
    if not args.group.isalpha():raise ValueError('Group must be a fixed alphabetic label')
    if args.operation=='independent':independent()
    elif args.operation=='guard':guard()
    elif args.operation=='cleanup':cleanup()
    elif args.operation=='setup':setup()
    elif args.operation=='open':open_group(args.group)
    else:observe(args.group)
if __name__=='__main__':main()
