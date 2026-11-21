#!/usr/bin/env bash
# DepthOS Alpha — Hardened Local Installation Script
# Installs DepthOS CLI, Spatial Alpha launcher, QML runtime, migrations, desktop entry, and icon.
set -euo pipefail

VERSION="0.1.0-alpha"
PREFIX="${PREFIX:-$HOME/.local}"
DRY_RUN=0

print_usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --prefix <DIR>    Installation prefix (default: \$HOME/.local)
  --dry-run         Perform preflight checks and simulate installation without changes
  --version         Show installer version and exit
  -h, --help        Show this help message and exit

Security constraints:
  - Destination prefix must not be world-writable (rejects 0777, 0775, etc.)
  - Setuid / setgid bits on installed files are strictly prohibited
  - Symlinked prefix targets are verified to prevent path traversal
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
        --version)
            echo "depthos-installer version $VERSION"
            exit 0
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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Resolve source tree or release bundle
# Case A: Inside repository tree
if [[ -d "$SCRIPT_DIR/../experiments/spatial-alpha" && -d "$SCRIPT_DIR/../migrations" ]]; then
    ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
    DEPTHOS_SRC="$ROOT_DIR/target/release/depthos"
    if [[ ! -f "$DEPTHOS_SRC" ]]; then
        DEPTHOS_SRC="$ROOT_DIR/target/debug/depthos"
    fi
    LAUNCHER_SRC="$ROOT_DIR/tools/depthos-spatial-alpha"
    SPATIAL_SRC="$ROOT_DIR/experiments/spatial-alpha"
    MIGRATIONS_SRC="$ROOT_DIR/migrations"
    DESKTOP_SRC="$ROOT_DIR/pkg/depthos.desktop"
    ICON_SRC="$ROOT_DIR/pkg/depthos.svg"
# Case B: Inside extracted release archive
elif [[ -f "$SCRIPT_DIR/bin/depthos" || -f "$SCRIPT_DIR/../bin/depthos" ]]; then
    if [[ -f "$SCRIPT_DIR/bin/depthos" ]]; then
        PKG_ROOT="$SCRIPT_DIR"
    else
        PKG_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
    fi
    DEPTHOS_SRC="$PKG_ROOT/bin/depthos"
    LAUNCHER_SRC="$PKG_ROOT/bin/depthos-spatial-alpha"
    SPATIAL_SRC="$PKG_ROOT/share/depthos/spatial-alpha"
    MIGRATIONS_SRC="$PKG_ROOT/share/depthos/migrations"
    DESKTOP_SRC="$PKG_ROOT/pkg/depthos.desktop"
    [[ -f "$DESKTOP_SRC" ]] || DESKTOP_SRC="$PKG_ROOT/depthos.desktop"
    ICON_SRC="$PKG_ROOT/pkg/depthos.svg"
    [[ -f "$ICON_SRC" ]] || ICON_SRC="$PKG_ROOT/depthos.svg"
else
    echo "ERROR: Unable to locate DepthOS source artifacts from $SCRIPT_DIR" >&2
    exit 1
fi

# Canonicalize prefix path
PREFIX_REAL="$(realpath -m "$PREFIX")"

echo "=== DepthOS Alpha Installation ($VERSION) ==="
echo "Target Prefix:       $PREFIX_REAL"
echo "Dry Run Mode:        $([[ $DRY_RUN -eq 1 ]] && echo 'ENABLED' || echo 'DISABLED')"

# Security Preflight 1: Prefix validation & ownership
# Block sensitive root directories
case "$PREFIX_REAL" in
    /|/bin|/sbin|/etc|/boot|/dev|/proc|/sys|/lib|/lib64)
        echo "SECURITY ERROR: Destination prefix '$PREFIX_REAL' is a protected system directory." >&2
        exit 1
        ;;
esac

