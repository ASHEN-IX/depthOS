//! Phase 4.1.1 first-run persistence tests (DEF-10).
//!
//! Every test runs against a private temporary directory used as both `HOME`
//! and `XDG_DATA_HOME`; the user's real `~/.local/share/depthos` is never read
//! or written.

use depthos_core::sqlite::init::{
    DbState, InitOptions, InitOutcome, InitStage, initialize_with_hook, probe,
};
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::{OpenOptionsExt, PermissionsExt, symlink},
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn cli(at: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_depthos"))
        .current_dir(at)
        .env_clear()
        .env("HOME", at.join("empty-home"))
        .env("XDG_DATA_HOME", at.join("data"))
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent/depthos-test",
        )
        .args(args)
        .output()
        .unwrap()
}

fn ok_json(result: Output) -> Value {
    assert!(
        result.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

fn default_db(at: &Path) -> PathBuf {
    at.join("data/depthos/workspaces.sqlite3")
}

fn status(at: &Path, db: Option<&Path>) -> Value {
    let mut args: Vec<&str> = vec![];
    let db_string;
    if let Some(db) = db {
        db_string = db.to_str().unwrap().to_owned();
        args.extend(["--db", &db_string]);
    }
    args.extend(["status", "--json"]);
    ok_json(cli(at, &args))
}

fn init(at: &Path, db: Option<&Path>, seed: bool) -> Output {
    let mut args: Vec<String> = vec![];
    if let Some(db) = db {
        args.extend(["--db".into(), db.to_str().unwrap().into()]);
    }
    args.push("init".into());
    if seed {
        args.push("--default-hierarchy".into());
    }
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    cli(at, &refs)
}

fn private_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

fn zero_byte_file(path: &Path, mode: u32) {
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(path)
        .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn counts(value: &Value) -> (u64, u64, u64) {
    let c = &value["counts"];
    (
        c["worlds"].as_u64().unwrap(),
        c["workspaces"].as_u64().unwrap(),
        c["desktops"].as_u64().unwrap(),
    )
}

#[test]
fn status_of_missing_default_database_is_read_only() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let report = status(at, None);
    assert_eq!(report["state"], "missing");
    assert_eq!(report["exists"], false);
    assert_eq!(report["path"], default_db(at).to_str().unwrap());
    assert!(report["schemaVersion"].is_null());
    assert!(report["counts"].is_null());
    assert!(
        !at.join("data").exists(),
        "status must not create the data dir"
    );
    assert!(!at.join("empty-home").exists());
}

#[test]
fn tree_never_creates_or_migrates_a_database() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let result = cli(at, &["tree"]);
    assert!(
        !result.status.success(),
        "tree on a missing default DB must fail"
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("depthos init"));
    assert!(
        !at.join("data").exists(),
        "tree must not create the data dir"
    );

    let explicit = at.join("absent.sqlite3");
    let result = cli(at, &["--db", explicit.to_str().unwrap(), "tree"]);
    assert!(!result.status.success());
    assert!(!explicit.exists(), "tree must not create an explicit DB");

    // A 0-byte leftover must stay 0 bytes: reading must not migrate it.
    zero_byte_file(&explicit, 0o600);
    let result = cli(at, &["--db", explicit.to_str().unwrap(), "tree"]);
    assert!(!result.status.success());
    assert_eq!(fs::metadata(&explicit).unwrap().len(), 0);
}

#[test]
fn read_commands_do_not_create_a_missing_database() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    for args in [
        &["doctor"][..],
        &["world", "list"],
        &[
            "workspace",
            "list",
            "--world",
            "00000000-0000-0000-0000-000000000000",
        ],
        &[
            "desktop",
            "list",
            "--workspace",
            "00000000-0000-0000-0000-000000000000",
        ],
        &["--db", "absent.sqlite3", "doctor"],
        &["--db", "absent.sqlite3", "world", "list"],
    ] {
        let result = cli(at, args);
        assert!(
            !result.status.success(),
            "{args:?} created or succeeded: {}",
            String::from_utf8_lossy(&result.stderr)
        );
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(
            stderr.contains("depthos init"),
            "{args:?} stderr did not point at init: {stderr}"
        );
    }
    assert!(!at.join("data").exists(), "a read created the data dir");
    assert!(
        !at.join("absent.sqlite3").exists(),
        "a read created an explicit DB"
    );
}

