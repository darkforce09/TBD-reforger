//! Mapping.

use super::*;

/// v1 `chrome` → v2 surface — deterministic, no marker.
pub(super) const CHROME_MAP: &[(&str, &str)] = &[
    ("left", "dock_left"),
    ("right", "dock_right"),
    ("map", "map_canvas"),
    ("top", "top_strip"),
    ("bottom", "toolbelt"),
    ("attr", "attr_panel"),
];

/// Editor owns-inference: `apps/website/frontend/src/<module>` → (component, surface).
/// Every row is grounded in a real module; `eden_dock_right` is the symmetric partner
/// of the spec table's `eden_dock_left` (24 live owns hits — measured 2026-08-15).
/// `router`/`app_routes` belong to the shell component, not mission_creator.
pub(super) const EDITOR_MODULES: &[(&str, &str, &str)] = &[
    ("mission_editor", "mission_creator", "map_canvas"),
    ("eden_dock_left", "mission_creator", "dock_left"),
    ("eden_dock_right", "mission_creator", "dock_right"),
    ("attributes", "mission_creator", "attr_panel"),
    ("eden_top_strip", "mission_creator", "top_strip"),
    ("eden_toolbelt", "mission_creator", "toolbelt"),
    ("eden_tree", "mission_creator", "outliner"),
    ("outliner", "mission_creator", "outliner"),
    ("asset_catalog", "mission_creator", "asset_browser"),
    ("eden_env", "mission_creator", "env_settings"),
    ("eden_settings", "mission_creator", "env_settings"),
    ("ruler_tool", "mission_creator", "tools"),
    ("los_tool", "mission_creator", "tools"),
    ("select_tool", "mission_creator", "tools"),
    ("mortar", "mission_creator", "tools"),
    ("place_helpers", "mission_creator", "tools"),
    ("mission_doc", "mission_creator", "doc_store"),
    ("mission_hydrate", "mission_creator", "doc_store"),
    ("yrs_persist", "mission_creator", "doc_store"),
    ("editor_session", "mission_creator", "doc_store"),
    ("editor_ops", "mission_creator", "ops_undo"),
    ("mission_commands", "mission_creator", "ops_undo"),
    ("validation_panel", "mission_creator", "validation"),
    ("eden_chrome", "mission_creator", "layout_chrome"),
    ("eden_layout", "mission_creator", "layout_chrome"),
    ("split_pane", "mission_creator", "layout_chrome"),
    ("context_menu", "mission_creator", "layout_chrome"),
    ("router", "shell", "router"),
    ("app_routes", "shell", "router"),
];

/// v1 website backend layer → v2 component (deterministic 1:1).
pub(super) const BACKEND_COMPONENTS: &[(&str, &str)] = &[
    ("api", "http_api"),
    ("db", "db"),
    ("auth", "auth"),
    ("realtime", "realtime"),
];

/// Explicit v1 ModLayer → v2 (layer, component) — deterministic 1:1 into
/// scripts/assets/workbench/worlds. `feature` is absent on purpose: it resolves via
/// owns-inference.
pub(super) const MOD_LAYERS: &[(&str, &str, Option<&str>)] = &[
    ("ui", "scripts", Some("ui")),
    ("gamemode", "scripts", Some("gamemode")),
    ("backend", "scripts", Some("backend")),
    ("prefab", "assets", Some("prefabs")),
    ("data", "assets", Some("data")),
    ("workbench", "workbench", None),
    ("worlds", "worlds", None),
];

/// Mod owns-inference: Enfusion path segment → (layer, component). Segments outside
/// this table (AI, Vehicles, README.md) simply do not vote.
pub(super) const MOD_SEGMENTS: &[(&str, &str, &str)] = &[
    ("Backend", "scripts", "backend"),
    ("Zones", "scripts", "zones"),
    ("Radio", "scripts", "radio"),
    ("Gamemode", "scripts", "gamemode"),
    ("GameMode", "scripts", "gamemode"),
    ("UI", "scripts", "ui"),
    ("Markers", "scripts", "markers"),
    ("Objectives", "scripts", "objectives"),
    ("Registry", "scripts", "registry"),
    ("Spectator", "scripts", "spectator"),
    ("Core", "scripts", "core"),
    ("Prefabs", "assets", "prefabs"),
    ("Configs", "assets", "configs"),
    ("Missions", "assets", "missions"),
    ("Data", "assets", "data"),
    ("worlds", "worlds", ""),
    ("Worlds", "worlds", ""),
];

