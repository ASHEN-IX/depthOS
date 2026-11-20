use std::{
    collections::HashMap,
    env, fs,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

use rusqlite::{Connection, OpenFlags, Params, Row, Transaction, TransactionBehavior, params};
use uuid::Uuid;

use crate::{
    associations::{Rule, RuleKind},
    domain::*,
    service::{Change, Store},
};

const APP_ID: i64 = 0x44505448;
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/001_initial.sql"),
    include_str!("../../migrations/002_desktop_association_rules.sql"),
    include_str!("../../migrations/003_hierarchy_management_and_preferences.sql"),
];
const VERSION: i64 = MIGRATIONS.len() as i64;
const WORLD_COLUMNS: &str = "id,name,description,sort_order,created_at_ms,updated_at_ms,visual_identity,artwork_identifier,is_archived,archived_at_ms,version";
const WORKSPACE_COLUMNS: &str = "id,world_id,name,description,root_directory,sort_order,created_at_ms,updated_at_ms,visual_identity,is_archived,archived_at_ms,version";
const DESKTOP_COLUMNS: &str = "id,workspace_id,name,role,sort_order,created_at_ms,updated_at_ms,visual_identity,is_archived,archived_at_ms,version";
const UPDATED: &str =
    "updated_at_ms = max(updated_at_ms + 1, CAST(unixepoch('subsec') * 1000 AS INTEGER))";

pub struct SqliteStore {
    connection: Connection,
}

pub fn default_path() -> Result<PathBuf> {
    let base = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .map(|p| p.join(".local/share"))
        })
        .ok_or_else(|| {
            Error::StoragePath("set an absolute HOME or XDG_DATA_HOME, or use --db".into())
        })?;
    Ok(base.join("depthos/workspaces.sqlite3"))
}

fn path_error(path: &Path, error: impl std::fmt::Display) -> Error {
    Error::StoragePath(format!("{:?}: {error}", path))
}

impl SqliteStore {
    pub fn open_default() -> Result<Self> {
        let path = default_path()?;
        let directory = path.parent().expect("default path has parent");
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)
            .map_err(|e| path_error(directory, e))?;
        let metadata = fs::symlink_metadata(directory).map_err(|e| path_error(directory, e))?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(path_error(
                directory,
                "expected a private directory (mode 0700)",
            ));
        }
        Self::open(&path)
    }

    /// Explicit overrides require an existing, trusted parent directory.
    pub fn open(path: &Path) -> Result<Self> {
        let mut store = Self::open_writable(path, true)?;
        migrate(&mut store.connection, MIGRATIONS)?;
        Ok(store)
    }

    fn open_writable(path: &Path, create: bool) -> Result<Self> {
        if path.file_name().is_none() {
            return Err(path_error(path, "expected a database filename"));
        }
        if rusqlite::version_number() < 3_042_000 {
            return Err(Error::SqliteVersion(rusqlite::version()));
        }
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let parent_metadata = fs::metadata(parent).map_err(|e| path_error(parent, e))?;
        if !parent_metadata.is_dir() || parent_metadata.permissions().mode() & 0o022 != 0 {
            return Err(path_error(
                parent,
                "expected a directory writable only by its owner",
            ));
        }
        if create {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(path)
            {
                Ok(_) => (),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
                Err(e) => return Err(path_error(path, e)),
            }
        }
        let metadata = fs::symlink_metadata(path).map_err(|e| path_error(path, e))?;
        if !metadata.is_file()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(path_error(
                path,
                "expected a private regular file without links (mode 0600)",
            ));
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = Connection::open_with_flags(path, flags)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "trusted_schema", false)?;
        Ok(Self { connection })
    }

    /// Read-only probe: refuses to create, migrate or repair anything.
    pub fn open_read_only(path: &Path) -> Result<Self> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(path_error(
                    path,
                    "database does not exist; run `depthos init` to create it",
                ));
            }
            Err(e) => return Err(path_error(path, e)),
        };
        if !metadata.is_file()
            || metadata.uid() != rustix::process::getuid().as_raw()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(path_error(
                path,
                "expected a private regular file without links (mode 0600)",
            ));
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let connection = Connection::open_with_flags(path, flags)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "trusted_schema", false)?;
        let app: i64 = connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        match (app, version) {
            (APP_ID, 1..=VERSION) => Ok(Self { connection }),
            (APP_ID, _) => Err(Error::SchemaVersion(version)),
            _ => Err(Error::ForeignDatabase),
        }
    }

    /// Explicit configuration operation. Requires an existing DepthOS database.
    pub fn configure_association(
        path: &Path,
        kind: RuleKind,
        key: &str,
        target: Option<(WorkspaceId, Option<DesktopId>)>,
    ) -> Result<()> {
        let key = kind
            .key(key)
            .ok_or_else(|| Error::InvalidMetadata("association rule key"))?;
        drop(Self::open_read_only(path)?);
        let mut store = Self::open_writable(path, false)?;
        let tx = store
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        migrate_tx(&tx, MIGRATIONS)?;
        if let Some((workspace, desktop)) = target {
            require(&tx, Entity::Workspace, workspace.0)?;
            if let Some(desktop) = desktop {
                let valid: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM desktops WHERE workspace_id=?1 AND id=?2)",
                    params![workspace.0, desktop.0],
                    |row| row.get(0),
                )?;
                if !valid {
                    return Err(Error::InvalidMetadata(
                        "association Desktop must belong to the Workspace",
                    ));
                }
            }
            tx.execute(
                "INSERT INTO desktop_association_rules(kind,rule_key,workspace_id,desktop_id)
                 VALUES (?1,?2,?3,?4) ON CONFLICT(kind,rule_key) DO UPDATE SET
                 workspace_id=excluded.workspace_id,desktop_id=excluded.desktop_id",
                params![kind.as_str(), key, workspace.0, desktop.map(|id| id.0)],
            )?;
        } else {
            tx.execute(
                "DELETE FROM desktop_association_rules WHERE kind=?1 AND rule_key=?2",
                params![kind.as_str(), key],
            )?;
        }
        let count: i64 = tx.query_row(
            "SELECT count(*) FROM desktop_association_rules",
            [],
            |row| row.get(0),
        )?;
        if count > 4096 {
            return Err(Error::InvalidMetadata("association rule limit exceeded"));
        }
        tx.commit()?;
        Ok(())
    }

    pub(crate) fn association_view(&self) -> Result<(Universe, Vec<Rule>)> {
        let tx = self.connection.unchecked_transaction()?;
        let tree = read_tree(&tx)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        let rules = match version {
            1 => Vec::new(),
            2..=VERSION => rows(
                &tx,
                "SELECT kind,rule_key,workspace_id,desktop_id FROM desktop_association_rules LIMIT 4097",
                [],
                |row| {
                    let kind = match row.get_ref(0)?.as_str()? {
                        "native-desktop" => RuleKind::NativeDesktop,
                        "application" => RuleKind::Application,
                        _ => return Err(rusqlite::Error::InvalidQuery),
                    };
                    Ok(Rule {
                        kind,
                        key: row.get(1)?,
                        workspace: WorkspaceId(row.get(2)?),
                        desktop: row.get::<_, Option<Uuid>>(3)?.map(DesktopId),
                    })
                },
            )?,
            _ => return Err(Error::SchemaVersion(version)),
        };
        if rules.len() > 4096 {
            return Err(Error::InvalidMetadata("association rule limit exceeded"));
        }
        tx.commit()?;
        Ok((tree, rules))
    }

    pub fn doctor(&self) -> Result<Doctor> {
        let connection = self.connection.unchecked_transaction()?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let (world_cols, ws_cols, dt_cols) = if version < 3 {
            (
                "id,name,description,sort_order,created_at_ms,updated_at_ms",
                "id,world_id,name,description,root_directory,sort_order,created_at_ms,updated_at_ms",
                "id,workspace_id,name,role,sort_order,created_at_ms,updated_at_ms",
            )
        } else {
            (WORLD_COLUMNS, WORKSPACE_COLUMNS, DESKTOP_COLUMNS)
        };
        for (table, columns) in [
            ("worlds", world_cols),
            ("workspaces", ws_cols),
            ("desktops", dt_cols),
        ] {
            connection.prepare(&format!("SELECT {columns} FROM {table} LIMIT 0"))?;
        }
        Ok(Doctor {
            schema_version: version,
            sqlite_version: rusqlite::version(),
            foreign_keys: connection.pragma_query_value(None, "foreign_keys", |r| r.get(0))?,
            integrity: rows(&connection, "PRAGMA quick_check", [], |r| r.get(0))?,
            foreign_key_violations: rows(&connection, "PRAGMA foreign_key_check", [], |_| Ok(()))?
                .len(),
        })
    }
}

