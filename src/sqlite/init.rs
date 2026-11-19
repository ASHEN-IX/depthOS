//! First-run initialization and read-only status probing (Phase 4.1.1, DEF-10).
//!
//! Safety contract:
//! * [`probe`] is strictly read-only: it never creates a directory or file,
//!   never migrates and never writes.
//! * [`initialize`] never overwrites, truncates or replaces an existing path.
//!   A missing database is built in a private (0700) temporary directory next
//!   to the destination — schema and the optional default hierarchy in ONE
//!   transaction — and published with `renameat2(RENAME_NOREPLACE)`, which
//!   fails if the destination appeared in the meantime.
//! * A 0-byte leftover (e.g. an interrupted `open`) holds no data, so it is
//!   initialized in place inside one IMMEDIATE transaction that rolls back on
//!   any failure.
//! * Symlinks, hardlinks, foreign databases, corrupt files, unsupported schema
//!   versions and unsafe permissions are refused without modification.
use std::{
    fs,
    io::ErrorKind,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use rusqlite::{Connection, ErrorCode, OpenFlags, TransactionBehavior, params};
use serde::Serialize;
use uuid::Uuid;

use super::{APP_ID, MIGRATIONS, VERSION, default_path, migrate_tx, path_error};
use crate::domain::{Error, Result};

pub const DEFAULT_WORLD: &str = "Default World";
pub const DEFAULT_WORKSPACE: &str = "Primary Workspace";
pub const DEFAULT_DESKTOP: &str = "Desktop 1";
const TEMP_PREFIX: &str = ".depthos-init-";
const STALE_AFTER: Duration = Duration::from_secs(60);

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DbState {
    Missing,
    /// 0-byte file, or an SQLite file with no objects at all.
    Empty,
    Valid,
    Corrupt,
    UnsupportedSchema,
    PermissionDenied,
    UnsafePermissions,
    Symlink,
    Hardlink,
    Foreign,
}

impl DbState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Empty => "empty",
            Self::Valid => "valid",
            Self::Corrupt => "corrupt",
            Self::UnsupportedSchema => "unsupported-schema",
            Self::PermissionDenied => "permission-denied",
            Self::UnsafePermissions => "unsafe-permissions",
            Self::Symlink => "symlink",
            Self::Hardlink => "hardlink",
            Self::Foreign => "foreign",
        }
    }
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counts {
    pub worlds: i64,
    pub workspaces: i64,
    pub desktops: i64,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub path: PathBuf,
    pub exists: bool,
    pub state: DbState,
    pub schema_version: Option<i64>,
    pub supported_schema_version: i64,
    pub counts: Option<Counts>,
    pub detail: Option<String>,
}

fn status_of(path: &Path, state: DbState, detail: impl Into<Option<String>>) -> Status {
    Status {
        path: path.to_owned(),
        exists: state != DbState::Missing,
        state,
        schema_version: None,
        supported_schema_version: VERSION,
        counts: None,
        detail: detail.into(),
    }
}

fn sqlite_class(error: &rusqlite::Error) -> Option<DbState> {
    match error.sqlite_error_code() {
        Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt) => Some(DbState::Corrupt),
        Some(ErrorCode::PermissionDenied | ErrorCode::CannotOpen | ErrorCode::ReadOnly) => {
            Some(DbState::PermissionDenied)
        }
        _ => None,
    }
}

/// Classifies `path` without ever writing to it or its directory.
pub fn probe(path: &Path) -> Result<Status> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
            let detail = parent
                .filter(|p| !p.exists())
                .map(|_| "parent directory does not exist".to_owned());
            return Ok(status_of(path, DbState::Missing, detail));
        }
        Err(e) if e.kind() == ErrorKind::PermissionDenied => {
            return Ok(status_of(
                path,
                DbState::PermissionDenied,
                Some(format!("cannot stat database path: {e}")),
            ));
        }
        Err(e) => return Err(path_error(path, e)),
    };
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return Ok(status_of(
            path,
            DbState::Symlink,
            Some("database path is a symbolic link; refusing to follow it".to_owned()),
        ));
    }
    if !file_type.is_file() {
        return Ok(status_of(
            path,
            DbState::Foreign,
            Some("database path is not a regular file".to_owned()),
        ));
    }
    if metadata.nlink() != 1 {
        return Ok(status_of(
            path,
            DbState::Hardlink,
            Some(format!(
                "database file has {} hard links; refusing to use it",
                metadata.nlink()
            )),
        ));
    }
    if metadata.uid() != rustix::process::geteuid().as_raw() {
        return Ok(status_of(
            path,
            DbState::Foreign,
            Some("database file is owned by another user".to_owned()),
        ));
    }
    if metadata.permissions().mode() & 0o077 != 0 {
        return Ok(status_of(
            path,
            DbState::UnsafePermissions,
            Some(format!(
                "database file mode is {:04o}; expected 0600 (not changed automatically)",
                metadata.permissions().mode() & 0o7777
            )),
        ));
    }
    if metadata.len() == 0 {
        return Ok(status_of(
            path,
            DbState::Empty,
            Some("0-byte file (no data); safe to initialize".to_owned()),
        ));
    }
    inspect_sqlite(path)
}

