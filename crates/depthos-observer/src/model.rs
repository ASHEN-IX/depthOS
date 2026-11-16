use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub struct Geometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Classification {
    Application,
    Dialog,
    Utility,
    System,
    Unknown,
}
/// Private metadata intentionally does not implement Debug.
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub runtime_id: String,
    pub native_id: Option<String>,
    pub windowing_system: Option<String>,
    pub title: Option<String>,
    pub application_id: Option<String>,
    pub resource_class: Option<String>,
    pub resource_name: Option<String>,
    pub classification: Classification,
    pub type_flags: BTreeMap<String, Option<bool>>,
    pub parent_runtime_id: Option<String>,
    pub desktop_ids: Option<Vec<String>>,
    pub activity_ids: Option<Vec<String>>,
    pub on_all_desktops: Option<bool>,
    pub on_all_activities: Option<bool>,
    pub geometry: Option<Geometry>,
    pub screen_id: Option<String>,
    pub minimized: Option<bool>,
    pub fullscreen: Option<bool>,
    pub skip_taskbar: Option<bool>,
    pub exclude_from_capture: Option<bool>,
    pub truncated_fields: Vec<String>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Desktop {
    pub id: String,
    pub native_id: Option<String>,
    pub name: Option<String>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
pub struct Output {
    pub id: String,
    pub name: Option<String>,
    pub geometry: Option<Geometry>,
    pub scale: Option<f64>,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub missing_signals: Vec<String>,
    pub windowing_system: String,
}
#[derive(Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NativeState {
    pub windows: Vec<Window>,
    pub desktops: Option<Vec<Desktop>>,
    pub activities: Option<Vec<String>>,
    pub screens: Option<Vec<Output>>,
    pub application_window_ids: Vec<String>,
    pub active_window_id: Option<String>,
    pub active_desktop_id: Option<String>,
    pub active_activity_id: Option<String>,
    pub active_desktop_by_screen: Option<BTreeMap<String, Option<String>>>,
    pub capabilities: Capabilities,
}
#[derive(Clone, Deserialize, Serialize, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WindowIdentity {
    pub session_id: String,
    pub runtime_id: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedSnapshot {
    /// Receiver-local continuity evidence; never a native or durable identity.
    pub observer_epoch: String,
    /// Bounded to live windows; changes when a runtime ID is removed/re-added.
    pub window_generations: BTreeMap<String, String>,
    pub schema_version: u32,
    pub session_id: String,
    pub sequence: u64,
    pub revision: u64,
    pub state: NativeState,
}
impl ObservedSnapshot {
    pub fn redact_titles(&mut self) {
        for window in &mut self.state.windows {
            if window.title.is_some() {
                window.title = Some("<redacted>".into());
            }
        }
    }
    pub fn window_identity(&self, runtime_id: &str) -> Option<WindowIdentity> {
        self.state
            .windows
            .iter()
            .any(|w| w.runtime_id == runtime_id)
            .then(|| WindowIdentity {
                session_id: self.session_id.clone(),
                runtime_id: runtime_id.into(),
            })
    }
}