#[test]
fn init_creates_private_default_hierarchy_atomically() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let created = ok_json(init(at, None, true));
    assert_eq!(created["outcome"], "created");
    assert_eq!(created["state"], "valid");
    assert_eq!(created["seeded"], true);
    assert_eq!(counts(&created), (1, 1, 1));

    let db = default_db(at);
    let dir = db.parent().unwrap();
    assert_eq!(
        fs::metadata(dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&db).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(entries(dir), ["workspaces.sqlite3"], "no temp residue");

    let tree = ok_json(cli(at, &["tree"]));
    let world = &tree["worlds"][0];
    assert_eq!(world["world"]["name"], "Default World");
    assert_eq!(
        world["workspaces"][0]["workspace"]["name"],
        "Primary Workspace"
    );
    assert!(world["workspaces"][0]["workspace"]["root_directory"].is_null());
    assert_eq!(
        world["workspaces"][0]["desktops"].as_array().unwrap().len(),
        1
    );

    let after = status(at, None);
    assert_eq!(after["state"], "valid");
    assert_eq!(after["schemaVersion"], 1);
    assert_eq!(counts(&after), (1, 1, 1));
    assert!(!at.join("empty-home").exists());
}

#[test]
fn init_without_default_hierarchy_creates_empty_schema_only() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let created = ok_json(init(at, None, false));
    assert_eq!(created["outcome"], "created");
    assert_eq!(created["seeded"], false);
    assert_eq!(counts(&created), (0, 0, 0));
    assert_eq!(status(at, None)["state"], "valid");
}

#[test]
fn repeated_init_is_idempotent_and_does_not_touch_the_file() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    ok_json(init(at, None, true));
    let db = default_db(at);
    let bytes = fs::read(&db).unwrap();
    let mtime = fs::metadata(&db).unwrap().modified().unwrap();
    for _ in 0..3 {
        let again = ok_json(init(at, None, true));
        assert_eq!(again["outcome"], "already-initialized");
        assert_eq!(again["seeded"], false);
        assert_eq!(counts(&again), (1, 1, 1));
    }
    assert_eq!(fs::read(&db).unwrap(), bytes, "bytes must be identical");
    assert_eq!(fs::metadata(&db).unwrap().modified().unwrap(), mtime);
}

#[test]
fn init_never_overwrites_or_extends_an_existing_user_database() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let db = default_db(at);
    private_dir(db.parent().unwrap());
    let d = db.to_str().unwrap();
    ok_json(cli(at, &["--db", d, "world", "create", "Mine"]));
    let bytes = fs::read(&db).unwrap();

    let result = ok_json(init(at, None, true));
    assert_eq!(result["outcome"], "already-initialized");
    assert_eq!(fs::read(&db).unwrap(), bytes);
    let tree = ok_json(cli(at, &["tree"]));
    assert_eq!(tree["worlds"].as_array().unwrap().len(), 1);
    assert_eq!(tree["worlds"][0]["world"]["name"], "Mine");
}

#[test]
fn init_seeds_a_valid_but_empty_database_exactly_once() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    ok_json(init(at, None, false));
    let seeded = ok_json(init(at, None, true));
    assert_eq!(seeded["outcome"], "seeded-existing");
    assert_eq!(counts(&seeded), (1, 1, 1));
    let again = ok_json(init(at, None, true));
    assert_eq!(again["outcome"], "already-initialized");
    assert_eq!(counts(&again), (1, 1, 1));
}

