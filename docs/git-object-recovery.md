# Git object recovery

**GIT OBJECT RECOVERY PARTIALLY ACCEPTED.** All 12 authorized staged objects were repaired. The 171-path index verifies against the original approved candidate. Repository-wide verification still reports unrelated corruption.

## Diagnosis and evidence

The three worktrees share canonical .git. Git uses SHA-1 objects. There were zero packs and no alternates. Available space was approximately 363 GiB. The 12 failures were readable, zero-byte loose files with 0o444, UID 1000, GID 1000. Their length and Git's read errors establish truncation. The cause was outside the evidence. [Before diagnostics](../artifacts/git-object-recovery-rTmZ/diagnostics.json) · [Before fsck](../artifacts/git-object-recovery-rTmZ/fsck-before.txt)

| Staged path | Expected object ID | Readability |
| --- | --- | --- |
| experiments/kwin-observer/contents/code/main.js | a96a9e5ba68a9ff84295b24fcccae8c1a1ac2ca6 | Unreadable → verified blob |
| experiments/kwin-observer/metadata.json | ba9f4ecbc9285e6861fbcb64d89a2b883c9bb363 | Unreadable → verified blob |
| scripts/install-observer.fish | 5caca3fadd72774c1718f0a5a29fae6186d82a79 | Unreadable → verified blob |
| scripts/validate-observer.fish | 7dc95b9ca55aa459d78d12f93e3c46bd1161e4ab | Unreadable → verified blob |
| tests/integration/check-inspector-bus.py | c51e81bc752f97e4f400f147b0e61b269a81b391 | Unreadable → verified blob |
| tests/integration/check-observer-engine.py | 6ea5104bdd41666a4acffdaf6cae614e8699b77d | Unreadable → verified blob |
| tests/observer/benchmark.cjs | 6ea04e606459856e0a3beb2ba177e675164e0887 | Unreadable → verified blob |
| tests/observer/fixtures.cjs | 0442f917c0e79184bc760d029df0049b447ef746 | Unreadable → verified blob |
| tests/observer/test-observer.cjs | 100e6fd0ecc9009ea4bf311faf57d3728c7a438e | Unreadable → verified blob |
| tests/observer/test_replica.py | 172b537a5cd7ea2a7a7c3eb429f44c904c95c79c | Unreadable → verified blob |
| tools/state-inspector/main.py | 5380fabab4ad88f007a4713e670daff3394b63c7 | Unreadable → verified blob |
| tools/state-inspector/state.py | 2ded05bcd0516e59132582c7328369d913a4d51b | Unreadable → verified blob |

## Recovery procedure

For each failed object, the script read the approved backup file under /home/ashenix/Projects/DepthOS-backups/phase31-preflight-09x0a1co/source, checked its SHA-256 digest and independently computed the Git blob hash from its header and contents. All matched the index. It wrote the replacement with git hash-object -t blob --no-filters -w in a separate object directory and verified its type and exact contents there. The original zero-byte files were copied into original-loose/ in the evidence directory before repair.

The script checked for active repository Git processes, writable index or object descriptors and an index lock before repair. It rechecked each original file's device, inode, size, timestamp and checksum, removed the verified empty file, and hard-linked the verified replacement into the vacant object path with no overwrite. It immediately read the installed blob through Git and compared it with its backup. [Recovery script](../artifacts/git-object-recovery-rTmZ/recover.py)

## Index outcome and remaining blockers

All 171 staged blobs are readable and match the expected object IDs and approved SHA-256 digests. The index bytes, staged paths, approved working-tree source, candidate manifest and linked-worktree registrations match their pre-repair state. [Verification result](../artifacts/git-object-recovery-rTmZ/result.json)

The full git fsck --full --no-reflogs check still exits 11. The before and after logs identify 21 other zero-byte loose objects, missing tree references and an invalid Codex checkpoint ref. These were outside the authorized repair scope. [Remaining object and ref diagnostics](../artifacts/git-object-recovery-rTmZ/fsck-after.txt)

The staged whitespace check still reports trailing whitespace at [docs/phase-2.md](phase-2.md), line 15. No correction was applied. Canonical main remains unborn. This report is untracked and outside the approved 171-path candidate. [Whitespace check](../artifacts/git-object-recovery-rTmZ/whitespace.txt)
