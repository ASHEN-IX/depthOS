use super::*;

#[test]
fn failed_migration_rolls_back_ddl_and_header() {
    let mut db = Connection::open_in_memory().unwrap();
    let failed = migrate(
        &mut db,
        &["CREATE TABLE partial (id INTEGER); INVALID SQL;"],
    );
    assert!(matches!(failed, Err(Error::Migration(_))));
    for pragma in ["application_id", "user_version"] {
        assert_eq!(
            db.pragma_query_value(None, pragma, |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    assert_eq!(
        db.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    migrate(&mut db, MIGRATIONS).unwrap();
}

#[test]
fn version_one_reader_observes_atomic_upgrade_and_rule_cascade() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("workspaces.sqlite3");
    let mut old = SqliteStore::open_writable(&path, true).unwrap();
    migrate(&mut old.connection, &MIGRATIONS[..1]).unwrap();
    let world = WorldId(Uuid::new_v4());
    let workspace = WorkspaceId(Uuid::new_v4());
    let other = WorkspaceId(Uuid::new_v4());
    let desktop = DesktopId(Uuid::new_v4());
    old.apply(Change::CreateWorld(world, "World".into(), None))
        .unwrap();
    for (id, name) in [(workspace, "Workspace"), (other, "Other")] {
        old.apply(Change::CreateWorkspace(id, world, name.into(), None, None))
            .unwrap();
    }
    old.apply(Change::CreateDesktop(
        desktop,
        workspace,
        "Desktop".into(),
        None,
    ))
    .unwrap();

    let reader = SqliteStore::open_read_only(&path).unwrap();
    assert_eq!(reader.doctor().unwrap().schema_version, 1);
    let native = Uuid::new_v4().to_string();
    assert!(
        SqliteStore::configure_association(
            &path,
            RuleKind::NativeDesktop,
            &native,
            Some((other, Some(desktop))),
        )
        .is_err()
    );
    assert_eq!(
        reader.doctor().unwrap().schema_version,
        1,
        "failed write rolls the migration back"
    );
    let tree_before = reader.tree().unwrap();
    for _ in 0..2 {
        SqliteStore::configure_association(
            &path,
            RuleKind::NativeDesktop,
            &native,
            Some((workspace, Some(desktop))),
        )
        .unwrap();
    }
    let (tree, rules) = reader.association_view().unwrap();
    assert_eq!(tree, tree_before);
    assert_eq!(reader.doctor().unwrap().schema_version, 2);
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].desktop, Some(desktop));
    old.apply(Change::DeleteDesktop(desktop)).unwrap();
    assert!(reader.association_view().unwrap().1.is_empty());
}

#[test]
fn association_configuration_does_not_create_or_open_foreign_databases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing");
    assert!(
        SqliteStore::configure_association(&path, RuleKind::Application, "test.app", None).is_err()
    );
    assert!(!path.exists());
    let db = SqliteStore::open_writable(&path, true).unwrap();
    db.connection
        .execute("CREATE TABLE unrelated (id INTEGER)", [])
        .unwrap();
    drop(db);
    let before = fs::read(&path).unwrap();
    assert!(matches!(
        SqliteStore::configure_association(&path, RuleKind::Application, "test.app", None),
        Err(Error::ForeignDatabase)
    ));
    assert_eq!(before, fs::read(&path).unwrap());
}