#[test]
fn zero_byte_leftover_is_initialized_in_place_non_destructively() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let db = default_db(at);
    private_dir(db.parent().unwrap());
    zero_byte_file(&db, 0o600);
    let report = status(at, None);
    assert_eq!(report["state"], "empty");
    assert_eq!(report["exists"], true);

    let created = ok_json(init(at, None, true));
    assert_eq!(created["outcome"], "initialized-in-place");
    assert_eq!(counts(&created), (1, 1, 1));
    assert_eq!(status(at, None)["state"], "valid");
    assert_eq!(
        fs::metadata(&db).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn unsafe_permissions_and_links_are_refused_without_modification() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("explicit");
    private_dir(&dir);

    // World-readable 0-byte file.
    let open_mode = dir.join("open.sqlite3");
    zero_byte_file(&open_mode, 0o644);
    assert_eq!(status(at, Some(&open_mode))["state"], "unsafe-permissions");
    let refused = init(at, Some(&open_mode), true);
    assert_eq!(refused.status.code(), Some(3));
    assert_eq!(fs::metadata(&open_mode).unwrap().len(), 0);
    assert_eq!(
        fs::metadata(&open_mode).unwrap().permissions().mode() & 0o777,
        0o644,
        "init must not chmod"
    );

    // Symlinks (dangling and live) are refused and never followed.
    let target = dir.join("target.sqlite3");
    let dangling = dir.join("dangling.sqlite3");
    symlink(&target, &dangling).unwrap();
    assert_eq!(status(at, Some(&dangling))["state"], "symlink");
    assert_eq!(init(at, Some(&dangling), true).status.code(), Some(3));
    assert!(!target.exists(), "symlink target must not be created");

    ok_json(init(at, Some(&target), true));
    let live = dir.join("live.sqlite3");
    symlink(&target, &live).unwrap();
    assert_eq!(status(at, Some(&live))["state"], "symlink");
    let before = fs::read(&target).unwrap();
    assert_eq!(init(at, Some(&live), true).status.code(), Some(3));
    assert_eq!(fs::read(&target).unwrap(), before);

    // Hardlinked database.
    let linked = dir.join("linked.sqlite3");
    fs::hard_link(&target, &linked).unwrap();
    assert_eq!(status(at, Some(&linked))["state"], "hardlink");
    assert_eq!(init(at, Some(&linked), true).status.code(), Some(3));
    assert_eq!(fs::read(&target).unwrap(), before);

    // Directory in place of the database.
    let directory = dir.join("is-a-dir.sqlite3");
    private_dir(&directory);
    assert_eq!(status(at, Some(&directory))["state"], "foreign");
    assert_eq!(init(at, Some(&directory), true).status.code(), Some(3));
}

#[test]
fn foreign_corrupt_and_future_schema_databases_are_refused_byte_for_byte() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("explicit");
    private_dir(&dir);

    let foreign = dir.join("foreign.sqlite3");
    {
        let c = rusqlite::Connection::open(&foreign).unwrap();
        c.execute_batch("CREATE TABLE notes(id INTEGER); INSERT INTO notes VALUES (1);")
            .unwrap();
    }
    fs::set_permissions(&foreign, fs::Permissions::from_mode(0o600)).unwrap();

    let corrupt = dir.join("corrupt.sqlite3");
    fs::write(
        &corrupt,
        b"this is definitely not a sqlite database\n".repeat(64),
    )
    .unwrap();
    fs::set_permissions(&corrupt, fs::Permissions::from_mode(0o600)).unwrap();

    let future = dir.join("future.sqlite3");
    {
        let c = rusqlite::Connection::open(&future).unwrap();
        c.execute_batch(
            "PRAGMA application_id = 1146115144; PRAGMA user_version = 99;
             CREATE TABLE from_the_future(x);",
        )
        .unwrap();
    }
    fs::set_permissions(&future, fs::Permissions::from_mode(0o600)).unwrap();

    for (path, state) in [
        (&foreign, "foreign"),
        (&corrupt, "corrupt"),
        (&future, "unsupported-schema"),
    ] {
        let before = fs::read(path).unwrap();
        let report = status(at, Some(path));
        assert_eq!(report["state"], state, "{path:?}");
        let refused = init(at, Some(path), true);
        assert_eq!(refused.status.code(), Some(3), "{path:?}");
        assert_eq!(fs::read(path).unwrap(), before, "{path:?} modified");
        assert_eq!(entries(&dir).len(), 3, "no temp residue next to {path:?}");
        let tree = cli(at, &["--db", path.to_str().unwrap(), "tree"]);
        assert!(!tree.status.success(), "tree must reject {path:?}");
        assert_eq!(fs::read(path).unwrap(), before, "{path:?} modified by tree");
    }
    let report = status(at, Some(&future));
    assert_eq!(report["schemaVersion"], 99);
    let message = String::from_utf8_lossy(&init(at, Some(&future), true).stderr).into_owned();
    assert!(
        message.contains("99"),
        "schema error must name the version: {message}"
    );
}

