#!/usr/bin/env bash
# DepthOS Alpha — Reproducible Local Installation Script
# Installs DepthOS CLI, Spatial Alpha launcher, QML modules, desktop entry, and icon.
set -euo pipefail

PREFIX="${PREFIX:-$HOME/.local}"
XDG_DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
DRY_RUN=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --prefix)
            PREFIX="$2"
            shift 2
            ;;
        --dry-run)
            DRY_RUN=1
            shift
            ;;
        -h|--help)
            echo "Usage: $0 [--prefix <DIR>] [--dry-run]"
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/depthos"
APPS_DIR="$XDG_DATA_HOME/applications"
ICON_DIR="$XDG_DATA_HOME/icons/hicolor/scalable/apps"

echo "=== DepthOS Alpha Local Installation ==="
echo "Target Prefix:       $PREFIX"
echo "Binary Directory:    $BIN_DIR"
echo "Spatial Assets:      $SHARE_DIR/spatial-alpha"
echo "Desktop Entry:       $APPS_DIR/depthos.desktop"
echo "Application Icon:    $ICON_DIR/depthos.svg"

# Verify source binary
DEPTHOS_SRC="$ROOT_DIR/target/release/depthos"
if [[ ! -f "$DEPTHOS_SRC" ]]; then
    DEPTHOS_SRC="$ROOT_DIR/target/debug/depthos"
fi

if [[ ! -f "$DEPTHOS_SRC" ]]; then
    echo "ERROR: DepthOS binary not found at target/release/depthos or target/debug/depthos." >&2
    echo "Run 'cargo build --release' first." >&2
    exit 1
fi

LAUNCHER_SRC="$ROOT_DIR/tools/depthos-spatial-alpha"
DESKTOP_SRC="$ROOT_DIR/pkg/depthos.desktop"
ICON_SRC="$ROOT_DIR/pkg/depthos.svg"
SPATIAL_SRC="$ROOT_DIR/experiments/spatial-alpha"

# Security Preflight
echo "Running packaging security preflight checks..."
for src in "$DEPTHOS_SRC" "$LAUNCHER_SRC" "$DESKTOP_SRC" "$ICON_SRC"; do
    if [[ ! -f "$src" ]]; then
        echo "ERROR: Required source file missing: $src" >&2
        exit 1
    fi
    # Check no setuid or setgid
    if [[ -u "$src" || -g "$src" ]]; then
        echo "SECURITY ERROR: Source file $src has dangerous setuid/setgid bits" >&2
        exit 1
    fi
done

if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Dry run completed successfully. Zero changes written."
    exit 0
fi

# Create directories
mkdir -p "$BIN_DIR" "$SHARE_DIR" "$APPS_DIR" "$ICON_DIR"
chmod 755 "$BIN_DIR" "$APPS_DIR" "$ICON_DIR" "$SHARE_DIR"

# Install binaries
echo "Installing binaries..."
install -m 755 "$DEPTHOS_SRC" "$BIN_DIR/depthos"
install -m 755 "$LAUNCHER_SRC" "$BIN_DIR/depthos-spatial-alpha"

# Install spatial QML modules
echo "Installing spatial QML runtime modules..."
rm -rf "$SHARE_DIR/spatial-alpha"
cp -r "$SPATIAL_SRC" "$SHARE_DIR/spatial-alpha"
find "$SHARE_DIR/spatial-alpha" -type d -exec chmod 755 {} +
find "$SHARE_DIR/spatial-alpha" -type f -exec chmod 644 {} +

# Install desktop entry and icon
echo "Installing desktop integration assets..."
install -m 644 "$DESKTOP_SRC" "$APPS_DIR/depthos.desktop"
install -m 644 "$ICON_SRC" "$ICON_DIR/depthos.svg"

# Post-installation verification
echo "Validating installed permissions..."
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

echo "SUCCESS: DepthOS Alpha installed cleanly."
echo "Launch with: depthos-spatial-alpha"
