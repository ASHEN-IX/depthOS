#!/usr/bin/env python3
"""Compile the native observer against the exact pinned installed KWin ABI."""
import hashlib
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
PATCH = ROOT / "experiments/kwin-authority/phase-3-7/kwin-6.8.3-depthos.patch"
ARTIFACTS = ROOT / "artifacts/phase-3-7"
INSTALL = {
    Path("/usr/bin/kwin_wayland"): "33350d420b7b196c1e1f90ffa8c1280b4b3102d4e4b60e1929bad525be93df5e",
    Path("/usr/lib/libkwin.so.6"): "25259a191bc4c0e2e0eae74a5b4ec34b68890564b157884b7c00fc5bea9155d1",
}
SOURCE_DIGEST = "fd5cb783f220ea4526a9afee2f72b582c3e87ef7f91b19bc5c7a6662ffe7fbdc"


def require_hash(path, expected):
    with path.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    if digest != expected:
        raise RuntimeError(f"Unqualified build input: {path}")


def run(argv, **options):
    return subprocess.run(argv, check=True, **options)


def main(archive):
    for path, digest in INSTALL.items():
        require_hash(path, digest)
    require_hash(archive, SOURCE_DIGEST)
    os.umask(0o077)
    ARTIFACTS.mkdir(parents=True, exist_ok=True)
    out = Path(tempfile.mkdtemp(prefix="private-plugin-build-", dir=ARTIFACTS))
    run(["/usr/bin/tar", "-xf", str(archive), "-C", str(out)])
    source = out / "kwin-6.8.3"
    run(["/usr/bin/git", "init", "-q"], cwd=source)
    run(["/usr/bin/git", "apply", "--check", str(PATCH)], cwd=source)
    run(["/usr/bin/git", "apply", str(PATCH)], cwd=source)
    src = out / "private-plugin.cpp"
    shutil.copyfile(HERE / src.name, src)
    shutil.copyfile(HERE / "private-plugin.json", out / "private-plugin.json")
    qt = shlex.split(subprocess.check_output(
        ["/usr/bin/pkg-config", "--cflags", "--libs",
         "Qt6Core", "Qt6Gui", "Qt6DBus", "Qt6Quick", "Qt6Widgets"], text=True))
    includes = [source / "src", Path("/usr/include/kwin"), Path("/usr/include/PlasmaActivities"),
                Path("/usr/include/KDecoration3"), *sorted(Path("/usr/include/KF6").glob("K*"))]
    flags = ["-DKWIN_DEPTHOS_AUTHORITY_EXPERIMENT", *(f"-I{p}" for p in includes)]
    moc = out / "private-plugin.moc"
    run(["/usr/lib/qt6/moc", *flags, str(src), "-o", str(moc)])
    plugin = out / "depthos-phase37-private.so"
    run(["/usr/bin/c++", "-std=c++23", "-shared", "-fPIC", "-Wl,-z,defs", *flags,
         str(src), "-o", str(plugin), "-lkwin", *qt])
    print(f"{hashlib.sha256(plugin.read_bytes()).hexdigest()}  {plugin}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        raise SystemExit("Usage: build-private-plugin.py KWIN_RELEASE_ARCHIVE")
    main(Path(sys.argv[1]).resolve(strict=True))