#[derive(serde::Serialize)]
pub struct Doctor {
    pub schema_version: i64,
    pub sqlite_version: &'static str,
    pub foreign_keys: bool,
    pub integrity: Vec<String>,
    pub foreign_key_violations: usize,
}

fn migrate(connection: &mut Connection, migrations: &[&str]) -> Result<()> {
    let tx = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(Error::Migration)?;
    migrate_tx(&tx, migrations)?;
    tx.commit().map_err(Error::Migration)
}

/// Applies pending migrations inside the caller's transaction so that
/// initialization can compose schema creation and seeding atomically.
fn migrate_tx(tx: &Transaction<'_>, migrations: &[&str]) -> Result<()> {
    let app: i64 = tx
        .pragma_query_value(None, "application_id", |r| r.get(0))
        .map_err(Error::Migration)?;
    let version: i64 = tx
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(Error::Migration)?;
    match (app, version) {
        (APP_ID, 1..=VERSION) => (),
        (APP_ID, _) => return Err(Error::SchemaVersion(version)),
        (0, 0) => {
            let objects: i64 = tx
                .query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))
                .map_err(Error::Migration)?;
            if objects != 0 {
                return Err(Error::ForeignDatabase);
            }
            tx.pragma_update(None, "application_id", APP_ID)
                .map_err(Error::Migration)?;
        }
        _ => return Err(Error::ForeignDatabase),
    }
    for (index, sql) in migrations.iter().enumerate().skip(version as usize) {
        tx.execute_batch(sql).map_err(Error::Migration)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)
            .map_err(Error::Migration)?;
    }
    Ok(())
}

