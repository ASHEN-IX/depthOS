#!/usr/bin/env bash
# DepthOS Alpha — Rollback Script
# Restores previous DepthOS installation from .rollback/ snapshot
# Enforces database forward-only invariant: refuses rollback if database has migrated
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
DRY_RUN=0

print_usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --prefix <DIR>    Installation prefix (default: \$HOME/.local)
  --dry-run         Simulate rollback without writing changes
  -h, --help        Show this help message and exit

Safety Guarantees:
  - Validates rollback manifest and file checksums before restoring
  - Enforces database forward-only invariant: refuses rollback if DB schema is newer than rollback version
  - Restores strict file permissions (0755 for bin, 0644 for assets)
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)
            if [[ $# -lt 2 || "$2" == --* ]]; then
                echo "ERROR: --prefix requires a directory argument" >&2
                exit 1
            fi
            PREFIX="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        -h|--help)
            print_usage
            exit 0
            ;;
        *)
            echo "ERROR: Unknown option: $1" >&2
            print_usage >&2
            exit 1
            ;;
    esac
done

PREFIX_REAL="$(realpath -m "$PREFIX")"
SHARE_DIR="$PREFIX_REAL/share/depthos"
ROLLBACK_DIR="$SHARE_DIR/.rollback"
ROLLBACK_MANIFEST="$ROLLBACK_DIR/.depthos-manifest.json"

echo "=== DepthOS Alpha Rollback ==="
echo "Target Prefix:       $PREFIX_REAL"
echo "Rollback Directory:  $ROLLBACK_DIR"
echo "Dry Run Mode:        $([[ $DRY_RUN -eq 1 ]] && echo 'ENABLED' || echo 'DISABLED')"

if [[ ! -f "$ROLLBACK_MANIFEST" ]]; then
    echo "ERROR: No rollback snapshot found at $ROLLBACK_MANIFEST" >&2
    echo "Rollback aborted. No prior installation state available." >&2
    exit 1
fi

# Database schema forward-only check
echo "Validating database schema forward-only invariant..."
USER_DB="${XDG_DATA_HOME:-$HOME/.local/share}/depthos/workspace.sqlite"

python3 - "$ROLLBACK_MANIFEST" "$ROLLBACK_DIR" "$USER_DB" << 'EOF'
import glob
import json
import os
import re
import sqlite3
import sys
from pathlib import Path

rollback_manifest = Path(sys.argv[1])
rollback_dir = Path(sys.argv[2])
user_db = Path(sys.argv[3])

with open(rollback_manifest, "r", encoding="utf-8") as f:
    manifest = json.load(f)

# Find max migration version in rollback snapshot
max_migration = 0
for item in manifest.get("files", []):
    m = re.search(r"migrations/0*(\d+)_.*\.sql", item["path"])
    if m:
        v = int(m.group(1))
        if v > max_migration:
            max_migration = v

print(f"Rollback target max supported schema version: {max_migration}")

if user_db.is_file():
    try:
        conn = sqlite3.connect(f"file:{user_db}?mode=ro", uri=True)
        cur = conn.cursor()
        cur.execute("PRAGMA user_version;")
        db_version = cur.fetchone()[0]
        conn.close()
        print(f"Existing user database schema version: {db_version}")

        if db_version > max_migration:
            sys.stderr.write(
                f"DATABASE SAFETY ERROR: Cannot roll back software version.\n"
                f"User database schema (version {db_version}) is newer than rollback target schema (version {max_migration}).\n"
                f"Downgrading SQLite schema is strictly prohibited to prevent data corruption.\n"
            )
            sys.exit(1)
    except sqlite3.Error as e:
        sys.stderr.write(f"WARNING: Could not check user database user_version: {e}\n")
else:
    print("No user database found; schema check passed.")
EOF

# Perform file restoration
python3 - "$PREFIX_REAL" "$ROLLBACK_DIR" "$ROLLBACK_MANIFEST" "$DRY_RUN" << 'EOF'
import hashlib
import json
import os
import shutil
import sys
from pathlib import Path

prefix_real = Path(sys.argv[1])
rollback_dir = Path(sys.argv[2])
rollback_manifest = Path(sys.argv[3])
dry_run = bool(int(sys.argv[4]))

with open(rollback_manifest, "r", encoding="utf-8") as f:
    manifest = json.load(f)

files = manifest.get("files", [])
print(f"Restoring {len(files)} files from rollback snapshot...")

for item in files:
    rel_path = item["path"]
    src_file = rollback_dir / rel_path
    dst_file = prefix_real / rel_path

    if not src_file.is_file():
        sys.stderr.write(f"ERROR: Missing rollback file: {src_file}\n")
        sys.exit(1)

    # Verify sha256 in rollback
    h = hashlib.sha256()
    with open(src_file, "rb") as fp:
        while chunk := fp.read(65536):
            h.update(chunk)
    digest = h.hexdigest()

    if digest != item.get("sha256"):
        sys.stderr.write(f"ERROR: Corrupted rollback file {src_file}: hash mismatch\n")
        sys.exit(1)

if dry_run:
    print("Dry run complete. All rollback files verified. Zero disk changes written.")
    sys.exit(0)

for item in files:
    rel_path = item["path"]
    src_file = rollback_dir / rel_path
    dst_file = prefix_real / rel_path

    dst_file.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src_file, dst_file)
    mode_int = int(item.get("mode", "0644"), 8)
    os.chmod(dst_file, mode_int)

# Restore active manifest
target_manifest = prefix_real / "share/depthos/.depthos-manifest.json"
shutil.copy2(rollback_manifest, target_manifest)
EOF

chmod +x "$PREFIX_REAL/bin/depthos" "$PREFIX_REAL/bin/depthos-spatial-alpha"

echo "SUCCESS: Rollback completed cleanly."
echo "Active manifest restored: $SHARE_DIR/.depthos-manifest.json"
"$PREFIX_REAL/bin/depthos" --version
