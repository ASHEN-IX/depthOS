# Phase 4.2 observer host deployment plan

**Nothing in this plan was executed on the host compositor.** No package was copied into `~/.local/share/kwin`. No script was loaded into the host KWin (pid 1727 at the start of this stage). `kwinrc`, `kglobalshortcutsrc`, and `kdeglobals` were not edited by this campaign. `~/.local/share/depthos` was not created. The only install that has happened is the private-session install described in `docs/phase-4-1-1-observer-feasibility.md`, under a disposable `XDG_DATA_HOME`.

This document is not approval.

## Files that would be installed

Source tree, read-only input:

- `experiments/kwin-observer/metadata.json`
- `experiments/kwin-observer/contents/code/main.js`

Destination, and the only path `scripts/install-observer.fish` writes:

- `${XDG_DATA_HOME:-$HOME/.local/share}/kwin/scripts/depthos-system-observer/`

Runtime id, not a file: `depthos-system-observer-session` (the second argument to `loadScript`).

Not installed by these scripts: KWin binaries, effects, `kwinrc`, `kglobalshortcutsrc`, `kdeglobals`, autostart entries, shortcuts, gestures, a systemd unit, or a production consent token.

## Compatibility

Private KWin 6.8.3 loaded this package, delivered snapshot / window add / focus / window remove / a second snapshot, and unloaded cleanly.

`scripts/install-observer.fish` and `scripts/observer-session.fish` compare `kwin_wayland --version` to the exact string `kwin 6.8.1`. On this host the command prints `kwin 6.8.3`, so both scripts exit 1 before they touch a package. A host install with the current fish scripts will not happen until that pin is updated and the user approves the new pin.

## Commands

Do not run these until the approvals below are explicit. From the repo root, as the desktop user, not root.

Install:

```
fish scripts/install-observer.fish --install
```

That runs `kpackagetool6 --type KWin/Script --packageroot "$XDG_DATA_HOME/kwin/scripts" --install experiments/kwin-observer`. It refuses if the destination already exists. It does not load the script.

Start, only after `tools/state-inspector/main.py` is already in the foreground on the host session bus:

```
fish scripts/observer-session.fish --start
```

That calls `loadScript` on the installed `main.js` with plugin name `depthos-system-observer-session`, then `Script.run` on that script object. It does not call `/Scripting.start`.

Stop:

```
fish scripts/observer-session.fish --stop
```

Removal, only when both `depthos-system-observer` and `depthos-system-observer-session` report `isScriptLoaded == false`:

```
fish scripts/uninstall-observer.fish --remove
```

That runs `kpackagetool6 --type KWin/Script --packageroot "$XDG_DATA_HOME/kwin/scripts" --remove depthos-system-observer`.

## Restart and rollback

`loadScript` / `Script.run` / `unloadScript` took effect on the private compositor without restarting it. A host KWin restart is not required to load or unload. A host KWin restart clears the runtime script. The package files stay until `--remove`.

Rollback is `--stop`, then `--remove`. If `--remove` refuses because the script is still loaded, `--stop` again and retry. If a symlink is found at the package path, the uninstaller stops and a person has to look at it. There is no automatic config restore because the installer does not edit KDE config. If someone later enables the script in System Settings, that is a separate `kwinrc` change and is outside this plan.

## Security and privacy

The script sends window captions as `title` to `org.depthos.StateInspector` on the session bus. Authorization is "caller is this user's KWin". Any process that can own that bus name as this user can receive titles, including `excludeFromCapture` windows, because `main.js` copies `caption` unconditionally. The inspector's default snapshot redacts titles. Passing `--include-titles` does not.

The script does not set window geometry, desktops, or focus. It does not register a shortcut. It does not capture screen pixels.

`isScriptLoaded("depthos-observer")` will stay false after a correct install. The ids that matter are `depthos-system-observer` (package) and `depthos-system-observer-session` (running script).

## Exact approval required before any host action

1. The user agrees the version pin may move from 6.8.1 to 6.8.3, or agrees to run the kpackagetool6 lines by hand.
2. The user agrees to copy the package into `~/.local/share/kwin/scripts/depthos-system-observer`.
3. The user agrees to a runtime `loadScript` of that `main.js` into the host KWin, knowing captions will be sent to a same-user inspector.
4. The user starts the inspector themselves. This plan does not start it.
5. The user agrees that uninstall is `--stop` then `--remove`, and that a later KWin restart drops the script until `--start` is repeated.

Until those five are explicit, the host stays as it is: script not loaded, package not present.