fn rows<T>(
    db: &Connection,
    sql: &str,
    values: impl Params,
    decode: fn(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let mut statement = db.prepare_cached(sql)?;
    Ok(statement
        .query_map(values, decode)?
        .collect::<rusqlite::Result<_>>()?)
}

fn world(r: &Row<'_>) -> rusqlite::Result<World> {
    Ok(World {
        id: WorldId(r.get(0)?),
        name: r.get(1)?,
        description: r.get(2)?,
        sort_order: r.get(3)?,
        created_at_ms: r.get(4)?,
        updated_at_ms: r.get(5)?,
        visual_identity: r.get(6)?,
        artwork_identifier: r.get(7)?,
        is_archived: r.get::<_, i64>(8)? != 0,
        archived_at_ms: r.get(9)?,
        version: r.get(10)?,
    })
}

fn workspace(r: &Row<'_>) -> rusqlite::Result<Workspace> {
    Ok(Workspace {
        id: WorkspaceId(r.get(0)?),
        world_id: WorldId(r.get(1)?),
        name: r.get(2)?,
        description: r.get(3)?,
        root_directory: r.get(4)?,
        sort_order: r.get(5)?,
        created_at_ms: r.get(6)?,
        updated_at_ms: r.get(7)?,
        visual_identity: r.get(8)?,
        is_archived: r.get::<_, i64>(9)? != 0,
        archived_at_ms: r.get(10)?,
        version: r.get(11)?,
    })
}

fn desktop(r: &Row<'_>) -> rusqlite::Result<LogicalDesktop> {
    Ok(LogicalDesktop {
        id: DesktopId(r.get(0)?),
        workspace_id: WorkspaceId(r.get(1)?),
        name: r.get(2)?,
        role: r.get(3)?,
        sort_order: r.get(4)?,
        created_at_ms: r.get(5)?,
        updated_at_ms: r.get(6)?,
        visual_identity: r.get(7)?,
        is_archived: r.get::<_, i64>(8)? != 0,
        archived_at_ms: r.get(9)?,
        version: r.get(10)?,
    })
}

#[derive(Clone, Copy)]
enum Entity {
    World,
    Workspace,
    Desktop,
}

impl Entity {
    fn table(self) -> &'static str {
        match self {
            Self::World => "worlds",
            Self::Workspace => "workspaces",
            Self::Desktop => "desktops",
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Workspace => "workspace",
            Self::Desktop => "logical desktop",
        }
    }
}

fn require(db: &Connection, kind: Entity, id: Uuid) -> Result<()> {
    let exists: bool = db.query_row(
        &format!("SELECT EXISTS(SELECT 1 FROM {} WHERE id=?1)", kind.table()),
        [id],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(Error::Missing(kind.label()));
    }
    Ok(())
}

fn check_version(
    tx: &Transaction<'_>,
    kind: Entity,
    id: Uuid,
    expected: Option<i64>,
) -> Result<i64> {
    let current: i64 = tx
        .query_row(
            &format!("SELECT version FROM {} WHERE id=?1", kind.table()),
            [id],
            |r| r.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => Error::Missing(kind.label()),
            other => Error::Database(other),
        })?;
    if let Some(expected_ver) = expected
        && current != expected_ver
    {
        return Err(Error::Conflict {
            entity: kind.label(),
            expected: expected_ver,
            actual: current,
        });
    }
    Ok(current)
}

fn audit_record(
    tx: &Transaction<'_>,
    op_id: Option<&str>,
    entity_type: &str,
    entity_id: Option<&str>,
    action: &str,
    details: Option<&str>,
) -> Result<()> {
    let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version < 3 {
        return Ok(());
    }
    let id_buf;
    let final_id = match op_id {
        Some(s) if !s.is_empty() => s,
        _ => {
            id_buf = Uuid::new_v4().to_string();
            &id_buf
        }
    };
    tx.execute(
        "INSERT INTO operation_audit(operation_id, entity_type, entity_id, action, details)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(operation_id) DO NOTHING",
        params![final_id, entity_type, entity_id, action, details],
    )?;
    Ok(())
}

fn write(db: &Connection, sql: &str, values: impl Params, kind: Entity) -> Result<()> {
    if execute(db, sql, values, kind)? == 0 {
        return Err(Error::Missing(kind.label()));
    }
    Ok(())
}

fn execute(db: &Connection, sql: &str, values: impl Params, kind: Entity) -> Result<usize> {
    match db.execute(sql, values) {
        Err(e)
            if e.sqlite_error()
                .is_some_and(|e| e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE) =>
        {
            Err(Error::Duplicate(kind.label()))
        }
        result => Ok(result?),
    }
}

fn rename(db: &Connection, kind: Entity, id: Uuid, name: String) -> Result<()> {
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let sql = if version < 3 {
        format!("UPDATE {} SET name=?1, {UPDATED} WHERE id=?2", kind.table())
    } else {
        format!(
            "UPDATE {} SET name=?1, version=version+1, {UPDATED} WHERE id=?2",
            kind.table()
        )
    };
    write(db, &sql, params![name, id], kind)
}

fn delete(db: &Connection, kind: Entity, id: Uuid) -> Result<()> {
    let children = match kind {
        Entity::World => Some("SELECT EXISTS(SELECT 1 FROM workspaces WHERE world_id=?1)"),
        Entity::Workspace => Some("SELECT EXISTS(SELECT 1 FROM desktops WHERE workspace_id=?1)"),
        Entity::Desktop => None,
    };
    if let Some(sql) = children
        && db.query_row(sql, [id], |r| r.get::<_, bool>(0))?
    {
        return Err(Error::HasChildren(kind.label()));
    }
    write(
        db,
        &format!("DELETE FROM {} WHERE id=?1", kind.table()),
        [id],
        kind,
    )
}

impl Store for SqliteStore {
    fn apply(&mut self, change: Change) -> Result<()> {
        self.apply_with_audit(change, None)
    }

