#!/usr/bin/env fish
# Development-only physical-key bypass. Invoke ONE existing KGlobalAccel action.
# Does not load/enable the effect, register keys, or alter configuration.
if test (count $argv) -ne 1; or test "$argv[1]" != --toggle
    printf 'Usage: fish scripts/trigger-poc.fish --toggle\nToggles the loaded DepthOS overlay through KGlobalAccel; affects the active session.\nRequires explicit approval before use. No effect is loaded automatically.\n'
    if test (count $argv) -eq 1; and test "$argv[1]" = --help
        exit 0
    end
    exit 2
end
if not command -q qdbus6
    printf 'Missing qdbus6.\n' >&2
    exit 1
end
set -l loaded (qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded depthos-spatial-poc)
if test "$loaded" != true
    printf 'DepthOS is not loaded or its state is unknown. Nothing triggered.\n' >&2
    exit 1
end
set -l names (qdbus6 org.kde.kglobalaccel /component/kwin org.kde.kglobalaccel.Component.shortcutNames)
or exit 1
if not contains -- depthos-spatial-poc-toggle $names
    printf 'DepthOS shortcut action is absent. Nothing triggered.\n' >&2
    exit 1
end
qdbus6 org.kde.kglobalaccel /component/kwin org.kde.kglobalaccel.Component.invokeShortcut depthos-spatial-poc-toggle
or exit 1
printf 'Invocation sent; confirm shortcut callback, delegate creation, animations, and rendering separately.\n'
