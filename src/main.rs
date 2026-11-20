use std::{
    io::{self, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use depthos_core::{
    application::Application,
    associations::RuleKind,
    domain::{DesktopId, Error, Result, WorkspaceId, WorldId},
    observed::ObservedStateSource,
    service::{DesktopUpdate, WorkspaceUpdate, WorldUpdate, inspect_project_directory},
    sqlite::{
        SqliteStore, default_path,
        init::{InitOptions, initialize, initialize_default, probe},
    },
};
use serde::Serialize;

#[derive(Parser)]
#[command(
    version,
    about = "Manage local DepthOS workspaces and inspect native observations"
)]
struct Cli {
    /// Database file override. Its parent directory must already exist.
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(subcommand)]
    World(WorldCommand),
    #[command(subcommand)]
    Workspace(WorkspaceCommand),
    #[command(subcommand)]
    Desktop(DesktopCommand),
    /// Configure a logical-only association rule in an existing SQLite database.
    #[command(subcommand)]
    Association(AssociationCommand),
    /// Manage user preferences.
    #[command(subcommand)]
    Preferences(PreferencesCommand),
    /// Inspect local project directory and non-interactive Git metadata.
    #[command(subcommand)]
    Project(ProjectCommand),
    /// List archived hierarchy records.
    #[command(subcommand)]
    Archived(ArchivedCommand),
    /// Execute a transactional management operation from structured JSON.
    #[command(subcommand)]
    Manage(ManageCommand),
    /// Print the complete logical hierarchy as JSON. Strictly read-only: never
    /// creates, migrates or repairs a database (run `depthos init` first).
    Tree,
    /// Explicitly initialize a user-owned database (never overwrites anything).
    Init {
        /// Also create Default World / Primary Workspace / Desktop 1 when the
        /// database holds no worlds yet.
        #[arg(long)]
        default_hierarchy: bool,
    },
    /// Read-only probe of the database state (never creates or modifies it).
    Status {
        /// Machine-readable output (always JSON on stdout).
        #[arg(long)]
        json: bool,
    },
    /// Check schema version, foreign keys and database integrity. Read-only:
    /// never creates or migrates a database.
    Doctor,
    /// Observe native state in a foreground session-bus receiver; never opens SQLite.
    #[command(subcommand)]
    System(SystemCommand),
    /// Run a foreground logical/observation/context controller.
    #[command(subcommand)]
    Controller(ControllerCommand),
    /// Launch the standalone Spatial Workspace Alpha UI.
    Ui(UiCommand),
}

#[derive(Subcommand)]
enum AssociationCommand {
    /// Set a native Desktop mapping or a Workspace-scoped application rule.
    Set {
        #[arg(value_enum)]
        kind: RuleKind,
        key: String,
        #[arg(long)]
        workspace: WorkspaceId,
        #[arg(long)]
        desktop: Option<DesktopId>,
    },
    /// Remove a configured association rule.
    Remove {
        #[arg(value_enum)]
        kind: RuleKind,
        key: String,
    },
}

#[derive(clap::Args, Clone, Debug)]
pub struct UiCommand {
    /// Force standalone demo/mock mode even if DepthOS Controller D-Bus is available
    #[arg(long, short)]
    pub mock: bool,
    /// Enable reduced motion mode (instant animations)
    #[arg(long)]
    pub reduced_motion: bool,
    /// Run using offscreen QPA platform (for automated test validation)
    #[arg(long)]
    pub offscreen: bool,
    /// Automatically quit after specified milliseconds (for testing)
    #[arg(long, default_value_t = 0)]
    pub quit_after_ms: u64,
}

#[derive(Subcommand)]
enum ControllerCommand {
    Run {
        /// Optional bounded foreground run for diagnostics and tests.
        #[arg(long, value_parser=clap::value_parser!(u64).range(1..=300))]
        timeout_seconds: Option<u64>,
    },
}

#[derive(clap::Args)]
struct SystemOptions {
    /// Foreground observation window; no script is loaded automatically.
    #[arg(long, default_value_t = 5, value_parser = clap::value_parser!(u64).range(1..=300))]
    timeout_seconds: u64,
}
#[derive(Subcommand)]
enum SystemCommand {
    Health {
        #[command(flatten)]
        options: SystemOptions,
    },
    Inspect {
        #[command(flatten)]
        options: SystemOptions,
    },
    Snapshot {
        /// Titles are always redacted, including when this explicit flag is omitted.
        #[arg(long)]
        redact_titles: bool,
        #[command(flatten)]
        options: SystemOptions,
    },
}

#[derive(Subcommand)]
enum WorldCommand {
    Create {
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        visual_identity: Option<String>,
        #[arg(long)]
        artwork: Option<String>,
    },
    List {
        #[arg(long)]
        archived: bool,
    },
    Rename {
        id: WorldId,
        name: String,
    },
    Update {
        id: WorldId,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        visual_identity: Option<String>,
        #[arg(long)]
        artwork: Option<String>,
        #[arg(long)]
        version: Option<i64>,
    },
    Reorder {
        #[arg(required = true)]
        ids: Vec<WorldId>,
    },
    Archive {
        id: WorldId,
        #[arg(long)]
        version: Option<i64>,
    },
    Restore {
        id: WorldId,
        #[arg(long)]
        version: Option<i64>,
    },
    /// Delete an empty world.
    Delete {
        id: WorldId,
    },
}