    fn apply_with_audit(&mut self, change: Change, op_id: Option<String>) -> Result<()> {
        use Entity::{Desktop, Workspace, World};

        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;

        match change {
            Change::CreateWorld(id, name, description) => {
                let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
                if version < 3 {
                    write(
                        &tx,
                        "INSERT INTO worlds(id,name,description,sort_order)
                         VALUES (?1,?2,?3,(SELECT coalesce(max(sort_order)+1,0) FROM worlds))",
                        params![id.0, name, description],
                        World,
                    )?;
                } else {
                    write(
                        &tx,
                        "INSERT INTO worlds(id,name,description,sort_order,visual_identity,artwork_identifier,is_archived,archived_at_ms,version)
                         VALUES (?1,?2,?3,(SELECT coalesce(max(sort_order)+1,0) FROM worlds WHERE is_archived=0),NULL,NULL,0,NULL,1)",
                        params![id.0, name, description],
                        World,
                    )?;
                }
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "world",
                    Some(&id.0.to_string()),
                    "create",
                    Some(&name),
                )?;
            }
            Change::CreateWorldFull {
                id,
                name,
                description,
                visual_identity,
                artwork_identifier,
            } => {
                write(
                    &tx,
                    "INSERT INTO worlds(id,name,description,sort_order,visual_identity,artwork_identifier,is_archived,archived_at_ms,version)
                     VALUES (?1,?2,?3,(SELECT coalesce(max(sort_order)+1,0) FROM worlds WHERE is_archived=0),?4,?5,0,NULL,1)",
                    params![id.0, name, description, visual_identity, artwork_identifier],
                    World,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "world",
                    Some(&id.0.to_string()),
                    "create",
                    Some(&name),
                )?;
            }
            Change::UpdateWorld {
                id,
                update,
                expected_version,
            } => {
                check_version(&tx, World, id.0, expected_version)?;
                let current_world: crate::domain::World = tx.query_row(
                    &format!("SELECT {WORLD_COLUMNS} FROM worlds WHERE id=?1"),
                    [id.0],
                    world,
                )?;
                let new_name = update.name.unwrap_or(current_world.name);
                let new_desc = match update.description {
                    Some(val) => val,
                    None => current_world.description,
                };
                let new_vis = match update.visual_identity {
                    Some(val) => val,
                    None => current_world.visual_identity,
                };
                let new_art = match update.artwork_identifier {
                    Some(val) => val,
                    None => current_world.artwork_identifier,
                };
                write(
                    &tx,
                    &format!(
                        "UPDATE worlds SET name=?1, description=?2, visual_identity=?3, artwork_identifier=?4, version=version+1, {UPDATED} WHERE id=?5"
                    ),
                    params![new_name, new_desc, new_vis, new_art, id.0],
                    World,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "world",
                    Some(&id.0.to_string()),
                    "update",
                    Some(&new_name),
                )?;
            }
            Change::ReorderWorlds(ids) => {
                let count: i64 = tx.query_row(
                    "SELECT count(*) FROM worlds WHERE is_archived = 0",
                    [],
                    |r| r.get(0),
                )?;
                if count as usize != ids.len()
                    || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
                {
                    return Err(Error::InvalidOrder);
                }
                let mut update = tx.prepare_cached(&format!(
                    "UPDATE worlds SET sort_order=?1, version=version+1, {UPDATED} WHERE id=?2 AND is_archived = 0"
                ))?;
                for (index, id) in ids.into_iter().enumerate() {
                    if update.execute(params![index as i64, id.0])? != 1 {
                        return Err(Error::InvalidOrder);
                    }
                }
                audit_record(&tx, op_id.as_deref(), "world", None, "reorder", None)?;
            }
            Change::ArchiveWorld {
                id,
                expected_version,
            } => {
                check_version(&tx, World, id.0, expected_version)?;
                let is_archived: i64 =
                    tx.query_row("SELECT is_archived FROM worlds WHERE id=?1", [id.0], |r| {
                        r.get(0)
                    })?;
                if is_archived != 0 {
                    return Err(Error::AlreadyArchived("world"));
                }
                write(
                    &tx,
                    &format!(
                        "UPDATE worlds SET is_archived=1, archived_at_ms=CAST(unixepoch('subsec')*1000 AS INTEGER), version=version+1, {UPDATED} WHERE id=?1"
                    ),
                    params![id.0],
                    World,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "world",
                    Some(&id.0.to_string()),
                    "archive",
                    None,
                )?;
            }
            Change::RestoreWorld {
                id,
                expected_version,
            } => {
                check_version(&tx, World, id.0, expected_version)?;
                let (is_archived, name): (i64, String) = tx.query_row(
                    "SELECT is_archived, name FROM worlds WHERE id=?1",
                    [id.0],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?;
                if is_archived == 0 {
                    return Err(Error::NotArchived("world"));
                }
                let dup: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM worlds WHERE is_archived=0 AND name=?1 COLLATE NOCASE)",
                    [&name],
                    |r| r.get(0),
                )?;
                if dup {
                    return Err(Error::Duplicate("world"));
                }
                write(
                    &tx,
                    &format!(
                        "UPDATE worlds SET is_archived=0, archived_at_ms=NULL, version=version+1, {UPDATED} WHERE id=?1"
                    ),
                    params![id.0],
                    World,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "world",
                    Some(&id.0.to_string()),
                    "restore",
                    Some(&name),
                )?;
            }

