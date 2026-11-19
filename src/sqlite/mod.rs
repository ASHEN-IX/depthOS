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
    domain::*,
    service::{Change, Store},
};

const APP_ID: i64 = 0x44505448;
const MIGRATIONS: &[&str] = &[include_str!("../../migrations/001_initial.sql")];
const VERSION: i64 = MIGRATIONS.len() as i64;
const WORLD_COLUMNS: &str = "id,name,description,sort_order,created_at_ms,updated_at_ms";
const WORKSPACE_COLUMNS: &str =
    "id,world_id,name,description,root_directory,sort_order,created_at_ms,updated_at_ms";
const DESKTOP_COLUMNS: &str = "id,workspace_id,name,role,sort_order,created_at_ms,updated_at_ms";
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
        let metadata = fs::symlink_metadata(path).map_err(|e| path_error(path, e))?;
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(path_error(
                path,
                "expected a private regular file without links (mode 0600)",
            ));
        }
        let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW;
        let mut connection = Connection::open_with_flags(path, flags)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "trusted_schema", false)?;
        migrate(&mut connection, MIGRATIONS)?;
        Ok(Self { connection })
    }

    /// Opens an existing database strictly read-only: never creates the file,
    /// never creates its directory and never migrates. Used by read paths
    /// (`depthos tree`, the UI) so that merely looking cannot create user data.
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

    pub fn doctor(&self) -> Result<Doctor> {
        let connection = self.connection.unchecked_transaction()?;
        for (table, columns) in [
            ("worlds", WORLD_COLUMNS),
            ("workspaces", WORKSPACE_COLUMNS),
            ("desktops", DESKTOP_COLUMNS),
        ] {
            connection.prepare(&format!("SELECT {columns} FROM {table} LIMIT 0"))?;
        }
        Ok(Doctor {
            schema_version: connection.pragma_query_value(None, "user_version", |r| r.get(0))?,
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
    write(
        db,
        &format!("UPDATE {} SET name=?1, {UPDATED} WHERE id=?2", kind.table()),
        params![name, id],
        kind,
    )
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
        use Entity::{Desktop, Workspace, World};

        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        match change {
            Change::CreateWorld(id, name, description) => write(
                &tx,
                "INSERT INTO worlds(id,name,description,sort_order)
                 VALUES (?1,?2,?3,(SELECT coalesce(max(sort_order)+1,0) FROM worlds))",
                params![id.0, name, description],
                World,
            )?,
            Change::CreateWorkspace(id, parent, name, description, root) => {
                require(&tx, World, parent.0)?;
                write(&tx, "INSERT INTO workspaces(id,world_id,name,description,root_directory,sort_order)
                    VALUES (?1,?2,?3,?4,?5,(SELECT coalesce(max(sort_order)+1,0) FROM workspaces WHERE world_id=?2))",
                    params![id.0,parent.0,name,description,root], Workspace)?;
            }
            Change::CreateDesktop(id, parent, name, role) => {
                require(&tx, Workspace, parent.0)?;
                write(&tx, "INSERT INTO desktops(id,workspace_id,name,role,sort_order)
                    VALUES (?1,?2,?3,?4,(SELECT coalesce(max(sort_order)+1,0) FROM desktops WHERE workspace_id=?2))",
                    params![id.0,parent.0,name,role], Desktop)?;
            }
            Change::RenameWorld(id, name) => rename(&tx, World, id.0, name)?,
            Change::RenameWorkspace(id, name) => rename(&tx, Workspace, id.0, name)?,
            Change::RenameDesktop(id, name) => rename(&tx, Desktop, id.0, name)?,
            Change::MoveWorkspace(id, parent) => {
                require(&tx, World, parent.0)?;
                require(&tx, Workspace, id.0)?;
                execute(
                    &tx,
                    &format!("UPDATE workspaces SET world_id=?1,
                        sort_order=(SELECT coalesce(max(sort_order)+1,0) FROM workspaces WHERE world_id=?1),
                        {UPDATED} WHERE id=?2 AND world_id<>?1"),
                    params![parent.0, id.0],
                    Workspace,
                )?;
            }
            Change::ReorderDesktops(parent, ids) => {
                require(&tx, Workspace, parent.0)?;
                let count: i64 = tx.query_row(
                    "SELECT count(*) FROM desktops WHERE workspace_id=?1",
                    [parent.0],
                    |r| r.get(0),
                )?;
                if count as usize != ids.len()
                    || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len()
                {
                    return Err(Error::InvalidOrder);
                }
                let mut update = tx.prepare_cached(&format!(
                    "UPDATE desktops SET sort_order=?1, {UPDATED} WHERE id=?2 AND workspace_id=?3"
                ))?;
                for (index, id) in ids.into_iter().enumerate() {
                    if update.execute(params![index as i64, id.0, parent.0])? != 1 {
                        return Err(Error::InvalidOrder);
                    }
                }
            }
            Change::DeleteWorld(id) => delete(&tx, World, id.0)?,
            Change::DeleteWorkspace(id) => delete(&tx, Workspace, id.0)?,
            Change::DeleteDesktop(id) => delete(&tx, Desktop, id.0)?,
        }
        tx.commit()?;
        Ok(())
    }

    fn worlds(&self) -> Result<Vec<World>> {
        rows(
            &self.connection,
            &format!("SELECT {WORLD_COLUMNS} FROM worlds ORDER BY sort_order,id"),
            [],
            world,
        )
    }

    fn workspaces(&self, parent: WorldId) -> Result<Vec<Workspace>> {
        let tx = self.connection.unchecked_transaction()?;
        require(&tx, Entity::World, parent.0)?;
        let result = rows(
            &tx,
            &format!(
                "SELECT {WORKSPACE_COLUMNS} FROM workspaces WHERE world_id=?1 ORDER BY sort_order,id"
            ),
            [parent.0],
            workspace,
        )?;
        tx.commit()?;
        Ok(result)
    }

    fn desktops(&self, parent: WorkspaceId) -> Result<Vec<LogicalDesktop>> {
        let tx = self.connection.unchecked_transaction()?;
        require(&tx, Entity::Workspace, parent.0)?;
        let result = rows(
            &tx,
            &format!(
                "SELECT {DESKTOP_COLUMNS} FROM desktops WHERE workspace_id=?1 ORDER BY sort_order,id"
            ),
            [parent.0],
            desktop,
        )?;
        tx.commit()?;
        Ok(result)
    }

    fn tree(&self) -> Result<Universe> {
        let tx = self.connection.unchecked_transaction()?;
        let mut desktops = HashMap::<WorkspaceId, Vec<_>>::new();
        for desktop in rows(
            &tx,
            &format!("SELECT {DESKTOP_COLUMNS} FROM desktops ORDER BY workspace_id,sort_order,id"),
            [],
            desktop,
        )? {
            desktops
                .entry(desktop.workspace_id)
                .or_default()
                .push(desktop);
        }
        let mut workspaces = HashMap::<WorldId, Vec<_>>::new();
        for workspace in rows(
            &tx,
            &format!("SELECT {WORKSPACE_COLUMNS} FROM workspaces ORDER BY world_id,sort_order,id"),
            [],
            workspace,
        )? {
            let desktops = desktops.remove(&workspace.id).unwrap_or_default();
            workspaces
                .entry(workspace.world_id)
                .or_default()
                .push(WorkspaceBranch {
                    workspace,
                    desktops,
                });
        }
        let worlds = rows(
            &tx,
            &format!("SELECT {WORLD_COLUMNS} FROM worlds ORDER BY sort_order,id"),
            [],
            world,
        )?
        .into_iter()
        .map(|world| WorldBranch {
            workspaces: workspaces.remove(&world.id).unwrap_or_default(),
            world,
        })
        .collect();
        tx.commit()?;
        Ok(Universe { worlds })
    }
}

pub mod init;

#[cfg(test)]
mod tests;
