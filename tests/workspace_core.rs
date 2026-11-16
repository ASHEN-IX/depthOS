use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

use depthos_core::{domain::*, service::WorkspaceService, sqlite::SqliteStore};
use rusqlite::{Connection, params};
use tempfile::TempDir;
use uuid::Uuid;

fn database() -> (TempDir, WorkspaceService<SqliteStore>) {
    let temp = tempfile::tempdir().unwrap();
    let store = SqliteStore::open(&temp.path().join("test.sqlite3")).unwrap();
    (temp, WorkspaceService::new(store))
}

#[test]
fn builds_and_serializes_ordered_hierarchy_across_reopen() {
    let (temp, mut service) = database();
    assert!(service.tree().unwrap().worlds.is_empty());
    let world = service
        .create_world("  Work  ", Some("Notes".into()))
        .unwrap();
    let workspace = service
        .create_workspace(world, "Editor", None, Some("/missing/project".into()))
        .unwrap();
    let second_world = service
        .create_world("A name that sorts first", None)
        .unwrap();
    let desktop = service
        .create_desktop(workspace, "Development", Some("coding".into()))
        .unwrap();
    let tree = service.tree().unwrap();
    assert_eq!(tree.worlds[1].world.id, second_world);
    assert_eq!(tree.worlds[0].workspaces[0].desktops[0].id, desktop);
    assert_eq!(tree.worlds[0].world.name, "Work");
    let json = serde_json::to_value(&tree).unwrap();
    assert_eq!(json["worlds"][0]["world"]["id"], world.0.to_string());
    assert_eq!(serde_json::from_value::<Universe>(json).unwrap(), tree);
    drop(service);
    let path = temp.path().join("test.sqlite3");
    let first = WorkspaceService::new(SqliteStore::open(&path).unwrap());
    let second = WorkspaceService::new(SqliteStore::open(&path).unwrap());
    assert_eq!(first.tree().unwrap(), tree);
    assert_eq!(second.tree().unwrap(), tree);
    assert_eq!(
        first.workspaces(world).unwrap()[0]
            .root_directory
            .as_deref(),
        Some("/missing/project")
    );
}

#[test]
fn validates_names_metadata_references_and_scoped_duplicates() {
    let (_temp, mut service) = database();
    for invalid in ["", "  ", "tab\there", &"x".repeat(121)] {
        assert!(matches!(
            service.create_world(invalid, None),
            Err(Error::InvalidName)
        ));
    }
    let world = service.create_world("Work", None).unwrap();
    let second = service.create_world("Other", None).unwrap();
    assert!(matches!(
        service.create_world(" work ", None),
        Err(Error::Duplicate("world"))
    ));
    assert!(matches!(
        service.rename_world(second, "WORK"),
        Err(Error::Duplicate("world"))
    ));
    assert!(matches!(
        service.create_workspace(world, "Example", None, Some("../project".into())),
        Err(Error::InvalidMetadata(_))
    ));
    let missing = WorldId(Uuid::new_v4());
    assert!(matches!(
        service.create_workspace(missing, "Example", None, None),
        Err(Error::Missing("world"))
    ));
    assert!(matches!(
        service.workspaces(missing),
        Err(Error::Missing("world"))
    ));
    assert!(matches!(
        service.create_desktop(WorkspaceId(Uuid::new_v4()), "Editor", None),
        Err(Error::Missing("workspace"))
    ));
    let project = service
        .create_workspace(world, "Example", None, None)
        .unwrap();
    let project2 = service
        .create_workspace(second, "example", None, None)
        .unwrap();
    assert!(matches!(
        service.create_workspace(world, "EXAMPLE", None, None),
        Err(Error::Duplicate("workspace"))
    ));
    service.create_desktop(project, "Editor", None).unwrap();
    service.create_desktop(project2, "Editor", None).unwrap();
    assert!(matches!(
        service.create_desktop(project, "editor", None),
        Err(Error::Duplicate("logical desktop"))
    ));
}

#[test]
fn moving_workspace_is_atomic_and_keeps_children() {
    let (_temp, mut service) = database();
    let from = service.create_world("From", None).unwrap();
    let to = service.create_world("To", None).unwrap();
    let project = service
        .create_workspace(from, "Project", None, None)
        .unwrap();
    let collision = service.create_workspace(to, "Project", None, None).unwrap();
    let child = service.create_desktop(project, "Desktop", None).unwrap();
    let before = service.tree().unwrap();
    assert!(matches!(
        service.move_workspace(project, to),
        Err(Error::Duplicate("workspace"))
    ));
    assert_eq!(service.tree().unwrap(), before);
    service.move_workspace(project, from).unwrap();
    assert_eq!(service.tree().unwrap(), before);
    service.rename_workspace(collision, "Renamed").unwrap();
    service.move_workspace(project, to).unwrap();
    assert!(service.workspaces(from).unwrap().is_empty());
    let workspaces = service.workspaces(to).unwrap();
    assert_eq!(workspaces[1].id, project);
    assert_eq!(
        workspaces[1].created_at_ms,
        before.worlds[0].workspaces[0].workspace.created_at_ms
    );
    assert!(workspaces[1].updated_at_ms > workspaces[1].created_at_ms);
    assert_eq!(service.desktops(project).unwrap()[0].id, child);
}

