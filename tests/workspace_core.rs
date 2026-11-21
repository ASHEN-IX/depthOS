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
    assert_eq!(report.schema_version, 3);
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

#[test]
fn phase_4_4_world_crud_reorder_archive_restore_and_concurrency() {
    let (_temp, mut service) = database();
    let w1 = service
        .create_world_full(
            "Engineering",
            Some("Dev projects".into()),
            Some("#12E6A0".into()),
            Some("projects".into()),
        )
        .unwrap();
    let w2 = service
        .create_world_full(
            "Research",
            Some("Science & reading".into()),
            Some("#8B5CF6".into()),
            Some("research".into()),
        )
        .unwrap();

    let tree = service.tree().unwrap();
    assert_eq!(tree.worlds.len(), 2);
    assert_eq!(
        tree.worlds[0].world.visual_identity.as_deref(),
        Some("#12E6A0")
    );
    assert_eq!(
        tree.worlds[0].world.artwork_identifier.as_deref(),
        Some("projects")
    );
    assert_eq!(tree.worlds[0].world.version, 1);

    // Concurrency conflict test
    assert!(matches!(
        service.update_world(
            w1,
            depthos_core::service::WorldUpdate {
                name: Some("Engineering Renamed".into()),
                ..Default::default()
            },
            Some(99) // Stale version
        ),
        Err(Error::Conflict {
            entity: "world",
            expected: 99,
            actual: 1
        })
    ));

    // Successful update with matching version
    service
        .update_world(
            w1,
            depthos_core::service::WorldUpdate {
                name: Some("Core Engineering".into()),
                description: Some(Some("Updated notes".into())),
                visual_identity: Some(Some("#22C7F2".into())),
                artwork_identifier: Some(Some("focus".into())),
            },
            Some(1),
        )
        .unwrap();

    let updated = service.worlds().unwrap();
    let e = updated.iter().find(|w| w.id == w1).unwrap();
    assert_eq!(e.name, "Core Engineering");
    assert_eq!(e.description.as_deref(), Some("Updated notes"));
    assert_eq!(e.visual_identity.as_deref(), Some("#22C7F2"));
    assert_eq!(e.version, 2);

    // Reorder worlds
    service.reorder_worlds(vec![w2, w1]).unwrap();
    let reordered = service.worlds().unwrap();
    assert_eq!(reordered[0].id, w2);
    assert_eq!(reordered[1].id, w1);

    // Archive world
    service.archive_world(w1, None).unwrap();
    assert!(matches!(
        service.archive_world(w1, None),
        Err(Error::AlreadyArchived("world"))
    ));

    // Active tree only shows w2
    assert_eq!(service.worlds().unwrap().len(), 1);
    assert_eq!(service.worlds().unwrap()[0].id, w2);

    // Archived items shows w1
    let archived = service.archived_items().unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].kind, "world");
    assert_eq!(archived[0].id, w1.0.to_string());
    assert_eq!(archived[0].name, "Core Engineering");

    // Restoring w1
    service.restore_world(w1, None).unwrap();
    assert!(matches!(
        service.restore_world(w1, None),
        Err(Error::NotArchived("world"))
    ));
    assert_eq!(service.worlds().unwrap().len(), 2);
    assert!(service.archived_items().unwrap().is_empty());
}

#[test]
fn phase_4_4_workspace_crud_reorder_archive_restore_and_concurrency() {
    let (_temp, mut service) = database();
    let world = service.create_world("Production", None).unwrap();
    let ws1 = service
        .create_workspace_full(
            world,
            "Backend",
            Some("Rust servers".into()),
            Some("/tmp".into()),
            Some("#12E6A0".into()),
        )
        .unwrap();
    let ws2 = service
        .create_workspace_full(
            world,
            "Frontend",
            Some("QML / Wayland UI".into()),
            Some("/tmp".into()),
            Some("#22C7F2".into()),
        )
        .unwrap();

    let dt = service.create_desktop(ws1, "Primary", None).unwrap();

    // Concurrency conflict test
    assert!(matches!(
        service.update_workspace(
            ws1,
            depthos_core::service::WorkspaceUpdate {
                name: Some("Backend API".into()),
                ..Default::default()
            },
            Some(42)
        ),
        Err(Error::Conflict {
            entity: "workspace",
            expected: 42,
            actual: 1
        })
    ));

    // Reorder workspaces
    service.reorder_workspaces(world, vec![ws2, ws1]).unwrap();
    let reordered = service.workspaces(world).unwrap();
    assert_eq!(reordered[0].id, ws2);
    assert_eq!(reordered[1].id, ws1);

    // Archive workspace
    service.archive_workspace(ws1, None).unwrap();
    assert_eq!(service.workspaces(world).unwrap().len(), 1);
    assert_eq!(service.workspaces(world).unwrap()[0].id, ws2);

    // Tree hides ws1 and its desktop dt
    let tree = service.tree().unwrap();
    assert_eq!(tree.worlds[0].workspaces.len(), 1);
    assert_eq!(tree.worlds[0].workspaces[0].workspace.id, ws2);

    // Archived items shows ws1 with child count 1
    let archived = service.archived_items().unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].kind, "workspace");
    assert_eq!(archived[0].id, ws1.0.to_string());
    assert_eq!(archived[0].child_count, 1);

    // Archive parent world, then attempt restoring child workspace -> refused
    service.archive_world(world, None).unwrap();
    assert!(matches!(
        service.restore_workspace(ws1, None),
        Err(Error::ParentArchived("world"))
    ));

    // Restore parent world, then restore workspace -> succeeds
    service.restore_world(world, None).unwrap();
    service.restore_workspace(ws1, None).unwrap();
    assert_eq!(service.workspaces(world).unwrap().len(), 2);
    assert_eq!(service.desktops(ws1).unwrap()[0].id, dt);
}

