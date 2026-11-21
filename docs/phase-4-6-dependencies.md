# DepthOS Phase 4.6 Dependency and Environment Specification

## 1. System Requirements & Kernel Boundaries

| Subsystem | Minimum Version | Recommended Version | Purpose / Security Constraint |
|---|---|---|---|
| **Linux Kernel** | $\ge 5.10$ | $\ge 6.6$ LTS | Core POSIX, abstract UNIX domain sockets |
| **Landlock LSM** | ABI v1 ($\ge 5.13$) | ABI v3+ ($\ge 6.2$) | Unprivileged filesystem mutation containment and socket scoping |
| **Wayland Compositor** | Any Wayland compliant | KWin $\ge 6.0$ | Physical Wayland presentation timing via `wp_presentation` v2 |
| **D-Bus Session Bus** | FreeDesktop D-Bus | D-Bus 1.14+ | Local private session broker for controller and observer communication |

---

## 2. Runtime Dependencies

| Component | Minimum Version | Distribution Package (Arch / Debian / Fedora) | Purpose |
|---|---|---|---|
| **Python** | $\ge 3.10$ | `python` / `python3` / `python3` | Host bridge, launcher script, lifecycle tools |
| **PyQt6** | $\ge 6.5.0$ | `python-pyqt6` / `python3-pyqt6` / `python3-pyqt6` | Qt Quick QML runtime host, D-Bus event integration |
| **Qt Quick / QML** | $\ge 6.5.0$ | `qt6-declarative` / `qml6-module-qtquick` / `qt6-qtdeclarative` | Neon Spatial cyber-tech scene graph & shaders |
| **python-dbus** | $\ge 1.3.0$ | `python-dbus` / `python3-dbus` / `python3-dbus` | Native session bus IPC for spatial launcher bridge |
| **SQLite** | $\ge 3.35.0$ | `sqlite` / `libsqlite3-0` / `sqlite-libs` | Strict typing (`STRICT`), WAL mode, JSON functions |

---

## 3. Build-Time Dependencies

| Toolchain / Library | Minimum Version | Purpose |
|---|---|---|
| **Rust Toolchain** | $\ge 1.85.0$ | Compilation of `depthos` CLI binary and `depthos-observer` library |
| **Cargo** | $\ge 1.85.0$ | Workspace package management and offline lockfile builds (`--locked --offline`) |
| **C Compiler** | GCC $\ge 11$ / Clang $\ge 14$ | Native C runtime compilation for rusqlite / libsqlite3-sys |
| **pkg-config** | $\ge 0.29$ | System library auto-discovery |

---

## 4. Security & Permissions Specification

In accordance with DepthOS private-alpha specifications:
- **Executable binaries (`depthos`, `depthos-spatial-alpha`)**: Mode `0755`, owned by executing user or `root`. Zero setuid/setgid bits permitted.
- **Desktop entries & icons**: Mode `0644`.
- **Database directory (`$XDG_DATA_HOME/depthos`)**: Mode `0700` strictly enforced.
- **Database files (`workspace.sqlite`)**: Mode `0600` strictly enforced.
- **Authorization Boundary**: Strictly **DENY-ONLY**.
- **Observation Boundary**: Strictly **READ-ONLY**.
- **Network Boundaries**: Zero external network egress, telemetry reporting, or credential transmission endpoints.