fn inspect_sqlite(path: &Path) -> Result<Status> {
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let connection = match Connection::open_with_flags(path, flags) {
        Ok(c) => c,
        Err(e) => {
            return match sqlite_class(&e) {
                Some(state) => Ok(status_of(path, state, Some(e.to_string()))),
                None => Err(e.into()),
            };
        }
    };
    let _ = connection.busy_timeout(Duration::from_secs(5));
    let classify = |e: rusqlite::Error| match sqlite_class(&e) {
        Some(state) => Ok(status_of(path, state, Some(e.to_string()))),
        None => Err(Error::from(e)),
    };
    let app: i64 = match connection.pragma_query_value(None, "application_id", |r| r.get(0)) {
        Ok(v) => v,
        Err(e) => return classify(e),
    };
    let version: i64 = match connection.pragma_query_value(None, "user_version", |r| r.get(0)) {
        Ok(v) => v,
        Err(e) => return classify(e),
    };
    if app == 0 && version == 0 {
        let objects: i64 =
            match connection.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0)) {
                Ok(v) => v,
                Err(e) => return classify(e),
            };
        return Ok(if objects == 0 {
            status_of(
                path,
                DbState::Empty,
                Some("SQLite file without any objects".to_owned()),
            )
        } else {
            status_of(
                path,
                DbState::Foreign,
                Some("SQLite database that is not a DepthOS database".to_owned()),
            )
        });
    }
    if app != APP_ID {
        return Ok(status_of(
            path,
            DbState::Foreign,
            Some("SQLite database that is not a DepthOS database".to_owned()),
        ));
    }
    if !(1..=VERSION).contains(&version) {
        let mut status = status_of(
            path,
            DbState::UnsupportedSchema,
            Some(format!(
                "unsupported database schema version {version}; this DepthOS supports 1..={VERSION}"
            )),
        );
        status.schema_version = Some(version);
        return Ok(status);
    }
    let quick: Vec<String> = match connection
        .prepare("PRAGMA quick_check")
        .and_then(|mut s| s.query_map([], |r| r.get(0))?.collect())
    {
        Ok(v) => v,
        Err(e) => return classify(e),
    };
    if quick != ["ok"] {
        let mut status = status_of(
            path,
            DbState::Corrupt,
            Some(format!("integrity check failed: {}", quick.join("; "))),
        );
        status.schema_version = Some(version);
        return Ok(status);
    }
    let counted: rusqlite::Result<Counts> = (|| {
        let count = |table: &str| -> rusqlite::Result<i64> {
            connection.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        };
        Ok(Counts {
            worlds: count("worlds")?,
            workspaces: count("workspaces")?,
            desktops: count("desktops")?,
        })
    })();
    match counted {
        Ok(counts) => {
            let mut status = status_of(path, DbState::Valid, None);
            status.schema_version = Some(version);
            status.counts = Some(counts);
            Ok(status)
        }
        Err(e) if sqlite_class(&e) == Some(DbState::Corrupt) || e.sqlite_error().is_some() => {
            let mut status = status_of(
                path,
                DbState::Corrupt,
                Some(format!("schema tables unreadable: {e}")),
            );
            status.schema_version = Some(version);
            Ok(status)
        }
        Err(e) => Err(e.into()),
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InitOptions {
    /// Seed Default World / Primary Workspace / Desktop 1 when (and only when)
    /// the database contains no worlds.
    pub default_hierarchy: bool,
}

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InitOutcome {
    /// A new database was built privately and atomically published.
    Created,
    /// A 0-byte leftover was initialized in place in one transaction.
    InitializedInPlace,
    /// A valid, empty database received the default hierarchy.
    SeededExisting,
    /// A valid database already existed; nothing was written.
    AlreadyInitialized,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitReport {
    pub path: PathBuf,
    pub outcome: InitOutcome,
    pub state: DbState,
    pub schema_version: Option<i64>,
    pub counts: Option<Counts>,
    pub seeded: bool,
}

/// Fault-injection points used by tests to simulate interruption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitStage {
    /// Inside the transaction, after schema+seed, before `COMMIT`.
    BeforeCommit,
    /// Temporary database complete, not yet published.
    BeforePublish,
}

pub fn initialize(path: &Path, options: InitOptions) -> Result<InitReport> {
    initialize_with_hook(path, options, &|_| Ok(()))
}

/// Initializes the default per-user database, creating its private directory
/// (mode 0700) when absent. Existing directories are validated, never chmodded.
pub fn initialize_default(options: InitOptions) -> Result<InitReport> {
    let path = default_path()?;
    let directory = path.parent().expect("default path has parent");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)
        .map_err(|e| path_error(directory, e))?;
    // Same rule as `SqliteStore::open_default`: a real, private directory.
    let metadata = fs::symlink_metadata(directory).map_err(|e| path_error(directory, e))?;
    if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(Error::InitRefused(format!(
            "{directory:?} must be a private directory (mode 0700); it is not changed automatically"
        )));
    }
    initialize(&path, options)
}

