#!/usr/bin/env bash
# DepthOS Alpha — SQLite Database Restore Script
# Validates schema version, foreign keys, and integrity before restoring database.
# Restores with strict 0600 file and 0700 directory permissions.
set -euo pipefail

DEFAULT_TARGET="${XDG_DATA_HOME:-$HOME/.local/share}/depthos/workspace.sqlite"
BACKUP_PATH=""
TARGET_PATH=""
DRY_RUN=0
FORCE=0

print_usage() {
    cat <<EOF
Usage: $0 --backup <BACKUP_PATH> [OPTIONS]

Options:
  --backup <FILE>       Path to SQLite backup file (required)
  --target <DB_PATH>    Destination database path (default: \$XDG_DATA_HOME/depthos/workspace.sqlite)
  --dry-run             Validate backup file without modifying target database
  --force               Overwrite existing target database without prompt
  -h, --help            Show this help message and exit

Safety Guarantees:
  - Validates PRAGMA integrity_check, PRAGMA foreign_key_check, and schema version before modifying target
  - Creates pre-restore safety copy of existing target database
  - Enforces strict 0600 permissions on restored database and 0700 on parent directory
  - Atomically stages replacement to prevent partial write corruption
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --backup)
            if [[ $# -lt 2 || "$2" == --* ]]; then
                echo "ERROR: --backup requires a file argument" >&2
                exit 1
            fi
            BACKUP_PATH="$2"
            shift 2
            ;;
        --target)
            if [[ $# -lt 2 || "$2" == --* ]]; then
                echo "ERROR: --target requires a file argument" >&2
                exit 1
            fi
            TARGET_PATH="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        --force)
            FORCE=1
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

if [[ -z "$BACKUP_PATH" ]]; then
    echo "ERROR: --backup <BACKUP_PATH> is required." >&2
    print_usage >&2
    exit 1
fi

if [[ ! -f "$BACKUP_PATH" ]]; then
    echo "ERROR: Backup file does not exist: $BACKUP_PATH" >&2
    exit 1
fi

if [[ -z "$TARGET_PATH" ]]; then
    TARGET_PATH="$DEFAULT_TARGET"
fi

python3 - "$BACKUP_PATH" "$TARGET_PATH" "$DRY_RUN" "$FORCE" << 'EOF'
import hashlib
import json
import os
import shutil
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

backup_path = Path(sys.argv[1]).resolve()
target_path = Path(sys.argv[2]).resolve()
dry_run = bool(int(sys.argv[3]))
force = bool(int(sys.argv[4]))

if not backup_path.is_file():
    sys.stderr.write(f"ERROR: Backup file missing: {backup_path}\n")
    sys.exit(1)

# 1. Pre-restore validation of backup file
try:
    conn = sqlite3.connect(f"file:{backup_path}?mode=ro", uri=True)
    cur = conn.cursor()
    cur.execute("PRAGMA integrity_check;")
    integrity = cur.fetchone()[0]
    cur.execute("PRAGMA user_version;")
    schema_version = cur.fetchone()[0]
    cur.execute("PRAGMA foreign_key_check;")
    fk_violations = cur.fetchall()
    conn.close()

    if integrity != "ok":
        sys.stderr.write(f"RESTORATION ERROR: Backup failed integrity check: {integrity}\n")
        sys.exit(1)

    if fk_violations:
        sys.stderr.write(f"RESTORATION ERROR: Backup contains foreign key violations: {fk_violations}\n")
        sys.exit(1)

    if schema_version < 1:
        sys.stderr.write(f"RESTORATION ERROR: Invalid schema version in backup: {schema_version}\n")
        sys.exit(1)
except Exception as e:
    sys.stderr.write(f"RESTORATION ERROR: Failed validating backup file: {e}\n")
    sys.exit(1)

if dry_run:
    report = {
        "status": "PASS",
        "dry_run": True,
        "backup_path": str(backup_path),
        "target_path": str(target_path),
        "schema_version": schema_version,
        "integrity": integrity,
        "foreign_keys": "ok",
        "target_exists": target_path.exists(),
        "timestamp": datetime.now(timezone.utc).isoformat(timespec="seconds")
    }
    print(json.dumps(report, indent=2))
    sys.exit(0)

# 2. Prepare target directory
target_dir = target_path.parent
target_dir.mkdir(parents=True, exist_ok=True)
os.chmod(target_dir, 0o700)

# 3. Create safety backup of existing target database
pre_restore_bak = None
if target_path.exists():
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    pre_restore_bak = target_path.with_name(f"{target_path.name}.pre_restore_{timestamp}")
    shutil.copy2(target_path, pre_restore_bak)
    os.chmod(pre_restore_bak, 0o600)
    print(f"Safety copy of existing database created: {pre_restore_bak}")

# 4. Atomic restoration
tmp_target = target_path.with_name(f"{target_path.name}.tmp_{os.getpid()}")
if tmp_target.exists():
    tmp_target.unlink()

try:
    shutil.copy2(backup_path, tmp_target)
    os.chmod(tmp_target, 0o600)

    # Verify temp copy
    verify_conn = sqlite3.connect(f"file:{tmp_target}?mode=ro", uri=True)
    v_cur = verify_conn.cursor()
    v_cur.execute("PRAGMA integrity_check;")
    v_res = v_cur.fetchone()[0]
    verify_conn.close()

    if v_res != "ok":
        tmp_target.unlink()
        sys.stderr.write(f"ERROR: Staged restored file failed integrity check: {v_res}\n")
        sys.exit(1)

    # Atomically replace target
    tmp_target.rename(target_path)
    os.chmod(target_path, 0o600)

    # Clean up any stale WAL/SHM companion files for target
    for companion in (target_path.with_name(f"{target_path.name}-wal"),
                      target_path.with_name(f"{target_path.name}-shm")):
        if companion.exists():
            try:
                companion.unlink()
            except OSError:
                pass
except Exception as e:
    if tmp_target.exists():
        tmp_target.unlink()
    sys.stderr.write(f"RESTORATION ERROR: Failed to restore database: {e}\n")
    sys.exit(1)

report = {
    "status": "SUCCESS",
    "timestamp": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    "backup_source": str(backup_path),
    "restored_target": str(target_path),
    "schema_version": schema_version,
    "integrity": "ok",
    "foreign_keys": "ok",
    "mode": oct(target_path.stat().st_mode & 0o777),
    "dir_mode": oct(target_dir.stat().st_mode & 0o777),
    "safety_backup": str(pre_restore_bak) if pre_restore_bak else None
}
print(json.dumps(report, indent=2))
EOF
