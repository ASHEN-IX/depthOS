# DepthOS Alpha — Dependency Manifest & Environment Specification

## 1. System Requirements & Kernel Boundaries

| Subsystem | Minimum Version | Recommended Version | Purpose / Security Constraint |
|---|---|---|---|
| **Linux Kernel** | $\ge 5.10$ | $\ge 6.6$ LTS | Core POSIX, UNIX domain abstract socket mediation |
| **Landlock LSM** | ABI v1 ($\ge 5.13$) | ABI v3+ ($\ge 6.2$) | Unprivileged filesystem mutation containment and socket scoping |
| **Wayland Compositor** | Any Wayland compliant | KWin $\ge 6.0$ | Physical Wayland presentation timing via `wp_presentation` v2 |
| **D-Bus Session Bus** | FreeDesktop D-Bus | D-Bus 1.14+ | Local private session broker for controller and observer communication |

---

## 2. Runtime Dependencies

| Package / Library | Minimum Version | Distribution Package (Debian/Ubuntu / Arch / Fedora) | Purpose |
|---|---|---|---|
| **Python** | $\ge 3.10$ | `python3` / `python` / `python3` | Host bridge, CLI management glue, test harnesses |
| **PyQt6** | $\ge 6.5.0$ | `python3-pyqt6` / `python-pyqt6` / `python3-pyqt6` | Qt Quick QML runtime host, D-Bus event integration |
| **Qt Quick / QML** | $\ge 6.5.0$ | `qml6-module-qtquick` / `qt6-declarative` | Neon Spatial cyber-tech scene graph & shaders |
| **python-dbus** | $\ge 1.3.0$ | `python3-dbus` / `python-dbus` / `python3-dbus` | Native session bus IPC |
| **SQLite** | $\ge 3.35.0$ | `libsqlite3-0` / `sqlite` / `sqlite-libs` | Strict typing (`STRICT`), WAL mode, JSON functions |

---

## 3. Build-Time Dependencies

| Toolchain / Library | Minimum Version | Purpose |
|---|---|---|
| **Rust Toolchain** | $\ge 1.85.0$ | Compilation of `depthos` CLI binary and `depthos-observer` library |
| **Cargo** | $\ge 1.85.0$ | Workspace package management and offline lockfile builds |
| **C Compiler** | GCC $\ge 11$ / Clang $\ge 14$ | Native C runtime compilation for rusqlite / libsqlite3 |
| **pkg-config** | $\ge 0.29$ | System library auto-discovery |

---

## 4. File Modes & Security Constraints

In accordance with DepthOS security specifications:
- **Executable binaries (`depthos`, `depthos-spatial-alpha`)**: Mode `0755`, owned by the executing user or system `root`. Zero setuid/setgid bits permitted.
- **Desktop entries & icons**: Mode `0644`.
- **Database directory (`$XDG_DATA_HOME/depthos`)**: Mode `0700` required. The DepthOS database engine strictly refuses initialization if the directory is group- or world-writable (`0777`, `0775`).
- **Database files (`workspace.sqlite`)**: Mode `0600` strictly enforced.
- **Embedded secrets**: Zero private keys, auth tokens, or telemetry reporting endpoints exist in the package.
