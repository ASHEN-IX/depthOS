#!/usr/bin/env fish
if test (count $argv) -ne 1; or test "$argv[1]" != --remove
    printf 'Usage: fish scripts/uninstall-observer.fish --remove\nRemoves only the unloaded user-local observer package; requires approval.\n'
    if test (count $argv) -eq 1; and test "$argv[1]" = --help
        exit 0
    end
    exit 2
end
if test (id -u) -eq 0
    printf 'Refusing root removal.\n' >&2
    exit 1
end
for tool in qdbus6 kpackagetool6
    if not command -q $tool
        printf 'Missing %s.\n' $tool >&2
        exit 1
    end
end
set -l data_home "$HOME/.local/share"
if set -q XDG_DATA_HOME; and test -n "$XDG_DATA_HOME"
    set data_home "$XDG_DATA_HOME"
end
if not string match -q '/*' -- "$data_home"
    printf 'XDG_DATA_HOME must be absolute.\n' >&2
    exit 1
end
set -l root "$data_home/kwin/scripts"
if test -L "$root/depthos-system-observer"
    printf 'Refusing a symlink package; inspect manually.\n' >&2
    exit 1
end
for owned_id in depthos-system-observer depthos-system-observer-session
    set -l loaded (qdbus6 org.kde.KWin /Scripting org.kde.kwin.Scripting.isScriptLoaded "$owned_id")
    if test "$loaded" != false
        printf 'Observer is loaded or state is unknown. Stop only DepthOS first.\n' >&2
        exit 1
    end
end
if not test -e "$root/depthos-system-observer"
    printf 'Observer package already absent.\n'
    exit 0
end
kpackagetool6 --type KWin/Script --packageroot "$root" --remove depthos-system-observer
or exit 1
printf 'Removed only depthos-system-observer. No KDE preferences were changed.\n'
