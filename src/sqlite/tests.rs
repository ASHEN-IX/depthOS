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
