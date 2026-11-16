#!/usr/bin/env fish
# Remove one fixed user-local package; never implicitly disable or unload effects.
if test (count $argv) -ne 1; or test "$argv[1]" != --remove
    printf 'Usage: fish scripts/uninstall-poc.fish --remove\nFirst disable and unload DepthOS using README.md. Only its package is removed.\n'
    if test (count $argv) -eq 1; and test "$argv[1]" = --help
        exit 0
    end
    exit 2
end
if test (id -u) -eq 0
    printf 'Refusing to remove packages as root.\n' >&2
    exit 1
end
for tool in kpackagetool6 qdbus6
    if not command -q $tool
        printf 'Missing required tool: %s\n' $tool >&2
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
set -l effect_id depthos-spatial-poc
set -l package_root "$data_home/kwin/effects"
if test -L "$package_root/$effect_id"
    printf 'Refusing a symlink package. Inspect it manually.\n' >&2
    exit 1
end
if not test -e "$package_root/$effect_id"
    printf 'DepthOS is already absent from the user package root.\n'
    exit 0
end
set -l loaded (qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded "$effect_id")
if test "$loaded" != false
    printf 'DepthOS is loaded, or its state is unknown. Disable it in Desktop Effects and unload it first; see README.md.\n' >&2
    exit 1
end
kpackagetool6 --type KWin/Effect --packageroot "$package_root" --remove "$effect_id"
or exit 1
printf 'Removed only the DepthOS user-local package.\nKDE may retain its shortcut and disabled-state preference; see README.md.\n'