#[derive(Subcommand)]
enum WorkspaceCommand {
    Create {
        #[arg(long)]
        world: WorldId,
        #[arg(long)]
        name: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        root_directory: Option<String>,
        #[arg(long)]
        visual_identity: Option<String>,
    },
    List {
        #[arg(long)]
        world: WorldId,
        #[arg(long)]
        archived: bool,
    },
    Rename {
        id: WorkspaceId,
        name: String,
    },
    Update {
        id: WorkspaceId,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        root_directory: Option<String>,
        #[arg(long)]
        visual_identity: Option<String>,
        #[arg(long)]
        version: Option<i64>,
    },
    Reorder {
        #[arg(long)]
        world: WorldId,
        #[arg(required = true)]
        ids: Vec<WorkspaceId>,
    },
    Move {
        id: WorkspaceId,
        #[arg(long)]
        world: WorldId,
    },
    Archive {
        id: WorkspaceId,
        #[arg(long)]
        version: Option<i64>,
    },
    Restore {
        id: WorkspaceId,
        #[arg(long)]
        version: Option<i64>,
    },
    /// Delete an empty workspace.
    Delete {
        id: WorkspaceId,
    },
}

#[derive(Subcommand)]
enum DesktopCommand {
    Create {
        #[arg(long)]
        workspace: WorkspaceId,
        #[arg(long)]
        name: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        visual_identity: Option<String>,
    },
    List {
        #[arg(long)]
        workspace: WorkspaceId,
        #[arg(long)]
        archived: bool,
    },
    Rename {
        id: DesktopId,
        name: String,
    },
    Update {
        id: DesktopId,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        visual_identity: Option<String>,
        #[arg(long)]
        version: Option<i64>,
    },
    /// Supply all desktop IDs in their desired order.
    Reorder {
        #[arg(long)]
        workspace: WorkspaceId,
        #[arg(required = true)]
        ids: Vec<DesktopId>,
    },
    Archive {
        id: DesktopId,
        #[arg(long)]
        version: Option<i64>,
    },
    Restore {
        id: DesktopId,
        #[arg(long)]
        version: Option<i64>,
    },
    Delete {
        id: DesktopId,
    },
}

#[derive(Subcommand)]
enum PreferencesCommand {
    /// Retrieve a preference value by key.
    Get { key: String },
    /// Store a preference key-value pair.
    Set { key: String, value: String },
    /// List all stored preferences.
    List,
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Inspect a directory path for existence, permissions, and safe Git repository metadata.
    Inspect { path: String },
}

#[derive(Subcommand)]
enum ArchivedCommand {
    /// List all archived worlds, workspaces, and desktops.
    List,
}

#[derive(Subcommand)]
enum ManageCommand {
    /// Execute a structured JSON management request (from string or stdin).
    Exec {
        #[arg(long)]
        request: Option<String>,
    },
}

fn json(value: impl Serialize) -> Result<()> {
    let mut output = io::stdout().lock();
    writeln!(output, "{}", serde_json::to_string_pretty(&value)?)?;
    Ok(())
}

fn status(id: impl Serialize) -> Result<()> {
    json(serde_json::json!({ "id": id }))
}

fn database_path(cli: &Cli) -> Result<PathBuf> {
    cli.db.clone().map(Ok).unwrap_or_else(default_path)
}

fn run_status(cli: &Cli, json_output: bool) -> Result<()> {
    let status = probe(&database_path(cli)?)?;
    if json_output {
        return json(&status);
    }
    let mut output = io::stdout().lock();
    writeln!(
        output,
        "{}: {}",
        status.path.display(),
        status.state.as_str()
    )?;
    if let Some(detail) = &status.detail {
        writeln!(output, "  {detail}")?;
    }
    if let Some(counts) = status.counts {
        writeln!(
            output,
            "  {} worlds, {} workspaces, {} desktops",
            counts.worlds, counts.workspaces, counts.desktops
        )?;
    }
    Ok(())
}

fn run_init(cli: &Cli, default_hierarchy: bool) -> Result<()> {
    let options = InitOptions { default_hierarchy };
    let report = match &cli.db {
        Some(path) => initialize(path, options)?,
        None => initialize_default(options)?,
    };
    json(&report)
}

fn command_is_read_only(command: &Command) -> bool {
    match command {
        Command::Tree
        | Command::Doctor
        | Command::World(WorldCommand::List { .. })
        | Command::Workspace(WorkspaceCommand::List { .. })
        | Command::Desktop(DesktopCommand::List { .. })
        | Command::Preferences(PreferencesCommand::Get { .. } | PreferencesCommand::List)
        | Command::Project(ProjectCommand::Inspect { .. })
        | Command::Archived(ArchivedCommand::List) => true,
        Command::Manage(ManageCommand::Exec { request }) => {
            let action = request
                .as_deref()
                .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
                .and_then(|v| v["action"].as_str().map(String::from));
            matches!(
                action.as_deref(),
                Some("getPreferences" | "getArchived" | "getAssociationImpact")
            )
        }
        _ => false,
    }
}

