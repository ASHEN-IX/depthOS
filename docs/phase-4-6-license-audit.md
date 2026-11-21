# DepthOS Phase 4.6 Licensing and Redistribution Audit

## 1. Executive Summary

A comprehensive intellectual property and licensing audit was performed across all DepthOS Phase 4.6 source components, compiled binaries, scripts, and runtime dependencies. The purpose of this audit is to verify compliance with all third-party open-source obligations, establish proper notice documentation (`NOTICES.md`), and delineate the boundaries of the DepthOS Private Alpha evaluation license.

### Audit Verdict
- **DepthOS Core**: Governed by the DepthOS Private Alpha Evaluation Agreement (`pkg/LICENSE`).
- **Rust Dependencies**: 113 audited crates. All third-party dependencies are licensed under standard permissive open-source licenses (MIT, Apache-2.0, Unicode-3.0, Zlib, Unlicense). Zero copyleft contamination exists in compiled native Rust artifacts.
- **Python / GUI Runtime**: PyQt6 is governed by GPL-3.0 or commercial license. Host bridge source code is distributed uncompiled alongside the package. Dynamic linkage to host Qt 6 libraries satisfies LGPL-3.0 obligations.
- **Visual Assets & Artwork**: 100% original procedurally generated vector art and static textures created for DepthOS.

---

## 2. Rust Dependency Breakdown

The 113 Rust workspace dependencies resolved in `Cargo.lock` were audited via `cargo metadata --format-version 1`:

| License Category (SPDX) | Package Count | Representative Packages | Compliance Requirement |
|---|---|---|---|
| `MIT OR Apache-2.0` | 65 | `serde`, `clap`, `thiserror`, `anstyle` | Notice attribution in `NOTICES.md` |
| `MIT` | 25 | `tokio`, `zbus`, `rusqlite`, `mio`, `bytes` | Notice attribution in `NOTICES.md` |
| `Apache-2.0 OR MIT` | 11 | `autocfg`, `fastrand`, `event-listener` | Notice attribution in `NOTICES.md` |
| `Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT` | 3 | `rustix`, `linux-raw-sys`, `wasi` | Notice attribution in `NOTICES.md` |
| `MIT/Apache-2.0` | 3 | `fallible-iterator`, `fallible-streaming-iterator`, `vcpkg` | Notice attribution in `NOTICES.md` |
| `(MIT OR Apache-2.0) AND Unicode-3.0` | 1 | `unicode-ident` | Unicode notice attribution |
| `Zlib` | 1 | `foldhash` | Zlib notice attribution |
| `Unlicense OR MIT` | 1 | `memchr` | Public domain / permissive notice |
| `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | 1 | `r-efi` | Dual-licensed (MIT/Apache selected) |
| `Proprietary / Internal` | 2 | `depthos-core`, `depthos-observer` | DepthOS Private Alpha terms |

---

## 3. Runtime & GUI Framework Analysis

### PyQt6 (Riverbank Computing Limited)
- **License**: GNU General Public License v3 (GPL-3.0-only) or Commercial.
- **Analysis**: The DepthOS spatial launcher (`tools/depthos-spatial-alpha`) is an interpreted Python script communicating with Qt 6 via PyQt6 bindings. In accordance with GPL-3.0 terms, all Python launcher scripts, QML assets, and host bridge logic are distributed in plain, human-readable source format in the distribution archive.

### Qt 6 Declarative / Qt Quick (The Qt Company)
- **License**: GNU Lesser General Public License v3 (LGPL-3.0-only) / Commercial.
- **Analysis**: DepthOS does not bundle or modify shared Qt 6 runtime libraries (`libQt6Quick.so`, `libQt6Qml.so`). The application dynamically links to the host distribution's packages, satisfying LGPL-3.0 user replacement and dynamic linking requirements.

### SQLite (SQLite Development Team)
- **Status**: Public Domain / SQLite Blessing ("May you do good and not evil...").
- **Analysis**: SQLite is linked through `rusqlite` / `libsqlite3-sys` with zero licensing restrictions.

---

## 4. Media and Visual Assets Audit

- **Application Icon (`pkg/depthos.svg`)**: Original 512x512 vector illustration authored directly in SVG.
- **Stage Backgrounds (`experiments/spatial-alpha/art/`)**: Procedural static textures rendered by `experiments/spatial-alpha/art/generate-atmosphere.py`. No third-party stock photography or copyrighted models are utilized.
- **Typography**: No proprietary font binaries are packaged; scene rendering uses host fontconfig system fonts.

---

## 5. Compliance Checklist

- [x] Full dependency inventory audited via `cargo metadata`.
- [x] All 113 Rust crates verified for non-copyleft compatibility.
- [x] `NOTICES.md` authored with complete attribution tables and copyright notices.
- [x] `pkg/LICENSE` authored reflecting the DepthOS Private Alpha terms.
- [x] Source code for all interpreted Python and QML files included in distribution package.
- [x] Zero embedded credentials, telemetry endpoints, or private keys confirmed.
