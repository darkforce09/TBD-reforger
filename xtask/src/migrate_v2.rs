//! T-917.2 — THE schema-v2 cutover migrator (`cargo xtask ticket migrate-v2`) plus the
//! standing `ticket scope-histogram` report verb.
//!
//! One-shot, Value→Value: typed v2 refuses v1 by definition (the flat `[scope]` table
//! deserializer cannot read the nested `[scope.website.editor]` tree), so the migrator
//! parses each file as `toml::Value`, transforms scope/class/estimated via the committed
//! mapping tables below, then validates EVERY output through the new
//! `parse_ticket_toml` + a byte-stable re-render assert before a single byte lands
//! (check-before-write, whole corpus staged first — the `Corpus::write_back` /
//! `migrate_live_tree` re-parse-gate precedent). Retained after the cutover for
//! corroboration, like `wave/legacy_plan.rs`; a second run refuses on the first
//! already-v2 scope it sees.
//!
//! Mapping rules (spec §Scope v2, old→new mechanical maps):
//!
//! - `[scope.website.editor]` → website/frontend/mission_creator. Nonempty `chrome`
//!   maps deterministically (left→dock_left, right→dock_right, map→map_canvas,
//!   top→top_strip, bottom→toolbelt, attr→attr_panel). Chrome-less editor tickets go
//!   through OWNS-INFERENCE: `apps/website/frontend/src/<module>` matched against the
//!   committed module table; owns crossing components pick the dominant component by
//!   path count, surfaces from its matches only.
//! - `[scope.website.page|shell|backend|tests]` → frontend/site_pages (+ obvious
//!   route-derived surface), frontend/shell, backend (+ layer→component map),
//!   tests — all deterministic.
//! - `[scope.mod]` explicit layers map 1:1 into scripts/assets/workbench/worlds; the
//!   `feature` landfill goes through owns-inference over the Enfusion path segments.
//! - `[scope.schema|engine]` → layer, component None. `[scope.repo]` → layer, with
//!   `xtask` component owns-inferred from the `xtask/src/<module>` table. A
//!   multi-layer v1 array (one live case: T-916.2 `["xtask", "tickets"]`) takes its
//!   FIRST layer — deterministic, tallied in the report.
//!
//! **`estimated = ["scope"]` marker — the honest escape:** appended exactly when the
//! migrator used owns-inference for the ticket's scope (editor-without-chrome,
//! mod/feature, repo/xtask component inference) — as opposed to the deterministic
//! chrome/layer maps. Inference is mechanical but derived, so it is *recorded*, and the
//! live-work surface rule (`check_live_work_surface`, ops made-live gate) accepts the
//! marker where no surface could be inferred.
//!
//! **`class` triage:** every work ticket gets `classify_work(title + summary)` — the
//! conservative-deterministic keyword map (same input → same class). This is metadata
//! triage, not provenance, so it carries NO `estimated[]` marker (the legal marker
//! value set is fixed by the spec and `class` is not in it — decision documented here).
//! Programs get no class requirement and none is invented for them.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use tbd_tickets::{Corpus, ScopeVocab, Ticket, TicketFile, parse_ticket_toml, render_ticket_toml};