fn run(cli: Cli) -> Result<bool> {
    if let Command::Project(ProjectCommand::Inspect { path }) = &cli.command {
        let inspection = inspect_project_directory(path);
        return json(inspection).map(|_| true);
    }
    if let Command::Association(command) = &cli.command {
        let (kind, key, target) = match command {
            AssociationCommand::Set {
                kind,
                key,
                workspace,
                desktop,
            } => (*kind, key, Some((*workspace, *desktop))),
            AssociationCommand::Remove { kind, key } => (*kind, key, None),
        };
        SqliteStore::configure_association(&database_path(&cli)?, kind, key, target)?;
        return Ok(true);
    }
    let store = if command_is_read_only(&cli.command) {
        // Reads must not create, migrate, or repair. `depthos init` is the
        // only implicit-path creator, and only when the user asks for it.
        SqliteStore::open_read_only(&database_path(&cli)?)?
    } else if let Some(path) = &cli.db {
        SqliteStore::open(path)?
    } else {
        SqliteStore::open_default()?
    };
    if matches!(cli.command, Command::Doctor) {
        let path = cli.db.map(Ok).unwrap_or_else(default_path)?;
        let report = store.doctor()?;
        let healthy =
            report.foreign_keys && report.foreign_key_violations == 0 && report.integrity == ["ok"];
        json(serde_json::json!({"path": path, "healthy": healthy, "database": report}))?;
        return Ok(healthy);
    }

    let mut application = Application::disconnected(store);
    let service = application.logical_mut();
    match cli.command {
        Command::World(command) => match command {
            WorldCommand::Create {
                name,
                description,
                visual_identity,
                artwork,
            } => {
                status(service.create_world_full(&name, description, visual_identity, artwork)?)?
            }
            WorldCommand::List { archived } => {
                if archived {
                    let items: Vec<_> = service
                        .archived_items()?
                        .into_iter()
                        .filter(|i| i.kind == "world")
                        .collect();
                    json(items)?
                } else {
                    json(service.worlds()?)?
                }
            }
            WorldCommand::Rename { id, name } => service.rename_world(id, &name)?,
            WorldCommand::Update {
                id,
                name,
                description,
                visual_identity,
                artwork,
                version,
            } => {
                service.update_world(
                    id,
                    WorldUpdate {
                        name,
                        description: description.map(Some),
                        visual_identity: visual_identity.map(Some),
                        artwork_identifier: artwork.map(Some),
                    },
                    version,
                )?;
                status(id)?
            }
            WorldCommand::Reorder { ids } => service.reorder_worlds(ids)?,
            WorldCommand::Archive { id, version } => {
                service.archive_world(id, version)?;
                status(id)?
            }
            WorldCommand::Restore { id, version } => {
                service.restore_world(id, version)?;
                status(id)?
            }
            WorldCommand::Delete { id } => service.delete_world(id)?,
        },
        Command::Workspace(command) => match command {
            WorkspaceCommand::Create {
                world,
                name,
                description,
                root_directory,
                visual_identity,
            } => status(service.create_workspace_full(
                world,
                &name,
                description,
                root_directory,
                visual_identity,
            )?)?,
            WorkspaceCommand::List { world, archived } => {
                if archived {
                    let items: Vec<_> = service
                        .archived_items()?
                        .into_iter()
                        .filter(|i| {
                            i.kind == "workspace" && i.parent_id == Some(world.0.to_string())
                        })
                        .collect();
                    json(items)?
                } else {
                    json(service.workspaces(world)?)?
                }
            }
            WorkspaceCommand::Rename { id, name } => service.rename_workspace(id, &name)?,
            WorkspaceCommand::Update {
                id,
                name,
                description,
                root_directory,
                visual_identity,
                version,
            } => {
                service.update_workspace(
                    id,
                    WorkspaceUpdate {
                        name,
                        description: description.map(Some),
                        root_directory: root_directory.map(Some),
                        visual_identity: visual_identity.map(Some),
                    },
                    version,
                )?;
                status(id)?
            }
            WorkspaceCommand::Reorder { world, ids } => service.reorder_workspaces(world, ids)?,
            WorkspaceCommand::Move { id, world } => service.move_workspace(id, world)?,
            WorkspaceCommand::Archive { id, version } => {
                service.archive_workspace(id, version)?;
                status(id)?
            }
            WorkspaceCommand::Restore { id, version } => {
                service.restore_workspace(id, version)?;
                status(id)?
            }
            WorkspaceCommand::Delete { id } => service.delete_workspace(id)?,
        },
        Command::Desktop(command) => match command {
            DesktopCommand::Create {
                workspace,
                name,
                role,
                visual_identity,
            } => status(service.create_desktop_full(workspace, &name, role, visual_identity)?)?,
            DesktopCommand::List {
                workspace,
                archived,
            } => {
                if archived {
                    let items: Vec<_> = service
                        .archived_items()?
                        .into_iter()
                        .filter(|i| {
                            i.kind == "desktop" && i.parent_id == Some(workspace.0.to_string())
                        })
                        .collect();
                    json(items)?
                } else {
                    json(service.desktops(workspace)?)?
                }
            }
            DesktopCommand::Rename { id, name } => service.rename_desktop(id, &name)?,
            DesktopCommand::Update {
                id,
                name,
                role,
                visual_identity,
                version,
            } => {
                service.update_desktop(
                    id,
                    DesktopUpdate {
                        name,
                        role: role.map(Some),
                        visual_identity: visual_identity.map(Some),
                    },
                    version,
                )?;
                status(id)?
            }
            DesktopCommand::Reorder { workspace, ids } => {
                service.reorder_desktops(workspace, ids)?
            }
            DesktopCommand::Archive { id, version } => {
                service.archive_desktop(id, version)?;
                status(id)?
            }
            DesktopCommand::Restore { id, version } => {
                service.restore_desktop(id, version)?;
                status(id)?
            }
            DesktopCommand::Delete { id } => service.delete_desktop(id)?,
        },
        Command::Preferences(command) => match command {
            PreferencesCommand::Get { key } => {
                let val = service.get_preference(&key)?;
                json(serde_json::json!({ "key": key, "value": val }))?
            }
            PreferencesCommand::Set { key, value } => {
                service.set_preference(&key, &value)?;
                json(serde_json::json!({ "key": key, "value": value }))?
            }
            PreferencesCommand::List => json(service.list_preferences()?)?,
        },
        Command::Project(_) => unreachable!(),
        Command::Archived(ArchivedCommand::List) => json(service.archived_items()?)?,
        Command::Manage(ManageCommand::Exec { request }) => {
            return handle_manage_exec(service, request.as_deref());
        }
        Command::Tree => json(service.tree()?)?,
        Command::Doctor
        | Command::Association(_)
        | Command::Init { .. }
        | Command::Status { .. }
        | Command::System(_)
        | Command::Controller(_)
        | Command::Ui(_) => {
            unreachable!()
        }
    }
    Ok(true)
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ManagementRequest {
    #[serde(default)]
    operation_id: Option<String>,
    action: String,
    #[serde(default)]
    entity: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    parent_id: Option<String>,
    #[serde(default)]
    expected_version: Option<i64>,
    #[serde(default)]
    payload: Option<serde_json::Value>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ManagementResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
}

fn handle_manage_exec<S: depthos_core::service::Store>(
    service: &mut depthos_core::service::WorkspaceService<S>,
    raw_request: Option<&str>,
) -> Result<bool> {
    let raw_input = match raw_request {
        Some("-") | None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            buf
        }
        Some(s) => s.to_string(),
    };

    let req: ManagementRequest = match serde_json::from_str(&raw_input) {
        Ok(r) => r,
        Err(e) => {
            let resp = ManagementResponse {
                success: false,
                operation_id: None,
                id: None,
                version: None,
                data: None,
                error: Some(format!("Invalid management request JSON: {e}")),
                code: Some("invalid_request".into()),
            };
            json(resp)?;
            return Ok(false);
        }
    };

    let op_id = req.operation_id.clone();
    let action = req.action.as_str();
    let entity = req.entity.as_deref().unwrap_or("");

    let result: std::result::Result<ManagementResponse, Error> = (|| match action {
        "inspectProject" => {
            let path = req
                .payload
                .as_ref()
                .and_then(|p| p.get("path"))
                .and_then(|v| v.as_str())
                .or(req.id.as_deref())
                .unwrap_or("");
            let inspection = inspect_project_directory(path);
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: None,
                version: None,
                data: Some(serde_json::to_value(inspection)?),
                error: None,
                code: None,
            })
        }
        "getArchived" => {
            let items = service.archived_items()?;
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: None,
                version: None,
                data: Some(serde_json::to_value(items)?),
                error: None,
                code: None,
            })
        }
        "getPreferences" => {
            let prefs = service.get_user_preferences()?;
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: None,
                version: None,
                data: Some(serde_json::to_value(prefs)?),
                error: None,
                code: None,
            })
        }
        "getPreference" => {
            let key = req
                .payload
                .as_ref()
                .and_then(|p| p.get("key"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::InvalidMetadata("missing key in preference request"))?;
            let val = service.get_preference(key)?;
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: None,
                version: None,
                data: Some(serde_json::json!({ "key": key, "value": val })),
                error: None,
                code: None,
            })
        }
        "setPreference" => {
            let p = req
                .payload
                .as_ref()
                .ok_or_else(|| Error::InvalidMetadata("missing payload in setPreference"))?;
            let key = p
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::InvalidMetadata("missing key"))?;
            let value = p
                .get("value")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::InvalidMetadata("missing value"))?;
            service.set_preference(key, value)?;
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: None,
                version: None,
                data: Some(serde_json::json!({ "key": key, "value": value })),
                error: None,
                code: None,
            })
        }
        "saveUserPreferences" => {
            let p = req
                .payload
                .as_ref()
                .ok_or_else(|| Error::InvalidMetadata("missing payload in saveUserPreferences"))?;
            let prefs: depthos_core::domain::UserPreferences = serde_json::from_value(p.clone())?;
            service.save_user_preferences(&prefs)?;
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: None,
                version: None,
                data: None,
                error: None,
                code: None,
            })
        }
        "getAssociationImpact" => {
            let id_str = req
                .id
                .as_deref()
                .ok_or_else(|| Error::InvalidMetadata("missing id"))?;
            let count = match entity {
                "desktop" => {
                    let did = id_str.parse::<DesktopId>()?;
                    service.desktop_association_count(did)?
                }
                "workspace" => {
                    let wid = id_str.parse::<WorkspaceId>()?;
                    service.workspace_association_count(wid)?
                }
                "world" => {
                    let wid = id_str.parse::<WorldId>()?;
                    service.world_association_count(wid)?
                }
                _ => 0,
            };
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: Some(id_str.into()),
                version: None,
                data: Some(
                    serde_json::json!({ "entity": entity, "id": id_str, "ruleCount": count }),
                ),
                error: None,
                code: None,
            })
        }
        "create" => {
            let p = req.payload.as_ref().unwrap_or(&serde_json::Value::Null);
            match entity {
                "world" => {
                    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = p
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let vis = p
                        .get("visualIdentity")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let art = p.get("artwork").and_then(|v| v.as_str()).map(String::from);
                    let id = service.create_world_full(name, desc, vis, art)?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: Some(id.0.to_string()),
                        version: Some(1),
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                "workspace" => {
                    let parent = req
                        .parent_id
                        .as_deref()
                        .ok_or_else(|| Error::InvalidMetadata("missing parentId for workspace"))?
                        .parse::<WorldId>()?;
                    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = p
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let root = p
                        .get("rootDirectory")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let vis = p
                        .get("visualIdentity")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let id = service.create_workspace_full(parent, name, desc, root, vis)?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: Some(id.0.to_string()),
                        version: Some(1),
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                "desktop" => {
                    let parent = req
                        .parent_id
                        .as_deref()
                        .ok_or_else(|| Error::InvalidMetadata("missing parentId for desktop"))?
                        .parse::<WorkspaceId>()?;
                    let name = p.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let role = p.get("role").and_then(|v| v.as_str()).map(String::from);
                    let vis = p
                        .get("visualIdentity")
                        .and_then(|v| v.as_str())
                        .map(String::from);
                    let id = service.create_desktop_full(parent, name, role, vis)?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: Some(id.0.to_string()),
                        version: Some(1),
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                _ => Err(Error::InvalidMetadata("unknown entity for create")),
            }
        }
        "update" => {
            let id_str = req
                .id
                .as_deref()
                .ok_or_else(|| Error::InvalidMetadata("missing id for update"))?;
            let p = req.payload.as_ref().unwrap_or(&serde_json::Value::Null);
            match entity {
                "world" => {
                    let id = id_str.parse::<WorldId>()?;
                    let name = p.get("name").and_then(|v| v.as_str()).map(String::from);
                    let desc = p.get("description").map(|v| v.as_str().map(String::from));
                    let vis = p
                        .get("visualIdentity")
                        .map(|v| v.as_str().map(String::from));
                    let art = p.get("artwork").map(|v| v.as_str().map(String::from));
                    service.update_world(
                        id,
                        WorldUpdate {
                            name,
                            description: desc,
                            visual_identity: vis,
                            artwork_identifier: art,
                        },
                        req.expected_version,
                    )?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: Some(id_str.into()),
                        version: req.expected_version.map(|v| v + 1),
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                "workspace" => {
                    let id = id_str.parse::<WorkspaceId>()?;
                    let name = p.get("name").and_then(|v| v.as_str()).map(String::from);
                    let desc = p.get("description").map(|v| v.as_str().map(String::from));
                    let root = p.get("rootDirectory").map(|v| v.as_str().map(String::from));
                    let vis = p
                        .get("visualIdentity")
                        .map(|v| v.as_str().map(String::from));
                    service.update_workspace(
                        id,
                        WorkspaceUpdate {
                            name,
                            description: desc,
                            root_directory: root,
                            visual_identity: vis,
                        },
                        req.expected_version,
                    )?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: Some(id_str.into()),
                        version: req.expected_version.map(|v| v + 1),
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                "desktop" => {
                    let id = id_str.parse::<DesktopId>()?;
                    let name = p.get("name").and_then(|v| v.as_str()).map(String::from);
                    let role = p.get("role").map(|v| v.as_str().map(String::from));
                    let vis = p
                        .get("visualIdentity")
                        .map(|v| v.as_str().map(String::from));
                    service.update_desktop(
                        id,
                        DesktopUpdate {
                            name,
                            role,
                            visual_identity: vis,
                        },
                        req.expected_version,
                    )?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: Some(id_str.into()),
                        version: req.expected_version.map(|v| v + 1),
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                _ => Err(Error::InvalidMetadata("unknown entity for update")),
            }
        }
        "reorder" => {
            let p = req.payload.as_ref().unwrap_or(&serde_json::Value::Null);
            let ids = p
                .get("ids")
                .and_then(|v| v.as_array())
                .ok_or_else(|| Error::InvalidMetadata("missing ids array in reorder payload"))?;
            match entity {
                "world" => {
                    let world_ids = ids
                        .iter()
                        .filter_map(|v| v.as_str()?.parse::<WorldId>().ok())
                        .collect();
                    service.reorder_worlds(world_ids)?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: None,
                        version: None,
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                "workspace" => {
                    let parent = req
                        .parent_id
                        .as_deref()
                        .ok_or_else(|| Error::InvalidMetadata("missing parentId"))?
                        .parse::<WorldId>()?;
                    let ws_ids = ids
                        .iter()
                        .filter_map(|v| v.as_str()?.parse::<WorkspaceId>().ok())
                        .collect();
                    service.reorder_workspaces(parent, ws_ids)?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: None,
                        version: None,
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                "desktop" => {
                    let parent = req
                        .parent_id
                        .as_deref()
                        .ok_or_else(|| Error::InvalidMetadata("missing parentId"))?
                        .parse::<WorkspaceId>()?;
                    let dt_ids = ids
                        .iter()
                        .filter_map(|v| v.as_str()?.parse::<DesktopId>().ok())
                        .collect();
                    service.reorder_desktops(parent, dt_ids)?;
                    Ok(ManagementResponse {
                        success: true,
                        operation_id: op_id.clone(),
                        id: None,
                        version: None,
                        data: None,
                        error: None,
                        code: None,
                    })
                }
                _ => Err(Error::InvalidMetadata("unknown entity for reorder")),
            }
        }
        "archive" => {
            let id_str = req
                .id
                .as_deref()
                .ok_or_else(|| Error::InvalidMetadata("missing id for archive"))?;
            match entity {
                "world" => {
                    let id = id_str.parse::<WorldId>()?;
                    service.archive_world(id, req.expected_version)?;
                }
                "workspace" => {
                    let id = id_str.parse::<WorkspaceId>()?;
                    service.archive_workspace(id, req.expected_version)?;
                }
                "desktop" => {
                    let id = id_str.parse::<DesktopId>()?;
                    service.archive_desktop(id, req.expected_version)?;
                }
                _ => return Err(Error::InvalidMetadata("unknown entity for archive")),
            }
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: Some(id_str.into()),
                version: req.expected_version.map(|v| v + 1),
                data: None,
                error: None,
                code: None,
            })
        }
        "restore" => {
            let id_str = req
                .id
                .as_deref()
                .ok_or_else(|| Error::InvalidMetadata("missing id for restore"))?;
            match entity {
                "world" => {
                    let id = id_str.parse::<WorldId>()?;
                    service.restore_world(id, req.expected_version)?;
                }
                "workspace" => {
                    let id = id_str.parse::<WorkspaceId>()?;
                    service.restore_workspace(id, req.expected_version)?;
                }
                "desktop" => {
                    let id = id_str.parse::<DesktopId>()?;
                    service.restore_desktop(id, req.expected_version)?;
                }
                _ => return Err(Error::InvalidMetadata("unknown entity for restore")),
            }
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: Some(id_str.into()),
                version: req.expected_version.map(|v| v + 1),
                data: None,
                error: None,
                code: None,
            })
        }
        "delete" => {
            let id_str = req
                .id
                .as_deref()
                .ok_or_else(|| Error::InvalidMetadata("missing id for delete"))?;
            match entity {
                "world" => {
                    let id = id_str.parse::<WorldId>()?;
                    service.delete_world(id)?;
                }
                "workspace" => {
                    let id = id_str.parse::<WorkspaceId>()?;
                    service.delete_workspace(id)?;
                }
                "desktop" => {
                    let id = id_str.parse::<DesktopId>()?;
                    service.delete_desktop(id)?;
                }
                _ => return Err(Error::InvalidMetadata("unknown entity for delete")),
            }
            Ok(ManagementResponse {
                success: true,
                operation_id: op_id.clone(),
                id: Some(id_str.into()),
                version: None,
                data: None,
                error: None,
                code: None,
            })
        }
        _ => Err(Error::InvalidMetadata("unknown management action")),
    })();

    match result {
        Ok(resp) => {
            json(resp)?;
            Ok(true)
        }
        Err(err) => {
            let code = match &err {
                Error::Conflict { .. } => "conflict",
                Error::ParentArchived(_) => "parent_archived",
                Error::AlreadyArchived(_) => "already_archived",
                Error::NotArchived(_) => "not_archived",
                Error::InvalidProjectDirectory(_) => "invalid_project_directory",
                Error::InvalidName => "invalid_name",
                Error::InvalidMetadata(_) => "invalid_metadata",
                Error::Missing(_) => "missing",
                Error::Duplicate(_) => "duplicate",
                Error::HasChildren(_) => "has_children",
                Error::InvalidOrder => "invalid_order",
                _ => "error",
            };
            let resp = ManagementResponse {
                success: false,
                operation_id: op_id,
                id: req.id,
                version: None,
                data: None,
                error: Some(err.to_string()),
                code: Some(code.into()),
            };
            json(resp)?;
            Err(err)
        }
    }
}

fn run_ui(cli: &Cli, command: &UiCommand) -> ExitCode {
    let mut candidate_paths = vec![
        PathBuf::from("tools/depthos-spatial-alpha"),
        PathBuf::from("./tools/depthos-spatial-alpha"),
    ];
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        candidate_paths.push(parent.join("tools/depthos-spatial-alpha"));
        if let Some(grandparent) = parent.parent() {
            candidate_paths.push(grandparent.join("tools/depthos-spatial-alpha"));
            if let Some(great) = grandparent.parent() {
                candidate_paths.push(great.join("tools/depthos-spatial-alpha"));
            }
        }
    }

    let runner = candidate_paths.into_iter().find(|p| p.exists());
    let runner_path = match runner {
        Some(path) => path,
        None => {
            eprintln!("depthos: could not locate tools/depthos-spatial-alpha");
            return ExitCode::FAILURE;
        }
    };

    let mut process = std::process::Command::new(&runner_path);
    if let Ok(exe) = std::env::current_exe() {
        // The launcher probes and initializes only through this binary, so a
        // relocated build does not fall back to a hard-coded source tree.
        process.env("DEPTHOS_BIN", &exe);
    }
    if let Some(db) = &cli.db {
        process.arg("--db").arg(db);
    }
    if command.mock {
        process.arg("--mock");
    }
    if command.reduced_motion {
        process.arg("--reduced-motion");
    }
    if command.offscreen {
        process.arg("--offscreen");
    }
    if command.quit_after_ms > 0 {
        process
            .arg("--quit-after-ms")
            .arg(command.quit_after_ms.to_string());
    }

    match process.status() {
        Ok(status) => {
            if status.success() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("depthos: failed to execute spatial UI runner: {e}");
            ExitCode::FAILURE
        }
    }
}

