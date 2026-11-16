#!/usr/bin/env fish
# Local/offscreen tests only; no installed packages or session configuration touched.
set -l repo_root (path resolve (path dirname (status filename))/..)
cd "$repo_root"
or exit 1
for script in scripts/*.fish
    fish --no-execute "$script"
    or exit 1
end
env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/overview -p 'test_*.py'
or exit 1
env PYTHONDONTWRITEBYTECODE=1 python3 tests/overview/validate.py
or exit 1
printf 'Overview local/offscreen checks passed. Native POC-02A and POC-02B acceptance remain separate.\n'