#[test]
fn inaccessible_directory_reports_permission_denied() {
    if rustix::process::geteuid().is_root() {
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("locked");
    private_dir(&dir);
    let db = dir.join("db.sqlite3");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o000)).unwrap();
    let report = status(at, Some(&db));
    let refused = init(at, Some(&db), true);
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(report["state"], "permission-denied");
    assert!(!refused.status.success());
    assert!(entries(&dir).is_empty());
}

#[test]
fn explicit_db_requires_an_existing_parent_directory() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let db = at.join("missing-parent/db.sqlite3");
    assert_eq!(status(at, Some(&db))["state"], "missing");
    assert!(!init(at, Some(&db), true).status.success());
    assert!(!at.join("missing-parent").exists());
}

#[test]
fn existing_default_directory_with_open_permissions_is_not_chmodded() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("data/depthos");
    fs::create_dir_all(&dir).unwrap();
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!init(at, None, true).status.success());
    assert_eq!(
        fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert!(entries(&dir).is_empty());
}

#[test]
fn concurrent_initialization_creates_exactly_one_default_hierarchy() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let outputs: Vec<Output> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| init(at, None, true)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut created = 0;
    for output in outputs {
        let value = ok_json(output);
        if value["outcome"] == "created" {
            created += 1;
        } else {
            assert_eq!(value["outcome"], "already-initialized");
        }
        assert_eq!(counts(&value), (1, 1, 1));
    }
    assert_eq!(created, 1, "exactly one process may publish the database");
    let dir = default_db(at).parent().unwrap().to_owned();
    assert_eq!(entries(&dir), ["workspaces.sqlite3"]);
    let tree = ok_json(cli(at, &["tree"]));
    assert_eq!(tree["worlds"].as_array().unwrap().len(), 1);
}

#[test]
fn concurrent_initialization_of_a_zero_byte_leftover_seeds_once() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let db = default_db(at);
    private_dir(db.parent().unwrap());
    zero_byte_file(&db, 0o600);
    let outputs: Vec<Output> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| scope.spawn(|| init(at, None, true)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut seeded = 0;
    for output in outputs {
        let value = ok_json(output);
        if value["outcome"] == "initialized-in-place" {
            seeded += 1;
        }
        assert_eq!(counts(&value), (1, 1, 1));
    }
    assert_eq!(seeded, 1);
    assert_eq!(
        ok_json(cli(at, &["tree"]))["worlds"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn stale_temp_directory_from_a_killed_init_is_inert_and_cleaned() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = default_db(at).parent().unwrap().to_owned();
    private_dir(&dir);
    // Simulate `kill -9` between temp-DB creation and publication.
    let stale = dir.join(".depthos-init-4194999-deadbeef");
    private_dir(&stale);
    fs::write(stale.join("init.sqlite3"), b"partial").unwrap();
    fs::File::open(&stale)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
        .unwrap();
    assert_eq!(
        status(at, None)["state"],
        "missing",
        "temp residue is not a database"
    );

    let created = ok_json(init(at, None, true));
    assert_eq!(created["outcome"], "created");
    assert_eq!(
        entries(&dir),
        ["workspaces.sqlite3"],
        "stale temp dir removed"
    );

    // A *live* peer's fresh temp dir must be left alone.
    let live = dir.join(format!(".depthos-init-{}-cafe", std::process::id()));
    private_dir(&live);
    ok_json(init(at, None, true));
    assert!(live.exists());
}

#[test]
fn interrupted_init_before_publish_leaves_no_database_and_no_residue() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("explicit");
    private_dir(&dir);
    let db = dir.join("db.sqlite3");
    let options = InitOptions {
        default_hierarchy: true,
    };
    for stage in [InitStage::BeforeCommit, InitStage::BeforePublish] {
        let failed = initialize_with_hook(&db, options, &|at| {
            if at == stage {
                Err(depthos_core::domain::Error::InitRefused(
                    "simulated crash".into(),
                ))
            } else {
                Ok(())
            }
        });
        assert!(failed.is_err());
        assert!(!db.exists(), "{stage:?}: destination must not appear");
        assert!(entries(&dir).is_empty(), "{stage:?}: temp residue left");
        assert_eq!(probe(&db).unwrap().state, DbState::Missing);
    }
    let report = initialize_with_hook(&db, options, &|_| Ok(())).unwrap();
    assert_eq!(report.outcome, InitOutcome::Created);
}

#[test]
fn failed_in_place_initialization_rolls_back_to_the_original_zero_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("explicit");
    private_dir(&dir);
    let db = dir.join("db.sqlite3");
    zero_byte_file(&db, 0o600);
    let options = InitOptions {
        default_hierarchy: true,
    };
    let failed = initialize_with_hook(&db, options, &|stage| {
        if stage == InitStage::BeforeCommit {
            Err(depthos_core::domain::Error::InitRefused(
                "simulated crash".into(),
            ))
        } else {
            Ok(())
        }
    });
    assert!(failed.is_err());
    assert_eq!(
        fs::metadata(&db).unwrap().len(),
        0,
        "rolled back to 0 bytes"
    );
    assert_eq!(probe(&db).unwrap().state, DbState::Empty);
    assert_eq!(entries(&dir), ["db.sqlite3"], "no journal residue");
    let report = initialize_with_hook(&db, options, &|_| Ok(())).unwrap();
    assert_eq!(report.outcome, InitOutcome::InitializedInPlace);
    assert_eq!(report.counts.unwrap().worlds, 1);
}

