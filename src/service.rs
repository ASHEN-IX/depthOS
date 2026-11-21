use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::*;

/// Storage applies one complete change per transaction and reads a consistent
/// hierarchy snapshot. Commands below are constructed by the service.
pub trait Store {
    fn apply(&mut self, change: Change) -> Result<()>;

    fn apply_with_audit(&mut self, change: Change, _operation_id: Option<String>) -> Result<()> {
        self.apply(change)
    }

    fn worlds(&self) -> Result<Vec<World>>;
    fn workspaces(&self, world: WorldId) -> Result<Vec<Workspace>>;
    fn desktops(&self, workspace: WorkspaceId) -> Result<Vec<LogicalDesktop>>;
    fn tree(&self) -> Result<Universe>;

    fn archived_items(&self) -> Result<Vec<ArchivedItem>> {
        Ok(Vec::new())
    }

    fn get_preference(&self, _key: &str) -> Result<Option<String>> {
        Ok(None)
    }

    fn set_preference(&mut self, _key: &str, _value: &str) -> Result<()> {
        Ok(())
    }

    fn list_preferences(&self) -> Result<Vec<(String, String)>> {
        Ok(Vec::new())
    }

    fn desktop_association_count(&self, _desktop_id: DesktopId) -> Result<usize> {
        Ok(0)
    }

    fn workspace_association_count(&self, _workspace_id: WorkspaceId) -> Result<usize> {
        Ok(0)
    }

