use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use depthos_core::{
    application::Application,
    domain::{DesktopId, Error, Result, WorkspaceId, WorldId},
    observed::ObservedStateSource,
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
    /// Check schema version, foreign keys and database integrity.
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
    },
    List,
    Rename {
        id: WorldId,
        name: String,
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
    },
    List {
        #[arg(long)]
        world: WorldId,
    },
    Rename {
        id: WorkspaceId,
        name: String,
    },
    Move {
        id: WorkspaceId,
        #[arg(long)]
        world: WorldId,
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
    },
    List {
        #[arg(long)]
        workspace: WorkspaceId,
    },
    Rename {
        id: DesktopId,
        name: String,
    },
    /// Supply all desktop IDs in their desired order.
    Reorder {
        #[arg(long)]
        workspace: WorkspaceId,
        #[arg(required = true)]
        ids: Vec<DesktopId>,
    },
    Delete {
        id: DesktopId,
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

fn run(cli: Cli) -> Result<bool> {
    let store = match (&cli.command, &cli.db) {
        // Read-only inspector: must never create or migrate user data.
        (Command::Tree, _) => SqliteStore::open_read_only(&database_path(&cli)?)?,
        (_, Some(path)) => SqliteStore::open(path)?,
        (_, None) => SqliteStore::open_default()?,
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
            WorldCommand::Create { name, description } => {
                status(service.create_world(&name, description)?)?
            }
            WorldCommand::List => json(service.worlds()?)?,
            WorldCommand::Rename { id, name } => service.rename_world(id, &name)?,
            WorldCommand::Delete { id } => service.delete_world(id)?,
        },
        Command::Workspace(command) => match command {
            WorkspaceCommand::Create {
                world,
                name,
                description,
                root_directory,
            } => status(service.create_workspace(world, &name, description, root_directory)?)?,
            WorkspaceCommand::List { world } => json(service.workspaces(world)?)?,
            WorkspaceCommand::Rename { id, name } => service.rename_workspace(id, &name)?,
            WorkspaceCommand::Move { id, world } => service.move_workspace(id, world)?,
            WorkspaceCommand::Delete { id } => service.delete_workspace(id)?,
        },
        Command::Desktop(command) => match command {
            DesktopCommand::Create {
                workspace,
                name,
                role,
            } => status(service.create_desktop(workspace, &name, role)?)?,
            DesktopCommand::List { workspace } => json(service.desktops(workspace)?)?,
            DesktopCommand::Rename { id, name } => service.rename_desktop(id, &name)?,
            DesktopCommand::Reorder { workspace, ids } => {
                service.reorder_desktops(workspace, ids)?
            }
            DesktopCommand::Delete { id } => service.delete_desktop(id)?,
        },
        Command::Tree => json(service.tree()?)?,
        Command::Doctor
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
    candidate_paths.push(PathBuf::from(
        "/home/ashenix/Projects/DepthOS-phase-4-1/tools/depthos-spatial-alpha",
    ));

    let runner = candidate_paths.into_iter().find(|p| p.exists());
    let runner_path = match runner {
        Some(path) => path,
        None => {
            eprintln!("depthos: could not locate tools/depthos-spatial-alpha");
            return ExitCode::FAILURE;
        }
    };

    let mut process = std::process::Command::new(&runner_path);
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
                | Error::InvalidOrder => ExitCode::from(2),
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
    let store = match database {
        Some(path) => SqliteStore::open(path),
        None => SqliteStore::open_default(),
    }
    .map_err(|_| "Could not open logical workspace database".to_owned())?;
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
