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
    assert_eq!(doctor["database"]["schema_version"], 3);
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

#[test]
fn phase_4_4_cli_crud_archival_and_preferences() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();

    // 1. Create world with Phase 4.4 metadata
    let world = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "world",
            "create",
            "DevWorld",
            "--description",
            "Developer World",
            "--visual-identity",
            "neon-cyan",
            "--artwork",
            "cyber-grid",
        ],
    ));
    let world_id = world["id"].as_str().unwrap();

    // 2. Update world
    let _ = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "world",
            "update",
            world_id,
            "--description",
            "Updated Description",
            "--version",
            "1",
        ],
    ));

    // 3. Create workspace
    let ws = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "workspace",
            "create",
            "--world",
            world_id,
            "--name",
            "CoreWorkspace",
            "--root-directory",
            at.to_str().unwrap(),
        ],
    ));
    let ws_id = ws["id"].as_str().unwrap();

    // 4. Create desktop
    let dt = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "desktop",
            "create",
            "--workspace",
            ws_id,
            "--name",
            "Terminal",
            "--role",
            "dev",
        ],
    ));
    let dt_id = dt["id"].as_str().unwrap();

    // 5. Archive desktop
    let _ = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "desktop",
            "archive",
            dt_id,
            "--version",
            "1",
        ],
    ));
    let dt_active = output(cli(
        at,
        &["--db", "p44.db", "desktop", "list", "--workspace", ws_id],
    ));
    assert_eq!(dt_active.as_array().unwrap().len(), 0);

    let dt_archived = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "desktop",
            "list",
            "--workspace",
            ws_id,
            "--archived",
        ],
    ));
    assert_eq!(dt_archived.as_array().unwrap().len(), 1);

    // 6. Restore desktop
    let _ = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "desktop",
            "restore",
            dt_id,
            "--version",
            "2",
        ],
    ));
    let dt_restored = output(cli(
        at,
        &["--db", "p44.db", "desktop", "list", "--workspace", ws_id],
    ));
    assert_eq!(dt_restored.as_array().unwrap().len(), 1);

    // 7. Preferences CLI
    let _ = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "preferences",
            "set",
            "presentation.reduced_motion",
            "true",
        ],
    ));
    let pref = output(cli(
        at,
        &[
            "--db",
            "p44.db",
            "preferences",
            "get",
            "presentation.reduced_motion",
        ],
    ));
    assert_eq!(pref["value"], "true");

    let all_prefs = output(cli(at, &["--db", "p44.db", "preferences", "list"]));
    assert!(!all_prefs.as_array().unwrap().is_empty());

    // 8. Project inspection CLI
    let inspection = output(cli(at, &["project", "inspect", at.to_str().unwrap()]));
    assert_eq!(inspection["exists"], true);
    assert_eq!(inspection["isDirectory"], true);
    assert_eq!(inspection["isAbsolute"], true);
}

#[test]
fn phase_4_4_cli_manage_exec_and_conflict() {
    let temp = tempfile::tempdir().unwrap();
    let at = temp.path();

    // Initialize db
    let _ = output(cli(at, &["--db", "manage.db", "init"]));

    // Create World via manage exec
    let create_world_req = serde_json::json!({
        "operationId": "op-1",
        "action": "create",
        "entity": "world",
        "payload": {
            "name": "ManageWorld",
            "description": "Via JSON",
            "visualIdentity": "neon-amber",
            "artwork": "cyber-city"
        }
    });
    let world_resp = output(cli(
        at,
        &[
            "--db",
            "manage.db",
            "manage",
            "exec",
            "--request",
            &create_world_req.to_string(),
        ],
    ));
    assert_eq!(world_resp["success"], true);
    let world_id = world_resp["id"].as_str().unwrap();

    // Stale update (expectedVersion: 999) -> Conflict, exit code 4
    let stale_req = serde_json::json!({
        "operationId": "op-stale",
        "action": "update",
        "entity": "world",
        "id": world_id,
        "expectedVersion": 999,
        "payload": {
            "name": "StaleRename"
        }
    });
    let stale_out = cli(
        at,
        &[
            "--db",
            "manage.db",
            "manage",
            "exec",
            "--request",
            &stale_req.to_string(),
        ],
    );
    assert_eq!(stale_out.status.code(), Some(4));
    let stale_json: Value = serde_json::from_slice(&stale_out.stdout).unwrap();
    assert_eq!(stale_json["success"], false);
    assert_eq!(stale_json["code"], "conflict");
}
