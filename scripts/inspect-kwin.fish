#!/usr/bin/env fish
# Read-only: never loads effects, registers shortcuts, or changes configuration.
set -l effect_id depthos-spatial-poc
set -l data_home "$HOME/.local/share"
if set -q XDG_DATA_HOME; and test -n "$XDG_DATA_HOME"
    set data_home "$XDG_DATA_HOME"
end

printf 'DepthOS POC-01 compatibility audit (read-only)\n'
set -l failed 0
for tool in kwin_wayland plasmashell kpackagetool6 qdbus6
    if not command -q $tool
        printf 'Missing required tool: %s\n' $tool >&2
        set failed 1
    end
end
if test $failed -ne 0
    exit 1
end
kwin_wayland --version
plasmashell --version
printf 'Session: %s; desktop: %s\n' "$XDG_SESSION_TYPE" "$XDG_CURRENT_DESKTOP"
if test "$XDG_SESSION_TYPE" != wayland
    printf 'This POC targets Wayland only.\n' >&2
    set failed 1
end
set -l supported_versions 'kwin 6.8.1' 'kwin 6.8.2' 'kwin 6.8.3'
set -l current_ver (kwin_wayland --version 2>/dev/null)
if not contains -- "$current_ver" $supported_versions
    printf 'Version outside audited targets (%s). Only KWin 6.8.1, 6.8.2, 6.8.3 are audited; re-audit before installing.\n' "$current_ver" >&2
    set failed 1
end

printf '\nInstalled effect packages:\n'
kpackagetool6 --type KWin/Effect --list
or set failed 1
if test -e "$data_home/kwin/effects/$effect_id"
    printf 'DepthOS local package exists: %s\n' "$data_home/kwin/effects/$effect_id"
else
    printf 'DepthOS is not installed in the user package root.\n'
end
printf '\nLoaded effects (baseline for comparison):\n'
qdbus6 org.kde.KWin /Effects org.freedesktop.DBus.Properties.Get org.kde.kwin.Effects loadedEffects
or set failed 1
printf '\nDepthOS loaded (separate from visibility):\n'
qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded "$effect_id"
or set failed 1
printf '\nCurrently active effects (not all are fullscreen):\n'
qdbus6 org.kde.KWin /Effects org.freedesktop.DBus.Properties.Get org.kde.kwin.Effects activeEffects
or set failed 1
printf '\nDepthOS action in KGlobalAccel (may be retained after unloading):\n'
set -l shortcut_names (qdbus6 org.kde.kglobalaccel /component/kwin org.kde.kglobalaccel.Component.shortcutNames)
or set failed 1
if contains -- depthos-spatial-poc-toggle $shortcut_names
    printf 'depthos-spatial-poc-toggle is recorded; this alone does not prove a live callback.\n'
else
    printf 'DepthOS action is absent.\n'
end
printf '\nRegistered owner(s) of the preferred key:\n'
qdbus6 --literal org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel.getGlobalShortcutsByKey 385876026
or set failed 1
if command -q kreadconfig6
    printf '\nDepthOS persistent enabled preference (independent of session load):\n'
    kreadconfig6 --file kwinrc --group Plugins --key depthos-spatial-pocEnabled --default '<unset>'
end
printf '\nPreferred shortcut availability (true means free):\n'
# Qt::META | CTRL | SHIFT | Key_F11 = 0x1700003a = 385876026.
# Empty component checks all registered components, without an exemption.
qdbus6 org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel.isGlobalShortcutAvailable 385876026 ''
or set failed 1
printf '\nKWin D-Bus scripting interface:\n'
qdbus6 org.kde.KWin /Scripting
or set failed 1
printf '\nPOC-02A observer loaded (separate script; no automatic start):\n'
qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.isScriptLoaded depthos-system-observer-session
or set failed 1
printf '\nNo settings or packages were changed. See docs/compatibility.md.\n'
exit $failed