            Change::CreateWorkspace(id, parent, name, description, root) => {
                require(&tx, World, parent.0)?;
                let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
                if version < 3 {
                    write(
                        &tx,
                        "INSERT INTO workspaces(id,world_id,name,description,root_directory,sort_order)
                         VALUES (?1,?2,?3,?4,?5,(SELECT coalesce(max(sort_order)+1,0) FROM workspaces WHERE world_id=?2))",
                        params![id.0, parent.0, name, description, root],
                        Workspace,
                    )?;
                } else {
                    write(
                        &tx,
                        "INSERT INTO workspaces(id,world_id,name,description,root_directory,sort_order,visual_identity,is_archived,archived_at_ms,version)
                         VALUES (?1,?2,?3,?4,?5,(SELECT coalesce(max(sort_order)+1,0) FROM workspaces WHERE world_id=?2 AND is_archived=0),NULL,0,NULL,1)",
                        params![id.0, parent.0, name, description, root],
                        Workspace,
                    )?;
                }
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "create",
                    Some(&name),
                )?;
            }
            Change::CreateWorkspaceFull {
                id,
                world_id,
                name,
                description,
                root_directory,
                visual_identity,
            } => {
                require(&tx, World, world_id.0)?;
                write(
                    &tx,
                    "INSERT INTO workspaces(id,world_id,name,description,root_directory,sort_order,visual_identity,is_archived,archived_at_ms,version)
                     VALUES (?1,?2,?3,?4,?5,(SELECT coalesce(max(sort_order)+1,0) FROM workspaces WHERE world_id=?2 AND is_archived=0),?6,0,NULL,1)",
                    params![id.0, world_id.0, name, description, root_directory, visual_identity],
                    Workspace,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "create",
                    Some(&name),
                )?;
            }
            Change::UpdateWorkspace {
                id,
                update,
                expected_version,
            } => {
                check_version(&tx, Workspace, id.0, expected_version)?;
                let current_ws: crate::domain::Workspace = tx.query_row(
                    &format!("SELECT {WORKSPACE_COLUMNS} FROM workspaces WHERE id=?1"),
                    [id.0],
                    workspace,
                )?;
                let new_name = update.name.unwrap_or(current_ws.name);
                let new_desc = match update.description {
                    Some(val) => val,
                    None => current_ws.description,
                };
                let new_root = match update.root_directory {
                    Some(val) => val,
                    None => current_ws.root_directory,
                };
                let new_vis = match update.visual_identity {
                    Some(val) => val,
                    None => current_ws.visual_identity,
                };
                write(
                    &tx,
                    &format!(
                        "UPDATE workspaces SET name=?1, description=?2, root_directory=?3, visual_identity=?4, version=version+1, {UPDATED} WHERE id=?5"
                    ),
                    params![new_name, new_desc, new_root, new_vis, id.0],
                    Workspace,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "update",
                    Some(&new_name),
                )?;
            }
            Change::ReorderWorkspaces(world_id, ids) => {
                require(&tx, World, world_id.0)?;
                let count: i64 = tx.query_row(
                    "SELECT count(*) FROM workspaces WHERE world_id=?1 AND is_archived = 0",
                    [world_id.0],
                    |r| r.get(0),
                )?;
                if count as usize != ids.len()
                    || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
                {
                    return Err(Error::InvalidOrder);
                }
                let mut update = tx.prepare_cached(&format!(
                    "UPDATE workspaces SET sort_order=?1, version=version+1, {UPDATED} WHERE id=?2 AND world_id=?3 AND is_archived = 0"
                ))?;
                for (index, id) in ids.into_iter().enumerate() {
                    if update.execute(params![index as i64, id.0, world_id.0])? != 1 {
                        return Err(Error::InvalidOrder);
                    }
                }
                audit_record(&tx, op_id.as_deref(), "workspace", None, "reorder", None)?;
            }
            Change::ArchiveWorkspace {
                id,
                expected_version,
            } => {
                check_version(&tx, Workspace, id.0, expected_version)?;
                let is_archived: i64 = tx.query_row(
                    "SELECT is_archived FROM workspaces WHERE id=?1",
                    [id.0],
                    |r| r.get(0),
                )?;
                if is_archived != 0 {
                    return Err(Error::AlreadyArchived("workspace"));
                }
                write(
                    &tx,
                    &format!(
                        "UPDATE workspaces SET is_archived=1, archived_at_ms=CAST(unixepoch('subsec')*1000 AS INTEGER), version=version+1, {UPDATED} WHERE id=?1"
                    ),
                    params![id.0],
                    Workspace,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "archive",
                    None,
                )?;
            }
            Change::RestoreWorkspace {
                id,
                expected_version,
            } => {
                check_version(&tx, Workspace, id.0, expected_version)?;
                let (is_archived, world_id, name): (i64, Uuid, String) = tx.query_row(
                    "SELECT is_archived, world_id, name FROM workspaces WHERE id=?1",
                    [id.0],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )?;
                if is_archived == 0 {
                    return Err(Error::NotArchived("workspace"));
                }
                let parent_archived: i64 = tx.query_row(
                    "SELECT is_archived FROM worlds WHERE id=?1",
                    [world_id],
                    |r| r.get(0),
                )?;
                if parent_archived != 0 {
                    return Err(Error::ParentArchived("world"));
                }
                let dup: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM workspaces WHERE world_id=?1 AND is_archived=0 AND name=?2 COLLATE NOCASE)",
                    params![world_id, name],
                    |r| r.get(0),
                )?;
                if dup {
                    return Err(Error::Duplicate("workspace"));
                }
                write(
                    &tx,
                    &format!(
                        "UPDATE workspaces SET is_archived=0, archived_at_ms=NULL, version=version+1, {UPDATED} WHERE id=?1"
                    ),
                    params![id.0],
                    Workspace,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "restore",
                    Some(&name),
                )?;
            }

            Change::CreateDesktop(id, parent, name, role) => {
                require(&tx, Workspace, parent.0)?;
                let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
                if version < 3 {
                    write(
                        &tx,
                        "INSERT INTO desktops(id,workspace_id,name,role,sort_order)
                         VALUES (?1,?2,?3,?4,(SELECT coalesce(max(sort_order)+1,0) FROM desktops WHERE workspace_id=?2))",
                        params![id.0, parent.0, name, role],
                        Desktop,
                    )?;
                } else {
                    write(
                        &tx,
                        "INSERT INTO desktops(id,workspace_id,name,role,sort_order,visual_identity,is_archived,archived_at_ms,version)
                         VALUES (?1,?2,?3,?4,(SELECT coalesce(max(sort_order)+1,0) FROM desktops WHERE workspace_id=?2 AND is_archived=0),NULL,0,NULL,1)",
                        params![id.0, parent.0, name, role],
                        Desktop,
                    )?;
                }
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "desktop",
                    Some(&id.0.to_string()),
                    "create",
                    Some(&name),
                )?;
            }
            Change::CreateDesktopFull {
                id,
                workspace_id,
                name,
                role,
                visual_identity,
            } => {
                require(&tx, Workspace, workspace_id.0)?;
                write(
                    &tx,
                    "INSERT INTO desktops(id,workspace_id,name,role,sort_order,visual_identity,is_archived,archived_at_ms,version)
                     VALUES (?1,?2,?3,?4,(SELECT coalesce(max(sort_order)+1,0) FROM desktops WHERE workspace_id=?2 AND is_archived=0),?5,0,NULL,1)",
                    params![id.0, workspace_id.0, name, role, visual_identity],
                    Desktop,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "desktop",
                    Some(&id.0.to_string()),
                    "create",
                    Some(&name),
                )?;
            }
            Change::UpdateDesktop {
                id,
                update,
                expected_version,
            } => {
                check_version(&tx, Desktop, id.0, expected_version)?;
                let current_dt: LogicalDesktop = tx.query_row(
                    &format!("SELECT {DESKTOP_COLUMNS} FROM desktops WHERE id=?1"),
                    [id.0],
                    desktop,
                )?;
                let new_name = update.name.unwrap_or(current_dt.name);
                let new_role = match update.role {
                    Some(val) => val,
                    None => current_dt.role,
                };
                let new_vis = match update.visual_identity {
                    Some(val) => val,
                    None => current_dt.visual_identity,
                };
                write(
                    &tx,
                    &format!(
                        "UPDATE desktops SET name=?1, role=?2, visual_identity=?3, version=version+1, {UPDATED} WHERE id=?4"
                    ),
                    params![new_name, new_role, new_vis, id.0],
                    Desktop,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "desktop",
                    Some(&id.0.to_string()),
                    "update",
                    Some(&new_name),
                )?;
            }
            Change::ReorderDesktops(parent, ids) => {
                require(&tx, Workspace, parent.0)?;
                let count: i64 = tx.query_row(
                    "SELECT count(*) FROM desktops WHERE workspace_id=?1 AND is_archived = 0",
                    [parent.0],
                    |r| r.get(0),
                )?;
                if count as usize != ids.len()
                    || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
                {
                    return Err(Error::InvalidOrder);
                }
                let mut update = tx.prepare_cached(&format!(
                    "UPDATE desktops SET sort_order=?1, version=version+1, {UPDATED} WHERE id=?2 AND workspace_id=?3 AND is_archived = 0"
                ))?;
                for (index, id) in ids.into_iter().enumerate() {
                    if update.execute(params![index as i64, id.0, parent.0])? != 1 {
                        return Err(Error::InvalidOrder);
                    }
                }
                audit_record(&tx, op_id.as_deref(), "desktop", None, "reorder", None)?;
            }
            Change::ArchiveDesktop {
                id,
                expected_version,
            } => {
                check_version(&tx, Desktop, id.0, expected_version)?;
                let is_archived: i64 = tx.query_row(
                    "SELECT is_archived FROM desktops WHERE id=?1",
                    [id.0],
                    |r| r.get(0),
                )?;
                if is_archived != 0 {
                    return Err(Error::AlreadyArchived("logical desktop"));
                }
                write(
                    &tx,
                    &format!(
                        "UPDATE desktops SET is_archived=1, archived_at_ms=CAST(unixepoch('subsec')*1000 AS INTEGER), version=version+1, {UPDATED} WHERE id=?1"
                    ),
                    params![id.0],
                    Desktop,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "desktop",
                    Some(&id.0.to_string()),
                    "archive",
                    None,
                )?;
            }
            Change::RestoreDesktop {
                id,
                expected_version,
            } => {
                check_version(&tx, Desktop, id.0, expected_version)?;
                let (is_archived, ws_id, name): (i64, Uuid, String) = tx.query_row(
                    "SELECT is_archived, workspace_id, name FROM desktops WHERE id=?1",
                    [id.0],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )?;
                if is_archived == 0 {
                    return Err(Error::NotArchived("logical desktop"));
                }
                let parent_archived: i64 = tx.query_row(
                    "SELECT is_archived FROM workspaces WHERE id=?1",
                    [ws_id],
                    |r| r.get(0),
                )?;
                if parent_archived != 0 {
                    return Err(Error::ParentArchived("workspace"));
                }
                let dup: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM desktops WHERE workspace_id=?1 AND is_archived=0 AND name=?2 COLLATE NOCASE)",
                    params![ws_id, name],
                    |r| r.get(0),
                )?;
                if dup {
                    return Err(Error::Duplicate("logical desktop"));
                }
                write(
                    &tx,
                    &format!(
                        "UPDATE desktops SET is_archived=0, archived_at_ms=NULL, version=version+1, {UPDATED} WHERE id=?1"
                    ),
                    params![id.0],
                    Desktop,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "desktop",
                    Some(&id.0.to_string()),
                    "restore",
                    Some(&name),
                )?;
            }

            Change::RenameWorld(id, name) => {
                rename(&tx, World, id.0, name.clone())?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "world",
                    Some(&id.0.to_string()),
                    "rename",
                    Some(&name),
                )?;
            }
            Change::RenameWorkspace(id, name) => {
                rename(&tx, Workspace, id.0, name.clone())?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "rename",
                    Some(&name),
                )?;
            }
            Change::RenameDesktop(id, name) => {
                rename(&tx, Desktop, id.0, name.clone())?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "desktop",
                    Some(&id.0.to_string()),
                    "rename",
                    Some(&name),
                )?;
            }
            Change::MoveWorkspace(id, parent) => {
                require(&tx, World, parent.0)?;
                require(&tx, Workspace, id.0)?;
                execute(
                    &tx,
                    &format!("UPDATE workspaces SET world_id=?1,
                        sort_order=(SELECT coalesce(max(sort_order)+1,0) FROM workspaces WHERE world_id=?1 AND is_archived=0),
                        version=version+1,
                        {UPDATED} WHERE id=?2 AND world_id<>?1"),
                    params![parent.0, id.0],
                    Workspace,
                )?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "move",
                    Some(&parent.0.to_string()),
                )?;
            }
            Change::DeleteWorld(id) => {
                delete(&tx, World, id.0)?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "world",
                    Some(&id.0.to_string()),
                    "delete",
                    None,
                )?;
            }
            Change::DeleteWorkspace(id) => {
                delete(&tx, Workspace, id.0)?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "workspace",
                    Some(&id.0.to_string()),
                    "delete",
                    None,
                )?;
            }
            Change::DeleteDesktop(id) => {
                delete(&tx, Desktop, id.0)?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "desktop",
                    Some(&id.0.to_string()),
                    "delete",
                    None,
                )?;
            }

            Change::SetPreference(key, value) => {
                tx.execute(
                    "INSERT INTO preferences(key, value, updated_at_ms)
                     VALUES (?1, ?2, CAST(unixepoch('subsec')*1000 AS INTEGER))
                     ON CONFLICT(key) DO UPDATE SET value=excluded.value, updated_at_ms=excluded.updated_at_ms",
                    params![key, value],
                )?;
                audit_record(&tx, op_id.as_deref(), "preference", Some(&key), "set", None)?;
            }
            Change::DeletePreference(key) => {
                tx.execute("DELETE FROM preferences WHERE key=?1", params![key])?;
                audit_record(
                    &tx,
                    op_id.as_deref(),
                    "preference",
                    Some(&key),
                    "delete",
                    None,
                )?;
            }
        }

        tx.commit()?;
        Ok(())
    }

    fn worlds(&self) -> Result<Vec<World>> {
        let tx = self.connection.unchecked_transaction()?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let sql = if version < 3 {
            "SELECT id,name,description,sort_order,created_at_ms,updated_at_ms,NULL,NULL,0,NULL,1 FROM worlds ORDER BY sort_order,id"
        } else {
            "SELECT id,name,description,sort_order,created_at_ms,updated_at_ms,visual_identity,artwork_identifier,is_archived,archived_at_ms,version FROM worlds WHERE is_archived = 0 ORDER BY sort_order,id"
        };
        let result = rows(&tx, sql, [], world)?;
        tx.commit()?;
        Ok(result)
    }

    fn workspaces(&self, parent: WorldId) -> Result<Vec<Workspace>> {
        let tx = self.connection.unchecked_transaction()?;
        require(&tx, Entity::World, parent.0)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let sql = if version < 3 {
            "SELECT id,world_id,name,description,root_directory,sort_order,created_at_ms,updated_at_ms,NULL,0,NULL,1 FROM workspaces WHERE world_id=?1 ORDER BY sort_order,id"
        } else {
            "SELECT id,world_id,name,description,root_directory,sort_order,created_at_ms,updated_at_ms,visual_identity,is_archived,archived_at_ms,version FROM workspaces WHERE world_id=?1 AND is_archived = 0 ORDER BY sort_order,id"
        };
        let result = rows(&tx, sql, [parent.0], workspace)?;
        tx.commit()?;
        Ok(result)
    }

    fn desktops(&self, parent: WorkspaceId) -> Result<Vec<LogicalDesktop>> {
        let tx = self.connection.unchecked_transaction()?;
        require(&tx, Entity::Workspace, parent.0)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let sql = if version < 3 {
            "SELECT id,workspace_id,name,role,sort_order,created_at_ms,updated_at_ms,NULL,0,NULL,1 FROM desktops WHERE workspace_id=?1 ORDER BY sort_order,id"
        } else {
            "SELECT id,workspace_id,name,role,sort_order,created_at_ms,updated_at_ms,visual_identity,is_archived,archived_at_ms,version FROM desktops WHERE workspace_id=?1 AND is_archived = 0 ORDER BY sort_order,id"
        };
        let result = rows(&tx, sql, [parent.0], desktop)?;
        tx.commit()?;
        Ok(result)
    }

    fn tree(&self) -> Result<Universe> {
        let tx = self.connection.unchecked_transaction()?;
        let tree = read_tree(&tx)?;
        tx.commit()?;
        Ok(tree)
    }

    fn archived_items(&self) -> Result<Vec<ArchivedItem>> {
        let tx = self.connection.unchecked_transaction()?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 3 {
            tx.commit()?;
            return Ok(Vec::new());
        }

        let mut items = Vec::new();
        let worlds_stmt = "SELECT id, name, description, archived_at_ms, version,
                (SELECT count(*) FROM workspaces WHERE world_id=worlds.id)
             FROM worlds WHERE is_archived = 1 ORDER BY archived_at_ms DESC";
        let mut stmt = tx.prepare(worlds_stmt)?;
        let world_rows = stmt.query_map([], |r| {
            let id: Uuid = r.get(0)?;
            Ok(ArchivedItem {
                kind: "world".into(),
                id: id.to_string(),
                parent_id: None,
                name: r.get(1)?,
                description: r.get(2)?,
                archived_at_ms: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                version: r.get(4)?,
                child_count: r.get::<_, i64>(5)? as usize,
            })
        })?;
        for w in world_rows {
            items.push(w?);
        }
        drop(stmt);

        let ws_stmt = "SELECT id, world_id, name, description, archived_at_ms, version,
                (SELECT count(*) FROM desktops WHERE workspace_id=workspaces.id)
             FROM workspaces WHERE is_archived = 1 ORDER BY archived_at_ms DESC";
        let mut stmt = tx.prepare(ws_stmt)?;
        let ws_rows = stmt.query_map([], |r| {
            let id: Uuid = r.get(0)?;
            let parent_id: Uuid = r.get(1)?;
            Ok(ArchivedItem {
                kind: "workspace".into(),
                id: id.to_string(),
                parent_id: Some(parent_id.to_string()),
                name: r.get(2)?,
                description: r.get(3)?,
                archived_at_ms: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                version: r.get(5)?,
                child_count: r.get::<_, i64>(6)? as usize,
            })
        })?;
        for ws in ws_rows {
            items.push(ws?);
        }
        drop(stmt);

        let d_stmt = "SELECT id, workspace_id, name, role, archived_at_ms, version
             FROM desktops WHERE is_archived = 1 ORDER BY archived_at_ms DESC";
        let mut stmt = tx.prepare(d_stmt)?;
        let d_rows = stmt.query_map([], |r| {
            let id: Uuid = r.get(0)?;
            let parent_id: Uuid = r.get(1)?;
            Ok(ArchivedItem {
                kind: "desktop".into(),
                id: id.to_string(),
                parent_id: Some(parent_id.to_string()),
                name: r.get(2)?,
                description: r.get(3)?,
                archived_at_ms: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
                version: r.get(5)?,
                child_count: 0,
            })
        })?;
        for d in d_rows {
            items.push(d?);
        }
        drop(stmt);

        tx.commit()?;
        Ok(items)
    }

    fn get_preference(&self, key: &str) -> Result<Option<String>> {
        use rusqlite::OptionalExtension;
        let tx = self.connection.unchecked_transaction()?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 3 {
            tx.commit()?;
            return Ok(None);
        }
        let result = tx
            .query_row("SELECT value FROM preferences WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?;
        tx.commit()?;
        Ok(result)
    }

    fn set_preference(&mut self, key: &str, value: &str) -> Result<()> {
        self.apply(Change::SetPreference(key.to_owned(), value.to_owned()))
    }

    fn list_preferences(&self) -> Result<Vec<(String, String)>> {
        let tx = self.connection.unchecked_transaction()?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < 3 {
            tx.commit()?;
            return Ok(Vec::new());
        }
        let mut stmt = tx.prepare("SELECT key, value FROM preferences ORDER BY key")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        drop(stmt);
        tx.commit()?;
        Ok(list)
    }
}