fn finish(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("depthos: {error}");
            match error {
                // The target exists but is not safe to initialize or use.
                Error::InitRefused(_) => ExitCode::from(3),
                _ => ExitCode::FAILURE,
            }
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Command::Ui(command) = &cli.command {
        return run_ui(&cli, command);
    }
    if let Command::Status { json } = &cli.command {
        return finish(run_status(&cli, *json));
    }
    if let Command::Init { default_hierarchy } = &cli.command {
        return finish(run_init(&cli, *default_hierarchy));
    }
    if let Command::Controller(ControllerCommand::Run { timeout_seconds }) = &cli.command {
        let result = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|_| "Could not start controller runtime".to_owned())
            .and_then(|runtime| {
                let result = runtime.block_on(run_controller(cli.db.as_ref(), *timeout_seconds));
                runtime.shutdown_timeout(std::time::Duration::from_secs(2));
                result
            });
        return match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("depthos: {message}");
                ExitCode::FAILURE
            }
        };
    }
    if let Command::System(command) = &cli.command {
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "Could not start observer runtime".to_owned())
            .and_then(|runtime| runtime.block_on(run_system(command)));
        return match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("depthos: {message}");
                ExitCode::FAILURE
            }
        };
    }
    match run(cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("depthos: {error}");
            match error {
                Error::InvalidName
                | Error::InvalidMetadata(_)
                | Error::Missing(_)
                | Error::Duplicate(_)
                | Error::HasChildren(_)
                | Error::InvalidOrder
                | Error::InvalidProjectDirectory(_)
                | Error::AlreadyArchived(_)
                | Error::NotArchived(_)
                | Error::ParentArchived(_) => ExitCode::from(2),
                Error::Conflict { .. } => ExitCode::from(4),
                _ => ExitCode::FAILURE,
            }
        }
    }
}

