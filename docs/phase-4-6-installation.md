# DepthOS Phase 4.6 Installation Guide

## 1. Installation Overview

The DepthOS installation tool (`install.sh` / `pkg/install.sh`) provisions the DepthOS CLI, the Spatial Alpha viewport, SQL migrations, FreeDesktop application entry, and icon into a designated prefix without system-wide root privileges.

### Default Target Paths
- **Prefix**: `$HOME/.local` (customizable via `--prefix <DIR>`)
- **Binaries**: `<prefix>/bin/`
- **Data & QML Runtime**: `<prefix>/share/depthos/spatial-alpha/`
- **Migrations**: `<prefix>/share/depthos/migrations/`
- **Desktop Entry**: `<prefix>/share/applications/depthos.desktop`
- **Icon**: `<prefix>/share/icons/hicolor/scalable/apps/depthos.svg`
- **Manifest**: `<prefix>/share/depthos/.depthos-manifest.json`

---

## 2. Command-Line Options

```bash
./install.sh [OPTIONS]

Options:
  --prefix <DIR>    Installation prefix directory (default: $HOME/.local)
  --dry-run         Perform preflights and simulate installation without disk changes
  --version         Display installer version information
  -h, --help        Show usage help
```

### Installation Examples

#### Standard User-Space Installation
```bash
./install.sh
```

#### Custom Isolated Prefix Installation
```bash
./install.sh --prefix /opt/depthos-alpha
```

#### Dry-Run Preflight Simulation
```bash
./install.sh --prefix /opt/depthos-alpha --dry-run
```

---

## 3. Security Preflight & Validation Checks

Before any file is copied or staged, `install.sh` enforces strict security preflights:

1. **System Directory Blacklist**:
   Direct installation into critical system roots (`/`, `/bin`, `/sbin`, `/etc`, `/boot`, `/dev`, `/proc`, `/sys`, `/lib`) is strictly rejected.
2. **Permission Boundary Audit**:
   If the target prefix directory already exists:
   - It must **not** be world-writable (`mode & 0002 != 0`).
   - Permissions `0777` and `0775` are proactively rejected.
   - The directory must be owned by the current invoking user (`UID`) or `root`.
3. **Symlink Traversal Containment**:
   The installer canonicalizes the prefix using `realpath -m` and verifies that symlink chains do not escape intended filesystem boundaries.
4. **Setuid / Setgid Prohibition**:
   Every source binary, script, and asset is audited to verify zero setuid (`04000`) or setgid (`02000`) bits. Any violation immediately aborts execution.

---

## 4. Atomic Staging and Rollback Snapshots

If an existing installation is detected (identified by `<prefix>/share/depthos/.depthos-manifest.json`):
1. A rollback snapshot is created in `<prefix>/share/depthos/.rollback/` containing the previous manifest and exact copies of all previously installed files.
2. New files are staged into `<prefix>/share/depthos/.staging_<PID>/`.
3. SHA-256 hashes, sizes, and file modes are calculated in staging.
4. `.depthos-manifest.json` is generated.
5. Staged files are moved into final locations atomically.
6. Temporary staging directories are cleaned up.

---

## 5. Post-Installation Verification

Upon successful installation, verify the deployment:

```bash
# Verify CLI binary
depthos --version

# Verify Spatial Alpha launcher
depthos-spatial-alpha --help

# Verify installed permissions
stat -c "%a %n" ~/.local/bin/depthos ~/.local/bin/depthos-spatial-alpha
# Expected: 755 for each binary

stat -c "%a %n" ~/.local/share/applications/depthos.desktop
# Expected: 644
```
