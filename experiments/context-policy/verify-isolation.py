"""Verify protected files against the frozen source inventory."""
import hashlib
import json
from pathlib import Path


root = Path(__file__).resolve().parents[2]
manifest = json.loads(Path(__file__).with_name("source-manifest.json").read_text())
source = Path(manifest["source"])
baseline = manifest["files"]
allowed = {
    "docs/phase-2-context-design.md",
    "experiments/context-policy/policy.py",
    "experiments/context-policy/test_policy.py",
    "experiments/context-policy/verify-isolation.py",
    "experiments/context-policy/source-manifest.json",
}


def changes(base):
    return [
        name for name, digest in baseline.items()
        if (path := base / name).is_symlink() or not path.is_file()
        or hashlib.sha256(path.read_bytes()).hexdigest() != digest
    ]


def main():
    files = {p.relative_to(root).as_posix() for p in root.rglob("*") if p.is_file() or p.is_symlink()}
    report = {
        "baseline_files": len(baseline),
        "canonical_baseline_changes": changes(source),
        "protected_copy_changes": changes(root),
        "unexpected_paths": sorted(files - baseline.keys() - allowed),
        "missing_deliverables": sorted(allowed - files),
    }
    print(json.dumps(report, indent=2))
    return int(any(report[key] for key in ("protected_copy_changes", "unexpected_paths",
                                         "missing_deliverables")))


if __name__ == "__main__":
    raise SystemExit(main())