pub fn initialize_with_hook(
    path: &Path,
    options: InitOptions,
    hook: &dyn Fn(InitStage) -> Result<()>,
) -> Result<InitReport> {
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
        return Err(Error::InitRefused(format!(
            "directory {parent:?} is not writable only by its owner (mode {:04o}); \
             not changed automatically",
            parent_metadata.permissions().mode() & 0o7777
        )));
    }
    cleanup_stale_temp_dirs(parent);

    // A lost publication race re-probes; the winner's database is then Valid.
    for _ in 0..4 {
        let status = probe(path)?;
        match status.state {
            DbState::Missing => match create_and_publish(path, parent, options, hook)? {
                Some(report) => return Ok(report),
                None => continue,
            },
            DbState::Empty => return in_place(path, options, hook),
            DbState::Valid => {
                let empty = status.counts.is_some_and(|c| c.worlds == 0);
                if options.default_hierarchy && empty {
                    return in_place(path, options, hook);
                }
                return Ok(InitReport {
                    path: path.to_owned(),
                    outcome: InitOutcome::AlreadyInitialized,
                    state: DbState::Valid,
                    schema_version: status.schema_version,
                    counts: status.counts,
                    seeded: false,
                });
            }
            state => {
                return Err(Error::InitRefused(format!(
                    "{path:?} is {}: {}",
                    state.as_str(),
                    status.detail.unwrap_or_default()
                )));
            }
        }
    }
    Err(Error::InitRefused(format!(
        "{path:?} kept changing while initializing; try again"
    )))
}

struct TempDir(PathBuf);
impl Drop for TempDir {
    fn drop(&mut self) {
        // Only ever a directory this process created a moment ago.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn configure(connection: &Connection) -> Result<()> {
    connection.busy_timeout(Duration::from_secs(5))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "trusted_schema", false)?;
    Ok(())
}

/// Seeds the default hierarchy iff there are no worlds. Returns whether it did.
fn seed_default(tx: &rusqlite::Transaction<'_>) -> Result<bool> {
    let worlds: i64 = tx.query_row("SELECT count(*) FROM worlds", [], |r| r.get(0))?;
    if worlds != 0 {
        return Ok(false);
    }
    let (world, workspace, desktop) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    tx.execute(
        "INSERT INTO worlds(id,name,description,sort_order) VALUES (?1,?2,?3,0)",
        params![world, DEFAULT_WORLD, "Your default logical world"],
    )?;
    tx.execute(
        "INSERT INTO workspaces(id,world_id,name,description,root_directory,sort_order)
         VALUES (?1,?2,?3,?4,NULL,0)",
        params![
            workspace,
            world,
            DEFAULT_WORKSPACE,
            "Your primary logical workspace"
        ],
    )?;
    tx.execute(
        "INSERT INTO desktops(id,workspace_id,name,role,sort_order) VALUES (?1,?2,?3,?4,0)",
        params![desktop, workspace, DEFAULT_DESKTOP, "Placeholder desktop"],
    )?;
    Ok(true)
}

