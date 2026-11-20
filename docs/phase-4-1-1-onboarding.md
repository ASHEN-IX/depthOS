# Phase 4.1.1 first-run onboarding

The database is created only by an explicit action: `depthos init` on the command line, or **Create my workspace** in the spatial window. Opening the UI, running `depthos status`, and running `depthos tree` do not create or migrate a file.

`depthos init` and `depthos status` are the commands added in `00fec02`. This note is the UI and launcher behavior that sits on top of them.

## What the window shows

The launcher picks exactly one source. It does not fill the canvas with demo data because a controller or a file was missing.

| Situation | What you see |
| --- | --- |
| `--mock` / `--demo` | Source label **Demo**, watermark `DEMO DATA`, observer **Native Observer Unavailable**. |
| `--db PATH` and `depthos status` says `missing` or `empty` | Panel: the path, **Create my workspace**, **Continue in Demo Mode**. Nothing is written yet. Source label **No Source**. |
| `--db PATH` and the file is a valid DepthOS database | Source label **Local SQLite**. The hierarchy is loaded read-only via `depthos tree`. Observer stays **Native Observer Unavailable**. |
| `--db PATH` and the file is corrupt, a symlink, a hard link, foreign, the wrong schema, or has unsafe permissions | Panel explains the refusal. **Create my workspace** is not offered, because `depthos init` would refuse. **Continue in Demo Mode** and **Try again** are offered. |
| No `--db`, and `org.depthos.Controller` answers | Source label **Connected Controller**. **Native Observer Live** only if that controller reports observation `Synchronized`. A live controller connection is not treated as native observation. |
| No `--db`, no controller, default database missing | Same offer as a missing `--db`, using the path `depthos status --json` reports. That probe does not create the directory or the file. |
| **Create my workspace** | The button sets the panel to "creating", then the launcher runs `depthos --db PATH init --default-hierarchy`. Success says **Workspace created** (or **Workspace already exists** / "Nothing was overwritten" when init reports `already-initialized`). Failure shows the command's error text. **Open workspace** dismisses the panel. |
| **Continue in Demo Mode** | Loads the built-in sample and labels it Demo. This is the only way demo data replaces an empty start. |

The default hierarchy, when init is asked to seed it, is still Default World / Primary Workspace / Desktop 1. It does not invent project directories, git branches, or native desktops. Empty window lists say the observer is unavailable, or, when an observer really is live, that no native windows are associated.

Partial and unsafe databases stay on the init command's rules: a 0-byte file can be initialized in place; a real database is never overwritten; symlinks and unsafe modes are refused without being rewritten. The UI surfaces that result. It does not add a second writer.

## How to launch

```sh
# Read-only look at an existing database. Does not create one.
depthos --db /path/to/workspaces.sqlite3 status
depthos ui --db /path/to/workspaces.sqlite3

# Explicit create, then open.
depthos --db /path/to/workspaces.sqlite3 init --default-hierarchy
depthos ui --db /path/to/workspaces.sqlite3

# Sample data, clearly marked.
depthos ui --mock
```

`depthos ui` finds `tools/depthos-spatial-alpha` next to the checkout (or next to the binary) and sets `DEPTHOS_BIN` to itself. There is no hard-coded home-directory path.

Diagnostics, all optional:

```sh
./tools/depthos-spatial-alpha --db "$DB" \
  --frame-log /tmp/depthos-frames.jsonl \
  --memory-log /tmp/depthos-memory.jsonl \
  --screenshot /tmp/depthos.png
```

Screenshots are `grabToImage` of the DepthOS scene only.

## Checks against disposable databases

On this machine, with `XDG_DATA_HOME` pointed at a temp directory and `--db` under `/tmp`:

- Launching the UI on a path that does not exist left the path absent and showed the offer panel (Wayland screenshot `01-first-run-offer.png`).
- `depthos init --default-hierarchy` on a second temp file, then the UI, showed **Local SQLite**, Default World / Primary Workspace, and **Native Observer Unavailable** (`02-local-sqlite.png`).
- `--mock` showed **Demo**, the watermark, and **Native Observer Unavailable** (`03-demo.png`).
- `~/.local/share/depthos` did not exist before these runs and did not exist after them.

The QML suite `tests/spatial/tst_phase_4_1_1.qml` covers the offer, the create click (one request, then the "creating" state, a second click ignored), demo, error and success copy, and that scene keys do nothing while the panel is up. Rust `tests/first_run_init.rs` covers atomic create, permissions, idempotence, and the read-only `status` / `tree` paths.
