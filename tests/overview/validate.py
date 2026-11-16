#!/usr/bin/env python3
"""Production overview QML, mock KWin boundary, real Qt/Kirigami. Never loads KWin."""
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "experiments/kwin-overview"
QT = Path("/usr/lib/qt6/bin")
meta = json.loads((PACKAGE / "metadata.json").read_text())
assert meta["KPlugin"]["Id"] == "depthos-spatial-overview"
assert meta["KPlugin"]["EnabledByDefault"] is False
assert meta["X-Plasma-API"] == "declarativescript"
assert "License" not in meta["KPlugin"]
subprocess.run(["kpackagetool6", "--type", "KWin/Effect", "--show", str(PACKAGE)], check=True)
for file in (PACKAGE / "contents/ui").glob("*.qml"):
    subprocess.run([str(QT / "qmlformat"), str(file)], stdout=subprocess.DEVNULL, check=True)
with tempfile.TemporaryDirectory(prefix="depthos-overview-test-") as temporary:
    base = Path(temporary)
    for file in (PACKAGE / "contents/ui").iterdir():
        shutil.copyfile(file, base / file.name)
    module = base / "org/kde/kwin"
    module.mkdir(parents=True)
    for file in (ROOT / "tests/overview/mock-kwin").iterdir():
        shutil.copyfile(file, module / file.name)
    preview = ROOT / "artifacts/overview-fixture.png"
    preview.parent.mkdir(exist_ok=True)
    workspace = module / "Workspace.qml"
    workspace.write_text(workspace.read_text().replace('property string previewPath: ""', 'property string previewPath: ' + json.dumps(str(preview))))
    shutil.copyfile(ROOT / "tests/overview/tst_overview.qml", base / "tst_overview.qml")
    env = dict(os.environ, QT_QPA_PLATFORM="offscreen", QT_QUICK_BACKEND="software", QT_FORCE_STDERR_LOGGING="1")
    flags = shlex.split(subprocess.check_output(["pkg-config", "--cflags", "--libs", "Qt6Quick", "Qt6Qml"], text=True))
    probe = base / "delegate-context"
    subprocess.run(["g++", "-std=c++17", "-fPIC", str(ROOT / "tests/delegate-context.cpp"), "-o", str(probe), *flags], check=True)
    subprocess.run([str(probe), str(base), str(base / "main.qml")], env=env, check=True, timeout=20)
    print("Overview tests use MOCK native models; no compositor texture or live API validation.", flush=True)
    command = [str(QT / "qmltestrunner"), "-import", str(base), "-input", str(base / "tst_overview.qml")]
    start = time.monotonic()
    process = subprocess.Popen(command, env=env)
    while True:
        pid, status, usage = os.wait4(process.pid, os.WNOHANG)
        if pid:
            process.returncode = os.waitstatus_to_exitcode(status)
            break
        if time.monotonic() - start > 60:
            process.kill()
            _, status, _ = os.wait4(process.pid, 0)
            process.returncode = os.waitstatus_to_exitcode(status)
            raise subprocess.TimeoutExpired(command, 60)
        time.sleep(.02)
    if process.returncode:
        raise subprocess.CalledProcessError(process.returncode, command)
    print(f"Offscreen QtTest process ONLY (software/mock, not compositor overhead): elapsed_s={time.monotonic()-start:.2f} "
          f"user_s={usage.ru_utime:.2f} system_s={usage.ru_stime:.2f} peak_rss_kib={usage.ru_maxrss}")
