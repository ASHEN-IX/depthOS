#!/usr/bin/env bash
# DepthOS Alpha — Online SQLite Database Backup Script
# Uses the SQLite Online Backup API to produce WAL-consistent, verified snapshots.
set -euo pipefail

DEFAULT_DB="${XDG_DATA_HOME:-$HOME/.local/share}/depthos/workspace.sqlite"
DB_PATH=""
OUTPUT_PATH=""
DRY_RUN=0

print_usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --db <DB_PATH>        Source SQLite database path (default: \$XDG_DATA_HOME/depthos/workspace.sqlite)
  --output <FILE>       Destination backup file path (required)
  --dry-run             Verify database integrity without creating backup file
  -h, --help            Show this help message and exit

Safety & Consistency Guarantees:
  - Uses SQLite online backup API to ensure 100% WAL consistency and concurrent read safety
  - Runs PRAGMA integrity_check and PRAGMA foreign_key_check on backup
  - Sets strict 0600 permissions on backup file
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --db)
            if [[ $# -lt 2 || "$2" == --* ]]; then
                echo "ERROR: --db requires a file argument" >&2
                exit 1
            fi
            DB_PATH="$2"
            shift 2
            ;;
        --output)
            if [[ $# -lt 2 || "$2" == --* ]]; then
                echo "ERROR: --output requires a file argument" >&2
                exit 1
            fi
            OUTPUT_PATH="$2"
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

if [[ -z "$DB_PATH" ]]; then
    DB_PATH="$DEFAULT_DB"
fi

if [[ -z "$OUTPUT_PATH" && "$DRY_RUN" -eq 0 ]]; then
    echo "ERROR: --output <BACKUP_PATH> is required unless --dry-run is specified." >&2
    print_usage >&2
    exit 1
fi

if [[ ! -f "$DB_PATH" ]]; then
    echo "ERROR: Source database not found: $DB_PATH" >&2
    exit 1
fi

python3 - "$DB_PATH" "${OUTPUT_PATH:-}" "$DRY_RUN" << 'EOF'
import hashlib
import json
import os
import sqlite3
import sys
from datetime import datetime, timezone
from pathlib import Path

db_path = Path(sys.argv[1]).resolve()
output_arg = sys.argv[2]
dry_run = bool(int(sys.argv[3]))

if not db_path.is_file():
    sys.stderr.write(f"ERROR: Database file does not exist: {db_path}\n")
    sys.exit(1)

# Check read-only probe on source
try:
    src_conn = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    cur = src_conn.cursor()
    cur.execute("PRAGMA integrity_check;")
    src_check = cur.fetchone()[0]
    cur.execute("PRAGMA user_version;")
    schema_version = cur.fetchone()[0]
    src_conn.close()
    if src_check != "ok":
        sys.stderr.write(f"ERROR: Source database integrity check failed: {src_check}\n")
        sys.exit(1)
except Exception as e:
    sys.stderr.write(f"ERROR: Failed to connect to source database: {e}\n")
    sys.exit(1)

if dry_run:
    report = {
        "status": "PASS",
        "dry_run": True,
        "source_db": str(db_path),
        "schema_version": schema_version,
        "integrity_check": src_check,
        "timestamp": datetime.now(timezone.utc).isoformat(timespec="seconds")
    }
    print(json.dumps(report, indent=2))
    sys.exit(0)

output_path = Path(output_arg).resolve()
output_path.parent.mkdir(parents=True, exist_ok=True)
# Ensure parent directory is 0700
try:
    os.chmod(output_path.parent, 0o700)
except OSError:
    pass

# Perform Online Backup using SQLite Backup API
tmp_output = output_path.with_name(f"{output_path.name}.tmp_{os.getpid()}")
if tmp_output.exists():
    tmp_output.unlink()

try:
    src = sqlite3.connect(f"file:{db_path}?mode=ro", uri=True)
    dst = sqlite3.connect(str(tmp_output))
    src.backup(dst)
    dst.close()
    src.close()
except Exception as e:
    if tmp_output.exists():
        tmp_output.unlink()
    sys.stderr.write(f"ERROR: SQLite online backup failed: {e}\n")
    sys.exit(1)

# Verify backup integrity
try:
    check_conn = sqlite3.connect(f"file:{tmp_output}?mode=ro", uri=True)
    chk_cur = check_conn.cursor()
    chk_cur.execute("PRAGMA integrity_check;")
    integrity_result = chk_cur.fetchone()[0]
    chk_cur.execute("PRAGMA foreign_key_check;")
    fk_violations = chk_cur.fetchall()
    chk_cur.execute("PRAGMA user_version;")
    backup_schema_version = chk_cur.fetchone()[0]
    check_conn.close()

    if integrity_result != "ok":
        tmp_output.unlink()
        sys.stderr.write(f"ERROR: Backup file failed integrity check: {integrity_result}\n")
        sys.exit(1)

    if fk_violations:
        tmp_output.unlink()
        sys.stderr.write(f"ERROR: Backup file failed foreign key check: {fk_violations}\n")
        sys.exit(1)
except Exception as e:
    if tmp_output.exists():
        tmp_output.unlink()
    sys.stderr.write(f"ERROR: Failed validating backup file: {e}\n")
    sys.exit(1)

# Set strict permissions 0600
os.chmod(tmp_output, 0o600)
tmp_output.rename(output_path)

# Compute sha256 and size
h = hashlib.sha256()
with open(output_path, "rb") as fp:
    while chunk := fp.read(65536):
        h.update(chunk)
digest = h.hexdigest()
size_bytes = output_path.stat().st_size

report = {
    "status": "SUCCESS",
    "timestamp": datetime.now(timezone.utc).isoformat(timespec="seconds"),
    "source_db": str(db_path),
    "backup_path": str(output_path),
    "schema_version": backup_schema_version,
    "integrity": integrity_result,
    "foreign_keys": "ok",
    "mode": oct(output_path.stat().st_mode & 0o777),
    "sha256": digest,
    "size_bytes": size_bytes
}

print(json.dumps(report, indent=2))
EOF
