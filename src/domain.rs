use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

macro_rules! id {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord,
        )]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
                value.parse().map(Self)
            }
        }
    };
}

id!(WorldId);
id!(WorkspaceId);
id!(DesktopId);

/// A single implicit Universe per database.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Universe {
    pub worlds: Vec<WorldBranch>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldBranch {
    pub world: World,
    pub workspaces: Vec<WorkspaceBranch>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBranch {
    pub workspace: Workspace,
    pub desktops: Vec<LogicalDesktop>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct World {
    pub id: WorldId,
    pub name: String,
    pub description: Option<String>,
    pub sort_order: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub world_id: WorldId,
    pub name: String,
    pub description: Option<String>,
    pub root_directory: Option<String>,
    pub sort_order: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogicalDesktop {
    pub id: DesktopId,
    pub workspace_id: WorkspaceId,
    pub name: String,
    pub role: Option<String>,
    pub sort_order: i64,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

/// Reserved policy categories for a future workspace configuration format.
pub enum PolicyCategory {
    ApplicationAssociation,
    WindowOwnership,
    Launch,
    Restoration,
    ConflictHandling,
}

/// Future explicit association. The application key needs a defined namespace
/// before this contract is persisted or used to resolve ownership.
pub struct ApplicationAssociation {
    pub workspace_id: WorkspaceId,
    pub application_key: String,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(
        "name must contain 1 to 120 characters after trimming and contain no control characters"
    )]
    InvalidName,
    #[error("invalid {0}")]
    InvalidMetadata(&'static str),
    #[error("{0} does not exist")]
    Missing(&'static str),
    #[error("a {0} with that name already exists in this scope")]
    Duplicate(&'static str),
    #[error("{0} still has children; delete or move them first")]
    HasChildren(&'static str),
    #[error("desktop order must list each desktop in the workspace exactly once")]
    InvalidOrder,
    #[error("SQLite {0} is too old; DepthOS requires SQLite 3.42 or newer")]
    SqliteVersion(&'static str),
    #[error("unsupported database schema version {0}")]
    SchemaVersion(i64),
    #[error("file is not a DepthOS database")]
    ForeignDatabase,
    #[error("database path is unsafe or inaccessible: {0}")]
    StoragePath(String),
    #[error("refusing to initialize: {0}")]
    InitRefused(String),
    #[error("schema migration failed: {0}")]
    Migration(rusqlite::Error),
    #[error("database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("output failed: {0}")]
    Output(#[from] std::io::Error),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
