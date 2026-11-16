#!/usr/bin/env fish
# Offscreen/unit/private-bus tests only. Never installs or loads KWin code.
set -l repo_root (path resolve (path dirname (status filename))/..)
cd "$repo_root"
or exit 1
for script in scripts/*.fish
    fish --no-execute "$script"
    or exit 1
end
kpackagetool6 --type KWin/Script --show "$repo_root/experiments/kwin-observer"
or exit 1
node --check experiments/kwin-observer/contents/code/main.js
or exit 1
node --test tests/observer/test-observer.cjs
or exit 1
env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/observer -p 'test_*.py'
or exit 1
env PYTHONDONTWRITEBYTECODE=1 python3 tests/integration/check-observer-engine.py
or exit 1
env DEPTHOS_PRIVATE_BUS_TEST=1 PYTHONDONTWRITEBYTECODE=1 dbus-run-session -- python3 tests/integration/check-inspector-bus.py
or exit 1
printf 'Observer unit, QJSEngine, and isolated-bus tests passed. Live KWin validation is separate.\n'