/// Schema + optional seed in one IMMEDIATE transaction.
fn initialize_transaction(
    connection: &mut Connection,
    options: InitOptions,
    hook: &dyn Fn(InitStage) -> Result<()>,
) -> Result<(bool, bool)> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let before: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
    migrate_tx(&tx, MIGRATIONS)?;
    let seeded = options.default_hierarchy && seed_default(&tx)?;
    hook(InitStage::BeforeCommit)?; // an Err drops `tx`, rolling everything back
    tx.commit()?;
    Ok((before == 0, seeded))
}

fn report_for(path: &Path, outcome: InitOutcome, seeded: bool) -> Result<InitReport> {
    let status = probe(path)?;
    Ok(InitReport {
        path: path.to_owned(),
        outcome,
        state: status.state,
        schema_version: status.schema_version,
        counts: status.counts,
        seeded,
    })
}

fn in_place(
    path: &Path,
    options: InitOptions,
    hook: &dyn Fn(InitStage) -> Result<()>,
) -> Result<InitReport> {
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let mut connection = Connection::open_with_flags(path, flags)?;
    configure(&connection)?;
    let (schema_created, seeded) = initialize_transaction(&mut connection, options, hook)?;
    connection.close().map_err(|(_, e)| Error::from(e))?;
    let outcome = if schema_created {
        InitOutcome::InitializedInPlace
    } else if seeded {
        InitOutcome::SeededExisting
    } else {
        InitOutcome::AlreadyInitialized
    };
    report_for(path, outcome, seeded)
}

/// Returns `Ok(None)` if the destination appeared while we were building.
fn create_and_publish(
    path: &Path,
    parent: &Path,
    options: InitOptions,
    hook: &dyn Fn(InitStage) -> Result<()>,
) -> Result<Option<InitReport>> {
    let temp_dir = TempDir(parent.join(format!(
        "{TEMP_PREFIX}{}-{}",
        std::process::id(),
        Uuid::new_v4().simple()
    )));
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&temp_dir.0)
        .map_err(|e| path_error(&temp_dir.0, e))?;
    let temp_db = temp_dir.0.join("init.sqlite3");
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temp_db)
        .map_err(|e| path_error(&temp_db, e))?;

    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let mut connection = Connection::open_with_flags(&temp_db, flags)?;
    configure(&connection)?;
    let (_, seeded) = initialize_transaction(&mut connection, options, hook)?;
    connection.close().map_err(|(_, e)| Error::from(e))?;
    fs::File::open(&temp_db)
        .and_then(|f| f.sync_all())
        .map_err(|e| path_error(&temp_db, e))?;
    hook(InitStage::BeforePublish)?;

    match publish(&temp_db, path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => return Ok(None),
        Err(e) => return Err(path_error(path, e)),
    }
    // Make the new directory entry durable.
    let _ = fs::File::open(parent).and_then(|d| d.sync_all());
    drop(temp_dir);
    report_for(path, InitOutcome::Created, seeded).map(Some)
}

/// Atomic no-clobber publication. `RENAME_NOREPLACE` keeps the link count at 1
/// at every instant; the link+unlink fallback is used only on filesystems that
/// do not support it and still never replaces an existing destination.
fn publish(from: &Path, to: &Path) -> std::io::Result<()> {
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    match renameat_with(CWD, from, CWD, to, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(()),
        Err(e) if e == rustix::io::Errno::EXIST => Err(ErrorKind::AlreadyExists.into()),
        Err(e) if e == rustix::io::Errno::INVAL || e == rustix::io::Errno::NOSYS => {
            fs::hard_link(from, to)?;
            let _ = fs::remove_file(from);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Removes `.depthos-init-<pid>-*` directories abandoned by a dead process.
/// Restricted to real directories owned by us, named by our own pattern, whose
/// creator PID no longer exists and which are older than a minute.
fn cleanup_stale_temp_dirs(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let me = rustix::process::geteuid().as_raw();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(rest) = name.to_str().and_then(|n| n.strip_prefix(TEMP_PREFIX)) else {
            continue;
        };
        let Some(pid) = rest.split('-').next().and_then(|p| p.parse::<u32>().ok()) else {
            continue;
        };
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if !metadata.is_dir() || metadata.uid() != me || Path::new(&format!("/proc/{pid}")).exists()
        {
            continue;
        }
        let old = metadata
            .modified()
            .ok()
            .and_then(|m| SystemTime::now().duration_since(m).ok())
            .is_some_and(|age| age >= STALE_AFTER);
        if old {
            let _ = fs::remove_dir_all(&path);
        }
    }
}
