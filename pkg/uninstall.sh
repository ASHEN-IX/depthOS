#!/usr/bin/env bash
# DepthOS Alpha — Manifest-Guided Uninstallation Script
# Removes DepthOS installation files tracked in .depthos-manifest.json
# Strictly preserves user data, databases, preferences, and logs.
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
DRY_RUN=0
FORCE=0

print_usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --prefix <DIR>    Installation prefix (default: \$HOME/.local)
  --dry-run         Simulate uninstallation without modifying filesystem
  --force           Remove installed files even if modified after installation
  -h, --help        Show this help message and exit

Preservation Guarantees:
  - User databases, logs, and preferences in \$XDG_DATA_HOME/depthos/ are never modified or removed.
  - Files modified by the user since installation are preserved by default unless --force is given.
  - Parent system and shared directories (\$PREFIX/bin, \$PREFIX/share, etc.) are never deleted.
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

PREFIX_REAL="$(realpath -m "$PREFIX")"
SHARE_DIR="$PREFIX_REAL/share/depthos"
MANIFEST_FILE="$SHARE_DIR/.depthos-manifest.json"

echo "=== DepthOS Alpha Uninstallation ==="
echo "Target Prefix:       $PREFIX_REAL"
echo "Dry Run Mode:        $([[ $DRY_RUN -eq 1 ]] && echo 'ENABLED' || echo 'DISABLED')"
echo "Force Mode:          $([[ $FORCE -eq 1 ]] && echo 'ENABLED' || echo 'DISABLED')"

if [[ ! -f "$MANIFEST_FILE" ]]; then
    echo "ERROR: No DepthOS installation manifest found at $MANIFEST_FILE" >&2
    echo "Uninstall aborted. No files were removed." >&2
    exit 1
fi

# Run python uninstallation plan generator and execution
python3 - "$PREFIX_REAL" "$MANIFEST_FILE" "$DRY_RUN" "$FORCE" << 'EOF'
import hashlib
import json
import os
import sys
from pathlib import Path

prefix_real = Path(sys.argv[1])
manifest_file = Path(sys.argv[2])
dry_run = bool(int(sys.argv[3]))
force = bool(int(sys.argv[4]))
current_uid = os.getuid()

try:
    with open(manifest_file, "r", encoding="utf-8") as fp:
        manifest = json.load(fp)
except Exception as e:
    sys.stderr.write(f"ERROR: Failed to parse manifest {manifest_file}: {e}\n")
    sys.exit(1)

files = manifest.get("files", [])
to_delete = []
modified_preserved = []
missing = []
permission_denied = []

for entry in files:
    rel_path = entry["path"]
    expected_hash = entry.get("sha256")
    abs_path = prefix_real / rel_path

    if not abs_path.exists() and not abs_path.is_symlink():
        missing.append(abs_path)
        continue

    # Ownership check
    try:
        st = abs_path.stat()
        if current_uid != 0 and st.st_uid != current_uid:
            permission_denied.append((abs_path, f"Owned by UID {st.st_uid}, current UID {current_uid}"))
            continue
    except OSError as e:
        permission_denied.append((abs_path, str(e)))
        continue

    # Integrity verification
    try:
        h = hashlib.sha256()
        with open(abs_path, "rb") as fp:
            while chunk := fp.read(65536):
                h.update(chunk)
        actual_hash = h.hexdigest()
    except OSError as e:
        permission_denied.append((abs_path, str(e)))
        continue

    if actual_hash == expected_hash or force:
        to_delete.append(abs_path)
    else:
        modified_preserved.append((abs_path, expected_hash, actual_hash))

if permission_denied:
    sys.stderr.write("SECURITY ERROR: Permission checks failed for the following files:\n")
    for p, reason in permission_denied:
        sys.stderr.write(f"  - {p}: {reason}\n")
    sys.stderr.write("Uninstallation aborted without changes.\n")
    sys.exit(1)

print(f"Discovered {len(files)} files in manifest:")
print(f"  - Verified for removal: {len(to_delete)}")
print(f"  - Modified by user (preserved): {len(modified_preserved)}")
print(f"  - Already absent: {len(missing)}")

if modified_preserved:
    print("\nWARNING: The following files have been modified since installation and will NOT be deleted:")
    for p, exp, act in modified_preserved:
        print(f"  - {p} (use --force to remove)")

if dry_run:
    print("\nDry run simulation complete. Zero files were removed.")
    sys.exit(0)

# Perform deletion of verified files
print("\nRemoving verified files...")
for p in to_delete:
    try:
        p.unlink()
    except OSError as e:
        sys.stderr.write(f"WARNING: Failed to remove {p}: {e}\n")

# If all tracked files were removed, remove manifest
if not modified_preserved:
    try:
        manifest_file.unlink()
        print("Removed installation manifest.")
    except OSError as e:
        sys.stderr.write(f"WARNING: Could not remove manifest: {e}\n")
else:
    print("Installation manifest retained because modified files were preserved.")

# Clean up empty DepthOS application directories
# Strictly preserve broad directories like prefix/bin, prefix/share, applications, icons
app_dirs = [
    prefix_real / "share/depthos/spatial-alpha/theme",
    prefix_real / "share/depthos/spatial-alpha/art",
    prefix_real / "share/depthos/spatial-alpha",
    prefix_real / "share/depthos/migrations",
    prefix_real / "share/depthos/.rollback",
    prefix_real / "share/depthos"
]

for d in app_dirs:
    if d.is_dir():
        try:
            d.rmdir()  # rmdir only removes if directory is completely empty
            print(f"Removed empty application directory: {d}")
        except OSError:
            pass  # Directory not empty or not removable; leave intact

print("\nSUCCESS: DepthOS uninstallation completed.")
print("USER DATA PRESERVED: User workspaces, database records, and logs in")
print("  $XDG_DATA_HOME/depthos/ (or ~/.local/share/depthos/) remain untouched.")
EOF
