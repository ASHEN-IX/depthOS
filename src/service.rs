use std::{collections::HashSet, path::Path};

use uuid::Uuid;

use crate::domain::*;

/// Storage applies one complete change per transaction and reads a consistent
/// hierarchy snapshot. Commands below are constructed by the service.
pub trait Store {
    fn apply(&mut self, change: Change) -> Result<()>;
    fn worlds(&self) -> Result<Vec<World>>;
    fn workspaces(&self, world: WorldId) -> Result<Vec<Workspace>>;
    fn desktops(&self, workspace: WorkspaceId) -> Result<Vec<LogicalDesktop>>;
    fn tree(&self) -> Result<Universe>;
}

pub enum Change {
    CreateWorld(WorldId, String, Option<String>),
    CreateWorkspace(WorkspaceId, WorldId, String, Option<String>, Option<String>),
    CreateDesktop(DesktopId, WorkspaceId, String, Option<String>),
    RenameWorld(WorldId, String),
    RenameWorkspace(WorkspaceId, String),
    RenameDesktop(DesktopId, String),
    MoveWorkspace(WorkspaceId, WorldId),
    ReorderDesktops(WorkspaceId, Vec<DesktopId>),
    DeleteWorld(WorldId),
    DeleteWorkspace(WorkspaceId),
    DeleteDesktop(DesktopId),
}

pub struct WorkspaceService<S> {
    store: S,
}

fn name(value: &str) -> Result<String> {
    if value.chars().any(char::is_control) {
        return Err(Error::InvalidName);
    }
    let trimmed = value.trim();
    if !(1..=120).contains(&trimmed.chars().count()) {
        return Err(Error::InvalidName);
    }
    Ok(trimmed.to_owned())
}

fn metadata(value: Option<String>, field: &'static str) -> Result<Option<String>> {
    if value
        .as_ref()
        .is_some_and(|s| s.len() > 4096 || s.chars().any(char::is_control))
    {
        return Err(Error::InvalidMetadata(field));
    }
    Ok(value)
}

impl<S: Store> WorkspaceService<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }
    pub fn create_world(&mut self, label: &str, description: Option<String>) -> Result<WorldId> {
        let id = WorldId(Uuid::new_v4());
        self.store.apply(Change::CreateWorld(
            id,
            name(label)?,
            metadata(description, "description")?,
        ))?;
        Ok(id)
    }
    pub fn create_workspace(
        &mut self,
        world: WorldId,
        label: &str,
        description: Option<String>,
        root: Option<String>,
    ) -> Result<WorkspaceId> {
        let root = metadata(root, "root directory")?;
        if root
            .as_ref()
            .is_some_and(|value| !Path::new(value).is_absolute())
        {
            return Err(Error::InvalidMetadata(
                "root directory (expected an absolute path)",
            ));
        }
        let id = WorkspaceId(Uuid::new_v4());
        self.store.apply(Change::CreateWorkspace(
            id,
            world,
            name(label)?,
            metadata(description, "description")?,
            root,
        ))?;
        Ok(id)
    }
    pub fn create_desktop(
        &mut self,
        workspace: WorkspaceId,
        label: &str,
        role: Option<String>,
    ) -> Result<DesktopId> {
        let id = DesktopId(Uuid::new_v4());
        self.store.apply(Change::CreateDesktop(
            id,
            workspace,
            name(label)?,
            metadata(role, "role")?,
        ))?;
        Ok(id)
    }
    pub fn rename_world(&mut self, id: WorldId, label: &str) -> Result<()> {
        self.store.apply(Change::RenameWorld(id, name(label)?))
    }
    pub fn rename_workspace(&mut self, id: WorkspaceId, label: &str) -> Result<()> {
        self.store.apply(Change::RenameWorkspace(id, name(label)?))
    }
    pub fn rename_desktop(&mut self, id: DesktopId, label: &str) -> Result<()> {
        self.store.apply(Change::RenameDesktop(id, name(label)?))
    }
    pub fn move_workspace(&mut self, id: WorkspaceId, world: WorldId) -> Result<()> {
        self.store.apply(Change::MoveWorkspace(id, world))
    }
    pub fn reorder_desktops(&mut self, workspace: WorkspaceId, ids: Vec<DesktopId>) -> Result<()> {
        if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            return Err(Error::InvalidOrder);
        }
        self.store.apply(Change::ReorderDesktops(workspace, ids))
    }
    pub fn delete_world(&mut self, id: WorldId) -> Result<()> {
        self.store.apply(Change::DeleteWorld(id))
    }
    pub fn delete_workspace(&mut self, id: WorkspaceId) -> Result<()> {
        self.store.apply(Change::DeleteWorkspace(id))
    }
    pub fn delete_desktop(&mut self, id: DesktopId) -> Result<()> {
        self.store.apply(Change::DeleteDesktop(id))
    }
    pub fn worlds(&self) -> Result<Vec<World>> {
        self.store.worlds()
    }
    pub fn workspaces(&self, world: WorldId) -> Result<Vec<Workspace>> {
        self.store.workspaces(world)
    }
    pub fn desktops(&self, workspace: WorkspaceId) -> Result<Vec<LogicalDesktop>> {
        self.store.desktops(workspace)
    }
    pub(crate) fn store(&self) -> &S {
        &self.store
    }
    pub fn tree(&self) -> Result<Universe> {
        self.store.tree()
    }
}
