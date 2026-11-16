#!/usr/bin/env python3
"""Replay sanitized production frames through unchanged Python and Rust replicas."""
import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
sys.dont_write_bytecode = True

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / 'crates/depthos-observer/Cargo.toml'
spec = importlib.util.spec_from_file_location('python_replica', ROOT / 'tools/state-inspector/state.py')
state = importlib.util.module_from_spec(spec)
spec.loader.exec_module(state)
trace = json.loads(subprocess.check_output(['node', str(ROOT / 'tests/observer-adapter/generate-trace.cjs')], cwd=ROOT))
assert trace == json.loads((ROOT / 'tests/observer-adapter/trace.json').read_text()), 'Recorded fixture drift'

def packet(frames, position=None):
    last = frames[-1] if frames else {'sequence': 0, 'revision': 1}
    return {'schemaVersion': 1, 'sessionId': 'test-session', 'consumerId': 'test-consumer',
            'frames': copy.deepcopy(frames), 'position': position or {k: last[k] for k in ('sequence','revision')}}

frames = trace['frames']
scenarios = {
    'all-native-events': [packet([f]) for f in frames] + [packet([trace['final']])],
    'duplicate': [packet([frames[0]]), packet([frames[1]]), packet([frames[1]])],
    'sequence-gap-and-recovery': [packet([frames[0]]), packet([frames[2]]), packet([trace['final']])],
    'heartbeat-final-loss': [packet([frames[0]]), packet([], {'sequence': 999, 'revision':999}), packet([trace['final']])],
    'batch-position-ahead': [packet([frames[0]], {'sequence': 999, 'revision':999})],
    'receiver-starts-with-event': [packet([frames[1]]), packet([trace['final']])],
}
revision_gap=copy.deepcopy(frames[1]);revision_gap['revision'] += 10
scenarios['revision-gap']=[packet([frames[0]]),packet([revision_gap]),packet([trace['final']])]
subprocess.run(['cargo','build','--quiet','--locked','--manifest-path',str(MANIFEST),'--example','replay'],cwd=ROOT,check=True)
# Cargo controls artifact layout on this host; obtain the executable from JSON messages.
build=subprocess.check_output(['cargo','build','--quiet','--locked','--manifest-path',str(MANIFEST),'--example','replay','--message-format=json'],cwd=ROOT,text=True)
exe=next(json.loads(line)['executable'] for line in build.splitlines() if json.loads(line).get('executable'))
comparisons=0
for name,packets in scenarios.items():
    rust=json.loads(subprocess.check_output([exe],input=json.dumps(packets),text=True,cwd=ROOT))
    python=state.Replica()
    for packet_,result in zip(packets,rust,strict=True):
        python.accept(packet_,'test-consumer')
        assert result['error'] is False, name
        assert (result['health']=='Synchronized') == (not python.need_snapshot), name
        if not python.need_snapshot:
            snapshot=result['snapshot']
            assert snapshot['state'] == state.redacted(python.state), name
            assert snapshot['sequence']==python.sequence and snapshot['revision']==python.revision, name
        else:
            assert result['snapshot'] is None, name
        comparisons+=1
    print(f'PASS {name}')
print(f'Parity: {len(scenarios)} scenarios, {comparisons} packet comparisons')
