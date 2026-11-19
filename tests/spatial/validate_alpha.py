#!/usr/bin/env python3
"""Validation harness for DepthOS Spatial Workspace Alpha QML components.

Runs offscreen QtTest verification testing:
- Level A (World), Level B (Workspace), Level C (Logical Desktop) navigation
- H / L horizontal sibling navigation
- J / K vertical navigation
- Enter deep traversal and Backspace/Escape parent traversal
- Search command palette (/) and direct context navigation
- Selection preservation across levels
- Deterministic focus indicators and Home reset
- Observation status badges (Live, Stale, Reconnecting, Unavailable, Unknown)
- Invariant: Never claim stale observation is live
- Reduced motion toggle support
- Privacy redaction toggle support
- Generates and verifies offscreen rendered visual evidence artifacts
"""
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
QT = Path("/usr/lib/qt6/bin")
QMLTEST = QT / "qmltestrunner"
QMLFORMAT = QT / "qmlformat"
QMLLINT = QT / "qmllint"
SPATIAL_DIR = ROOT / "experiments/spatial-alpha"
TEST_FILE = ROOT / "tests/spatial/tst_spatial_alpha.qml"
CAPTURE_SCRIPT = ROOT / "tests/spatial/capture_visuals.py"
ARTIFACTS_DIR = ROOT / "artifacts/phase-4-1"


def main():
    print("=== DepthOS Spatial Workspace Alpha QML Test Suite ===")

    # 1. Check format & lint
    qml_files = list(SPATIAL_DIR.glob("*.qml"))
    print(f"Checking {len(qml_files)} Spatial Alpha QML components with qmllint...")
    for f in qml_files:
        res = subprocess.run([str(QMLLINT), str(f)], capture_output=True, text=True)
        if res.returncode != 0:
            print(f"qmllint failed on {f.name}:\n{res.stderr or res.stdout}")
            sys.exit(1)

    print("QML lint checks: PASS")

    # 2. Run qmltestrunner
    env = dict(
        os.environ,
        QT_QPA_PLATFORM="offscreen",
        QT_QUICK_BACKEND="software",
        QT_FORCE_STDERR_LOGGING="1"
    )

    cmd = [
        str(QMLTEST),
        "-import", str(SPATIAL_DIR),
        "-input", str(TEST_FILE)
    ]
    print(f"Executing offscreen test runner: {' '.join(cmd)}")
    start = time.monotonic()
    proc = subprocess.run(cmd, env=env, text=True)
    duration = time.monotonic() - start

    if proc.returncode != 0:
        print(f"qmltestrunner failed with exit code {proc.returncode}")
        sys.exit(proc.returncode)

    print(f"DepthOS Spatial Alpha QML tests PASSED in {duration:.2f}s")

    # 3. Generate offscreen visual evidence artifacts
    print("Generating visual evidence artifacts...")
    cap_proc = subprocess.run([sys.executable, str(CAPTURE_SCRIPT)], capture_output=True, text=True)
    if cap_proc.returncode != 0:
        print(f"capture_visuals.py failed:\n{cap_proc.stderr or cap_proc.stdout}")
        sys.exit(cap_proc.returncode)

    required_artifacts = [
        "level-a-world-overview.png",
        "level-b-workspace-overview.png",
        "level-c-desktop-overview.png",
        "level-c-titles-revealed.png",
        "search-palette-modal.png",
        "status-badges-gallery.png",
        "reduced-motion-view.png",
    ]
    for art in required_artifacts:
        art_path = ARTIFACTS_DIR / art
        if not art_path.exists() or art_path.stat().st_size == 0:
            print(f"Missing or empty required visual artifact: {art}")
            sys.exit(1)

    print(f"All {len(required_artifacts)} visual artifacts verified successfully in {ARTIFACTS_DIR}")


if __name__ == "__main__":
    main()