#[test]
fn desktop_reorder_rejects_foreign_ids_and_rolls_back_partial_updates() {
    let (_temp, mut service) = database();
    let world = service.create_world("Work", None).unwrap();
    let first = service
        .create_workspace(world, "First", None, None)
        .unwrap();
    let second = service
        .create_workspace(world, "Second", None, None)
        .unwrap();
    let a = service.create_desktop(first, "A", None).unwrap();
    let b = service.create_desktop(first, "B", None).unwrap();
    let foreign = service.create_desktop(second, "A", None).unwrap();
    let before = service.tree().unwrap();
    for order in [vec![b, foreign], vec![a, a], vec![a]] {
        assert!(matches!(
            service.reorder_desktops(first, order),
            Err(Error::InvalidOrder)
        ));
        assert_eq!(service.tree().unwrap(), before);
    }
    service.reorder_desktops(first, vec![b, a]).unwrap();
    assert_eq!(
        service
            .desktops(first)
            .unwrap()
            .into_iter()
            .map(|d| d.id)
            .collect::<Vec<_>>(),
        [b, a]
    );
    assert_eq!(service.desktops(second).unwrap()[0].id, foreign);
}

#[test]
fn deletes_only_leaves_and_empty_parents() {
    let (_temp, mut service) = database();
    let world = service.create_world("Work", None).unwrap();
    let project = service
        .create_workspace(world, "Project", None, None)
        .unwrap();
    let desktop = service.create_desktop(project, "Desktop", None).unwrap();
    assert!(matches!(
        service.delete_world(world),
        Err(Error::HasChildren("world"))
    ));
    assert!(matches!(
        service.delete_workspace(project),
        Err(Error::HasChildren("workspace"))
    ));
    service.rename_desktop(desktop, "Development").unwrap();
    service.rename_world(world, "Projects").unwrap();
    assert_eq!(service.desktops(project).unwrap()[0].name, "Development");
    service.delete_desktop(desktop).unwrap();
    service.delete_workspace(project).unwrap();
    service.delete_world(world).unwrap();
    assert!(matches!(
        service.delete_world(world),
        Err(Error::Missing("world"))
    ));
    assert!(service.worlds().unwrap().is_empty());
}

#[test]
fn sqlite_enforces_foreign_keys_and_unique_ids_and_reports_schema_version() {
    let (temp, mut service) = database();
    let world = service.create_world("Work", None).unwrap();
    let project = service
        .create_workspace(world, "Project", None, None)
        .unwrap();
    let path = temp.path().join("test.sqlite3");
    let raw = Connection::open(&path).unwrap();
    raw.pragma_update(None, "foreign_keys", true).unwrap();
    assert!(
        raw.execute("DELETE FROM worlds WHERE id=?1", [world.0])
            .is_err()
    );
    assert!(
        raw.execute(
            "UPDATE workspaces SET world_id=?1 WHERE id=?2",
            params![Uuid::new_v4(), project.0]
        )
        .is_err()
    );
    assert!(
        raw.execute(
            "INSERT INTO worlds(id,name,sort_order) VALUES(?1,'Other',1)",
            [world.0]
        )
        .is_err()
    );
    let report = SqliteStore::open(&path).unwrap().doctor().unwrap();
    assert!(report.foreign_keys);
    assert_eq!(report.schema_version, 1);
    assert_eq!(report.foreign_key_violations, 0);
    assert_eq!(report.integrity, ["ok"]);
    raw.pragma_update(None, "user_version", 99).unwrap();
    assert!(matches!(
        SqliteStore::open(&path),
        Err(Error::SchemaVersion(99))
    ));
}

#[test]
fn rejects_foreign_database_unsafe_permissions_and_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("foreign.sqlite3");
    let raw = Connection::open(&path).unwrap();
    raw.execute("CREATE TABLE existing (value TEXT)", [])
        .unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(matches!(
        SqliteStore::open(&path),
        Err(Error::ForeignDatabase)
    ));
    assert_eq!(
        raw.pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        0
    );
    let link = temp.path().join("link");
    symlink(&path, &link).unwrap();
    assert!(matches!(
        SqliteStore::open(&link),
        Err(Error::StoragePath(_))
    ));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        SqliteStore::open(&path),
        Err(Error::StoragePath(_))
    ));
}