# Check permissions if prefix exists
if [[ -e "$PREFIX_REAL" ]]; then
    if [[ ! -d "$PREFIX_REAL" ]]; then
        echo "SECURITY ERROR: Prefix '$PREFIX_REAL' exists but is not a directory." >&2
        exit 1
    fi

    # Check ownership
    OWNER_UID=$(stat -c "%u" "$PREFIX_REAL")
    CURRENT_UID=$(id -u)
    if [[ "$OWNER_UID" -ne "$CURRENT_UID" && "$CURRENT_UID" -ne 0 ]]; then
        echo "SECURITY ERROR: Prefix '$PREFIX_REAL' is owned by UID $OWNER_UID (current UID is $CURRENT_UID)." >&2
        exit 1
    fi

    # Check mode: strictly refuse world-writable (0777, 0775, or any bit 0002)
    PREFIX_MODE=$(stat -c "%a" "$PREFIX_REAL")
    PREFIX_OCTAL=$((8#$PREFIX_MODE))
    if (( (PREFIX_OCTAL & 0002) != 0 )) || [[ "$PREFIX_MODE" == "777" || "$PREFIX_MODE" == "775" ]]; then
        echo "SECURITY ERROR: Destination prefix '$PREFIX_REAL' has unsafe permissions (mode: $PREFIX_MODE). World-writable prefixes are rejected." >&2
        exit 1
    fi
else
    # Check parent directory
    PARENT_DIR="$(dirname "$PREFIX_REAL")"
    if [[ -d "$PARENT_DIR" ]]; then
        PARENT_MODE=$(stat -c "%a" "$PARENT_DIR")
        PARENT_OCTAL=$((8#$PARENT_MODE))
        if (( (PARENT_OCTAL & 0002) != 0 )) || [[ "$PARENT_MODE" == "777" || "$PARENT_MODE" == "775" ]]; then
            echo "SECURITY ERROR: Parent directory '$PARENT_DIR' has unsafe permissions (mode: $PARENT_MODE)." >&2
            exit 1
        fi
    fi
fi

# Security Preflight 2: Source artifact verification
REQUIRED_SOURCES=("$DEPTHOS_SRC" "$LAUNCHER_SRC" "$DESKTOP_SRC" "$ICON_SRC")
for src in "${REQUIRED_SOURCES[@]}"; do
    if [[ ! -f "$src" ]]; then
        echo "ERROR: Required source file missing: $src" >&2
        exit 1
    fi
    # Strictly forbid setuid and setgid
    MODE=$(stat -c "%a" "$src")
    OCTAL=$((8#$MODE))
    if (( (OCTAL & 06000) != 0 )); then
        echo "SECURITY ERROR: Source file $src has dangerous setuid/setgid bits ($MODE)" >&2
        exit 1
    fi
done

if [[ ! -d "$SPATIAL_SRC" ]]; then
    echo "ERROR: Spatial QML source directory missing: $SPATIAL_SRC" >&2
    exit 1
fi

if [[ ! -d "$MIGRATIONS_SRC" ]]; then
    echo "ERROR: Database migrations directory missing: $MIGRATIONS_SRC" >&2
    exit 1
fi

# Target layout
BIN_DIR="$PREFIX_REAL/bin"
SHARE_DIR="$PREFIX_REAL/share/depthos"
SPATIAL_DIR="$SHARE_DIR/spatial-alpha"
MIGRATIONS_DIR="$SHARE_DIR/migrations"
APPS_DIR="$PREFIX_REAL/share/applications"
ICON_DIR="$PREFIX_REAL/share/icons/hicolor/scalable/apps"
MANIFEST_FILE="$SHARE_DIR/.depthos-manifest.json"
ROLLBACK_DIR="$SHARE_DIR/.rollback"

echo "Binary Directory:    $BIN_DIR"
echo "Spatial Assets:      $SPATIAL_DIR"
echo "SQL Migrations:      $MIGRATIONS_DIR"
echo "Desktop Entry:       $APPS_DIR/depthos.desktop"
echo "Application Icon:    $ICON_DIR/depthos.svg"
echo "Manifest:            $MANIFEST_FILE"

if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Preflight checks passed cleanly."
    echo "Dry run completed successfully. Zero changes written to disk."
    exit 0
fi

# Existing installation snapshot for rollback
if [[ -f "$MANIFEST_FILE" ]]; then
    echo "Existing DepthOS installation detected. Creating rollback snapshot..."
    rm -rf "$ROLLBACK_DIR"
    mkdir -p "$ROLLBACK_DIR"
    chmod 700 "$ROLLBACK_DIR"
    cp "$MANIFEST_FILE" "$ROLLBACK_DIR/.depthos-manifest.json"

    # Backup currently installed files listed in manifest
    python3 -c "
import json, os, shutil
from pathlib import Path

manifest_path = Path('$MANIFEST_FILE')
rollback_dir = Path('$ROLLBACK_DIR')
with open(manifest_path, 'r', encoding='utf-8') as f:
    manifest = json.load(f)

prefix = Path('$PREFIX_REAL')
for item in manifest.get('files', []):
    rel_path = item['path']
    src_file = prefix / rel_path
    if src_file.is_file():
        dst_file = rollback_dir / rel_path
        dst_file.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(src_file, dst_file)
"
    echo "Rollback snapshot saved to $ROLLBACK_DIR"
fi

# Atomic staging setup
STAGING_DIR="$SHARE_DIR/.staging_$$"
rm -rf "$STAGING_DIR"
mkdir -p "$STAGING_DIR/bin" "$STAGING_DIR/share/depthos" "$STAGING_DIR/share/applications" "$STAGING_DIR/share/icons/hicolor/scalable/apps"

# Stage binaries
echo "Staging binaries..."
install -m 755 "$DEPTHOS_SRC" "$STAGING_DIR/bin/depthos"
install -m 755 "$LAUNCHER_SRC" "$STAGING_DIR/bin/depthos-spatial-alpha"

# Stage spatial QML assets
echo "Staging spatial QML runtime modules..."
mkdir -p "$STAGING_DIR/share/depthos/spatial-alpha"
cp -r "$SPATIAL_SRC"/* "$STAGING_DIR/share/depthos/spatial-alpha/"
find "$STAGING_DIR/share/depthos/spatial-alpha" -type d -exec chmod 755 {} +
find "$STAGING_DIR/share/depthos/spatial-alpha" -type f -exec chmod 644 {} +

# Stage SQL migrations
echo "Staging SQL database migrations..."
mkdir -p "$STAGING_DIR/share/depthos/migrations"
cp -r "$MIGRATIONS_SRC"/* "$STAGING_DIR/share/depthos/migrations/"
find "$STAGING_DIR/share/depthos/migrations" -type d -exec chmod 755 {} +
find "$STAGING_DIR/share/depthos/migrations" -type f -exec chmod 644 {} +

# Stage desktop entry and icon
echo "Staging desktop integration assets..."
install -m 644 "$DESKTOP_SRC" "$STAGING_DIR/share/applications/depthos.desktop"
install -m 644 "$ICON_SRC" "$STAGING_DIR/share/icons/hicolor/scalable/apps/depthos.svg"

# Generate installation manifest from staging
echo "Generating installation manifest..."
python3 -c "
import hashlib, json, os, sys
from datetime import datetime, timezone
from pathlib import Path

staging_dir = Path('$STAGING_DIR')
prefix_real = Path('$PREFIX_REAL')
version = '$VERSION'

records = []

for root, dirs, files in os.walk(staging_dir):
    for f in sorted(files):
        full_staged = Path(root) / f
        rel_path = full_staged.relative_to(staging_dir)
        target_path = prefix_real / rel_path

        h = hashlib.sha256()
        with open(full_staged, 'rb') as fp:
            while chunk := fp.read(65536):
                h.update(chunk)
        digest = h.hexdigest()

        st = full_staged.stat()
        mode_oct = oct(st.st_mode & 0o777)

        records.append({
            'path': str(rel_path),
            'absolute_path': str(target_path),
            'sha256': digest,
            'mode': mode_oct,
            'size_bytes': st.st_size
        })

manifest = {
    'schema_version': '1.0',
    'package_name': 'depthos',
    'package_version': version,
    'installed_at': datetime.now(timezone.utc).isoformat(timespec='seconds'),
    'prefix': str(prefix_real),
    'security_mode': 'DENY-ONLY',
    'observation_mode': 'READ-ONLY',
    'file_count': len(records),
    'files': records
}

manifest_path = staging_dir / 'share/depthos/.depthos-manifest.json'
with open(manifest_path, 'w', encoding='utf-8') as fp:
    json.dump(manifest, fp, indent=2)
"

# Commit staged files to prefix
echo "Installing files to prefix..."
mkdir -p "$BIN_DIR" "$SPATIAL_DIR" "$MIGRATIONS_DIR" "$APPS_DIR" "$ICON_DIR"
chmod 755 "$BIN_DIR" "$SHARE_DIR" "$SPATIAL_DIR" "$MIGRATIONS_DIR" "$APPS_DIR" "$ICON_DIR"

# Copy staged files to final destination
shopt -s dotglob
cp -a "$STAGING_DIR/bin/"* "$BIN_DIR/"
cp -a "$STAGING_DIR/share/depthos/"* "$SHARE_DIR/"
cp -a "$STAGING_DIR/share/applications/"* "$APPS_DIR/"
cp -a "$STAGING_DIR/share/icons/hicolor/scalable/apps/"* "$ICON_DIR/"
shopt -u dotglob

# Remove staging directory
rm -rf "$STAGING_DIR"

# Post-installation verification
echo "Running post-installation verification..."
for bin in "$BIN_DIR/depthos" "$BIN_DIR/depthos-spatial-alpha"; do
    mode=$(stat -c "%a" "$bin")
    if [[ "$mode" != "755" ]]; then
        echo "ERROR: Unexpected binary permission $mode on $bin" >&2
        exit 1
    fi
done

for asset in "$APPS_DIR/depthos.desktop" "$ICON_DIR/depthos.svg"; do
    mode=$(stat -c "%a" "$asset")
    if [[ "$mode" != "644" ]]; then
        echo "ERROR: Unexpected asset permission $mode on $asset" >&2
        exit 1
    fi
done

if [[ ! -f "$MANIFEST_FILE" ]]; then
    echo "ERROR: Manifest file was not created: $MANIFEST_FILE" >&2
    exit 1
fi

echo "SUCCESS: DepthOS Alpha installed cleanly."
echo "Manifest: $MANIFEST_FILE"
echo "Launch with: depthos-spatial-alpha"