    fn world_association_count(&self, _world_id: WorldId) -> Result<usize> {
        Ok(0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldUpdate {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub visual_identity: Option<Option<String>>,
    pub artwork_identifier: Option<Option<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceUpdate {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub root_directory: Option<Option<String>>,
    pub visual_identity: Option<Option<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DesktopUpdate {
    pub name: Option<String>,
    pub role: Option<Option<String>>,
    pub visual_identity: Option<Option<String>>,
}

pub enum Change {
    CreateWorld(WorldId, String, Option<String>),
    CreateWorldFull {
        id: WorldId,
        name: String,
        description: Option<String>,
        visual_identity: Option<String>,
        artwork_identifier: Option<String>,
    },
    UpdateWorld {
        id: WorldId,
        update: WorldUpdate,
        expected_version: Option<i64>,
    },
    ReorderWorlds(Vec<WorldId>),
    ArchiveWorld {
        id: WorldId,
        expected_version: Option<i64>,
    },
    RestoreWorld {
        id: WorldId,
        expected_version: Option<i64>,
    },

    CreateWorkspace(WorkspaceId, WorldId, String, Option<String>, Option<String>),
    CreateWorkspaceFull {
        id: WorkspaceId,
        world_id: WorldId,
        name: String,
        description: Option<String>,
        root_directory: Option<String>,
        visual_identity: Option<String>,
    },
    UpdateWorkspace {
        id: WorkspaceId,
        update: WorkspaceUpdate,
        expected_version: Option<i64>,
    },
    ReorderWorkspaces(WorldId, Vec<WorkspaceId>),
    ArchiveWorkspace {
        id: WorkspaceId,
        expected_version: Option<i64>,
    },
    RestoreWorkspace {
        id: WorkspaceId,
        expected_version: Option<i64>,
    },

    CreateDesktop(DesktopId, WorkspaceId, String, Option<String>),
    CreateDesktopFull {
        id: DesktopId,
        workspace_id: WorkspaceId,
        name: String,
        role: Option<String>,
        visual_identity: Option<String>,
    },
    UpdateDesktop {
        id: DesktopId,
        update: DesktopUpdate,
        expected_version: Option<i64>,
    },
    ReorderDesktops(WorkspaceId, Vec<DesktopId>),
    ArchiveDesktop {
        id: DesktopId,
        expected_version: Option<i64>,
    },
    RestoreDesktop {
        id: DesktopId,
        expected_version: Option<i64>,
    },

    RenameWorld(WorldId, String),
    RenameWorkspace(WorkspaceId, String),
    RenameDesktop(DesktopId, String),
    MoveWorkspace(WorkspaceId, WorldId),
    DeleteWorld(WorldId),
    DeleteWorkspace(WorkspaceId),
    DeleteDesktop(DesktopId),

    SetPreference(String, String),
    DeletePreference(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitMetadata {
    pub is_repository: bool,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub is_detached: bool,
    pub is_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInspection {
    pub path: String,
    pub is_absolute: bool,
    pub exists: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub canonical_path: Option<String>,
    pub is_readable: bool,
    pub is_writable: bool,
    pub git: Option<GitMetadata>,
    pub status: String,
    pub warning: Option<String>,
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

pub fn inspect_git_repository(root: &Path) -> Option<GitMetadata> {
    let git_dir = root.join(".git");
    let actual_git_dir = if git_dir.is_file() {
        let content = std::fs::read_to_string(&git_dir).ok()?;
        let line = content.lines().next()?;
        let rest = line.strip_prefix("gitdir:")?;
        let p = Path::new(rest.trim());
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            root.join(p)
        }
    } else if git_dir.is_dir() {
        git_dir
    } else {
        return None;
    };

    let head_path = actual_git_dir.join("HEAD");
    let head_content = std::fs::read_to_string(&head_path).ok()?;
    let trimmed = head_content.trim();

    let mut branch = None;
    let mut commit = None;
    let mut is_detached = false;

    if let Some(branch_ref) = trimmed.strip_prefix("ref: refs/heads/") {
        branch = Some(branch_ref.to_owned());
        let ref_path = actual_git_dir.join("refs/heads").join(branch_ref);
        if let Ok(c) = std::fs::read_to_string(&ref_path) {
            let c_trimmed = c.trim();
            if c_trimmed.len() >= 7 {
                commit = Some(c_trimmed[..7].to_owned());
            }
        }
    } else if trimmed.len() >= 7 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        is_detached = true;
        commit = Some(trimmed[..7].to_owned());
    }

    Some(GitMetadata {
        is_repository: true,
        branch,
        commit,
        is_detached,
        is_dirty: false,
    })
}

pub fn inspect_project_directory(path_str: &str) -> ProjectInspection {
    let trimmed = path_str.trim();
    let path = PathBuf::from(trimmed);
    let is_absolute = path.is_absolute();

    if trimmed.is_empty() {
        return ProjectInspection {
            path: trimmed.to_owned(),
            is_absolute,
            exists: false,
            is_directory: false,
            is_symlink: false,
            canonical_path: None,
            is_readable: false,
            is_writable: false,
            git: None,
            status: "empty_path".into(),
            warning: Some("Path cannot be empty".into()),
        };
    }

    if !is_absolute {
        return ProjectInspection {
            path: trimmed.to_owned(),
            is_absolute: false,
            exists: false,
            is_directory: false,
            is_symlink: false,
            canonical_path: None,
            is_readable: false,
            is_writable: false,
            git: None,
            status: "not_absolute".into(),
            warning: Some("Expected an absolute directory path".into()),
        };
    }

    let symlink_meta = std::fs::symlink_metadata(&path);
    let exists = symlink_meta.is_ok();
    let is_symlink = symlink_meta
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    let (is_directory, canonical_path, is_readable, is_writable, git, status, warning) = if exists {
        let meta = std::fs::metadata(&path);
        let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
        let canonical = path
            .canonicalize()
            .ok()
            .map(|p| p.to_string_lossy().into_owned());
        let readable = std::fs::read_dir(&path).is_ok();
        let writable = std::fs::metadata(&path)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false);
        let git_meta = if is_dir {
            inspect_git_repository(&path)
        } else {
            None
        };

        if !is_dir {
            (
                is_dir,
                canonical,
                readable,
                writable,
                git_meta,
                "not_directory".into(),
                Some("Path exists but is not a directory".into()),
            )
        } else if !readable {
            (
                is_dir,
                canonical,
                false,
                writable,
                git_meta,
                "permission_denied".into(),
                Some("Directory cannot be read (permission denied)".into()),
            )
        } else {
            (
                is_dir,
                canonical,
                true,
                writable,
                git_meta,
                "valid".into(),
                None,
            )
        }
    } else {
        (
            false,
            None,
            false,
            false,
            None,
            "nonexistent".into(),
            Some("Directory does not exist".into()),
        )
    };

    ProjectInspection {
        path: trimmed.to_owned(),
        is_absolute,
        exists,
        is_directory,
        is_symlink,
        canonical_path,
        is_readable,
        is_writable,
        git,
        status,
        warning,
    }
}

impl<S: Store> WorkspaceService<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &S {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    pub fn create_world(&mut self, label: &str, description: Option<String>) -> Result<WorldId> {
        self.create_world_full(label, description, None, None)
    }

    pub fn create_world_full(
        &mut self,
        label: &str,
        description: Option<String>,
        visual_identity: Option<String>,
        artwork_identifier: Option<String>,
    ) -> Result<WorldId> {
        let id = WorldId(Uuid::new_v4());
        self.store.apply(Change::CreateWorldFull {
            id,
            name: name(label)?,
            description: metadata(description, "description")?,
            visual_identity: metadata(visual_identity, "visual identity")?,
            artwork_identifier: metadata(artwork_identifier, "artwork identifier")?,
        })?;
        Ok(id)
    }

    pub fn update_world(
        &mut self,
        id: WorldId,
        update: WorldUpdate,
        expected_version: Option<i64>,
    ) -> Result<()> {
        let cleaned_name = match update.name {
            Some(ref n) => Some(name(n)?),
            None => None,
        };
        let cleaned_desc = match update.description {
            Some(Some(ref d)) => Some(metadata(Some(d.clone()), "description")?),
            Some(None) => Some(None),
            None => None,
        };
        let cleaned_vis = match update.visual_identity {
            Some(Some(ref v)) => Some(metadata(Some(v.clone()), "visual identity")?),
            Some(None) => Some(None),
            None => None,
        };
        let cleaned_art = match update.artwork_identifier {
            Some(Some(ref a)) => Some(metadata(Some(a.clone()), "artwork identifier")?),
            Some(None) => Some(None),
            None => None,
        };
        self.store.apply(Change::UpdateWorld {
            id,
            update: WorldUpdate {
                name: cleaned_name,
                description: cleaned_desc,
                visual_identity: cleaned_vis,
                artwork_identifier: cleaned_art,
            },
            expected_version,
        })
    }

    pub fn reorder_worlds(&mut self, ids: Vec<WorldId>) -> Result<()> {
        if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            return Err(Error::InvalidOrder);
        }
        self.store.apply(Change::ReorderWorlds(ids))
    }

    pub fn archive_world(&mut self, id: WorldId, expected_version: Option<i64>) -> Result<()> {
        self.store.apply(Change::ArchiveWorld {
            id,
            expected_version,
        })
    }

    pub fn restore_world(&mut self, id: WorldId, expected_version: Option<i64>) -> Result<()> {
        self.store.apply(Change::RestoreWorld {
            id,
            expected_version,
        })
    }

    pub fn create_workspace(
        &mut self,
        world: WorldId,
        label: &str,
        description: Option<String>,
        root: Option<String>,
    ) -> Result<WorkspaceId> {
        self.create_workspace_full(world, label, description, root, None)
    }

    pub fn create_workspace_full(
        &mut self,
        world: WorldId,
        label: &str,
        description: Option<String>,
        root: Option<String>,
        visual_identity: Option<String>,
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
        self.store.apply(Change::CreateWorkspaceFull {
            id,
            world_id: world,
            name: name(label)?,
            description: metadata(description, "description")?,
            root_directory: root,
            visual_identity: metadata(visual_identity, "visual identity")?,
        })?;
        Ok(id)
    }

    pub fn update_workspace(
        &mut self,
        id: WorkspaceId,
        update: WorkspaceUpdate,
        expected_version: Option<i64>,
    ) -> Result<()> {
        let cleaned_name = match update.name {
            Some(ref n) => Some(name(n)?),
            None => None,
        };
        let cleaned_desc = match update.description {
            Some(Some(ref d)) => Some(metadata(Some(d.clone()), "description")?),
            Some(None) => Some(None),
            None => None,
        };
        let cleaned_root = match update.root_directory {
            Some(Some(ref r)) => {
                let m = metadata(Some(r.clone()), "root directory")?;
                if m.as_ref().is_some_and(|v| !Path::new(v).is_absolute()) {
                    return Err(Error::InvalidMetadata(
                        "root directory (expected an absolute path)",
                    ));
                }
                Some(m)
            }
            Some(None) => Some(None),
            None => None,
        };
        let cleaned_vis = match update.visual_identity {
            Some(Some(ref v)) => Some(metadata(Some(v.clone()), "visual identity")?),
            Some(None) => Some(None),
            None => None,
        };
        self.store.apply(Change::UpdateWorkspace {
            id,
            update: WorkspaceUpdate {
                name: cleaned_name,
                description: cleaned_desc,
                root_directory: cleaned_root,
                visual_identity: cleaned_vis,
            },
            expected_version,
        })
    }

    pub fn reorder_workspaces(&mut self, world: WorldId, ids: Vec<WorkspaceId>) -> Result<()> {
        if ids.iter().collect::<HashSet<_>>().len() != ids.len() {
            return Err(Error::InvalidOrder);
        }
        self.store.apply(Change::ReorderWorkspaces(world, ids))
    }

    pub fn archive_workspace(
        &mut self,
        id: WorkspaceId,
        expected_version: Option<i64>,
    ) -> Result<()> {
        self.store.apply(Change::ArchiveWorkspace {
            id,
            expected_version,
        })
    }

    pub fn restore_workspace(
        &mut self,
        id: WorkspaceId,
        expected_version: Option<i64>,
    ) -> Result<()> {
        self.store.apply(Change::RestoreWorkspace {
            id,
            expected_version,
        })
    }

    pub fn create_desktop(
        &mut self,
        workspace: WorkspaceId,
        label: &str,
        role: Option<String>,
    ) -> Result<DesktopId> {
        self.create_desktop_full(workspace, label, role, None)
    }

    pub fn create_desktop_full(
        &mut self,
        workspace: WorkspaceId,
        label: &str,
        role: Option<String>,
        visual_identity: Option<String>,
    ) -> Result<DesktopId> {
        let id = DesktopId(Uuid::new_v4());
        self.store.apply(Change::CreateDesktopFull {
            id,
            workspace_id: workspace,
            name: name(label)?,
            role: metadata(role, "role")?,
            visual_identity: metadata(visual_identity, "visual identity")?,
        })?;
        Ok(id)
    }

    pub fn update_desktop(
        &mut self,
        id: DesktopId,
        update: DesktopUpdate,
        expected_version: Option<i64>,
    ) -> Result<()> {
        let cleaned_name = match update.name {
            Some(ref n) => Some(name(n)?),
            None => None,
        };
        let cleaned_role = match update.role {
            Some(Some(ref r)) => Some(metadata(Some(r.clone()), "role")?),
            Some(None) => Some(None),
            None => None,
        };
        let cleaned_vis = match update.visual_identity {
            Some(Some(ref v)) => Some(metadata(Some(v.clone()), "visual identity")?),
            Some(None) => Some(None),
            None => None,
        };
        self.store.apply(Change::UpdateDesktop {
            id,
            update: DesktopUpdate {
                name: cleaned_name,
                role: cleaned_role,
                visual_identity: cleaned_vis,
            },
            expected_version,
        })
    }

    pub fn archive_desktop(&mut self, id: DesktopId, expected_version: Option<i64>) -> Result<()> {
        self.store.apply(Change::ArchiveDesktop {
            id,
            expected_version,
        })
    }

    pub fn restore_desktop(&mut self, id: DesktopId, expected_version: Option<i64>) -> Result<()> {
        self.store.apply(Change::RestoreDesktop {
            id,
            expected_version,
        })
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

    pub fn tree(&self) -> Result<Universe> {
        self.store.tree()
    }

    pub fn archived_items(&self) -> Result<Vec<ArchivedItem>> {
        self.store.archived_items()
    }

    pub fn get_preference(&self, key: &str) -> Result<Option<String>> {
        self.store.get_preference(key)
    }

    pub fn set_preference(&mut self, key: &str, value: &str) -> Result<()> {
        self.store.set_preference(key, value)
    }

    pub fn list_preferences(&self) -> Result<Vec<(String, String)>> {
        self.store.list_preferences()
    }

    pub fn get_user_preferences(&self) -> Result<UserPreferences> {
        let mut prefs = UserPreferences::default();
        if let Some(val) = self.store.get_preference("presentation.reduced_motion")? {
            prefs.reduced_motion = val == "true" || val == "1";
        }
        if let Some(val) = self.store.get_preference("presentation.low_effects")? {
            prefs.low_effects = val == "true" || val == "1";
        }
        if let Some(val) = self.store.get_preference("presentation.high_contrast")? {
            prefs.high_contrast = val == "true" || val == "1";
        }
        if let Some(val) = self.store.get_preference("presentation.show_titles")? {
            prefs.show_titles = val == "true" || val == "1";
        }
        if let Some(val) = self.store.get_preference("presentation.accent_intensity")? {
            prefs.accent_intensity = val;
        }
        if let Some(val) = self
            .store
            .get_preference("presentation.default_landing_view")?
        {
            prefs.default_landing_view = val;
        }
        if let Some(val) = self.store.get_preference("context.last_world_id")?
            && let Ok(id) = val.parse::<WorldId>()
        {
            prefs.last_world_id = Some(id);
        }
        if let Some(val) = self.store.get_preference("context.last_workspace_id")?
            && let Ok(id) = val.parse::<WorkspaceId>()
        {
            prefs.last_workspace_id = Some(id);
        }
        if let Some(val) = self.store.get_preference("context.last_desktop_id")?
            && let Ok(id) = val.parse::<DesktopId>()
        {
            prefs.last_desktop_id = Some(id);
        }
        Ok(prefs)
    }

    pub fn save_user_preferences(&mut self, prefs: &UserPreferences) -> Result<()> {
        self.store.set_preference(
            "presentation.reduced_motion",
            if prefs.reduced_motion {
                "true"
            } else {
                "false"
            },
        )?;
        self.store.set_preference(
            "presentation.low_effects",
            if prefs.low_effects { "true" } else { "false" },
        )?;
        self.store.set_preference(
            "presentation.high_contrast",
            if prefs.high_contrast { "true" } else { "false" },
        )?;
        self.store.set_preference(
            "presentation.show_titles",
            if prefs.show_titles { "true" } else { "false" },
        )?;
        if !prefs.accent_intensity.is_empty() {
            self.store
                .set_preference("presentation.accent_intensity", &prefs.accent_intensity)?;
        }
        if !prefs.default_landing_view.is_empty() {
            self.store.set_preference(
                "presentation.default_landing_view",
                &prefs.default_landing_view,
            )?;
        }
        if let Some(id) = prefs.last_world_id {
            self.store
                .set_preference("context.last_world_id", &id.0.to_string())?;
        }
        if let Some(id) = prefs.last_workspace_id {
            self.store
                .set_preference("context.last_workspace_id", &id.0.to_string())?;
        }
        if let Some(id) = prefs.last_desktop_id {
            self.store
                .set_preference("context.last_desktop_id", &id.0.to_string())?;
        }
        Ok(())
    }

    pub fn desktop_association_count(&self, desktop_id: DesktopId) -> Result<usize> {
        self.store.desktop_association_count(desktop_id)
    }

    pub fn workspace_association_count(&self, workspace_id: WorkspaceId) -> Result<usize> {
        self.store.workspace_association_count(workspace_id)
    }

    pub fn world_association_count(&self, world_id: WorldId) -> Result<usize> {
        self.store.world_association_count(world_id)
    }
}
