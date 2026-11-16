#!/usr/bin/env python3
"""Read-only Phase 2.4 source/configuration verification; historical audits stay frozen."""
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
audit=json.loads(Path(__file__).with_name('manifest.json').read_text())
def differences(files):
    return [name for name,digest in files.items() if not (ROOT/name).is_file()
            or (ROOT/name).is_symlink() or hashlib.sha256((ROOT/name).read_bytes()).hexdigest()!=digest]
protected=differences(audit['protectedFiles']);changed=differences(audit['lifecycleFiles']);kde=[]
for name,digest in audit['kde'].items():
    p=Path(name);actual=hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None
    if actual!=digest:kde.append(name)
print(json.dumps({'backup':audit['backup'],'protectedFilesVerified':len(audit['protectedFiles']),
                 'lifecycleFilesVerified':len(audit['lifecycleFiles']),'protectedDifferences':protected,
                 'lifecycleDifferences':changed,'kdeConfigurationDifferences':kde},indent=2))
assert not protected and not changed and not kde,'Source/configuration differences require review'
