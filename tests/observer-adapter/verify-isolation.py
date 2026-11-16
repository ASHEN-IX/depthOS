#!/usr/bin/env python3
"""Read-only comparison of the recorded source baseline and protected copied files."""
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
# Phase 2.2B retains the original copy audit below, but verifies the integrated
# tree against its reviewed baseline when an integration manifest is present.
integration=ROOT/'tests/observer-adapter/isolation/integration-manifest.json'
if integration.exists():
    import sys
    audit=json.loads(integration.read_text())
    failures=[]
    for name,digest in audit['protectedSourceFiles'].items():
        p=ROOT/name
        if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest()!=digest:failures.append(name)
    for name,digest in audit['integrationFiles'].items():
        p=ROOT/name
        if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest()!=digest:failures.append(name)
    kde_changes=[]
    for name,digest in audit['kdeConfigurationHashes'].items():
        p=Path(name)
        actual=hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None
        if actual!=digest:kde_changes.append(name)
    print(json.dumps({'backup':audit['backup'],'protectedSourceFilesVerified':len(audit['protectedSourceFiles']),
                     'integrationFilesVerified':len(audit['integrationFiles']),'sourceDifferences':failures,
                     'kdeConfigurationDifferences':kde_changes},indent=2))
    assert not failures and not kde_changes, 'Integration source/configuration differs from recorded verification'
    sys.exit(0)

SOURCE=Path('/home/ashenix/Projects/DepthOS')
manifest=json.loads((ROOT/'tests/observer-adapter/isolation/source-manifest.json').read_text())
changed_original=[]
changed_copy=[]
for name,digest in manifest['files'].items():
    for base,failures in [(SOURCE,changed_original),(ROOT,changed_copy)]:
        p=base/name
        if not p.is_file() or hashlib.sha256(p.read_bytes()).hexdigest()!=digest:failures.append(name)
unexpected=[]
for p in ROOT.rglob('*'):
    if not p.is_file():continue
    name=p.relative_to(ROOT).as_posix()
    if name in manifest['files'] or name.startswith(('crates/depthos-observer/','tests/observer-adapter/')) or name=='docs/phase-2-observer.md':continue
    unexpected.append(name)
assert not unexpected, f'Unexpected files outside Agent B ownership: {unexpected}'
print(json.dumps({'snapshotStartedUtc':manifest['startedUtc'],'filesVerified':len(manifest['files']),
                 'originalBaselineChanges':changed_original,'protectedCopyChanges':changed_copy},indent=2))
assert not changed_copy, 'Protected copied source changed'
# Other agents may change the original concurrently; report precisely rather than overwrite it.
if changed_original:print('Concurrent original changes reported; no original files were written by this verifier.')
