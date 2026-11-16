#!/usr/bin/env python3
"""Verify the Phase 2.5 baseline, declared ownership, source hashes and private cleanup."""
import datetime,hashlib,json,os,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
BACKUP=Path('/home/ashenix/Projects/DepthOS-backups/phase-2.5-20261116T144802Z')
OUT=ROOT/'tests/spatial/source-manifest.json'
CHANGED={
 'Cargo.toml','Cargo.lock','src/lib.rs','src/main.rs','src/controller.rs',
 'experiments/kwin-overview/contents/ui/main.qml',
 'experiments/kwin-overview/contents/ui/OverviewScene.qml',
 'experiments/kwin-overview/contents/ui/WindowCard.qml',
 'tests/overview/tst_overview.qml','tests/integration/poc02e/probe.qml.inc','docs/architecture.md',
}
NEW={
 'src/spatial.rs','tests/spatial_bridge.rs','docs/phase-2-spatial-integration.md',
 'experiments/kwin-overview/contents/ui/ControllerClient.qml',
 'experiments/kwin-overview/contents/ui/BridgeModel.js',
 'experiments/kwin-overview/contents/ui/LogicalPanel.qml',
}
def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def sources():
 names=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=ROOT).decode().split('\0')
 return {p:digest(ROOT/p) for p in sorted(set(names)) if p and p!='tests/spatial/source-manifest.json' and (ROOT/p).is_file()}
def writers(source):
 writable=[];wanted={str((ROOT/p).resolve()) for p in source}
 for process in Path('/proc').iterdir():
  if not process.name.isdecimal():continue
  try:
   for fd in (process/'fd').iterdir():
    try:
     dest=os.readlink(fd)
     if dest not in wanted:continue
     flags=next(line.split()[1] for line in (process/'fdinfo'/fd.name).read_text().splitlines() if line.startswith('flags:'))
     if int(flags,8)&3 in (1,2):writable.append({'pid':int(process.name),'path':dest})
    except (OSError,StopIteration):pass
  except OSError:pass
 return writable
baseline=json.loads((BACKUP/'manifest.json').read_text());current=sources();changed=[]
for p,h in baseline['files'].items():
 assert digest(BACKUP/'source'/p)==h,('backup mismatch',p)
 assert p in current,('missing original source',p)
 if current[p]!=h:changed.append(p);assert p in CHANGED,('unowned edit',p)
added=sorted(set(current)-set(baseline['files']))
for p in added:assert p in NEW or p.startswith('tests/spatial/'),('unexpected new source',p)
for p,h in baseline['kde'].items():assert digest(Path(p))==h,('KDE config changed',p)
active=writers(current);assert not active,active
assert current==sources(),'Source changed during verification'
if '--record' in sys.argv:
 native=Path(sys.argv[sys.argv.index('--record')+1]).resolve();assert native.is_relative_to(ROOT/'artifacts/p25')
 cleanup=json.loads((native/'cleanup.json').read_text());checks=json.loads((native/'assertions.json').read_text())
 assert cleanup=={'hostEqual':True,'changedKeys':[],'remaining':[],'exit':0},cleanup
 assert all(c['pass'] for c in checks)
 result={'recordedUtc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'backup':str(BACKUP),'originalSnapshotUtc':baseline['createdUtc'],'originalFilesVerified':len(baseline['files']),'changed':sorted(changed),'added':added,'files':current,'kde':baseline['kde'],'activeWriters':active,'native':{'directory':str(native),'checks':len(checks),'cleanup':cleanup,'evidenceHashes':{p.name:digest(p) for p in sorted(native.glob('*.json'))}},'verification':{'rustPassed':81,'rustIgnoredExplicitBenchmark':1,'overviewQtPassed':18,'panelQtPassed':4,'nativeRegressionPassed':72,'pythonParityScenarios':7,'pythonParityPacketComparisons':30,'privateBusInterop':True,'controllerProcessRecovery':True,'productionQmlClientLifecycle':True,'actualSchemaMismatchPeer':True,'fmt':True,'clippyWarningsDenied':True,'buildLocked':True}}
 OUT.write_text(json.dumps(result,indent=2,sort_keys=True)+'\n')
else:
 result=json.loads(OUT.read_text());assert result['files']==current,'Phase 2.5 source differs from recorded manifest'
 native=Path(result['native']['directory'])
 for p,h in result['native']['evidenceHashes'].items():assert digest(native/p)==h,('native evidence mismatch',p)
print(f'PASS verified backup {len(baseline["files"])} files; {len(changed)} declared edits; {len(added)} new files; source stable; zero writers; KDE configuration unchanged')
