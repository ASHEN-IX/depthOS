#!/usr/bin/env fish
# Package installation only; no configuration edits or automatic script loading.
if test (count $argv) -ne 1; or test "$argv[1]" != --install
    printf 'Usage: fish scripts/install-observer.fish --install\nInstalls only depthos-system-observer locally. Requires approval; never loads it.\n'
    if test (count $argv) -eq 1; and test "$argv[1]" = --help
        exit 0
    end
    exit 2
end
if test (id -u) -eq 0
    printf 'Refusing root installation.\n' >&2
    exit 1
end
for tool in kpackagetool6 kwin_wayland
    if not command -q $tool
        printf 'Missing %s.\n' $tool >&2
        exit 1
    end
end
if test "$XDG_SESSION_TYPE" != wayland; or test (kwin_wayland --version) != 'kwin 6.8.1'
    printf 'Only KWin 6.8.1 Wayland is source-audited.\n' >&2
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
set -l root "$data_home/kwin/scripts"
set -l package (path resolve (path dirname (status filename))/../experiments/kwin-observer)
if test -e "$root/depthos-system-observer"; or test -L "$root/depthos-system-observer"
    printf 'Observer package already exists; refusing to overwrite or upgrade.\n' >&2
    exit 1
end
set -l info (env LC_ALL=C kpackagetool6 --type KWin/Script --show "$package")
or exit 1
if not string match -qr '^\s*Plugin\s*:\s*depthos-system-observer\s*$' -- $info
    printf 'Unexpected source package ID.\n' >&2
    exit 1
end
kpackagetool6 --type KWin/Script --packageroot "$root" --install "$package"
or exit 1
printf 'Installed observer only. No script loaded, shortcut registered, or preference changed.\nSee docs/poc-02a.md for the separately approved session lifecycle.\n'