/// v1 `chrome` → v2 surface — deterministic, no marker.
const CHROME_MAP: &[(&str, &str)] = &[
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
const EDITOR_MODULES: &[(&str, &str, &str)] = &[
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
const BACKEND_COMPONENTS: &[(&str, &str)] = &[
    ("api", "http_api"),
    ("db", "db"),
    ("auth", "auth"),
    ("realtime", "realtime"),
];

/// Explicit v1 ModLayer → v2 (layer, component) — deterministic 1:1 into
/// scripts/assets/workbench/worlds. `feature` is absent on purpose: it resolves via
/// owns-inference.
const MOD_LAYERS: &[(&str, &str, Option<&str>)] = &[
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
const MOD_SEGMENTS: &[(&str, &str, &str)] = &[
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
const ROUTE_SURFACES: &[&str] = &[
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
struct MappedScope {
    domain: &'static str,
    layer: String,
    component: Option<String>,
    surface: Vec<String>,
    owns_inferred: bool,
}

fn str_array(v: Option<&toml::Value>) -> Vec<String> {
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
fn dominant(counts: &BTreeMap<String, usize>) -> Option<String> {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
        .map(|(k, _)| k.clone())
}

/// Editor owns-inference: (component, surfaces-in-owns-order for the dominant
/// component). `None` component vote → mission_creator with no surfaces.
fn infer_editor(owns: &[String]) -> (String, Vec<String>) {
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
fn infer_mod(owns: &[String]) -> (String, Option<String>) {
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
fn infer_xtask(owns: &[String]) -> Option<String> {
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

/// Map one v1 `[scope]` Value onto the v2 shape. `Err` = an UNMAPPED shape — the
/// migrator collects these and refuses the whole run before writing.
fn map_scope(id: &str, scope: &toml::Value, owns: &[String]) -> Result<MappedScope> {
    let table = scope
        .as_table()
        .with_context(|| format!("{id}: [scope] is not a table"))?;
    if table.len() != 1 {
        bail!(
            "{id}: expected exactly one [scope.*] table, got {}",
            table.len()
        );
    }
    let (domain_key, dv) = table.iter().next().expect("len checked");
    let deterministic = |domain, layer: &str, component: Option<&str>, surface: Vec<String>| {
        Ok(MappedScope {
            domain,
            layer: layer.to_string(),
            component: component.map(str::to_string),
            surface,
            owns_inferred: false,
        })
    };
    match domain_key.as_str() {
        "website" => {
            let wt = dv
                .as_table()
                .with_context(|| format!("{id}: [scope.website] is not a table"))?;
            if wt.len() != 1 {
                bail!("{id}: expected exactly one [scope.website.*] table");
            }
            let (kind, kv) = wt.iter().next().expect("len checked");
            let kt = kv
                .as_table()
                .with_context(|| format!("{id}: [scope.website.{kind}] is not a table"))?;
            match kind.as_str() {
                "editor" => {
                    if kt.contains_key("capability") {
                        bail!("{id}: editor capability is populated — unmapped (0 measured)");
                    }
                    let chrome = str_array(kt.get("chrome"));
                    if !chrome.is_empty() {
                        let mut surface = Vec::new();
                        for c in &chrome {
                            let mapped = CHROME_MAP
                                .iter()
                                .find(|(k, _)| k == c)
                                .map(|(_, s)| (*s).to_string())
                                .with_context(|| format!("{id}: unknown chrome value {c}"))?;
                            if !surface.contains(&mapped) {
                                surface.push(mapped);
                            }
                        }
                        return deterministic(
                            "website",
                            "frontend",
                            Some("mission_creator"),
                            surface,
                        );
                    }
                    let (component, surface) = infer_editor(owns);
                    Ok(MappedScope {
                        domain: "website",
                        layer: "frontend".into(),
                        component: Some(component),
                        surface,
                        owns_inferred: true,
                    })
                }
                "page" => {
                    let surface = kt
                        .get("route")
                        .and_then(|r| r.as_str())
                        .and_then(|route| {
                            let seg = route.trim_start_matches('/').split('/').next()?;
                            if ROUTE_SURFACES.contains(&seg) {
                                Some(vec![seg.to_string()])
                            } else if matches!(seg, "auth" | "login") {
                                Some(vec!["auth_pages".to_string()])
                            } else {
                                None
                            }
                        })
                        .unwrap_or_default();
                    deterministic("website", "frontend", Some("site_pages"), surface)
                }
                "shell" => deterministic("website", "frontend", Some("shell"), vec![]),
                "backend" => {
                    let layers = str_array(kt.get("layers"));
                    let first = layers
                        .first()
                        .with_context(|| format!("{id}: backend without layers"))?;
                    let component = BACKEND_COMPONENTS
                        .iter()
                        .find(|(k, _)| k == first)
                        .map(|(_, c)| *c)
                        .with_context(|| format!("{id}: unknown backend layer {first}"))?;
                    deterministic("website", "backend", Some(component), vec![])
                }
                "tests" => deterministic("website", "tests", None, vec![]),
                other => bail!("{id}: unmapped website scope kind {other}"),
            }
        }
        "mod" => {
            let layers = str_array(dv.as_table().and_then(|t| t.get("layers")));
            let first = layers
                .first()
                .with_context(|| format!("{id}: mod without layers"))?;
            if let Some((_, layer, component)) = MOD_LAYERS.iter().find(|(k, _, _)| k == first) {
                return deterministic("mod", layer, *component, vec![]);
            }
            if first != "feature" {
                bail!("{id}: unmapped mod layer {first}");
            }
            let (layer, component) = infer_mod(owns);
            Ok(MappedScope {
                domain: "mod",
                layer,
                component,
                surface: vec![],
                owns_inferred: true,
            })
        }
        "schema" | "engine" => {
            let layers = str_array(dv.as_table().and_then(|t| t.get("layers")));
            let first = layers
                .first()
                .with_context(|| format!("{id}: {domain_key} without layers"))?;
            let domain = if domain_key == "schema" {
                "schema"
            } else {
                "engine"
            };
            deterministic(domain, first, None, vec![])
        }
        "repo" => {
            let layers = str_array(dv.as_table().and_then(|t| t.get("layers")));
            let first = layers
                .first()
                .with_context(|| format!("{id}: repo without layers"))?;
            if first == "xtask" {
                return Ok(MappedScope {
                    domain: "repo",
                    layer: "xtask".into(),
                    component: infer_xtask(owns),
                    surface: vec![],
                    owns_inferred: true,
                });
            }
            deterministic("repo", first, None, vec![])
        }
        other => bail!("{id}: unmapped scope domain {other}"),
    }
}

fn ticket_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let dir = crate::tickets_store::tickets_dir(root);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name().is_some_and(|n| {
                let n = n.to_string_lossy();
                n.starts_with("T-") && n.ends_with(".toml")
            })
        })
        .collect();
    paths.sort();
    Ok(paths)
}

/// THE cutover. Reads every `.ai/tickets/T-*.toml` as `toml::Value`, transforms,
/// validates the WHOLE corpus (typed parse + vocab legality + byte-stable re-render)
/// before writing a single byte, then lands each file temp+rename and regenerates the
/// sync surface from the reloaded registry.
pub fn cmd_migrate_v2(root: &Path) -> Result<()> {
    let vocab = ScopeVocab::load(root).map_err(anyhow::Error::msg)?;
    let paths = ticket_paths(root)?;
    let n_files = paths.len();
    let mut staged: Vec<(PathBuf, String)> = Vec::with_capacity(n_files);
    let mut unmapped: Vec<String> = Vec::new();
    let mut layers_multi: Vec<String> = Vec::new();
    let mut owns_inferred_ids = 0usize;

    for path in &paths {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let text = fs::read_to_string(path).with_context(|| format!("read {name}"))?;
        let mut doc: toml::Value = text.parse().with_context(|| format!("{name}: not TOML"))?;
        let table = doc
            .as_table_mut()
            .with_context(|| format!("{name}: root is not a table"))?;
        let id = table
            .get("id")
            .and_then(|v| v.as_str())
            .with_context(|| format!("{name}: missing id"))?
            .to_string();
        let kind = table
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if kind == "work" {
            let owns = str_array(table.get("owns"));
            let old_scope = table
                .remove("scope")
                .with_context(|| format!("{id}: work ticket without [scope]"))?;
            if old_scope
                .as_table()
                .is_some_and(|t| t.contains_key("domain"))
            {
                bail!(
                    "{id}: [scope] already carries `domain` — the tree is v2; migrate-v2 is one-shot"
                );
            }
            if str_array(
                old_scope
                    .as_table()
                    .and_then(|t| t.values().next())
                    .and_then(|d| d.as_table())
                    .and_then(|t| t.get("layers")),
            )
            .len()
                > 1
            {
                layers_multi.push(id.clone());
            }
            let mapped = match map_scope(&id, &old_scope, &owns) {
                Ok(m) => m,
                Err(e) => {
                    unmapped.push(format!("{e:#}"));
                    continue;
                }
            };
            let mut scope_table = toml::map::Map::new();
            scope_table.insert(
                "domain".into(),
                toml::Value::String(mapped.domain.to_string()),
            );
            scope_table.insert("layer".into(), toml::Value::String(mapped.layer.clone()));
            if let Some(c) = &mapped.component {
                scope_table.insert("component".into(), toml::Value::String(c.clone()));
            }
            if !mapped.surface.is_empty() {
                scope_table.insert(
                    "surface".into(),
                    toml::Value::Array(
                        mapped
                            .surface
                            .iter()
                            .map(|s| toml::Value::String(s.clone()))
                            .collect(),
                    ),
                );
            }
            table.insert("scope".into(), toml::Value::Table(scope_table));

            // class triage — v1 never carried the key (governance-pinned), so this is
            // an insert, never an overwrite.
            if table.contains_key("class") {
                bail!("{id}: v1 file already carries `class` — governance breach, refusing");
            }
            let title = table.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let summary = table.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            table.insert(
                "class".into(),
                toml::Value::String(
                    tbd_tickets::classify_work(&format!("{title} {summary}")).to_string(),
                ),
            );

            if mapped.owns_inferred {
                owns_inferred_ids += 1;
                if table.contains_key("estimated") {
                    bail!(
                        "{id}: v1 file already carries `estimated` — governance breach, refusing"
                    );
                }
                table.insert(
                    "estimated".into(),
                    toml::Value::Array(vec![toml::Value::String("scope".into())]),
                );
            }
        } else if table.get("scope").is_some() {
            bail!("{id}: program carries [scope] — v1 tree is broken, refusing");
        }

        // Typed validation + canonical render + byte-stable re-render gate, per file.
        let file: TicketFile = doc
            .try_into()
            .with_context(|| format!("{id}: transformed value does not deserialize as v2"))?;
        let ticket = file
            .into_ticket()
            .map_err(|e| anyhow::anyhow!("{id}: {e}"))?;
        if let Ticket::Work(w) = &ticket {
            vocab
                .check_scope(&w.id, &w.scope)
                .map_err(anyhow::Error::msg)?;
        }
        let rendered = render_ticket_toml(&ticket).map_err(anyhow::Error::msg)?;
        let back = parse_ticket_toml(&rendered)
            .map_err(|e| anyhow::anyhow!("{id}: rendered v2 does not re-parse: {e}"))?;
        if back != ticket {
            bail!("{id}: render → re-parse does not round-trip to the same ticket");
        }
        let again = render_ticket_toml(&back).map_err(anyhow::Error::msg)?;
        if again != rendered {
            bail!("{id}: re-render is not byte-stable");
        }
        staged.push((path.clone(), rendered));
    }

    println!("unmapped-scope list ({}):", unmapped.len());
    for u in &unmapped {
        println!("  {u}");
    }
    if !unmapped.is_empty() {
        bail!(
            "{} ticket(s) have unmapped scope — nothing written; widen the mapping tables",
            unmapped.len()
        );
    }
    if staged.len() != n_files {
        bail!(
            "staged {} of {n_files} files — refusing partial migration",
            staged.len()
        );
    }

    // Land the bytes, temp+rename per file (the write_back pattern).
    for (path, text) in &staged {
        let tmp = path.with_file_name(format!(
            ".{}.tmp",
            path.file_name().unwrap().to_string_lossy()
        ));
        fs::write(&tmp, text).with_context(|| format!("write {}", tmp.display()))?;
        fs::rename(&tmp, path)
            .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    }
    println!("migrated {}/{n_files} files", staged.len());
    if !layers_multi.is_empty() {
        println!(
            "multi-layer v1 arrays took their FIRST layer ({}): {}",
            layers_multi.len(),
            layers_multi.join(", ")
        );
    }
    println!("owns-inference used on {owns_inferred_ids} tickets (estimated += \"scope\")");

    // Full-corpus re-parse gate: the typed load (vocab legality included) must accept
    // the tree we just wrote.
    let corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    println!(
        "corpus reload: {} tickets parse v2-clean",
        corpus.tickets.len()
    );

    // Regenerate the sync surface from the reloaded registry (docs/TICKET_*.md,
    // queue.json, CLAUDE marker) — sync.rs copies summaries verbatim.
    let registry = crate::registry::load_registry(root)?;
    crate::sync::cmd_sync(root, &registry)?;

    print_scope_histogram(&corpus);
    Ok(())
}

/// `ticket scope-histogram` — standing read-only verb (also the tail of the migration
/// report): per-domain/layer/component counts, per-surface counts, the
/// "U surface-empty (scope ∈ estimated: E)" honesty counters per component bucket,
/// and the work-ticket class distribution.
pub fn cmd_scope_histogram(root: &Path) -> Result<()> {
    let corpus = Corpus::load(root).map_err(anyhow::Error::msg)?;
    print_scope_histogram(&corpus);
    Ok(())
}

fn print_scope_histogram(corpus: &Corpus) {
    let mut works = 0usize;
    let mut programs = 0usize;
    let mut buckets: BTreeMap<String, Vec<&tbd_tickets::WorkTicket>> = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for t in corpus.tickets.values() {
        match t {
            Ticket::Program(_) => programs += 1,
            Ticket::Work(w) => {
                works += 1;
                let key = match &w.scope.component {
                    Some(c) => format!("{}/{}/{c}", w.scope.domain.as_str(), w.scope.layer),
                    None => format!("{}/{}", w.scope.domain.as_str(), w.scope.layer),
                };
                buckets.entry(key).or_default().push(w);
                *classes
                    .entry(w.class.clone().unwrap_or_else(|| "(none)".into()))
                    .or_default() += 1;
            }
        }
    }
    println!("scope histogram — {works} work tickets, {programs} programs");
    for (key, tickets) in &buckets {
        println!("{key}: {}", tickets.len());
        let mut surfaces: BTreeMap<&str, usize> = BTreeMap::new();
        let mut empty = 0usize;
        let mut empty_marked = 0usize;
        for w in tickets {
            if w.scope.surface.is_empty() {
                empty += 1;
                if w.estimated.iter().any(|e| e == "scope") {
                    empty_marked += 1;
                }
            }
            for s in &w.scope.surface {
                *surfaces.entry(s.as_str()).or_default() += 1;
            }
        }
        if !surfaces.is_empty() {
            let list: Vec<String> = surfaces.iter().map(|(s, n)| format!("{s} {n}")).collect();
            println!("  surfaces: {}", list.join(", "));
        }
        if empty > 0 {
            println!("  {empty} surface-empty (scope ∈ estimated: {empty_marked})");
        }
    }
    let class_line: Vec<String> = classes.iter().map(|(c, n)| format!("{c} {n}")).collect();
    println!("class distribution (work): {}", class_line.join(", "));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn owns(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn chrome_map_is_deterministic_no_marker() {
        let scope: toml::Value = "[website.editor]\nchrome = [\"attr\", \"left\"]\n"
            .parse()
            .unwrap();
        let m = map_scope("T-x", &scope, &[]).unwrap();
        assert_eq!(m.domain, "website");
        assert_eq!(m.layer, "frontend");
        assert_eq!(m.component.as_deref(), Some("mission_creator"));
        assert_eq!(m.surface, vec!["attr_panel", "dock_left"]);
        assert!(!m.owns_inferred, "chrome map carries no marker");
    }

    #[test]
    fn editor_owns_inference_dominant_component() {
        // Two mission_creator votes vs one shell vote → mission_creator, its surfaces only.
        let (c, s) = infer_editor(&owns(&[
            "apps/website/frontend/src/mission_editor.rs",
            "apps/website/frontend/src/editor_ops.rs",
            "apps/website/frontend/src/router.rs",
        ]));
        assert_eq!(c, "mission_creator");
        assert_eq!(s, vec!["map_canvas", "ops_undo"]);
        // Shell dominance flips the component and drops mission_creator surfaces.
        let (c, s) = infer_editor(&owns(&[
            "apps/website/frontend/src/router.rs",
            "apps/website/frontend/src/app_routes.rs",
            "apps/website/frontend/src/mission_editor.rs",
        ]));
        assert_eq!(c, "shell");
        assert_eq!(s, vec!["router"]);
        // No table match → mission_creator, surface-empty (the marker population).
        let (c, s) = infer_editor(&owns(&["apps/website/frontend/src/arsenal.rs"]));
        assert_eq!(c, "mission_creator");
        assert!(s.is_empty());
    }

    #[test]
    fn chromeless_editor_is_owns_inferred_marked() {
        let scope: toml::Value = "[website.editor]\nchrome = []\n".parse().unwrap();
        let m = map_scope(
            "T-x",
            &scope,
            &owns(&["apps/website/frontend/src/attributes.rs"]),
        )
        .unwrap();
        assert_eq!(m.surface, vec!["attr_panel"]);
        assert!(m.owns_inferred, "owns-inference must be recorded");
    }

    #[test]
    fn mod_feature_infers_from_enfusion_segments() {
        let scope: toml::Value = "[mod]\nlayers = [\"feature\"]\n".parse().unwrap();
        let m = map_scope(
            "T-x",
            &scope,
            &owns(&[
                "apps/mod/tbd-framework/Scripts/Game/TBD/Markers/TBD_MarkerData.c",
                "apps/mod/tbd-framework/Scripts/Game/TBD/Markers/TBD_MarkerClient.c",
                "apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_Api.c",
            ]),
        )
        .unwrap();
        assert_eq!(m.layer, "scripts");
        assert_eq!(m.component.as_deref(), Some("markers"));
        assert!(m.owns_inferred);
        // Unvoted paths (AI) fall back to scripts/None, still marked.
        let m = map_scope(
            "T-y",
            &scope,
            &owns(&["apps/mod/tbd-framework/Scripts/Game/TBD/AI/TBD_WaypointRuntime.c"]),
        )
        .unwrap();
        assert_eq!(m.layer, "scripts");
        assert_eq!(m.component, None);
        assert!(m.owns_inferred);
        // Explicit layers stay deterministic.
        let scope: toml::Value = "[mod]\nlayers = [\"backend\"]\n".parse().unwrap();
        let m = map_scope("T-z", &scope, &[]).unwrap();
        assert_eq!(
            (m.layer.as_str(), m.component.as_deref()),
            ("scripts", Some("backend"))
        );
        assert!(!m.owns_inferred);
    }

    #[test]
    fn repo_xtask_component_prefix_rules() {
        assert_eq!(
            infer_xtask(&owns(&["xtask/src/wave_lock.rs", "xtask/src/wave/plan.rs"])),
            Some("wave".into())
        );
        assert_eq!(
            infer_xtask(&owns(&["xtask/src/cmds.rs", "xtask/src/tickets_store.rs"])),
            Some("tickets".into())
        );
        assert_eq!(
            infer_xtask(&owns(&["xtask/src/gate_mod_compile.rs"])),
            Some("gates".into())
        );
        assert_eq!(
            infer_xtask(&owns(&[
                "xtask/src/mk_ci_tasks.rs",
                "xtask/src/verify_ci_shell.rs"
            ])),
            Some("ci".into())
        );
        assert_eq!(infer_xtask(&owns(&["xtask/src/hostrun.rs"])), None);
        let scope: toml::Value = "[repo]\nlayers = [\"docs\"]\n".parse().unwrap();
        let m = map_scope("T-x", &scope, &[]).unwrap();
        assert_eq!(
            (m.domain, m.layer.as_str(), m.component),
            ("repo", "docs", None)
        );
        assert!(!m.owns_inferred, "repo/docs is a deterministic layer map");
    }

    #[test]
    fn multi_layer_takes_first() {
        let scope: toml::Value = "[repo]\nlayers = [\"xtask\", \"tickets\"]\n"
            .parse()
            .unwrap();
        let m = map_scope(
            "T-916.2",
            &scope,
            &owns(&["crates/tbd-tickets", "xtask/src"]),
        )
        .unwrap();
        assert_eq!(m.layer, "xtask");
        assert!(m.owns_inferred);
    }

    #[test]
    fn unmapped_shapes_refuse_naming_ticket() {
        let scope: toml::Value = "[galaxy]\nlayers = [\"far\"]\n".parse().unwrap();
        let err = map_scope("T-404", &scope, &[]).unwrap_err();
        assert!(format!("{err:#}").contains("T-404"), "{err:#}");
    }
}