async fn run_system(command: &SystemCommand) -> std::result::Result<(), String> {
    let options = match command {
        SystemCommand::Health { options }
        | SystemCommand::Inspect { options }
        | SystemCommand::Snapshot { options, .. } => options,
    };
    let receiver = match depthos_observer::Receiver::session().await {
        Ok(receiver) => receiver,
        Err(error) if error.is_name_taken() => return Err(
            "Another receiver owns org.depthos.StateInspector (possibly the Python inspector). Stop it before launching depthos system; its name was not taken.".into()),
        Err(_) => {
            json(serde_json::json!({"health":"Disconnected","snapshotAvailable":false,"reason":"session-bus-unavailable"})).map_err(|_| "Could not write diagnostic output")?;
            return Ok(());
        }
    };
    let source = receiver.observer();
    let mut changes = source.subscribe();
    eprintln!(
        "Waiting for an explicitly launched KWin observer; no script is loaded by this command."
    );
    let deadline = tokio::time::sleep(std::time::Duration::from_secs(options.timeout_seconds));
    tokio::pin!(deadline);
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|_| "Could not register shutdown signal")?;
    let result = loop {
        let observation = source.snapshot();
        let health = match &observation {
            Ok(_) => depthos_observer::ObservationHealth::Synchronized,
            Err(depthos_observer::ObservationError::Unavailable(health)) => *health,
            Err(_) => depthos_observer::ObservationHealth::Desynchronized,
        };
        if let Ok(mut snapshot) = observation {
            match command {
                SystemCommand::Snapshot { .. } => {
                    snapshot.redact_titles();
                    break json(snapshot).map_err(|_| "Could not write snapshot".to_owned());
                }
                SystemCommand::Health { .. } => break json(serde_json::json!({"health":"Synchronized","snapshotAvailable":true,"sessionId":snapshot.session_id,"sequence":snapshot.sequence,"revision":snapshot.revision})).map_err(|_| "Could not write health".to_owned()),
                SystemCommand::Inspect { .. } => {
                    if json(serde_json::json!({"health":"Synchronized","sessionId":snapshot.session_id,"sequence":snapshot.sequence,"revision":snapshot.revision,"windows":snapshot.state.windows.len(),"outputs":snapshot.state.screens.as_ref().map(Vec::len)})).is_err() {
                        break Err("Could not write inspection".into());
                    }
                }
            }
        } else if matches!(command, SystemCommand::Inspect { .. })
            && json(serde_json::json!({"health":health,"snapshotAvailable":false})).is_err()
        {
            break Err("Could not write inspection".into());
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break Ok(()),
            _ = terminate.recv() => break Ok(()),
            _ = &mut deadline => {
                let (health, available) = match source.snapshot() {
                    Ok(_) => (depthos_observer::ObservationHealth::Synchronized, true),
                    Err(depthos_observer::ObservationError::Unavailable(health)) => (health, false),
                    Err(_) => (depthos_observer::ObservationHealth::Desynchronized, false),
                };
                break json(serde_json::json!({"health":health,"snapshotAvailable":available,"reason":"observation-window-ended"})).map_err(|_| "Could not write health".to_owned());
            },
            changed = changes.changed() => if changed.is_err() { break Ok(()); },
        }
    };
    receiver
        .shutdown()
        .await
        .map_err(|_| "Observer shutdown failed".to_owned())?;
    result
}

