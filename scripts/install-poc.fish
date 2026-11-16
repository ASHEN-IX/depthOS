#!/usr/bin/env fish
# Explicit --install is required; no upgrade, enabling, loading, or config edits.
if test (count $argv) -ne 1; or test "$argv[1]" != --install
    printf 'Usage: fish scripts/install-poc.fish --install\nInstalls only the user-local DepthOS package. See README.md before running.\n'
    if test (count $argv) -eq 1; and test "$argv[1]" = --help
        exit 0
    end
    exit 2
end
if test (id -u) -eq 0
    printf 'Refusing to install as root.\n' >&2
    exit 1
end
for tool in kwin_wayland kpackagetool6 qdbus6
    if not command -q $tool
        printf 'Missing required tool: %s\n' $tool >&2
        exit 1
    end
end
if test "$XDG_SESSION_TYPE" != wayland; or test (kwin_wayland --version) != 'kwin 6.8.1'
    printf 'Refusing installation outside the audited KWin 6.8.1 Wayland target.\n' >&2
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
set -l effect_id depthos-spatial-poc
set -l package_root "$data_home/kwin/effects"
set -l repo_root (path resolve (path dirname (status filename))/..)
set -l package "$repo_root/experiments/kwin-effect"
if test -e "$package_root/$effect_id"; or test -L "$package_root/$effect_id"
    printf 'DepthOS already exists; refusing to overwrite or upgrade it. Disable and remove it first.\n' >&2
    exit 1
end
set -l available (qdbus6 org.kde.kglobalaccel /kglobalaccel org.kde.KGlobalAccel.isGlobalShortcutAvailable 385876026 '')
if test "$available" != true
    printf 'Shortcut is occupied or availability could not be checked. Nothing installed.\nResolve the conflict in System Settings without replacing another action, then retry.\n' >&2
    exit 1
end
set -l package_info (env LC_ALL=C kpackagetool6 --type KWin/Effect --show "$package")
or exit 1
if not string match --quiet --regex '^\s*Plugin\s*:\s*depthos-spatial-poc\s*$' -- $package_info
    printf 'Source package identity differs from depthos-spatial-poc; refusing installation.\n' >&2
    exit 1
end
printf '%s\n' $package_info
kpackagetool6 --type KWin/Effect --packageroot "$package_root" --install "$package"
or exit 1
printf '\nInstalled DepthOS only. It has NOT been enabled or loaded.\nEnable DepthOS Spatial POC in System Settings > Desktop Effects when ready.\nConfigure Toggle DepthOS Spatial POC under System Settings > Shortcuts > KWin.\nSee README.md for session-only loading, testing, and rollback.\n'
