# Phase 4.1.1 observer feasibility

The observer package was not modified between `901166a` and this review. The private-session evidence below is the Stage 1 run `artifacts/phase-4-1-1/agent2/raw/observer/observer-20261120T133614/`. This stage did not install the package again and did not load it into the host compositor. Host `isScriptLoaded("depthos-observer")` stays false. `~/.local/share/kwin/scripts/depthos-system-observer` is absent. `~/.local/share/depthos` is absent.

Status: the private session loaded the observer. This is not BLOCKED.

## What the component is

`experiments/kwin-observer` is a KPackage of type `KWin/Script` (`metadata.json`, `X-Plasma-API: javascript`). Plugin id: `depthos-system-observer`. It is not a KWin effect and not a C++ plugin. `contents/code/main.js` reads `workspace` (windows, desktops, screens, focus, activities) and pushes JSON to D-Bus service `org.depthos.StateInspector`, path `/org/depthos/StateInspector`, interface `org.depthos.StateInspector1`, methods `Hello` and `Push`. The Rust crate `crates/depthos-observer` is the consumer/replica side, not the compositor plugin. A hybrid is "KWin script plus same-user inspector process". There is no binary patch of KWin.

`scripts/install-observer.fish` and `scripts/observer-session.fish` refuse to run unless `kwin_wayland --version` prints `kwin 6.8.1`. This machine prints `kwin 6.8.3`, so those fish scripts exit before install or load. That gate is in the installer, not in the script. The private test bypassed the pin on purpose.

## Private session result

`kwin_wayland --virtual` on a private session bus. Package installed with `kpackagetool6 --type KWin/Script --packageroot <private XDG_DATA_HOME>/kwin/scripts --install experiments/kwin-observer`. Return code 0. The files landed under the run directory, not `~/.local/share/kwin`.

`loadScript(<private>/contents/code/main.js, "depthos-system-observer-session")` returned script number 0. `Script.run` returned ok.

| `isScriptLoaded` id | Before | After `run` | After unload | After second `run` | At end |
| --- | --- | --- | --- | --- | --- |
| `depthos-observer` | false | false | false | false | false |
| `depthos-system-observer` | false | false | false | false | false |
| `depthos-system-observer-session` | false | true | false | true | false |

The id the acceptance report and `host-isolation.sh` query, `depthos-observer`, stays false while the real session script is loaded. A host check of that string does not detect this package.

The private inspector (titles stripped before the file was written) recorded:

1. `snapshot` sequence 1
2. `window.added` `window-1`, then `window.focus.changed` to `window-1`
3. `window.added` `window-2`, then `window.focus.changed` to `window-2` after a private-only `workspace.activeWindow` assignment
4. `window.removed` `window-1` after that client exited
5. A second `snapshot` at sequence 1 after unload and a second `loadScript`/`run`

The second snapshot's sequence restarted at 1, which is a new observer session. The harness had required the sequence to keep climbing, so `results.json` says `PARTIAL`. The event list shows the new snapshot, `errors` is 0, and `isScriptLoaded` was true for the second run. Reconnect worked. `kwin-interesting.log` is empty.

Desktop association: one virtual output `Virtual-0` 1920×1080, desktop id `015ee9fd-59d0-4958-afcb-7dc701e65c56` named `Desktop 1`, `activeDesktopByScreen.screen-1` equal to that id, and the focused window's `desktopIds` contain that id.

Cleanup: `unloadScript` then `kpackagetool6 --remove depthos-system-observer` on the private packageroot, return code 0, package path gone, all three script ids false.

## Answers

1. **Form.** KWin JavaScript script plus an out-of-process inspector. Not an effect, not a host plugin binary.
2. **KWin 6.8.3.** The script loads, runs, and emits snapshot, add, focus, and remove. The fish installer still refuses anything but 6.8.1.
3. **Permissions.** Same-user session bus. `Hello`/`Push` accept the caller only when the sender is the unique name of `org.kde.KWin` and the Unix uid matches the inspector. KWin must be allowed to `callDBus`. No new portal and no Polkit rule showed up in the private run. `KWIN_WAYLAND_NO_PERMISSION_CHECKS` was set only inside the private session so the nested compositor would start.
4. **Read-only deploy without replacing KWin.** Yes. `kpackagetool6` copies a user-local package. `loadScript` + `Script.run` does not replace KWin binaries. The script has no window setter. The QA focus assignment was a different, throwaway script.
5. **Persistent config.** The fish session tool does not write `kwinrc` or shortcut files. `loadScript` is a runtime registration. Installing the package writes only `$XDG_DATA_HOME/kwin/scripts/depthos-system-observer/`. Enabling the package in KWin's script list would persist and was not done.
6. **Enable / disable.** Enable: inspector first, then `observer-session.fish --start` (or the `loadScript`/`run` pair). Disable: `observer-session.fish --stop` (`unloadScript`). Do not call `/Scripting.start`, which the fish header says would start every script.
7. **Uninstall.** `uninstall-observer.fish --remove` after the session id is unloaded. It refuses a symlink package and refuses removal while either id is loaded. Private-session remove succeeded.
8. **Metadata exposed.** Runtime id, native id, caption (as `title`), desktop file name, resource class/name, classification, type flags, parent, desktop ids, activity ids, geometry, screen, minimized, fullscreen, skipTaskbar, excludeFromCapture, truncation flag. Desktops, activities, screens, and the active ids go out as globals.
9. **Titles.** `windowState()` sets `title` from `caption` with no branch on `excludeFromCapture`. The private wire carried title strings; recorded lengths were 8, 8, and 8, matching the QA captions `QA-Obs-A` and `QA-Obs-B`. The saved JSON stores `titleRedacted: true` and does not store the characters. Default inspector output redacts. Protected-window captions are still copied by this code. This stage did not add a redaction branch, because that would change the Phase 3 wire format, and the script is not loaded on the host.
10. **Unavailable / restart.** With no inspector, the script handshakes and does not push (`NameHasOwner` false). After unload, `isScriptLoaded` is false and the inspector stops receiving. A second `run` produced a new session id and a fresh snapshot (sequence back to 1). A KWin restart drops runtime scripts; the package files remain until uninstall. The fish start path will not reload them by itself.

`kwin-interesting.log` has no error. Load is not BLOCKED.
