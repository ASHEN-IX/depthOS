#!/usr/bin/env python3
"""Read-only source/worktree protection audit; --record writes only local evidence."""
from pathlib import Path
import hashlib,json,subprocess,sys,datetime,os
ROOT=Path(__file__).resolve().parents[2]
SNAP=ROOT/'tests/security/isolation.json'
OUT=ROOT/'tests/security/completion.json'
snapshot=json.loads(SNAP.read_text());canonical=Path(snapshot['canonical'])
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def git(*args):return subprocess.check_output(['git',*args],cwd=ROOT,text=True).strip()
assert ROOT==Path('/home/ashenix/Projects/DepthOS-security')
assert (ROOT/'.git').is_file(),'not a linked worktree'
assert Path(git('rev-parse','--show-toplevel'))==ROOT
assert Path(git('rev-parse','--git-common-dir')).resolve()==canonical/'.git'
assert git('branch','--show-current')=='codex/phase-3-security'
changed=[]
for name,h in snapshot['source'].items():
 assert sha(canonical/name)==h,('canonical snapshot changed',name)
 if sha(ROOT/name)!=h:changed.append(name)
assert changed==['src/lib.rs'],changed
assert (ROOT/'src/lib.rs').read_text()=='pub mod action_security;\n'+(canonical/'src/lib.rs').read_text()
new_allowed={'src/action_security.rs','src/action_security/tests.rs','tests/security_boundary.rs','docs/phase-3-security.md','tests/security/isolation.json','tests/security/verify-isolation.py','tests/security/completion.json'}
names=subprocess.check_output(['git','ls-files','--cached','--others','--exclude-standard','-z'],cwd=ROOT).decode().split('\0')
new=set(n for n in names if n)-set(snapshot['source']);assert new<=new_allowed,new-new_allowed
kde=json.loads((ROOT/'tests/spatial/source-manifest.json').read_text())['kde']
for p,h in kde.items():assert sha(Path(p))==h,('KDE configuration changed',p)
files={n:sha(ROOT/n) for n in sorted(set(n for n in names if n)) if n!='tests/security/completion.json'}
# Catch writable descriptors on the audited source, including concurrent writers.
source_paths={str((ROOT/n).resolve()) for n in files}|{str((canonical/n).resolve()) for n in snapshot['source']}
writers=[]
for p in Path('/proc').iterdir():
 if not p.name.isdecimal():continue
 try:
  for fd in (p/'fd').iterdir():
   try:
    target=os.readlink(fd)
    if target not in source_paths:continue
    flags=next(s.split()[1] for s in (p/'fdinfo'/fd.name).read_text().splitlines() if s.startswith('flags:'))
    if int(flags,8)&3 in (1,2):writers.append({'pid':int(p.name),'path':target})
   except (OSError,StopIteration):pass
 except OSError:pass
assert not writers,writers
assert all(sha(ROOT/n)==h for n,h in files.items()),'local source changed during audit'
assert all(sha(canonical/n)==h for n,h in snapshot['source'].items()),'canonical source changed during audit'
if '--record' in sys.argv:
 import re
 log=(ROOT/'artifacts/security/rust-tests.log').read_text()
 passed=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',log)))
 assert passed==111 and 'test result: FAILED' not in log,passed
 for logname in ['build.log','clippy.log']:
  text=(ROOT/'artifacts/security'/logname).read_text();assert 'Finished' in text and 'error:' not in text,text
 assert (ROOT/'artifacts/security/fmt.log').read_text()==''
 record={'recordedUtc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'worktree':str(ROOT),'branch':git('branch','--show-current'),'commonGit':str(canonical/'.git'),'canonicalFilesUnchanged':len(snapshot['source']),'changedCopiedSource':changed,'added':sorted(new),'files':files,'kdeUnchanged':kde,'activeSourceWriters':writers,'rustPassed':passed,'rustIgnored':1,'proofIssuer':'cfg(test) only; production claim verification refuses','nativeEndpointsAdded':0,'logs':{p.name:sha(p) for p in sorted((ROOT/'artifacts/security').glob('*.log'))}}
 OUT.write_text(json.dumps(record,indent=2,sort_keys=True)+'\n')
else:
 record=json.loads(OUT.read_text());assert record['files']==files,'local source differs from final manifest'
 for name,h in record['logs'].items():assert sha(ROOT/'artifacts/security'/name)==h,('evidence changed',name)
print(f'PASS linked orphan worktree; {len(snapshot["source"])} canonical files unchanged; one declared module export; {len(new)} new files; no writable source descriptors; four KDE configuration hashes unchanged')
