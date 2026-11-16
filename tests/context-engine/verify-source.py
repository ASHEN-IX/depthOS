#!/usr/bin/env python3
"""Read-only Phase 2.3B audit. Earlier manifests remain immutable baselines."""
import hashlib
import json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
manifest=json.loads((Path(__file__).with_name('manifest.json')).read_text())
def differences(mapping):
    return [name for name,digest in mapping.items()
            if not (ROOT/name).is_file() or (ROOT/name).is_symlink()
            or hashlib.sha256((ROOT/name).read_bytes()).hexdigest()!=digest]
protected=differences(manifest['protectedFiles'])
engine=differences(manifest['engineFiles'])
kde=[]
for name,digest in manifest['kde'].items():
    p=Path(name)
    actual=hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None
    if actual!=digest:kde.append(name)
print(json.dumps({'backup':manifest['backup'],'protectedFilesVerified':len(manifest['protectedFiles']),
                 'engineFilesVerified':len(manifest['engineFiles']),'protectedDifferences':protected,
                 'engineDifferences':engine,'kdeConfigurationDifferences':kde},indent=2))
assert not protected and not engine and not kde, 'Recorded source/configuration differences require review'
