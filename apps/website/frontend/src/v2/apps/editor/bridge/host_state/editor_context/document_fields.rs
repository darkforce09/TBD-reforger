//! Role: the document's own fields read and written through the installed context — the mission
//! environment block, the title, and the slot roster as JSON.
//! Position: `editor/bridge/host_state/editor_context` in the frontend editor shell.
//! Signals & state: no signals of its own; every read and write goes to the hosted document.
//! Invariants: a write runs the post-edit tail so the change persists and the docks re-read, and a
//! read with no document installed answers the empty value rather than panicking — the editor can
//! paint its chrome before a mission is loaded.

use super::*;

/// Read terrain + environment from the doc meta (`small_maps_json` → `meta`).
pub fn read_env() -> MissionEnv {
    EDITOR_CONTEXT
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            website_map_engine::data::store::operations::environment::read_env(core)
        })
        .unwrap_or_default()
}

/// One raw `meta.environment` key, exactly as the document holds it — `None` when it is unset.
pub fn read_env_value(key: &str) -> Option<serde_json::Value> {
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let root: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
        root.get("meta")?.get("environment")?.get(key).cloned()
    })
}

/// Read title using the supplied domain data.
pub fn read_title() -> String {
    EDITOR_CONTEXT
        .with(|c| {
            let guard = c.borrow();
            let ctx = guard.as_ref()?;
            let d = ctx.doc.borrow();
            let core = d.as_ref()?;
            let root: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
            root.get("meta")?
                .get("title")
                .and_then(|t| t.as_str())
                .map(str::to_string)
        })
        .unwrap_or_default()
}

/// Set title using the supplied domain data.
pub fn set_title(title: &str) {
    let did = EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.set_title(title);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}

/// Slots json using the supplied domain data.
pub fn slots_json() -> Option<String> {
    EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let d = ctx.doc.borrow();
        d.as_ref().map(|core| core.slots_json())
    })
}

/// Update environment using the supplied domain data.
pub fn update_environment(patch_json: String) {
    let did = EDITOR_CONTEXT.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        core.update_environment(&patch_json);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
}
