#!/usr/bin/env bash
# DepthOS Alpha — Release Packaging Tool
# Builds release binary offline, packages all runtime components,
# and generates distribution tarball, checksums, and release manifest.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

VERSION="0.1.0-alpha"
ARCH="linux-x86_64"
PACKAGE_NAME="depthos-${VERSION}-${ARCH}"
DIST_DIR="$ROOT_DIR/artifacts/phase-4-6/dist"
STAGE_DIR="$DIST_DIR/.stage/${PACKAGE_NAME}"
TARBALL_PATH="$DIST_DIR/${PACKAGE_NAME}.tar.gz"
SHA256_FILE="$DIST_DIR/SHA256SUMS"
MANIFEST_FILE="$DIST_DIR/release-manifest.json"

echo "=== DepthOS Alpha Release Packager ==="
echo "Version:         $VERSION"
echo "Architecture:    $ARCH"
echo "Package:         $PACKAGE_NAME"
echo "Distribution:    $DIST_DIR"

# 1. Compile release binary offline
echo "Building depthos release binary with cargo --locked --offline..."
(
    cd "$ROOT_DIR"
    cargo build --release --locked --offline
)

DEPTHOS_BIN="$ROOT_DIR/target/release/depthos"
if [[ ! -f "$DEPTHOS_BIN" ]]; then
    echo "ERROR: Target release binary not found at $DEPTHOS_BIN" >&2
    exit 1
fi

# 2. Stage files
echo "Preparing packaging staging directory..."
rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/bin"
mkdir -p "$STAGE_DIR/share/depthos/spatial-alpha"
mkdir -p "$STAGE_DIR/share/depthos/migrations"
mkdir -p "$STAGE_DIR/pkg"

# Copy binaries
echo "Staging binaries..."
install -m 755 "$DEPTHOS_BIN" "$STAGE_DIR/bin/depthos"
install -m 755 "$ROOT_DIR/tools/depthos-spatial-alpha" "$STAGE_DIR/bin/depthos-spatial-alpha"

