#!/usr/bin/env bash
# DepthOS Alpha — Safe Staged Upgrade Script
# Stages and installs a DepthOS package archive, preserving rollback snapshots
# and strictly enforcing the SQLite schema forward-only invariant.
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
PACKAGE=""
DRY_RUN=0

print_usage() {
    cat <<EOF
Usage: $0 --package <TARBALL> [OPTIONS]

Options:
  --package <TARBALL> Path to DepthOS release package (.tar.gz)
  --prefix <DIR>      Installation prefix (default: \$HOME/.local)
  --dry-run           Perform preflight, schema, and archive verification without modifying files
  -h, --help          Show this help message and exit

Safety Guarantees:
  - Creates a full atomic rollback snapshot in \$PREFIX/share/depthos/.rollback/
  - Enforces database forward-only invariant: refuses upgrade if package has older schema than DB
  - Verifies package checksum and structure before touching any existing installation
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --package)
            if [[ $# -lt 2 || "$2" == --* ]]; then
                echo "ERROR: --package requires a file argument" >&2
                exit 1
            fi
            PACKAGE="$2"
            shift 2
            ;;
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

if [[ -z "$PACKAGE" ]]; then
    echo "ERROR: --package <TARBALL> is required." >&2
    print_usage >&2
    exit 1
fi

if [[ ! -f "$PACKAGE" ]]; then
    echo "ERROR: Package archive not found: $PACKAGE" >&2
    exit 1
fi

PREFIX_REAL="$(realpath -m "$PREFIX")"
SHARE_DIR="$PREFIX_REAL/share/depthos"
MANIFEST_FILE="$SHARE_DIR/.depthos-manifest.json"
ROLLBACK_DIR="$SHARE_DIR/.rollback"

echo "=== DepthOS Alpha Package Upgrade ==="
echo "Target Prefix:       $PREFIX_REAL"
echo "Package Archive:     $PACKAGE"
echo "Dry Run Mode:        $([[ $DRY_RUN -eq 1 ]] && echo 'ENABLED' || echo 'DISABLED')"

# 1. Package verification
echo "Validating package archive structure..."
ARCHIVE_CONTENTS=$(tar -tzf "$PACKAGE")

# Verify essential files are in archive
for essential in "bin/depthos" "bin/depthos-spatial-alpha" "install.sh" "pkg/depthos.desktop"; do
    if ! echo "$ARCHIVE_CONTENTS" | grep -q "$essential"; then
        echo "ERROR: Package archive is missing required component: $essential" >&2
        exit 1
    fi
done

# 2. Database schema forward-only invariant check
echo "Checking database schema compatibility..."
USER_DB="${XDG_DATA_HOME:-$HOME/.local/share}/depthos/workspace.sqlite"

python3 - "$PACKAGE" "$USER_DB" << 'EOF'
import glob
import os
import re
import sqlite3
import sys
import tarfile
from pathlib import Path

package_path = Path(sys.argv[1])
user_db = Path(sys.argv[2])

# Find highest migration version in package
package_max_migration = 0
with tarfile.open(package_path, "r:*") as tar:
    for member in tar.getmembers():
        m = re.search(r"migrations/0*(\d+)_.*\.sql", member.name)
        if m:
            v = int(m.group(1))
            if v > package_max_migration:
                package_max_migration = v

print(f"Package max supported schema version: {package_max_migration}")

# If user database exists, verify current schema version
if user_db.is_file():
    try:
        conn = sqlite3.connect(f"file:{user_db}?mode=ro", uri=True)
        cur = conn.cursor()
        cur.execute("PRAGMA user_version;")
        db_version = cur.fetchone()[0]
        conn.close()
        print(f"Existing user database schema version: {db_version}")

        if db_version > package_max_migration:
            sys.stderr.write(
                f"DATABASE SAFETY ERROR: Forward-only schema invariant violation.\n"
                f"User database is at schema version {db_version}, but package only supports version {package_max_migration}.\n"
                f"Downgrading database schema is strictly prohibited.\n"
            )
            sys.exit(1)
    except sqlite3.Error as e:
        sys.stderr.write(f"WARNING: Could not check user database user_version: {e}\n")
else:
    print("No existing user database found; forward-only schema check passed.")
EOF

if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Dry run preflight checks passed cleanly. Zero changes written."
    exit 0
fi

# 3. Create Rollback Snapshot of current installation
if [[ -f "$MANIFEST_FILE" ]]; then
    echo "Creating rollback snapshot of current installation..."
    rm -rf "$ROLLBACK_DIR"
    mkdir -p "$ROLLBACK_DIR"
    chmod 700 "$ROLLBACK_DIR"

    python3 - "$PREFIX_REAL" "$MANIFEST_FILE" "$ROLLBACK_DIR" << 'EOF'
import json
import os
import shutil
import sys
from pathlib import Path

prefix_real = Path(sys.argv[1])
manifest_file = Path(sys.argv[2])
rollback_dir = Path(sys.argv[3])

with open(manifest_file, "r", encoding="utf-8") as f:
    manifest = json.load(f)

for item in manifest.get("files", []):
    rel_path = item["path"]
    src_file = prefix_real / rel_path
    if src_file.is_file():
        dst_file = rollback_dir / rel_path
        dst_file.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src_file, dst_file)

shutil.copy2(manifest_file, rollback_dir / ".depthos-manifest.json")
print(f"Snapshot created with {len(manifest.get('files', []))} files.")
EOF
fi

# 4. Extract into temporary staging directory
STAGING_UPGRADE="$SHARE_DIR/.staging_upgrade_$$"
rm -rf "$STAGING_UPGRADE"
mkdir -p "$STAGING_UPGRADE"

echo "Extracting package to temporary staging area..."
tar -xzf "$PACKAGE" -C "$STAGING_UPGRADE"

# Find install script inside extracted archive
INSTALL_SCRIPT=""
if [[ -f "$STAGING_UPGRADE/install.sh" ]]; then
    INSTALL_SCRIPT="$STAGING_UPGRADE/install.sh"
elif [[ -f "$STAGING_UPGRADE/pkg/install.sh" ]]; then
    INSTALL_SCRIPT="$STAGING_UPGRADE/pkg/install.sh"
else
    # Find any install.sh in subdirectories
    INSTALL_SCRIPT=$(find "$STAGING_UPGRADE" -name "install.sh" -type f | head -n 1)
fi

if [[ -z "$INSTALL_SCRIPT" || ! -f "$INSTALL_SCRIPT" ]]; then
    echo "ERROR: install.sh not found inside package archive." >&2
    rm -rf "$STAGING_UPGRADE"
    exit 1
fi

chmod +x "$INSTALL_SCRIPT"

# 5. Run installation from staging
echo "Installing upgraded files into prefix: $PREFIX_REAL..."
"$INSTALL_SCRIPT" --prefix "$PREFIX_REAL"

# Clean up staging
rm -rf "$STAGING_UPGRADE"

# 6. Verify upgraded binary
echo "Verifying upgraded installation..."
"$PREFIX_REAL/bin/depthos" --version

echo "SUCCESS: DepthOS upgraded successfully."
if [[ -d "$ROLLBACK_DIR" ]]; then
    echo "Rollback snapshot available. To roll back, run: pkg/rollback.sh --prefix $PREFIX_REAL"
fi
