use crate::{ObservationError, model::*};
use serde::Deserialize;
use std::collections::BTreeMap;
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ITEMS: usize = 4096;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Packet {
    pub schema_version: u32,
    pub session_id: String,
    pub consumer_id: String,
    pub frames: Vec<Frame>,
    pub position: Position,
    pub status: Option<String>,
}
#[derive(Clone, Copy, Deserialize, PartialEq, Eq, Debug)]
pub(crate) struct Position {
    pub sequence: u64,
    pub revision: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Frame {
    pub schema_version: u32,
    pub session_id: String,
    pub sequence: u64,
    pub revision: u64,
    #[serde(flatten)]
    pub event: Event,
}
// Explicit nullable payloads; raw validation below enforces key presence.
#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct Nullable<T>(pub Option<T>);
#[derive(Deserialize)]
#[serde(tag = "type")]
pub(crate) enum Event {
    #[serde(rename = "snapshot")]
    Snapshot { reason: String, state: NativeState },
    #[serde(rename = "window.added")]
    Added {
        #[serde(rename = "windowId")]
        id: String,
        window: Window,
    },
    #[serde(rename = "window.metadata.changed", rename_all = "camelCase")]
    Metadata {
        window_id: String,
        changed_fields: Vec<String>,
        window: Window,
    },
    #[serde(rename = "window.desktop.changed", rename_all = "camelCase")]
    Desktop {
        window_id: String,
        changed_fields: Vec<String>,
        window: Window,
    },
    #[serde(rename = "window.activity.changed", rename_all = "camelCase")]
    Activity {
        window_id: String,
        changed_fields: Vec<String>,
        window: Window,
    },
    #[serde(rename = "window.geometry.changed", rename_all = "camelCase")]
    Geometry {
        window_id: String,
        changed_fields: Vec<String>,
        window: Window,
    },
    #[serde(rename = "window.state.changed", rename_all = "camelCase")]
    State {
        window_id: String,
        changed_fields: Vec<String>,
        window: Window,
    },
    #[serde(rename = "window.removed")]
    Removed {
        #[serde(rename = "windowId")]
        id: String,
    },
    #[serde(rename = "window.focus.changed")]
    Focus {
        #[serde(rename = "activeWindowId")]
        id: Nullable<String>,
    },
    #[serde(rename = "desktop.list.changed")]
    Desktops { desktops: Nullable<Vec<Desktop>> },
    #[serde(rename = "desktop.active.changed", rename_all = "camelCase")]
    ActiveDesktop {
        active_desktop_id: Nullable<String>,
        active_desktop_by_screen: Nullable<BTreeMap<String, Option<String>>>,
    },
    #[serde(rename = "activity.list.changed")]
    Activities { activities: Nullable<Vec<String>> },
    #[serde(rename = "activity.active.changed")]
    ActiveActivity {
        #[serde(rename = "activeActivityId")]
        id: Nullable<String>,
    },
    #[serde(rename = "screen.list.changed")]
    Screens { screens: Nullable<Vec<Output>> },
}
pub(crate) fn valid_session(s: &str) -> bool {
    !s.is_empty() && s.chars().count() <= 256
}
pub(crate) fn decode(raw: &str) -> Result<Packet, ObservationError> {
    if raw.len() > MAX_BYTES {
        return Err(ObservationError::InvalidPacket);
    }
    let p: Packet = serde_json::from_str(raw).map_err(|_| ObservationError::InvalidPacket)?;
    if p.schema_version != 1
        || !valid_session(&p.session_id)
        || p.frames.len() > 32
        || (p.status.is_some() && !p.frames.is_empty())
        || p.position.revision == 0
        || p.status
            .as_deref()
            .is_some_and(|s| s != "payload-too-large")
    {
        return Err(ObservationError::InvalidPacket);
    }
    for f in &p.frames {
        if f.schema_version != 1
            || f.session_id != p.session_id
            || f.sequence == 0
            || f.revision == 0
            || f.sequence > p.position.sequence
            || f.revision > p.position.revision
        {
            return Err(ObservationError::InvalidPacket);
        }
    }
    Ok(p)
}
// This validation is done against raw JSON too so absent nullable snapshot/window
// metadata cannot be confused with explicitly unknown native values.
pub(crate) fn validate_raw(raw: &str) -> Result<(), ObservationError> {
    if raw.len() > MAX_BYTES {
        return Err(ObservationError::InvalidPacket);
    }
    let v: serde_json::Value =
        serde_json::from_str(raw).map_err(|_| ObservationError::InvalidPacket)?;
    bound(&v)?;
    if let Some(frames) = v.get("frames").and_then(|x| x.as_array()) {
        for f in frames {
            if f.get("type").and_then(|x| x.as_str()) == Some("snapshot") {
                let s = &f["state"];
                require(
                    s,
                    &[
                        "desktops",
                        "activities",
                        "screens",
                        "activeWindowId",
                        "activeDesktopId",
                        "activeActivityId",
                        "activeDesktopByScreen",
                    ],
                )?;
                if let Some(w) = s["windows"].as_array() {
                    for w in w {
                        window_keys(w)?;
                    }
                }
            } else if let Some(w) = f.get("window") {
                window_keys(w)?;
            }
            let keys: &[&str] = match f.get("type").and_then(|x| x.as_str()) {
                Some("window.focus.changed") => &["activeWindowId"],
                Some("desktop.list.changed") => &["desktops"],
                Some("desktop.active.changed") => &["activeDesktopId", "activeDesktopByScreen"],
                Some("activity.list.changed") => &["activities"],
                Some("activity.active.changed") => &["activeActivityId"],
                Some("screen.list.changed") => &["screens"],
                _ => &[],
            };
            require(f, keys)?;
        }
    }
    Ok(())
}
fn window_keys(v: &serde_json::Value) -> Result<(), ObservationError> {
    if serde_json::to_vec(v)
        .map_err(|_| ObservationError::InvalidPacket)?
        .len()
        > 64 * 1024
    {
        return Err(ObservationError::InvalidPacket);
    }
    require(
        v,
        &[
            "nativeId",
            "windowingSystem",
            "title",
            "applicationId",
            "resourceClass",
            "resourceName",
            "parentRuntimeId",
            "desktopIds",
            "activityIds",
            "onAllDesktops",
            "onAllActivities",
            "geometry",
            "screenId",
            "minimized",
            "fullscreen",
            "skipTaskbar",
            "excludeFromCapture",
        ],
    )
}
fn require(v: &serde_json::Value, keys: &[&str]) -> Result<(), ObservationError> {
    if keys.iter().all(|k| v.get(k).is_some()) {
        Ok(())
    } else {
        Err(ObservationError::InvalidPacket)
    }
}
fn bound(v: &serde_json::Value) -> Result<(), ObservationError> {
    use serde_json::Value::*;
    let valid = match v {
        String(s) => s.encode_utf16().count() <= 4096,
        Array(a) => a.len() <= MAX_ITEMS && a.iter().all(|x| bound(x).is_ok()),
        Object(o) => {
            o.len() <= MAX_ITEMS && o.iter().all(|(k, v)| k.len() <= 512 && bound(v).is_ok())
        }
        _ => true,
    };
    if valid {
        Ok(())
    } else {
        Err(ObservationError::InvalidPacket)
    }
}