#[test]
fn phase_4_4_desktop_crud_reorder_archive_restore_and_concurrency() {
    let (_temp, mut service) = database();
    let world = service.create_world("Main", None).unwrap();
    let ws = service
        .create_workspace(world, "Project", None, None)
        .unwrap();
    let d1 = service
        .create_desktop_full(ws, "Code", Some("editor".into()), Some("#12E6A0".into()))
        .unwrap();
    let d2 = service
        .create_desktop_full(ws, "Docs", Some("reference".into()), Some("#8B5CF6".into()))
        .unwrap();

    // Concurrency conflict
    assert!(matches!(
        service.update_desktop(
            d1,
            depthos_core::service::DesktopUpdate {
                name: Some("Editor".into()),
                ..Default::default()
            },
            Some(5)
        ),
        Err(Error::Conflict {
            entity: "logical desktop",
            expected: 5,
            actual: 1
        })
    ));

    // Update
    service
        .update_desktop(
            d1,
            depthos_core::service::DesktopUpdate {
                name: Some("IDE".into()),
                role: Some(Some("development".into())),
                visual_identity: Some(Some("#22C7F2".into())),
            },
            Some(1),
        )
        .unwrap();

    let desktops = service.desktops(ws).unwrap();
    let ide = desktops.iter().find(|d| d.id == d1).unwrap();
    assert_eq!(ide.name, "IDE");
    assert_eq!(ide.role.as_deref(), Some("development"));
    assert_eq!(ide.version, 2);

    // Reorder
    service.reorder_desktops(ws, vec![d2, d1]).unwrap();
    let reordered = service.desktops(ws).unwrap();
    assert_eq!(reordered[0].id, d2);
    assert_eq!(reordered[1].id, d1);

    // Archive desktop
    service.archive_desktop(d1, None).unwrap();
    assert_eq!(service.desktops(ws).unwrap().len(), 1);
    assert_eq!(service.desktops(ws).unwrap()[0].id, d2);

    // Archived items
    let archived = service.archived_items().unwrap();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].kind, "desktop");
    assert_eq!(archived[0].id, d1.0.to_string());

    // Restore desktop
    service.restore_desktop(d1, None).unwrap();
    assert_eq!(service.desktops(ws).unwrap().len(), 2);
    assert!(service.archived_items().unwrap().is_empty());
}

#[test]
fn phase_4_4_preferences_persistence_and_user_defaults() {
    let (temp, mut service) = database();
    let world = service.create_world("TestWorld", None).unwrap();
    let ws = service
        .create_workspace(world, "TestWorkspace", None, None)
        .unwrap();
    let dt = service.create_desktop(ws, "TestDesktop", None).unwrap();

    let prefs = depthos_core::domain::UserPreferences {
        reduced_motion: true,
        low_effects: true,
        high_contrast: true,
        show_titles: false,
        accent_intensity: "vibrant".into(),
        default_landing_view: "workspaces".into(),
        last_world_id: Some(world),
        last_workspace_id: Some(ws),
        last_desktop_id: Some(dt),
    };

    service.save_user_preferences(&prefs).unwrap();
    drop(service);

    let path = temp.path().join("test.sqlite3");
    let reopened = WorkspaceService::new(SqliteStore::open(&path).unwrap());
    let loaded = reopened.get_user_preferences().unwrap();
    assert!(loaded.reduced_motion);
    assert!(loaded.low_effects);
    assert!(loaded.high_contrast);
    assert!(!loaded.show_titles);
    assert_eq!(loaded.accent_intensity, "vibrant");
    assert_eq!(loaded.default_landing_view, "workspaces");
    assert_eq!(loaded.last_world_id, Some(world));
    assert_eq!(loaded.last_workspace_id, Some(ws));
    assert_eq!(loaded.last_desktop_id, Some(dt));

    let all_prefs = reopened.list_preferences().unwrap();
    assert!(all_prefs.len() >= 6);
}

#[test]
fn phase_4_4_project_root_inspection_and_git_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let project_dir = temp.path().join("my-project");
    fs::create_dir(&project_dir).unwrap();

    // Inspection of plain directory without git
    let insp = depthos_core::service::inspect_project_directory(project_dir.to_str().unwrap());
    assert_eq!(insp.status, "valid");
    assert!(insp.exists);
    assert!(insp.is_directory);
    assert!(insp.git.is_none());

    // Create a mock .git directory with HEAD pointing to refs/heads/feature-phase-4-4
    let git_dir = project_dir.join(".git");
    fs::create_dir(&git_dir).unwrap();
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature-phase-4-4\n").unwrap();
    let refs_heads = git_dir.join("refs/heads");
    fs::create_dir_all(&refs_heads).unwrap();
    fs::write(
        refs_heads.join("feature-phase-4-4"),
        "61b56a4e4b335958139c591ff0dd6bd293a1962e\n",
    )
    .unwrap();

    let insp_git = depthos_core::service::inspect_project_directory(project_dir.to_str().unwrap());
    assert!(insp_git.git.is_some());
    let git = insp_git.git.unwrap();
    assert_eq!(git.branch.as_deref(), Some("feature-phase-4-4"));
    assert_eq!(git.commit.as_deref(), Some("61b56a4"));
    assert!(!git.is_detached);

    // Nonexistent directory
    let missing = depthos_core::service::inspect_project_directory("/nonexistent/path/xyz");
    assert_eq!(missing.status, "nonexistent");
    assert!(!missing.exists);

    // Relative path
    let relative = depthos_core::service::inspect_project_directory("relative/path");
    assert_eq!(relative.status, "not_absolute");
}