#[test]
fn other_commands_still_create_a_database_exactly_as_before() {
    // Backwards compatibility: mutating commands keep their lazy-create path.
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let created = ok_json(cli(at, &["world", "create", "Legacy"]));
    assert!(created["id"].is_string());
    assert_eq!(status(at, None)["state"], "valid");
    let init_again = ok_json(init(at, None, true));
    assert_eq!(init_again["outcome"], "already-initialized");
    assert_eq!(counts(&init_again), (1, 0, 0));
}

#[test]
fn losing_the_publication_race_never_clobbers_the_winner() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("explicit");
    private_dir(&dir);
    let db = dir.join("db.sqlite3");
    let options = InitOptions {
        default_hierarchy: true,
    };
    let winner_bytes = std::cell::RefCell::new(Vec::new());
    // A competitor publishes between our temp build and our rename.
    let report = initialize_with_hook(&db, options, &|stage| {
        if stage == InitStage::BeforePublish {
            let winner = depthos_core::sqlite::init::initialize(&db, options)?;
            assert_eq!(winner.outcome, InitOutcome::Created);
            *winner_bytes.borrow_mut() = fs::read(&db).unwrap();
        }
        Ok(())
    })
    .unwrap();
    assert_eq!(report.outcome, InitOutcome::AlreadyInitialized);
    assert_eq!(report.counts.unwrap().worlds, 1);
    assert_eq!(
        fs::read(&db).unwrap(),
        *winner_bytes.borrow(),
        "winner clobbered"
    );
    assert_eq!(entries(&dir), ["db.sqlite3"]);
}

#[test]
fn a_symlink_planted_before_publication_is_not_replaced_or_followed() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let dir = at.join("explicit");
    private_dir(&dir);
    let db = dir.join("db.sqlite3");
    let victim = dir.join("victim");
    fs::write(&victim, b"precious").unwrap();
    let options = InitOptions {
        default_hierarchy: true,
    };
    let result = initialize_with_hook(&db, options, &|stage| {
        if stage == InitStage::BeforePublish {
            symlink(&victim, &db).unwrap();
        }
        Ok(())
    });
    assert!(matches!(
        result,
        Err(depthos_core::domain::Error::InitRefused(_))
    ));
    assert_eq!(fs::read(&victim).unwrap(), b"precious");
    assert!(fs::symlink_metadata(&db).unwrap().file_type().is_symlink());
    assert_eq!(entries(&dir), ["db.sqlite3", "victim"]);
}
