#!/usr/bin/env fish
# Explicit manual integration. NEVER call /Scripting.start(): it affects other scripts.
if test (count $argv) -ne 1; or not contains -- "$argv[1]" --start --stop
    printf 'Usage: fish scripts/observer-session.fish --start|--stop\nRequires explicit approval. Starts/stops only DepthOS observer session IDs.\nStart the foreground state inspector first. No configuration is persisted.\n'
    if test (count $argv) -eq 1; and test "$argv[1]" = --help
        exit 0
    end
    exit 2
end
if not command -q qdbus6
    printf 'Missing qdbus6.\n' >&2
    exit 1
end
set -l package_id depthos-system-observer
# A disabled installed package is unloaded by KWin configuration reconciliation.
# Keep manual session ownership distinct; do not persist an enabled preference.
set -l observer_id depthos-system-observer-session
set -l any_loaded false
for owned_id in $observer_id $package_id
    set -l loaded (qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.isScriptLoaded "$owned_id")
    if test "$loaded" != true; and test "$loaded" != false
        printf 'Unknown KWin scripting state; aborting.\n' >&2
        exit 1
    end
    if test "$loaded" = true
        set any_loaded true
        if test "$argv[1]" = --stop
            set -l removed (qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.unloadScript "$owned_id")
            if test "$removed" != true
                printf 'Observer unload was not confirmed.\n' >&2
                exit 1
            end
        end
    end
end
if test "$argv[1]" = --stop
    printf 'DepthOS observer session IDs are unloaded.\n'
    exit 0
end
if test "$any_loaded" = true
    printf 'Observer already loaded; refusing duplicate initialization.\n' >&2
    exit 1
end
set -l supported_versions 'kwin 6.8.1' 'kwin 6.8.2' 'kwin 6.8.3'
set -l current_ver (kwin_wayland --version 2>/dev/null)
if not command -q kwin_wayland; or test "$XDG_SESSION_TYPE" != wayland; or not contains -- "$current_ver" $supported_versions
    printf 'Unsupported KWin version (%s) or session type (%s).\nOnly tested KWin Wayland versions are supported: 6.8.1, 6.8.2, 6.8.3.\n' "$current_ver" "$XDG_SESSION_TYPE" >&2
    exit 1
end
set -l available (qdbus6 org.freedesktop.DBus /org/freedesktop/DBus org.freedesktop.DBus.NameHasOwner org.depthos.StateInspector)
if test "$available" != true
    printf 'Start tools/state-inspector/main.py in a foreground terminal first.\n' >&2
    exit 1
end
set -l data_home "$HOME/.local/share"
if set -q XDG_DATA_HOME; and test -n "$XDG_DATA_HOME"
    set data_home "$XDG_DATA_HOME"
end
if not string match -q '/*' -- "$data_home"
    printf 'XDG_DATA_HOME must be absolute.\n' >&2
    exit 1
end
set -l source "$data_home/kwin/scripts/$package_id/contents/code/main.js"
if not test -f "$source"
    printf 'Observer package is not installed.\n' >&2
    exit 1
end
set -l script_id (qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.loadScript "$source" "$observer_id")
if not string match -qr '^[0-9]+$' -- "$script_id"
    printf 'KWin rejected observer script loading.\n' >&2
    exit 1
end
qdbus6 org.kde.KWin "/Scripting/Script$script_id" org.kde.kwin.Script.run
or begin
    qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.unloadScript "$observer_id"
    exit 1
end
printf 'Only the observer run request was sent. Confirm the inspector receives a native snapshot.\nRollback: fish scripts/observer-session.fish --stop\n'