async fn run_controller(
    database: Option<&PathBuf>,
    timeout: Option<u64>,
) -> std::result::Result<(), String> {
    use depthos_core::controller::{Controller, ControllerOptions, Lifecycle, RecoveryReason};
    let path = database
        .cloned()
        .map_or_else(default_path, Ok)
        .map_err(|err| err.to_string())?;
    let store = SqliteStore::open_read_only(&path).map_err(|err| err.to_string())?;
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|_| "Could not register shutdown signal".to_owned())?;
    let controller = Controller::start(store, ControllerOptions::default())
        .map_err(|_| "Could not start controller".to_owned())?;
    let mut bridge = match depthos_core::spatial::UiBridge::session(controller.clone()).await {
        Ok(bridge) => Some(bridge),
        Err(depthos_core::controller::ControllerError::Observation(error))
            if error.is_name_taken() =>
        {
            let _ = controller.shutdown().await;
            return Err("Another UI bridge owns org.depthos.Controller".into());
        }
        Err(_) => None,
    };
    let mut changes = controller.subscribe(); // before first status read
    eprintln!("Foreground DepthOS controller; no KWin script is loaded. Ctrl+C stops it.");
    let deadline = async {
        match timeout {
            Some(seconds) => tokio::time::sleep(std::time::Duration::from_secs(seconds)).await,
            None => std::future::pending::<()>().await,
        }
    };
    tokio::pin!(deadline);
    let mut displayed = None;
    let mut retried_bridge = false;
    let result = loop {
        let status = controller.last_health();
        if bridge.is_none() && status.receiver_connected && !retried_bridge {
            retried_bridge = true;
            match depthos_core::spatial::UiBridge::session(controller.clone()).await {
                Ok(connected) => bridge = Some(connected),
                Err(_) => break Err("UI bridge unavailable after receiver recovery".into()),
            }
        }
        let summary = (
            status.lifecycle,
            status.reason,
            status.observation,
            status.receiver_attempts,
            status.database_available,
        );
        if displayed != Some(summary) {
            if json(&status).is_err() {
                break Err("Could not write controller status".into());
            }
            displayed = Some(summary);
        }
        if status.reason == RecoveryReason::ReceiverConflict {
            break Err("Another receiver owns org.depthos.StateInspector. Stop it before launching the controller; its name was not taken.".into());
        }
        if status.reason == RecoveryReason::WorkerLost {
            break Err("Controller worker was interrupted".into());
        }
        if status.lifecycle == Lifecycle::Stopped {
            break Ok(());
        }
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break Ok(()),
            _ = terminate.recv() => break Ok(()),
            _ = async {bridge.as_ref().unwrap().closed().await}, if bridge.is_some() => break Err("UI bridge connection closed".into()),
            _ = &mut deadline => break Ok(()),
            changed = changes.changed() => {if changed.is_err() {break Err("Controller worker stopped unexpectedly".into());}}
        }
    };
    let bridge_clean = match bridge.take() {
        Some(bridge) => matches!(
            tokio::time::timeout(std::time::Duration::from_secs(2), bridge.shutdown()).await,
            Ok(Ok(()))
        ),
        None => true,
    };
    controller
        .shutdown()
        .await
        .map_err(|_| "Controller shutdown failed".to_owned())?;
    json(controller.last_health())
        .map_err(|_| "Could not write final controller status".to_owned())?;
    if !bridge_clean {
        return Err("UI bridge shutdown failed".into());
    }
    result
}
