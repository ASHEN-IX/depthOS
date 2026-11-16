#!/usr/bin/env python3
"""Prepare immutable QML revisions and explicitly manage only DepthOS overview dev packages.

prepare/status are safe local/read-only operations. install/load/toggle/unload/remove
require the user's separate approval, as documented in docs/poc-02b.md.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "experiments/kwin-overview"
ARTIFACTS = ROOT / "artifacts/overview-dev"
PREFIX = "depthos-overview-dev-"
ACTION = "depthos-spatial-overview-toggle"
REQUIRED = ("installation", "initialSnapshot", "existingWindows", "windowAdded", "windowRemoved",
            "focusChanged", "desktopChanged", "membershipChanged", "reconciliation", "cleanUnload", "unrelatedStatePreserved")


def dbus(*args):
    return subprocess.check_output(["qdbus6", *args], text=True, timeout=10).strip()


def source_files():
    files = {}
    for file in sorted(SOURCE.rglob("*")):
        if file.is_symlink():
            raise ValueError("Symlinks are not accepted in the overview source")
        if file.is_file():
            files[str(file.relative_to(SOURCE))] = file.read_bytes()
    return files


def revision(files):
    digest = hashlib.sha256()
    for name, data in sorted(files.items()):
        digest.update(name.encode() + b"\0" + data + b"\0")
    return digest.hexdigest()


def package_data():
    files = source_files()
    digest = revision(files)
    identifier = PREFIX + digest[:12]
    metadata = json.loads(files["metadata.json"])
    metadata["KPlugin"]["Id"] = identifier
    metadata["KPlugin"]["Name"] = "DepthOS Native Overview (dev " + digest[:12] + ")"
    metadata["X-DepthOS-SourceHash"] = digest
    files["metadata.json"] = (json.dumps(metadata, indent=4) + "\n").encode()
    return identifier, files


def same_package(path, files):
    if not path.is_dir() or path.is_symlink():
        return False
    present = {str(p.relative_to(path)): p.read_bytes() for p in path.rglob("*") if p.is_file() and not p.is_symlink()}
    return present == files and not any(p.is_symlink() for p in path.rglob("*"))


def prepare():
    identifier, files = package_data()
    path = ARTIFACTS / identifier
    if path.exists() and not same_package(path, files):
        raise ValueError("Prepared revision was modified; refusing overwrite")
    for name, data in files.items():
        target = path / name
        if not target.exists():
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    current = {"id": identifier, "path": str(path), "sourceHash": json.loads(files["metadata.json"])["X-DepthOS-SourceHash"]}
    (ARTIFACTS / "current.json").write_text(json.dumps(current, indent=2) + "\n")
    return current


def package_root():
    data = Path(os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share")
    if not data.is_absolute():
        raise ValueError("XDG_DATA_HOME must be absolute")
    return data / "kwin/effects"


def own(identifier):
    if not re.fullmatch(r"depthos-overview-dev-[0-9a-f]{12}", identifier):
        raise ValueError("Only content-addressed DepthOS overview IDs are accepted")
    path = package_root() / identifier
    if path.is_symlink():
        raise ValueError("Refusing a symlink package")
    metadata = json.loads((path / "metadata.json").read_text())
    if metadata["KPlugin"]["Id"] != identifier or not metadata.get("X-DepthOS-SourceHash", "").startswith(identifier[len(PREFIX):]):
        raise ValueError("Package ownership marker mismatch")
    return path


def loaded(identifier):
    value = dbus("org.kde.KWin", "/Effects", "org.kde.kwin.Effects.isEffectLoaded", identifier)
    if value not in ("true", "false"):
        raise ValueError("Unknown effect state")
    return value == "true"


def require_acceptance(path):
    data = json.loads(path.read_text())
    if data.get("status") != "PASS" or data.get("kwinVersion") != "6.8.1" or any(data.get("checks", {}).get(k) is not True for k in REQUIRED):
        raise ValueError("Native POC-02A acceptance is incomplete; do not activate POC-02B")
    observer_hash = hashlib.sha256((ROOT / "experiments/kwin-observer/contents/code/main.js").read_bytes()).hexdigest()
    if data.get("observerSourceHash") != observer_hash:
        raise ValueError("Observer source differs from the accepted revision; retest it")
    owner = dbus("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus.GetNameOwner", "org.kde.KWin")
    if data.get("compositorOwner") != owner:
        raise ValueError("Acceptance belongs to a different compositor session; retest the observer")
    bus_id = dbus("org.freedesktop.DBus", "/org/freedesktop/DBus", "org.freedesktop.DBus.GetId")
    if data.get("sessionBusId") != bus_id:
        raise ValueError("Acceptance belongs to a different session bus; retest the observer")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("prepare", "status", "install", "load", "toggle", "unload", "remove"))
    parser.add_argument("--id", help="An explicit previous DepthOS dev revision, for unload/remove")
    parser.add_argument("--acceptance", type=Path, default=ROOT / "artifacts/observer-acceptance.json")
    args = parser.parse_args()
    if args.operation == "prepare":
        current = prepare()
        template = ROOT / "artifacts/observer-acceptance.template.json"
        # Regenerate only the template, never the operator's acceptance record.
        template.write_text(json.dumps({"status": "PENDING", "kwinVersion": "6.8.1", "compositorOwner": "", "sessionBusId": "", "observerSourceHash": "",
            "checks": {key: False for key in REQUIRED}}, indent=2) + "\n")
        print(json.dumps(current, indent=2))
        return
    if args.operation == "status":
        for path in sorted(package_root().glob(PREFIX + "*")):
            own(path.name)
            print(path.name, "loaded=" + str(loaded(path.name)))
        return
    if os.getuid() == 0:
        raise ValueError("Refusing compositor package operations as root")
    if args.operation in ("install", "load", "toggle"):
        if os.environ.get("XDG_SESSION_TYPE") != "wayland" or subprocess.check_output(["kwin_wayland", "--version"], text=True).strip() != "kwin 6.8.1":
            raise ValueError("Only KWin 6.8.1 Wayland is source-audited")
    if args.id and args.operation not in ("unload", "remove"):
        raise ValueError("--id is only for scoped cleanup of previous revisions")
    if args.operation in ("unload", "remove"):
        # Rollback must still work while the source is edited or malformed.
        identifier = args.id or json.loads((ARTIFACTS / "current.json").read_text())["id"]
        files = None
    else:
        identifier, files = package_data()
    if args.operation == "install":
        path = ARTIFACTS / identifier
        if not same_package(path, files):
            raise ValueError("Run prepare first; this revision must match the reviewed source")
        if (package_root() / identifier).exists() or (package_root() / identifier).is_symlink():
            raise ValueError("Revision already exists; no upgrade or overwrite is allowed")
        subprocess.run(["kpackagetool6", "--type", "KWin/Effect", "--packageroot", str(package_root()), "--install", str(path)], check=True)
        return
    installed = own(identifier)
    if args.operation in ("load", "toggle"):
        require_acceptance(args.acceptance)
        if not same_package(installed, files):
            raise ValueError("Installed files differ from the current reviewed revision")
    if args.operation == "load":
        if loaded("depthos-spatial-overview"):
            raise ValueError("The unversioned overview is loaded; unload only it explicitly first")
        for other in package_root().glob(PREFIX + "*"):
            own(other.name)
            if loaded(other.name):
                raise ValueError("A DepthOS overview revision is already loaded; unload it explicitly first")
        if dbus("org.kde.KWin", "/Effects", "org.kde.kwin.Effects.loadEffect", identifier) != "true":
            raise ValueError("KWin rejected loading this revision")
        print("Loaded", identifier, "— activation and rendering are separate acceptance checks")
    elif args.operation == "toggle":
        if not loaded(identifier):
            raise ValueError("Load this exact revision first")
        dbus("org.kde.kglobalaccel", "/component/kwin", "org.kde.kglobalaccel.Component.invokeShortcut", ACTION)
    elif args.operation == "unload":
        dbus("org.kde.KWin", "/Effects", "org.kde.kwin.Effects.unloadEffect", identifier)
        if loaded(identifier):
            raise ValueError("Effect unload was not confirmed")
    elif args.operation == "remove":
        if loaded(identifier):
            raise ValueError("Unload only this revision before removal")
        subprocess.run(["kpackagetool6", "--type", "KWin/Effect", "--packageroot", str(package_root()), "--remove", identifier], check=True)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.SubprocessError, json.JSONDecodeError) as error:
        raise SystemExit(str(error)) from None
