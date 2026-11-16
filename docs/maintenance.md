# Maintaining DepthOS

Use this checkout for active development. The root Cargo workspace contains the application and the observer crate. The source in `src/`, `crates/`, `migrations/` and `experiments/` is authoritative for current implementation. Keep fixtures and validation scripts under `tests/`, `scripts/` and `tools/` so they travel with the source.

## Verify a checkout

Run from the repository root.

```sh
git status --short
git fsck --full --strict
cargo test --locked --workspace
```

The application needs SQLite 3.42 or newer. An external `CARGO_TARGET_DIR` keeps build output outside the source tree. Python creates `__pycache__` directories during imports. Cargo targets and Python bytecode can be regenerated. Before clearing them, stop tasks using the affected paths and preserve binaries required by historical evidence.

## Local artifacts and historical evidence

`artifacts/` is ignored by Git. Local acceptance and development tools use it for templates, run state and outputs. Preserve the files used by an active validation. Treat recordings, logs, window snapshots, database copies, and host configuration as private. Record shareable conclusions in `docs/` and keep raw material in restricted storage.

The local archive created during the November 2026 cleanup is `.depthos-archive` beside this repository. Its `historical/` directory contains the earlier source snapshots, independent accepted repository, original performance and security trees, and recovery records. Its `pre-cleanup/` directory contains a checked copy of the project and all historical trees as they existed before cleanup, including generated files and local artifacts. The archived performance and security gitfiles refer to the superseded common Git directory. Use their files and manifests as historical source evidence.

The [Git recovery report](git-object-recovery.md) and earlier qualification reports describe the conditions at the time they were written. The canonical checkout now has independent Git metadata and a committed `main`. This resolves the earlier local Git initialization blocker. The native authorization and evidence gates in the [Phase 3.1 preflight](phase-3-native-consent.md) still apply.
