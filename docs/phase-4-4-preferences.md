# Phase 4.4 User Preferences Persistence

DepthOS Phase 4.4 introduces persistent configuration storage for UI presentation, accessibility, and workspace navigation defaults.

## Storage Design

Preferences are persisted in the `user_preferences` table in SQLite:

```sql
CREATE TABLE IF NOT EXISTS user_preferences (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);
```

### Supported Preference Keys

| Preference Key | Type | Default | Description |
|---|---|---|---|
| `presentation.reduced_motion` | boolean (`"true"`/`"false"`) | `"false"` | Replaces camera orbit transitions with instantaneous cuts |
| `presentation.low_effects` | boolean (`"true"`/`"false"`) | `"false"` | Disables GPU-intensive shader blooms and glow effects |
| `presentation.high_contrast` | boolean (`"true"`/`"false"`) | `"false"` | Enhances text contrast and border outlines for accessibility |
| `presentation.show_titles` | boolean (`"true"`/`"false"`) | `"false"` | Toggles window title visibility on cards when titles are unredacted |
| `workspace.default_landing_view` | string (`"worlds"`, `"workspaces"`, `"desktops"`) | `"worlds"` | Default hierarchy level loaded on application startup |

## Control Center Integration

Preferences are managed directly within the Spatial Control Center (`ControlCenter.qml`):

1. **Immediate Reaction**: Modifying a toggle in the Control Center immediately updates runtime properties on `SpatialScene` (e.g. `scene.lowEffects`, `scene.showTitles`).
2. **Asynchronous Persistence**: A background call via `HostBridge.setPreference(key, value)` saves the preference to SQLite without freezing the render loop.
3. **Session Restoration**: When DepthOS launches, `HostBridge.load_preferences_into_ui()` restores all saved preferences before presenting the viewport.
