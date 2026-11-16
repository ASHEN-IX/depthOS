#!/usr/bin/env python3
"""Exercise production QML with fake KWin types, offscreen, outside KWin.

These fixtures verify Qt Quick behavior only, never compositor integration.
Python and QtTest are development tools, not installed effect dependencies.
"""
import json
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
PACKAGE = ROOT / "experiments/kwin-effect"
meta = json.loads((PACKAGE / "metadata.json").read_text())
assert meta["KPlugin"]["Id"] == "depthos-spatial-poc"
assert meta["KPlugin"]["EnabledByDefault"] is False
assert meta["KPackageStructure"] == "KWin/Effect"
assert meta["X-Plasma-API"] == "declarativescript"
assert "License" not in meta["KPlugin"], "No license has been approved"
print("Metadata contract: PASS", flush=True)

for script in sorted((ROOT / "scripts").glob("*.fish")):
    subprocess.run(["fish", "--no-execute", str(script)], check=True)
print("Fish syntax: PASS", flush=True)
subprocess.run(["kpackagetool6", "--type", "KWin/Effect", "--show", str(PACKAGE)], check=True)
qtbin = Path("/usr/lib/qt6/bin")
subprocess.run([str(qtbin / "qmlformat"), str(PACKAGE / "contents/ui/main.qml")],
               check=True, stdout=subprocess.DEVNULL)
print("QML parsing: PASS", flush=True)

with tempfile.TemporaryDirectory(prefix="depthos-qtquick-") as temporary:
    base = Path(temporary)
    module = base / "org/kde/kwin"
    module.mkdir(parents=True)
    (module / "qmldir").write_text(
        "module org.kde.kwin\nSceneEffect 1.0 SceneEffect.qml\n"
        "ShortcutHandler 1.0 ShortcutHandler.qml\n")
    (module / "SceneEffect.qml").write_text("""import QtQuick
QtObject {
    default property list<QtObject> data
    property Component delegate
    property bool visible: false
}
""")
    (module / "ShortcutHandler.qml").write_text("""import QtQuick
QtObject {
    property string name
    property string text
    property string sequence
    signal activated()
}
""")
    shutil.copyfile(PACKAGE / "contents/ui/main.qml", base / "Effect.qml")
    shutil.copyfile(ROOT / "tests/tst_lifecycle.qml", base / "tst_lifecycle.qml")
    environment = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software",
                       QT_FORCE_STDERR_LOGGING="1")
    # A createObject() fixture alone misses KWin's per-screen QQmlContext path.
    flags = shlex.split(subprocess.check_output(
        ["pkg-config", "--cflags", "--libs", "Qt6Quick", "Qt6Qml"], text=True))
    probe = base / "delegate-context"
    subprocess.run(["g++", "-std=c++17", "-fPIC", str(ROOT / "tests/delegate-context.cpp"),
                    "-o", str(probe), *flags], check=True, timeout=60)
    subprocess.run([str(probe), str(base), str(base / "Effect.qml")],
                   env=environment, check=True, timeout=15)
    # Negative control: prove the former pragma reproduces the exact failure.
    source = (base / "Effect.qml").read_text()
    assert "pragma ComponentBehavior: Unbound" in source
    (base / "BoundEffect.qml").write_text(source.replace(
        "pragma ComponentBehavior: Unbound", "pragma ComponentBehavior: Bound", 1))
    negative = subprocess.run([str(probe), str(base), str(base / "BoundEffect.qml")],
                              env=environment, capture_output=True, text=True, timeout=15)
    assert negative.returncode == 1, negative.stdout + negative.stderr
    assert "Cannot instantiate bound component outside its creation context" in negative.stderr
    print("Former Bound pragma: expected context failure reproduced (negative control)", flush=True)
    print("Running production QML with MOCK KWin types; no compositor runtime validation:", flush=True)
    subprocess.run([str(qtbin / "qmltestrunner"), "-input", str(base / "tst_lifecycle.qml"),
                    "-import", str(base)], env=environment, check=True, timeout=60)