# Copy spatial assets
echo "Staging spatial QML assets..."
cp -r "$ROOT_DIR/experiments/spatial-alpha"/* "$STAGE_DIR/share/depthos/spatial-alpha/"
find "$STAGE_DIR/share/depthos/spatial-alpha" -type d -exec chmod 755 {} +
find "$STAGE_DIR/share/depthos/spatial-alpha" -type f -exec chmod 644 {} +

# Copy migrations
echo "Staging SQL migrations..."
cp -r "$ROOT_DIR/migrations"/* "$STAGE_DIR/share/depthos/migrations/"
find "$STAGE_DIR/share/depthos/migrations" -type d -exec chmod 755 {} +
find "$STAGE_DIR/share/depthos/migrations" -type f -exec chmod 644 {} +

# Copy desktop integration assets
echo "Staging desktop integration assets..."
install -m 644 "$ROOT_DIR/pkg/depthos.desktop" "$STAGE_DIR/pkg/depthos.desktop"
install -m 644 "$ROOT_DIR/pkg/depthos.svg" "$STAGE_DIR/pkg/depthos.svg"

# Copy management and lifecycle scripts (accessible at package root and in pkg/)
echo "Staging lifecycle scripts..."
for script in install.sh uninstall.sh upgrade.sh rollback.sh backup.sh restore.sh; do
    install -m 755 "$ROOT_DIR/pkg/$script" "$STAGE_DIR/$script"
    install -m 755 "$ROOT_DIR/pkg/$script" "$STAGE_DIR/pkg/$script"
done

# Copy licenses and notices
echo "Staging license and notice documents..."
install -m 644 "$ROOT_DIR/pkg/LICENSE" "$STAGE_DIR/LICENSE"
install -m 644 "$ROOT_DIR/NOTICES.md" "$STAGE_DIR/NOTICES.md"

# 3. Security preflight on staging directory
echo "Executing packaging security preflight checks..."
python3 - "$STAGE_DIR" << 'EOF'
import os
import stat
import sys
from pathlib import Path

stage_dir = Path(sys.argv[1])
violations = []

for root, dirs, files in os.walk(stage_dir):
    for d in dirs:
        p = Path(root) / d
        st = p.stat()
        if st.st_mode & 0o002:
            violations.append(f"Directory {p} is world-writable ({oct(st.st_mode)})")
    for f in files:
        p = Path(root) / f
        st = p.stat()
        mode = st.st_mode
        # Prohibit setuid/setgid
        if mode & (stat.S_ISUID | stat.S_ISGID):
            violations.append(f"File {p} has setuid/setgid bits ({oct(mode)})")
        # Prohibit world-writable
        if mode & stat.S_IWOTH:
            violations.append(f"File {p} is world-writable ({oct(mode)})")

if violations:
    sys.stderr.write("PACKAGING SECURITY VIOLATIONS DETECTED:\n")
    for v in violations:
        sys.stderr.write(f"  - {v}\n")
    sys.exit(1)
print("Packaging security preflight checks passed cleanly.")
EOF

# 4. Generate initial file inventory
GIT_COMMIT=$(cd "$ROOT_DIR" && git rev-parse HEAD)
BUILD_DATE=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# 5. Build tarball
echo "Creating compressed distribution archive: $TARBALL_PATH..."
(
    cd "$DIST_DIR/.stage"
    # Create deterministic tarball
    tar --sort=name --owner=0 --group=0 --numeric-owner -czf "$TARBALL_PATH" "$PACKAGE_NAME"
)

# 6. Generate SHA256SUMS
echo "Calculating artifact checksums..."
(
    cd "$DIST_DIR"
    sha256sum "$(basename "$TARBALL_PATH")" > "$SHA256_FILE"
)

# 7. Generate release-manifest.json
echo "Generating release manifest..."
python3 - "$STAGE_DIR" "$TARBALL_PATH" "$VERSION" "$ARCH" "$GIT_COMMIT" "$BUILD_DATE" "$MANIFEST_FILE" << 'EOF'
import hashlib
import json
import os
import sys
from pathlib import Path

stage_dir = Path(sys.argv[1])
tarball_path = Path(sys.argv[2])
version = sys.argv[3]
arch = sys.argv[4]
git_commit = sys.argv[5]
build_date = sys.argv[6]
manifest_output = Path(sys.argv[7])

def compute_sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fp:
        while chunk := fp.read(65536):
            h.update(chunk)
    return h.hexdigest()

tarball_sha256 = compute_sha256(tarball_path)
tarball_size = tarball_path.stat().st_size

file_inventory = []
for root, dirs, files in os.walk(stage_dir):
    for f in sorted(files):
        p = Path(root) / f
        rel = p.relative_to(stage_dir)
        st = p.stat()
        file_inventory.append({
            "path": str(rel),
            "sha256": compute_sha256(p),
            "mode": oct(st.st_mode & 0o777),
            "size_bytes": st.st_size
        })

manifest = {
    "schema_version": "1.0",
    "package_name": "depthos",
    "version": version,
    "architecture": arch,
    "git_commit": git_commit,
    "build_timestamp": build_date,
    "archive_filename": tarball_path.name,
    "archive_sha256": tarball_sha256,
    "archive_size_bytes": tarball_size,
    "security_declaration": {
        "native_authorization": "DENY-ONLY",
        "native_observation": "READ-ONLY",
        "setuid_setgid_prohibited": True,
        "world_writable_prohibited": True
    },
    "runtime_requirements": {
        "os": "Linux",
        "kernel_minimum": "5.10",
        "landlock_abi_minimum": 1,
        "python_minimum": "3.10",
        "pyqt6_minimum": "6.5.0",
        "qt_quick_minimum": "6.5.0",
        "sqlite_minimum": "3.35.0",
        "compositor_recommended": "KWin >= 6.0"
    },
    "file_inventory": file_inventory
}

with open(manifest_output, "w", encoding="utf-8") as fp:
    json.dump(manifest, fp, indent=2)

print(f"Release manifest generated with {len(file_inventory)} files.")
EOF

# Clean up stage directory
rm -rf "$DIST_DIR/.stage"

echo "=== Release Build Complete ==="
echo "Tarball:   $TARBALL_PATH"
echo "Checksums: $SHA256_FILE"
echo "Manifest:  $MANIFEST_FILE"
cat "$SHA256_FILE"
