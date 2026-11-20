use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

fn cli(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_depthos"))
        .current_dir(directory)
        .env_clear()
        .env("HOME", directory.join("empty-home"))
        .env("XDG_DATA_HOME", directory.join("isolated-data"))
        .env(
            "DBUS_SESSION_BUS_ADDRESS",
            "unix:path=/nonexistent/depthos-test",
        )
        .args(args)
        .output()
        .unwrap()
}

fn output(result: Output) -> Value {
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
fn hierarchy_persists_across_processes_with_no_desktop_session() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let adversarial_name = "$(touch escaped); INSERT INTO worlds VALUES ('injected');";
    let world = output(cli(
        at,
        &["--db", "test.db", "world", "create", adversarial_name],
    ));
    let world = world["id"].as_str().unwrap();
    let workspace = output(cli(
        at,
        &[
            "workspace",
            "create",
            "--db",
            "test.db",
            "--world",
            world,
            "--name",
            "Core",
        ],
    ));
    let workspace = workspace["id"].as_str().unwrap();
    let desktop = output(cli(
        at,
        &[
            "--db",
            "test.db",
            "desktop",
            "create",
            "--workspace",
            workspace,
            "--name",
            "Edit",
        ],
    ));
    let tree = output(cli(at, &["--db", "test.db", "tree"]));
    assert_eq!(tree["worlds"][0]["world"]["name"], adversarial_name);
    assert_eq!(
        tree["worlds"][0]["workspaces"][0]["desktops"][0]["id"],
        desktop["id"]
    );
    assert_eq!(
        output(cli(at, &["--db", "test.db", "world", "list"]))[0]["id"],
        world
    );
    assert_eq!(
        output(cli(
            at,
            &["--db", "test.db", "workspace", "list", "--world", world]
        ))[0]["id"],
        workspace
    );
    assert_eq!(
        output(cli(
            at,
            &[
                "--db",
                "test.db",
                "desktop",
                "list",
                "--workspace",
                workspace
            ]
        ))[0]["id"],
        desktop["id"]
    );
    let doctor = output(cli(at, &["--db", "test.db", "doctor"]));
    assert_eq!(doctor["healthy"], true);
    assert_eq!(doctor["database"]["schema_version"], 2);
    assert_eq!(
        fs::metadata(at.join("test.db"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(fs::read_dir(at).unwrap().count(), 1);
}

#[test]
fn rejects_invalid_cli_input_and_missing_parent_without_populating_hierarchy() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    for args in [
        vec!["workspace", "list", "--world", "invalid-uuid"],
        vec!["desktop", "create", "--name", "missing-workspace"],
    ] {
        assert_eq!(cli(at, &args).status.code(), Some(2));
    }
    assert_eq!(fs::read_dir(at).unwrap().count(), 0);
    let id = uuid::Uuid::new_v4().to_string();
    let missing = cli(
        at,
        &[
            "--db",
            "test.db",
            "workspace",
            "create",
            "--world",
            &id,
            "--name",
            "Invalid",
        ],
    );
    assert_eq!(missing.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing.stderr).contains("world does not exist"));
    assert_eq!(
        output(cli(at, &["--db", "test.db", "tree"]))["worlds"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn default_database_uses_only_isolated_xdg_data_and_private_permissions() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let created = output(cli(at, &["init"]));
    let dir = at.join("isolated-data/depthos");
    let db = dir.join("workspaces.sqlite3");
    assert_eq!(created["path"].as_str().unwrap(), db.to_str().unwrap());
    assert_eq!(
        fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(&db).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let doctor = output(cli(at, &["doctor"]));
    assert_eq!(doctor["healthy"], true);
    assert_eq!(doctor["path"].as_str().unwrap(), db.to_str().unwrap());
    assert!(!at.join("empty-home").exists());
}

#[test]
fn concurrent_first_open_migrates_once_and_preserves_uniqueness() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();
    let results = std::thread::scope(|scope| {
        let command = || cli(at, &["--db", "race.db", "world", "create", "Same"]);
        let first = scope.spawn(command);
        let second = scope.spawn(command);
        [first.join().unwrap(), second.join().unwrap()]
    });
    let mut codes = results.map(|result| result.status.code().unwrap());
    codes.sort();
    assert_eq!(codes, [0, 2]);
    assert_eq!(
        output(cli(at, &["--db", "race.db", "world", "list"]))
            .as_array()
            .unwrap()
            .len(),
        1
    );
}
