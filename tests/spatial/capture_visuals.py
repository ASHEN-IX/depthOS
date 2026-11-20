#!/usr/bin/env python3
"""Capture high-fidelity offscreen visual evidence of DepthOS Spatial Alpha.

Renders real QML frames offscreen and saves them to artifacts/phase-4-1/:
1. level-a-world-overview.png (Level A: World Overview)
2. level-b-workspace-overview.png (Level B: Workspace Overview)
3. level-c-desktop-overview.png (Level C: Logical Desktop Overview & Window Cards)
4. level-c-titles-revealed.png (Level C with Titles Revealed)
5. search-palette-modal.png (Search Command Palette with category badges)
6. status-badges-gallery.png (Comprehensive view of all observation badge states)
7. reduced-motion-view.png (Reduced Motion mode active)
"""
import os
from pathlib import Path
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS_DIR = ROOT / "artifacts/phase-4-1"
ARTIFACTS_DIR.mkdir(parents=True, exist_ok=True)

# Configure offscreen software rendering
os.environ["QT_QPA_PLATFORM"] = "offscreen"
os.environ["QT_QUICK_BACKEND"] = "software"

from PyQt6.QtCore import QUrl
from PyQt6.QtGui import QGuiApplication
from PyQt6.QtQml import QQmlApplicationEngine


def capture_item(engine, app, item, output_path):
    if output_path.exists():
        output_path.unlink()
    escaped_path = str(output_path).replace("\\", "/")
    cb = engine.evaluate(f'(function(result) {{ result.saveToFile("{escaped_path}"); }})')
    item.grabToImage(cb)
    for _ in range(30):
        app.processEvents()
        if output_path.exists() and output_path.stat().st_size > 0:
            return True
        time.sleep(0.02)
    return output_path.exists() and output_path.stat().st_size > 0


def main():
    print("=== DepthOS Spatial Alpha: Generating Offscreen Visual Artifacts ===")
    app = QGuiApplication(sys.argv)
    engine = QQmlApplicationEngine()
    engine.addImportPath(str(ROOT / "experiments/spatial-alpha"))

    qml_file = ROOT / "experiments/spatial-alpha/main.qml"
    engine.load(QUrl.fromLocalFile(str(qml_file)))

    if not engine.rootObjects():
        sys.stderr.write("Failed to load QML root object\n")
        sys.exit(1)

    root = engine.rootObjects()[0]
    root.show()
    app.processEvents()

    scene = root.property("scene")
    backend = root.property("backend")

    root.setProperty("reducedMotion", False)
    backend.loadMock()
    app.processEvents()

    worlds = backend.property("worlds").toVariant()

    # 1. Level A: World Overview
    scene.setProperty("currentLevel", 1)
    scene.setProperty("worldIndex", 0)
    backend.setProperty("selectedWorldId", worlds[0]["id"])
    scene.syncIndices()
    for _ in range(5):
        app.processEvents()
        time.sleep(0.01)

    path_a = ARTIFACTS_DIR / "level-a-world-overview.png"
    if capture_item(engine, app, scene, path_a):
        print(f"[OK] Captured Level A: {path_a.name} ({path_a.stat().st_size} bytes)")
    else:
        print(f"[FAIL] Failed to capture Level A")

    # 2. Level B: Workspace Overview
    scene.setProperty("currentLevel", 2)
    scene.setProperty("workspaceIndex", 0)
    scene.syncIndices()
    for _ in range(5):
        app.processEvents()
        time.sleep(0.01)

    path_b = ARTIFACTS_DIR / "level-b-workspace-overview.png"
    if capture_item(engine, app, scene, path_b):
        print(f"[OK] Captured Level B: {path_b.name} ({path_b.stat().st_size} bytes)")
    else:
        print(f"[FAIL] Failed to capture Level B")

    # 3. Level C: Logical Desktop Overview (Privacy Redacted Titles by default)
    scene.setProperty("currentLevel", 3)
    scene.setProperty("desktopIndex", 0)
    scene.setProperty("showTitles", False)
    scene.syncIndices()
    for _ in range(5):
        app.processEvents()
        time.sleep(0.01)

    path_c = ARTIFACTS_DIR / "level-c-desktop-overview.png"
    if capture_item(engine, app, scene, path_c):
        print(f"[OK] Captured Level C (Redacted): {path_c.name} ({path_c.stat().st_size} bytes)")
    else:
        print(f"[FAIL] Failed to capture Level C")

    # 4. Level C: Titles Revealed (Toggle via T)
    scene.setProperty("showTitles", True)
    for _ in range(5):
        app.processEvents()
        time.sleep(0.01)

    path_c_titles = ARTIFACTS_DIR / "level-c-titles-revealed.png"
    if capture_item(engine, app, scene, path_c_titles):
        print(f"[OK] Captured Level C (Titles Revealed): {path_c_titles.name} ({path_c_titles.stat().st_size} bytes)")
    else:
        print(f"[FAIL] Failed to capture Level C Titles")

    scene.setProperty("showTitles", False)

    # 5. Search Command Palette Modal
    scene.setProperty("currentLevel", 1)
    scene.syncIndices()
    scene.openSearch()
    for _ in range(5):
        app.processEvents()
        time.sleep(0.01)

    path_search = ARTIFACTS_DIR / "search-palette-modal.png"
    if capture_item(engine, app, scene, path_search):
        print(f"[OK] Captured Search Palette: {path_search.name} ({path_search.stat().st_size} bytes)")
    else:
        print(f"[FAIL] Failed to capture Search Palette")

    # Close search palette
    scene.returnToParent()
    for _ in range(5):
        app.processEvents()
        time.sleep(0.01)

    # 6. Status Badges Gallery (Switch to desk-kernel-build or desk-spatial-scene which has Stale / Reconnecting / Unavailable)
    desktops = backend.property("desktops").toVariant()
    build_desktop = next((d for d in desktops if d.get("id") == "desk-kernel-build"), None)
    if build_desktop:
        backend.setProperty("selectedDesktopId", build_desktop["id"])
        scene.setProperty("currentLevel", 3)
        scene.syncIndices()
        for _ in range(5):
            app.processEvents()
            time.sleep(0.01)

    path_badges = ARTIFACTS_DIR / "status-badges-gallery.png"
    if capture_item(engine, app, scene, path_badges):
        print(f"[OK] Captured Status Badges Gallery: {path_badges.name} ({path_badges.stat().st_size} bytes)")
    else:
        print(f"[FAIL] Failed to capture Status Badges")

    # 7. Reduced Motion View
    root.setProperty("reducedMotion", True)
    scene.setProperty("currentLevel", 2)
    scene.syncIndices()
    for _ in range(5):
        app.processEvents()
        time.sleep(0.01)

    path_motion = ARTIFACTS_DIR / "reduced-motion-view.png"
    if capture_item(engine, app, scene, path_motion):
        print(f"[OK] Captured Reduced Motion View: {path_motion.name} ({path_motion.stat().st_size} bytes)")
    else:
        print(f"[FAIL] Failed to capture Reduced Motion View")

    print(f"=== Visual artifact generation complete. Saved 7 artifacts to {ARTIFACTS_DIR} ===")


if __name__ == "__main__":
    main()
