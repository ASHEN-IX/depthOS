// Synthetic observer-only microbenchmark; never connects to KWin or D-Bus.
const {performance} = require('node:perf_hooks');
const {fixture, window} = require('./fixtures.cjs');
const before = performance.now();
const windows = Array.from({length: 100}, () => window());
const f = fixture(windows);
const initialMs = performance.now() - before;
const start = performance.now();
for (let i = 0; i < 10000; ++i) {
    const w = windows[i % windows.length];
    w.frameGeometry.x = i;
    w.frameGeometryChanged.emit();
}
f.observer.flush();
const pending = f.observer.stats().queuedEvents;
let events = 0, batch;
while ((batch = f.observer.take()).length) events += batch.length;
console.log(JSON.stringify({kind: 'synthetic-node-microbenchmark', windows: windows.length,
    initialMs, geometrySignals: 10000, coalescedEvents: events, pendingBeforeDrain: pending,
    geometryAndDrainMs: performance.now() - start,
    initialSnapshotBytes: Buffer.byteLength(JSON.stringify(f.initial)),
    overflows: f.observer.stats().overflows}, null, 2));
f.observer.stop();
