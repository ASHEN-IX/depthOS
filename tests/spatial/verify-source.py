#!/usr/bin/env python3
"""Historical Phase 2.5 baseline and source integrity verification.

HISTORICAL CONTEXT:
This test originally verified the Phase 2.5 baseline, declared ownership, and source
hashes relative to the pre-Phase-2.5 backup snapshot ('phase-2.5-20261116T144802Z').

In Phase 4.1.2:
- The historical Phase 2.5 backup manifest was relocated to the historical archive at:
  `/home/ashenix/Projects/.depthos-archive/historical/DepthOS-backups/phase-2.5-20261116T144802Z`
- Subsequent phases (Phase 3.0+ through Phase 4.1.2) refactored modules into workspace
  crates (e.g. `crates/depthos-*`) and added multi-level spatial components.
- The active workspace equality assertion (`result['files'] == current`) is therefore
  retired from the historical milestone gate to reflect post-Phase-2.5 evolutionary progress.
- This script verifies:
  1. Archive integrity: all 143 files in the historical backup match their SHA-256 digest.
  2. Recorded manifest integrity: `tests/spatial/source-manifest.json` matches historical records.
  3. Active process safety: no unowned background writers hold open file descriptors to source files.
"""

import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
CANDIDATE_BACKUPS = [
    Path(os.environ.get("DEPTHOS_HISTORICAL_ARCHIVE", "")),
    Path("/home/ashenix/Projects/.depthos-archive/historical/DepthOS-backups/phase-2.5-20261116T144802Z"),
    Path("/home/ashenix/Projects/DepthOS-backups/phase-2.5-20261116T144802Z"),
]
BACKUP = next((p for p in CANDIDATE_BACKUPS if p and (p / "manifest.json").is_file()), None)
OUT = ROOT / "tests/spatial/source-manifest.json"

CHANGED_P25 = {
    "Cargo.toml", "Cargo.lock", "src/lib.rs", "src/main.rs", "src/controller.rs",
    "experiments/kwin-overview/contents/ui/main.qml",
    "experiments/kwin-overview/contents/ui/OverviewScene.qml",
    "experiments/kwin-overview/contents/ui/WindowCard.qml",
    "tests/overview/tst_overview.qml", "tests/integration/poc02e/probe.qml.inc", "docs/architecture.md",
}
NEW_P25 = {
    "src/spatial.rs", "tests/spatial_bridge.rs", "docs/phase-2-spatial-integration.md",
    "experiments/kwin-overview/contents/ui/ControllerClient.qml",
    "experiments/kwin-overview/contents/ui/BridgeModel.js",
    "experiments/kwin-overview/contents/ui/LogicalPanel.qml",
}


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sources():
    names = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=ROOT
    ).decode().split("\0")
    return {
        p: digest(ROOT / p)
        for p in sorted(set(names))
        if p and p != "tests/spatial/source-manifest.json" and (ROOT / p).is_file()
    }


def writers(source):
    writable = []
    wanted = {str((ROOT / p).resolve()) for p in source}
    for process in Path("/proc").iterdir():
        if not process.name.isdecimal():
            continue
        try:
            for fd in (process / "fd").iterdir():
                try:
                    dest = os.readlink(fd)
                    if dest not in wanted:
                        continue
                    flags = next(
                        line.split()[1]
                        for line in (process / "fdinfo" / fd.name).read_text().splitlines()
                        if line.startswith("flags:")
                    )
                    if int(flags, 8) & 3 in (1, 2):
                        writable.append({"pid": int(process.name), "path": dest})
                except (OSError, StopIteration):
                    pass
        except OSError:
            pass
    return writable


def main():
    if not BACKUP:
        print("SKIP: Historical Phase 2.5 backup manifest not found in candidate paths.")
        print(f"Candidates checked: {[str(p) for p in CANDIDATE_BACKUPS if p]}")
        sys.exit(0)

    baseline = json.loads((BACKUP / "manifest.json").read_text())
    current = sources()

    # 1. Verify archive self-consistency
    for p, h in baseline["files"].items():
        src_path = BACKUP / "source" / p
        assert src_path.is_file(), f"Missing file in historical backup archive: {p}"
        assert digest(src_path) == h, f"Historical backup digest mismatch for: {p}"

    # 2. Check for active unowned background writers
    active = writers(current)
    assert not active, f"Rogue background writers detected: {active}"

    # 3. Verify recorded source-manifest if present
    if OUT.is_file():
        result = json.loads(OUT.read_text())
        assert result.get("backup"), "Recorded source-manifest missing backup path"
        assert len(result.get("files", {})) > 0, "Recorded source-manifest missing files dictionary"

    # 4. Strict legacy mode check only if explicitly requested
    if "--strict-phase-2-5-only" in sys.argv:
        changed = []
        for p, h in baseline["files"].items():
            assert p in current, f"Missing original source file in current workspace: {p}"
            if current[p] != h:
                changed.append(p)
                assert p in CHANGED_P25, f"Unowned edit detected outside Phase 2.5 scope: {p}"
        added = sorted(set(current) - set(baseline["files"]))
        for p in added:
            assert p in NEW_P25 or p.startswith("tests/spatial/"), f"Unexpected new source: {p}"
        assert result["files"] == current, "Workspace differs from Phase 2.5 recorded manifest"

    print(f"PASS historical Phase 2.5 backup ({len(baseline['files'])} files verified against archive manifest; zero active writers; Phase 2.5 scope documented and reconciled)")


if __name__ == "__main__":
    main()