fn read_tree(tx: &Transaction<'_>) -> Result<Universe> {
    let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;

    let (d_sql, ws_sql, w_sql) = if version < 3 {
        (
            "SELECT id,workspace_id,name,role,sort_order,created_at_ms,updated_at_ms,NULL,0,NULL,1 FROM desktops ORDER BY workspace_id,sort_order,id",
            "SELECT id,world_id,name,description,root_directory,sort_order,created_at_ms,updated_at_ms,NULL,0,NULL,1 FROM workspaces ORDER BY world_id,sort_order,id",
            "SELECT id,name,description,sort_order,created_at_ms,updated_at_ms,NULL,NULL,0,NULL,1 FROM worlds ORDER BY sort_order,id",
        )
    } else {
        (
            "SELECT id,workspace_id,name,role,sort_order,created_at_ms,updated_at_ms,visual_identity,is_archived,archived_at_ms,version FROM desktops WHERE is_archived = 0 ORDER BY workspace_id,sort_order,id",
            "SELECT id,world_id,name,description,root_directory,sort_order,created_at_ms,updated_at_ms,visual_identity,is_archived,archived_at_ms,version FROM workspaces WHERE is_archived = 0 ORDER BY world_id,sort_order,id",
            "SELECT id,name,description,sort_order,created_at_ms,updated_at_ms,visual_identity,artwork_identifier,is_archived,archived_at_ms,version FROM worlds WHERE is_archived = 0 ORDER BY sort_order,id",
        )
    };

    let mut desktops = HashMap::<WorkspaceId, Vec<_>>::new();
    for desktop in rows(tx, d_sql, [], desktop)? {
        desktops
            .entry(desktop.workspace_id)
            .or_default()
            .push(desktop);
    }
    let mut workspaces = HashMap::<WorldId, Vec<_>>::new();
    for workspace in rows(tx, ws_sql, [], workspace)? {
        let desktops = desktops.remove(&workspace.id).unwrap_or_default();
        workspaces
            .entry(workspace.world_id)
            .or_default()
            .push(WorkspaceBranch {
                workspace,
                desktops,
            });
    }
    let worlds = rows(tx, w_sql, [], world)?
        .into_iter()
        .map(|world| WorldBranch {
            workspaces: workspaces.remove(&world.id).unwrap_or_default(),
            world,
        })
        .collect();
    Ok(Universe { worlds })
}

pub mod init;

#[cfg(test)]
mod tests;