/// Route first-segments that ARE site_pages surfaces ("route-derived surface where
/// obvious"); everything else stays surface-empty.
pub(super) const ROUTE_SURFACES: &[&str] = &[
    "events",
    "missions",
    "wiki",
    "leaderboards",
    "dashboard",
    "orbat",
    "personnel",
    "arsenal",
    "modpacks",
    "servers",
    "announcements",
    "deployments",
    "approvals",
];

/// The mapped v2 scope plus whether owns-inference was used (the marker input).
#[derive(Debug)]
pub(super) struct MappedScope {
    pub(super) domain: &'static str,
    pub(super) layer: String,
    pub(super) component: Option<String>,
    pub(super) surface: Vec<String>,
    pub(super) owns_inferred: bool,
}

pub(super) fn str_array(v: Option<&toml::Value>) -> Vec<String> {
    v.and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Dominant key by count; ties break to the alphabetically-first key (BTreeMap order)
/// — deterministic either way.
pub(super) fn dominant(counts: &BTreeMap<String, usize>) -> Option<String> {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(k, _)| k.clone())
}

/// Editor owns-inference: (component, surfaces-in-owns-order for the dominant
/// component). `None` component vote → mission_creator with no surfaces.
pub(super) fn infer_editor(owns: &[String]) -> (String, Vec<String>) {
    let mut votes: BTreeMap<String, usize> = BTreeMap::new();
    let mut matches: Vec<(String, String)> = Vec::new(); // (component, surface) in owns order
    for path in owns {
        let Some(rest) = path.strip_prefix("apps/website/frontend/src/") else {
            continue;
        };
        let module = rest.split('/').next().unwrap_or("").trim_end_matches(".rs");
        if let Some((_, component, surface)) = EDITOR_MODULES.iter().find(|(m, _, _)| *m == module)
        {
            *votes.entry((*component).to_string()).or_default() += 1;
            matches.push(((*component).to_string(), (*surface).to_string()));
        }
    }
    let Some(component) = dominant(&votes) else {
        return ("mission_creator".into(), vec![]);
    };
    let mut surfaces: Vec<String> = Vec::new();
    for (c, s) in matches {
        if c == component && !surfaces.contains(&s) {
            surfaces.push(s);
        }
    }
    (component, surfaces)
}

/// Mod owns-inference over Enfusion path segments: dominant (layer, component) by
/// path count; failure falls back to (scripts, None) — the measured dominant reality.
pub(super) fn infer_mod(owns: &[String]) -> (String, Option<String>) {
    let mut votes: BTreeMap<String, usize> = BTreeMap::new(); // "layer\0component"
    for path in owns {
        if !path.starts_with("apps/mod/") {
            continue;
        }
        let vote = if path.ends_with(".layout") {
            Some(("scripts", "ui"))
        } else {
            path.split('/').find_map(|seg| {
                MOD_SEGMENTS
                    .iter()
                    .find(|(s, _, _)| *s == seg)
                    .map(|(_, l, c)| (*l, *c))
            })
        };
        if let Some((layer, component)) = vote {
            *votes.entry(format!("{layer}\u{0}{component}")).or_default() += 1;
        }
    }
    match dominant(&votes) {
        Some(key) => {
            let (layer, component) = key.split_once('\u{0}').expect("keyed with NUL");
            (
                layer.to_string(),
                (!component.is_empty()).then(|| component.to_string()),
            )
        }
        None => ("scripts".into(), None),
    }
}

/// repo/xtask owns-inference: `xtask/src/<module>` → component by the committed
/// prefix rules; dominant by path count; failure → component None.
pub(super) fn infer_xtask(owns: &[String]) -> Option<String> {
    let mut votes: BTreeMap<String, usize> = BTreeMap::new();
    for path in owns {
        let Some(rest) = path.strip_prefix("xtask/src/") else {
            continue;
        };
        let module = rest.split('/').next().unwrap_or("").trim_end_matches(".rs");
        let component = if module.starts_with("wave") {
            "wave"
        } else if module == "check" {
            "check"
        } else if matches!(module, "cmds" | "tickets_store" | "registry") {
            "tickets"
        } else if module.starts_with("gate_") {
            "gates"
        } else if module.starts_with("deploy_") {
            "deploy"
        } else if module.starts_with("mk_db") {
            "db"
        } else if module.starts_with("mcp") {
            "mcp"
        } else if module.starts_with("mk_ci") || module.starts_with("verify_ci") {
            "ci"
        } else if module.starts_with("metrics") {
            "metrics"
        } else {
            continue;
        };
        *votes.entry(component.to_string()).or_default() += 1;
    }
    dominant(&votes)
}
