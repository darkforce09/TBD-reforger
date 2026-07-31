//! Smart-Arsenal domain core (T-167) — the Rust port of React `arsenalRules.ts` +
//! `arsenalDollModel.ts` (tag T-159.29.2). Pure, framework-free, native-tested: the 14 loadout
//! rows (incl. the compat **edge** rows optic/magazine), the compat edge graph + `items_for`,
//! per-row option building (abstract/variant filtered, stranded-pick preserved), loadout
//! validation, the paper-doll region model, and the honest weight readout.
//!
//! The UI (`arsenal.rs`) and the persisted `SlotLoadoutV2` shape (owned by `arsenal.rs`
//! `picks_to_loadout`) sit on top of this — this module holds only the decisions.
//!
//! T-240 checked whether the blanket `allow` below is still earned, because while it is on, the
//! compiler cannot tell anyone that a rule in here has no caller — which is how `cargo_capacity_errors`
//! could have shipped unwired and silent. **It is still earned, by exactly three items in the
//! shipping (wasm32) build:** `PRIMARY_SUB_REGIONS` (:185), `DollRegion::kind` (:412) and
//! `DOLL_REGIONS` (:479) — the paper-doll region model, whose consumer went away. A native
//! `cargo check` lists more, but those are consumers behind `cfg(target_arch = "wasm32")`, not
//! real deadness. Remove the `allow` the moment those three find a caller or go; do not delete
//! them to get there.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap, HashSet};

use crate::dto::{RegistryCompatEdge, RegistryItem};

/// How a row sources its options.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RowSource {
    /// Flat registry catalog filtered by `kind` — never compat-constrained (clothing mix-and-match
    /// is deliberate). Weapon rows carry the engine slot mapping.
    Kind {
        kind: &'static str,
        weapon: Option<(i64, &'static str)>,
    },
    /// Compat-graph fed: options come from `items_for(picks[depends_on], edge)`; empty until the
    /// dependency is picked.
    Edge {
        edge: &'static str,
        depends_on: &'static str,
    },
}

pub struct LoadoutRow {
    pub key: &'static str,
    pub label: &'static str,
    pub source: RowSource,
}

/// `LOADOUT_ROWS` — render/compile order. optic + magazine are **edge** rows sitting immediately
/// after `primary` (this ordering is the load-bearing fact from the React ref).
pub const LOADOUT_ROWS: &[LoadoutRow] = &[
    LoadoutRow {
        key: "primary",
        label: "Primary",
        source: RowSource::Kind {
            kind: "gear_primary",
            weapon: Some((0, "primary")),
        },
    },
    LoadoutRow {
        key: "optic",
        label: "Optic",
        source: RowSource::Edge {
            edge: "optic_on_weapon",
            depends_on: "primary",
        },
    },
    LoadoutRow {
        key: "magazine",
        label: "Magazine",
        source: RowSource::Edge {
            edge: "mag_in_weapon",
            depends_on: "primary",
        },
    },
    LoadoutRow {
        key: "launcher",
        label: "Launcher / 2nd rifle",
        source: RowSource::Kind {
            kind: "gear_launcher",
            weapon: Some((1, "primary")),
        },
    },
    LoadoutRow {
        key: "handgun",
        label: "Handgun",
        source: RowSource::Kind {
            kind: "gear_handgun",
            weapon: Some((2, "secondary")),
        },
    },
    LoadoutRow {
        key: "throwable",
        label: "Throwable",
        source: RowSource::Kind {
            kind: "gear_throwable",
            weapon: Some((3, "grenade")),
        },
    },
    LoadoutRow {
        key: "headCover",
        label: "Helmet",
        source: RowSource::Kind {
            kind: "gear_helmet",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "jacket",
        label: "Jacket",
        source: RowSource::Kind {
            kind: "gear_jacket",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "pants",
        label: "Pants",
        source: RowSource::Kind {
            kind: "gear_pants",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "boots",
        label: "Boots",
        source: RowSource::Kind {
            kind: "gear_boots",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "vest",
        label: "Vest (chest rig)",
        source: RowSource::Kind {
            kind: "gear_vest",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "armoredVest",
        label: "Armored vest",
        source: RowSource::Kind {
            kind: "gear_armored_vest",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "backpack",
        label: "Backpack",
        source: RowSource::Kind {
            kind: "gear_backpack",
            weapon: None,
        },
    },
    LoadoutRow {
        key: "handwear",
        label: "Gloves",
        source: RowSource::Kind {
            kind: "gear_gloves",
            weapon: None,
        },
    },
];

/// The 4 weapon keys → their engine `(slotIndex, slotType)`.
pub const WEAPON_SLOTS: &[(&str, i64, &str)] = &[
    ("primary", 0, "primary"),
    ("launcher", 1, "primary"),
    ("handgun", 2, "secondary"),
    ("throwable", 3, "grenade"),
];

/// The 8 wear keys (the `wear{}` map; excludes optic/magazine which ride the rifle).
pub const WEAR_PICK_KEYS: &[&str] = &[
    "headCover",
    "jacket",
    "pants",
    "boots",
    "vest",
    "armoredVest",
    "backpack",
    "handwear",
];

/// The two primary sub-slots the doll folds onto the rifle rather than showing as body regions.
pub const PRIMARY_SUB_REGIONS: &[&str] = &["optic", "magazine"];

/// Look up a row by key.
pub fn row(key: &str) -> Option<&'static LoadoutRow> {
    LOADOUT_ROWS.iter().find(|r| r.key == key)
}

/* ───────────────────────────── compat edge graph ───────────────────────────── */

/// In-memory compat graph (collapses the React Comlink worker to a plain map). Keyed by
/// `edge_type` → adjacency (`node` → set of accepted counterpart nodes, both directions), so
/// `items_for(host, edge)` is a single lookup regardless of the seed's from/to convention.
#[derive(Default, Clone)]
pub struct CompatGraph {
    by_edge: HashMap<String, HashMap<String, HashSet<String>>>,
}

impl CompatGraph {
    pub fn from_edges(edges: &[RegistryCompatEdge]) -> Self {
        let mut by_edge: HashMap<String, HashMap<String, HashSet<String>>> = HashMap::new();
        for e in edges {
            let adj = by_edge.entry(e.edge_type.clone()).or_default();
            adj.entry(e.from_node.clone())
                .or_default()
                .insert(e.to_node.clone());
            adj.entry(e.to_node.clone())
                .or_default()
                .insert(e.from_node.clone());
        }
        Self { by_edge }
    }

    /// Sorted list of items the `host` accepts across `edge` (the counterpart node of every
    /// `edge`-typed edge touching `host`). Empty if the host has no such edges.
    pub fn items_for(&self, host: &str, edge: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .by_edge
            .get(edge)
            .and_then(|adj| adj.get(host))
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    /// Whether `host` accepts `item` over `edge`.
    pub fn accepts(&self, host: &str, item: &str, edge: &str) -> bool {
        self.by_edge
            .get(edge)
            .and_then(|adj| adj.get(host))
            .map(|set| set.contains(item))
            .unwrap_or(false)
    }
}

/// The compat feed status (mirrors React `loading | ready | unavailable`).
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CompatStatus {
    #[default]
    Loading,
    Ready,
    Unavailable,
}

/// The compat feed as one reactive value (status + graph) threaded to the Arsenal tab.
#[derive(Clone, Default)]
pub struct CompatFeed {
    pub status: CompatStatus,
    pub graph: CompatGraph,
}

impl CompatFeed {
    /// `graph` only when the feed is actually ready (edge rows show nothing pre-ready).
    pub fn ready_graph(&self) -> Option<&CompatGraph> {
        matches!(self.status, CompatStatus::Ready).then_some(&self.graph)
    }
}

/* ───────────────────────────── option building ───────────────────────────── */

/// One `<option>` — `(resource_name, display_name)`; `incompatible` flags a stranded live pick.
#[derive(Clone, PartialEq)]
pub struct RowOption {
    pub value: String,
    pub label: String,
    pub incompatible: bool,
}

/// Build a row's option list (React `rowValues`):
/// 1. raw = kind rows → catalog of the kind; edge rows → `items_for(picks[depends_on], edge)`.
/// 2. drop `abstract == true` OR `variant_of.is_some()` — EXCEPT never drop the live `current` pick.
/// 3. locale-ish sort by display_name.
/// 4. if `current` is set but not in the allowed values, append it as "… — incompatible".
///
/// `catalog_by_name` resolves display names; `graph` may be `None` (compat unavailable → edge rows
/// degrade to the full catalog of the counterpart kind is NOT possible without a host, so an edge
/// row with no graph / no dependency yields just the current pick, if any).
pub fn row_options(
    row: &LoadoutRow,
    current: &str,
    picks: &HashMap<String, String>,
    items: &[RegistryItem],
    catalog_by_name: &HashMap<String, &RegistryItem>,
    graph: Option<&CompatGraph>,
) -> Vec<RowOption> {
    let display = |rn: &str| {
        catalog_by_name
            .get(rn)
            .map(|it| it.display_name.clone())
            .unwrap_or_else(|| rn.to_string())
    };

    // 1. raw candidate resource_names.
    let raw: Vec<String> = match row.source {
        RowSource::Kind { kind, .. } => items
            .iter()
            .filter(|it| it.kind == kind)
            .map(|it| it.resource_name.clone())
            .collect(),
        RowSource::Edge { edge, depends_on } => {
            let host = picks.get(depends_on).map(String::as_str).unwrap_or("");
            match (graph, host.is_empty()) {
                (Some(g), false) => g.items_for(host, edge),
                _ => Vec::new(),
            }
        }
    };

    // 2. filter abstract/variant (keep current), then de-dup preserving.
    let mut allowed: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for rn in raw {
        let keep = rn == current
            || catalog_by_name
                .get(rn.as_str())
                .map(|it| it.r#abstract != Some(true) && it.variant_of.is_none())
                .unwrap_or(true);
        if keep && seen.insert(rn.clone()) {
            allowed.push(rn);
        }
    }

    // 3. sort by display name.
    allowed.sort_by(|a, b| display(a).cmp(&display(b)));

    let mut out: Vec<RowOption> = allowed
        .iter()
        .map(|rn| RowOption {
            value: rn.clone(),
            label: display(rn),
            incompatible: false,
        })
        .collect();

    // 4. stranded current pick (non-empty, not in the allowed set) stays visible, flagged.
    if !current.is_empty() && !out.iter().any(|o| o.value == current) {
        out.push(RowOption {
            value: current.to_string(),
            label: format!("{} — incompatible", display(current)),
            incompatible: true,
        });
    }
    out
}

/* ───────────────────────────── validation ───────────────────────────── */

// `Debug` (T-240): a fault is now a *refusal reason* a caller can propagate through a `Result`,
// and `expect`/`unwrap_err` on that Result needs to be able to print what it refused on.
#[derive(Clone, Debug, PartialEq)]
pub struct RowError {
    pub key: &'static str,
    pub message: String,
}

/// Validate every edge row against the compat feed (kind rows never fail). Empty picks are valid.
/// Returns the per-row errors; `is_empty()` == valid. Mirrors React `validateLoadout`.
pub fn validate_loadout(
    picks: &HashMap<String, String>,
    graph: Option<&CompatGraph>,
    status: CompatStatus,
) -> Vec<RowError> {
    let mut errs = Vec::new();
    // When the feed is unavailable, degrade gracefully: no edge validation (React degrades to the
    // dumb dropdowns and does not block export on a feed it never got).
    if status != CompatStatus::Ready {
        return errs;
    }
    for r in LOADOUT_ROWS {
        let RowSource::Edge { edge, depends_on } = r.source else {
            continue;
        };
        let value = picks.get(r.key).map(String::as_str).unwrap_or("");
        if value.is_empty() {
            continue; // an unset optional slot is always valid.
        }
        let host = picks.get(depends_on).map(String::as_str).unwrap_or("");
        let dep_label = row(depends_on).map(|d| d.label).unwrap_or(depends_on);
        if host.is_empty() {
            errs.push(RowError {
                key: r.key,
                message: format!("Requires a {dep_label} pick"),
            });
            continue;
        }
        if let Some(g) = graph {
            if !g.accepts(host, value, edge) {
                errs.push(RowError {
                    key: r.key,
                    message: format!("Not compatible with the selected {dep_label}"),
                });
            }
        }
    }
    errs
}

/* ───────────────────────────── paper-doll region model ───────────────────────────── */

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    Weapon,
    Wear,
}

pub struct DollRegion {
    pub key: &'static str,
    pub kind: RegionKind,
}

/// `RAIL_REGIONS` — the A3 slot-rail order (all 14 keys; weapons + rifle attachments first, then
/// head-to-toe wear). **Differs from `LOADOUT_ROWS` order**: vest/armoredVest/backpack pulled up
/// after helmet/jacket, pants/boots dropped to the end.
pub const RAIL_REGIONS: &[DollRegion] = &[
    DollRegion {
        key: "primary",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "optic",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "magazine",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "launcher",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "handgun",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "throwable",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "headCover",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "jacket",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "vest",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "armoredVest",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "backpack",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "handwear",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "pants",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "boots",
        kind: RegionKind::Wear,
    },
];

/// `DOLL_REGIONS` — the SVG doll's clickable regions (12; optic/magazine excluded — they ride the
/// rifle as `PRIMARY_SUB_REGIONS`).
pub const DOLL_REGIONS: &[DollRegion] = &[
    DollRegion {
        key: "headCover",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "jacket",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "vest",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "armoredVest",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "backpack",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "handwear",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "pants",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "boots",
        kind: RegionKind::Wear,
    },
    DollRegion {
        key: "primary",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "launcher",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "handgun",
        kind: RegionKind::Weapon,
    },
    DollRegion {
        key: "throwable",
        kind: RegionKind::Weapon,
    },
];

/* ───────────────────────────── weight ───────────────────────────── */

/// Honest loadout weight (React `loadoutWeight`): sum numeric `weight_kg`; a `None` weight is a
/// counted-but-unknown item (engine class default), NEVER guessed as 0.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct LoadoutWeight {
    pub known_kg: f64,
    pub unknown_count: u32,
    pub item_count: u32,
}

pub fn loadout_weight(
    picks: &HashMap<String, String>,
    catalog_by_name: &HashMap<String, &RegistryItem>,
) -> LoadoutWeight {
    let mut w = LoadoutWeight::default();
    // Deterministic order over the 14 canonical keys (BTreeMap only for stable iteration in tests).
    let ordered: BTreeMap<&str, &String> = LOADOUT_ROWS
        .iter()
        .filter_map(|r| picks.get(r.key).map(|v| (r.key, v)))
        .collect();
    for (_k, rn) in ordered {
        if rn.is_empty() {
            continue;
        }
        w.item_count += 1;
        match catalog_by_name.get(rn.as_str()).and_then(|it| it.weight_kg) {
            Some(kg) => w.known_kg += kg,
            None => w.unknown_count += 1,
        }
    }
    w
}

/// `formatLoadoutWeight` — "≥ X kg · N item(s) without weight data" when any unknown, else
/// "X kg · N item(s)".
pub fn format_loadout_weight(w: &LoadoutWeight) -> String {
    if w.unknown_count > 0 {
        format!(
            "≥ {:.1} kg · {} item{} without weight data",
            w.known_kg,
            w.unknown_count,
            if w.unknown_count == 1 { "" } else { "s" }
        )
    } else {
        format!(
            "{:.1} kg · {} item{}",
            w.known_kg,
            w.item_count,
            if w.item_count == 1 { "" } else { "s" }
        )
    }
}

/// Build the `resource_name → &RegistryItem` index the option/weight helpers take.
pub fn index_by_name(items: &[RegistryItem]) -> HashMap<String, &RegistryItem> {
    items
        .iter()
        .map(|it| (it.resource_name.clone(), it))
        .collect()
}

// ---- T-068.15.2 — cargo (SlotLoadoutV2.cargo[], loadout-export v2 shape) ----

/// One cargo row on `SlotLoadoutV2.cargo[]` (`{container, item, qty}` — the
/// loadout-export v2 skeleton, volume/weight budget model, no grid cells).
#[derive(Clone, Debug, PartialEq)]
pub struct CargoRow {
    pub container: String,
    pub item: String,
    pub qty: i64,
}

/// Wear keys that are cargo containers (capacity readout + cargo groups).
/// `armoredVest` shares the `vest` container key on the cargo side (spike lock:
/// container = first `TargetStorage=` path segment, and the engine emits `Vest/…`).
pub const CARGO_CONTAINERS: &[&str] = &["vest", "pants", "jacket", "backpack"];

/// Wear rows whose picked garment carries a capacity readout.
pub const CAPACITY_KEYS: &[&str] = &["vest", "armoredVest", "jacket", "pants", "backpack"];

/// Map a `character_default_cargo` evidence string (`TargetStorage=<path>`) to its
/// Arsenal container key: first path segment `Pants…`→pants, `Jacket…`→jacket,
/// `Vest…`→vest, `Back…`→backpack (spike lock). `None` = unknown segment (skipped).
pub fn cargo_container_from_evidence(evidence: &str) -> Option<&'static str> {
    let path = evidence.strip_prefix("TargetStorage=")?;
    let seg = path.split('/').next().unwrap_or("").to_ascii_lowercase();
    if seg.starts_with("pants") {
        Some("pants")
    } else if seg.starts_with("jacket") {
        Some("jacket")
    } else if seg.starts_with("vest") {
        Some("vest")
    } else if seg.starts_with("back") {
        Some("backpack")
    } else {
        None
    }
}

/// Per-character default cargo from the RAW compat edge rows (the `CompatGraph`
/// drops evidence + qty, so the seed map is built beside it). Aggregated by
/// (container, item) with qty summed; deterministic container/item order.
pub fn cargo_defaults_by_character(edges: &[RegistryCompatEdge]) -> HashMap<String, Vec<CargoRow>> {
    let mut by_char: HashMap<String, BTreeMap<(String, String), i64>> = HashMap::new();
    for e in edges {
        if e.edge_type != "character_default_cargo" {
            continue;
        }
        let Some(container) = cargo_container_from_evidence(&e.evidence) else {
            continue;
        };
        *by_char
            .entry(e.to_node.clone())
            .or_default()
            .entry((container.to_string(), e.from_node.clone()))
            .or_insert(0) += e.qty;
    }
    by_char
        .into_iter()
        .map(|(rn, m)| {
            let rows = m
                .into_iter()
                .map(|((container, item), qty)| CargoRow {
                    container,
                    item,
                    qty,
                })
                .collect();
            (rn, rows)
        })
        .collect()
}

/// Read `cargo[]` off a `SlotLoadoutV2` JSON → `(rows, key_present)`. A present key —
/// `[]`, `null` (normalized to no rows), or rows — is **user state** (seed-ineligible);
/// malformed rows are dropped, not errors.
pub fn cargo_from_loadout(loadout_json: Option<&str>) -> (Vec<CargoRow>, bool) {
    let Some(json) = loadout_json else {
        return (Vec::new(), false);
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return (Vec::new(), false);
    };
    let Some(c) = v.get("cargo") else {
        return (Vec::new(), false);
    };
    let rows = c
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|r| {
                    Some(CargoRow {
                        container: r.get("container")?.as_str()?.to_string(),
                        item: r.get("item")?.as_str()?.to_string(),
                        qty: r.get("qty")?.as_i64().filter(|q| *q >= 1)?,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (rows, true)
}

/// `cargo[]` as the canonical JSON array (loadout-export v2 row shape).
pub fn cargo_rows_json(rows: &[CargoRow]) -> serde_json::Value {
    serde_json::Value::Array(
        rows.iter()
            .map(|r| serde_json::json!({ "container": r.container, "item": r.item, "qty": r.qty }))
            .collect(),
    )
}

/// T-068.15.2 seed rule: eligible only when the loadout has **no `cargo` key** (or no
/// loadout at all); once seeded the writer always emits the key, so a user-cleared
/// list sticks. Returns the new loadout JSON when the seed applies.
pub fn seed_cargo(loadout_json: Option<&str>, defaults: &[CargoRow]) -> Option<String> {
    if defaults.is_empty() {
        return None;
    }
    let (_, key_present) = cargo_from_loadout(loadout_json);
    if key_present {
        return None;
    }
    let mut v = loadout_json
        .and_then(|j| serde_json::from_str::<serde_json::Value>(j).ok())
        .unwrap_or_else(|| {
            let mut wear = serde_json::Map::new();
            for k in WEAR_PICK_KEYS {
                wear.insert((*k).to_string(), serde_json::Value::Null);
            }
            serde_json::json!({ "version": 2, "wear": wear, "weapons": [] })
        });
    if !v.is_object() {
        return None;
    }
    v["cargo"] = cargo_rows_json(defaults);
    Some(v.to_string())
}

/// Cargo budget for one container group vs the picked garment's capacity
/// (absent capacity ⇒ no verdict — never invented).
pub struct CargoBudget {
    pub weight: f64,
    pub volume: f64,
    pub max_weight: Option<f64>,
    pub max_volume: Option<f64>,
}

impl CargoBudget {
    /// The overflow verdict. T-240: this is the predicate the block is built on — see
    /// [`cargo_capacity_errors`], which turns it into a [`RowError`] alongside the compat faults.
    pub fn over(&self) -> bool {
        self.max_weight.is_some_and(|m| self.weight > m)
            || self.max_volume.is_some_and(|m| self.volume > m)
    }
}

pub fn cargo_budget(
    idx: &HashMap<String, &RegistryItem>,
    garment: Option<&RegistryItem>,
    rows: &[CargoRow],
) -> CargoBudget {
    let mut weight = 0.0;
    let mut volume = 0.0;
    for r in rows {
        if let Some(it) = idx.get(r.item.as_str()) {
            weight += it.weight_kg.unwrap_or(0.0) * r.qty as f64;
            volume += it.volume_cm3.unwrap_or(0.0) * r.qty as f64;
        }
    }
    CargoBudget {
        weight,
        volume,
        max_weight: garment.and_then(|g| g.max_weight_kg),
        max_volume: garment.and_then(|g| g.max_volume_cm3),
    }
}

/* ─────────────── T-240 — cargo capacity as a *fault*, not a tint ─────────────── */

/// The worn garment backing a cargo container key. `vest` accepts the `armoredVest` row —
/// the two share one container on the cargo side (see [`CARGO_CONTAINERS`]). Returns the
/// **row key** the pick sits on as well, so a fault lands on the row the author must change.
pub fn cargo_garment<'a>(
    picks: &'a HashMap<String, String>,
    container: &'static str,
) -> Option<(&'static str, &'a str)> {
    let live = |k: &'static str| {
        picks
            .get(k)
            .map(String::as_str)
            .filter(|v| !v.is_empty())
            .map(|v| (k, v))
    };
    match container {
        "vest" => live("vest").or_else(|| live("armoredVest")),
        _ => live(container),
    }
}

/// Why an over-capacity fault is a **refusal and not a prediction**, appended to every
/// [`cargo_capacity_errors`] message.
///
/// The honest provenance: `max_weight_kg` / `max_volume_cm3` reach the website from a
/// Workbench-time scan (`TBD_RegistryScan.c` `DeriveCargoGrid`, :896-909 — `cells =
/// Ceil(maxVolume/50)`, grid width hardcoded to 4), and **the game never reads that export
/// back**. There is no runtime capacity arithmetic under `Scripts/Game/` at all: the equip
/// helper pushes at the engine and reads a bool. So this model is a heuristic over data that
/// is stale by design, and the wording must not pretend otherwise.
///
/// What *is* measured, and what justifies blocking rather than tinting: on a cargo unit the
/// authored container rejects, `TBD_LoadoutEquipHelper.c` retries into *any* storage on the
/// character, and if nothing accepts it deletes the entity and `break`s the row's qty loop —
/// dropping **the whole remaining quantity of that row**, not one item. Completeness is then
/// answered by `IsComplete()`: **T-415** `ReportVerdict` ERROR-refuses an incomplete pass, and
/// **T-541** `TBD_SpawnManager` consumes the same answer at the spawn boundary (settle after
/// `MaterializeSlotBodies`, refuse LOBBY/deploy when `IsComplete=0`). The website capacity
/// check is still an estimate of a figure the game never reads back — it exists so authors
/// see the likely drop *before* the mod refuses at spawn.
pub const CARGO_CAPACITY_CAVEAT: &str = "Capacity is a build-time catalogue figure the game never reads back, so treat it as an estimate, not a guarantee. The failure it points at is real: at spawn, cargo the character cannot hold is moved to another container or dropped — the rest of that row goes with it — and incomplete delivery is refused (IsComplete at ReportVerdict and the SpawnManager spawn boundary).";

/// T-240 — the over-capacity rows, in the same [`RowError`] shape the compat faults use, so a
/// consumer that already refuses on `validate_loadout` refuses on these too.
///
/// Keyed on the row carrying the **garment** pick (`armoredVest` when the vest container is
/// backed by one), matching the existing convention that a fault surfaces on the row whose
/// pick the author must change. Only the dimension(s) actually over are named.
///
/// Deliberately silent in two cases, both "never invent capacity":
/// * the garment has no `max_weight_kg` / `max_volume_cm3` — the scan had nothing to say;
/// * no garment is worn at all — there is no container to overflow.
///
/// T-504 — that second case is still not *this* function's business (a container with no capacity
/// cannot be over it), but it is no longer nobody's: cargo authored against a container the loadout
/// does not wear is now named by [`cargo_unworn_container_errors`], which **warns and never
/// blocks**. Keep the two apart — this one gates the export, that one must not.
pub fn cargo_capacity_errors(
    picks: &HashMap<String, String>,
    rows: &[CargoRow],
    idx: &HashMap<String, &RegistryItem>,
) -> Vec<RowError> {
    let mut errs = Vec::new();
    for container in CARGO_CONTAINERS {
        let container: &'static str = container;
        let Some((row_key, garment_rn)) = cargo_garment(picks, container) else {
            continue;
        };
        let garment = idx.get(garment_rn).copied();
        let group: Vec<CargoRow> = rows
            .iter()
            .filter(|r| r.container == container)
            .cloned()
            .collect();
        let budget = cargo_budget(idx, garment, &group);
        if !budget.over() {
            continue;
        }
        // Same figures, same formatting as the panel readout — the author must not have to
        // reconcile two different renderings of one number.
        let mut dims: Vec<String> = Vec::new();
        if let Some(m) = budget.max_weight.filter(|m| budget.weight > *m) {
            dims.push(format!("{:.1} / {m} kg", budget.weight));
        }
        if let Some(m) = budget.max_volume.filter(|m| budget.volume > *m) {
            dims.push(format!("{:.0} / {m} cm³", budget.volume));
        }
        let garment_label = garment.map_or(garment_rn, |g| g.display_name.as_str());
        errs.push(RowError {
            key: row_key,
            message: format!(
                "{container} cargo is over the catalogued capacity of {garment_label} — {}. {CARGO_CAPACITY_CAVEAT}",
                dims.join(" · ")
            ),
        });
    }
    errs
}

/* ───── T-504 — cargo with nowhere known to go ───── */

/// Why an unworn-container fault is a **warning and not a refusal**, appended to every
/// [`cargo_unworn_container_errors`] message.
///
/// What actually happens to the row, read out of `TBD_LoadoutEquipHelper.c` rather than guessed:
/// the container is resolved **at spawn**, after the wear pass has settled, by `GarmentForContainer`
/// → `SCR_CharacterInventoryStorageComponent.GetClothFromArea(…)` (:1047-1063) — i.e. from what the
/// body is *actually wearing*. Nothing worn there ⇒ `InsertCargo` raises
/// `Degrade("cargo:<container>", …, "this slot's kit wears no <container> — mission/kit authoring
/// mismatch, NOT a mod fault")` (:1121-1123) and the any-storage fallback re-homes the units
/// somewhere else on the body. That degrade is not cosmetic: `IsComplete()` is
/// `m_aFailures.IsEmpty() && m_aDegraded.IsEmpty()` (:209-212), so **one** such row makes the whole
/// pass incomplete, and `TBD_SpawnManager` consumes that at the spawn boundary — `LOBBY refused …
/// (IsComplete=0)` and `deploy DENIED … (IsComplete=0 …)`. The mod already says all of this out
/// loud; the author, who is here and not reading server logs, never hears it. That is the gap.
///
/// **Why it warns instead of refusing** (the export gate is deliberately not extended): "the
/// loadout picks no garment here" is **not** the same claim as "nothing will be worn here".
/// `TBD_LoadoutEquipHelper.IssueEquip` returns early on an empty gear field — *"absent gear slot —
/// kit garment (if any) is deliberately retained"* (:407-408) — so a slot whose kit prefab ships a
/// vest satisfies a `vest` cargo row with no `vest` pick anywhere in this editor. The website
/// cannot see inside the kit prefab, so it cannot tell those two apart, and a refusal would stop
/// authoring dead on loadouts that deliver perfectly. Weigh the two failures: a wrong refusal
/// blocks work that would have shipped, while an ignored warning lands on a defect the mod
/// **already** refuses loudly at spawn. Warn, count it, never block.
pub const CARGO_UNWORN_CAVEAT: &str = "The Arsenal only sees the wear this loadout picks, so if the slot's kit prefab wears one of its own the cargo still lands correctly — which is why this warns instead of refusing. If nothing wears it, the mod re-homes the items into whatever storage will take them and counts the row as degraded, and a degraded pass is refused at the spawn boundary (IsComplete=0 → LOBBY/deploy will not open).";

/// The edge type carrying a character prefab's own carried items — the seed source
/// ([`cargo_defaults_by_character`]) and the vouching evidence [`cargo_unworn_container_errors`]
/// uses. `from_node` is the item, `to_node` the character, so a [`CompatGraph`] lookup keyed on the
/// character returns its default items.
pub const CHARACTER_DEFAULT_CARGO_EDGE: &str = "character_default_cargo";

/// T-504 — cargo rows the website can find **no evidence** will be delivered, in the same
/// [`RowError`] shape the compat and capacity faults use, keyed on the wear row the author would
/// pick to fix it (`vest` / `jacket` / `pants` / `backpack` are all [`LOADOUT_ROWS`] keys).
///
/// # What counts as evidence, and why the naive rule is wrong
///
/// "This loadout picks no vest" is not enough on its own. The mod keeps the kit prefab's own
/// garment for any wear slot the loadout leaves empty (see [`CARGO_UNWORN_CAVEAT`]), and — decisive
/// for the shape of this rule — the **open-time seed fills cargo into exactly those kit-worn
/// containers**: `character_default_cargo` is a Workbench scan of what the character prefab already
/// carries, 16k+ edges of it in the shipped registry, keyed `TargetStorage=Vest/…`, `Pants/…`,
/// `Jacket/…`, `Back/…`. A rule that faulted every unpicked container with rows would therefore
/// fire on essentially every freshly opened Arsenal, and a verdict badge that is wrong that often
/// is a verdict badge nobody reads.
///
/// So `kit_defaults` carries the character's own default items, and a row whose item is one of them
/// is **vouched**: the scan found that item inside that container on the prefab, so the prefab wears
/// it. Only the rows left over — cargo the author added, into a container this loadout does not
/// pick and the kit is not known to carry anything in — are named.
///
/// `None` means the evidence was unavailable (compat feed not ready, or a slot with no `assetId`)
/// and the rule stays **silent**, the same degradation [`validate_loadout`] makes: a feed we never
/// received must not fail a loadout. An empty-but-present set is real evidence — a kit that carries
/// nothing by default vouches for nothing.
///
/// Residual, stated rather than hidden: a kit can wear an *empty* vest, which leaves no default-cargo
/// edge to vouch with, so an author-added row there is still named. That is the honest limit of a
/// build-time scan, and it is why this warns instead of refusing.
///
/// Feeds `arsenal::loadout_faults` (the verdict badge + the per-row line). It must **not** feed
/// [`cargo_capacity_errors`] or the export refusal.
pub fn cargo_unworn_container_errors(
    picks: &HashMap<String, String>,
    rows: &[CargoRow],
    kit_defaults: Option<&HashSet<String>>,
) -> Vec<RowError> {
    let Some(kit_defaults) = kit_defaults else {
        return Vec::new();
    };
    let mut errs = Vec::new();
    for container in CARGO_CONTAINERS {
        let container: &'static str = container;
        if cargo_garment(picks, container).is_some() {
            continue;
        }
        let n = rows
            .iter()
            .filter(|r| r.container == container && !kit_defaults.contains(&r.item))
            .count();
        if n == 0 {
            continue;
        }
        errs.push(RowError {
            key: container,
            message: format!(
                "{n} {container} cargo row(s) have nowhere known to go — this loadout picks no {container}, and the slot's kit is not catalogued as carrying anything there. Pick a {container} here, or move the cargo to a container this loadout wears. {CARGO_UNWORN_CAVEAT}"
            ),
        });
    }
    errs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(from: &str, to: &str, ty: &str) -> RegistryCompatEdge {
        RegistryCompatEdge {
            id: String::new(),
            modpack_id: String::new(),
            from_node: from.into(),
            to_node: to.into(),
            edge_type: ty.into(),
            evidence: String::new(),
            qty: 1,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn item(rn: &str, name: &str, kind: &str) -> RegistryItem {
        RegistryItem {
            id: String::new(),
            modpack_id: String::new(),
            resource_name: rn.into(),
            display_name: name.into(),
            category: String::new(),
            icon_url: None,
            kind: kind.into(),
            r#abstract: None,
            arsenal_type: None,
            weight_kg: None,
            volume_cm3: None,
            max_weight_kg: None,
            max_volume_cm3: None,
            cargo_grid_w: None,
            cargo_grid_h: None,
            addon: None,
            variant_of: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn picks(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn rows_cover_all_14_keys_incl_edge_rows() {
        assert_eq!(LOADOUT_ROWS.len(), 14);
        // optic + magazine are edge rows immediately after primary.
        assert_eq!(LOADOUT_ROWS[0].key, "primary");
        assert!(matches!(
            LOADOUT_ROWS[1].source,
            RowSource::Edge {
                edge: "optic_on_weapon",
                depends_on: "primary"
            }
        ));
        assert!(matches!(
            LOADOUT_ROWS[2].source,
            RowSource::Edge {
                edge: "mag_in_weapon",
                depends_on: "primary"
            }
        ));
        // rail order ≠ loadout order (vest pulled up before pants).
        let rail: Vec<&str> = RAIL_REGIONS.iter().map(|r| r.key).collect();
        let vest_i = rail.iter().position(|k| *k == "vest").unwrap();
        let pants_i = rail.iter().position(|k| *k == "pants").unwrap();
        assert!(vest_i < pants_i);
        // doll excludes optic/magazine.
        assert_eq!(DOLL_REGIONS.len(), 12);
        assert!(!DOLL_REGIONS
            .iter()
            .any(|r| r.key == "optic" || r.key == "magazine"));
    }

    #[test]
    fn items_for_returns_counterpart_both_directions() {
        let g = CompatGraph::from_edges(&[
            edge("weap_m4", "optic_acog", "optic_on_weapon"),
            edge("mag_stanag", "weap_m4", "mag_in_weapon"),
        ]);
        assert_eq!(
            g.items_for("weap_m4", "optic_on_weapon"),
            vec!["optic_acog"]
        );
        assert_eq!(g.items_for("weap_m4", "mag_in_weapon"), vec!["mag_stanag"]);
        assert!(g.accepts("weap_m4", "optic_acog", "optic_on_weapon"));
        assert!(!g.accepts("weap_m4", "optic_eotech", "optic_on_weapon"));
        assert!(g.items_for("weap_ak", "optic_on_weapon").is_empty());
    }

    #[test]
    fn optic_row_options_filtered_by_edges_and_current_preserved() {
        let items = vec![
            item("optic_acog", "ACOG", "gear_optic"),
            item("optic_eotech", "EOTech", "gear_optic"),
        ];
        let idx = index_by_name(&items);
        let g = CompatGraph::from_edges(&[edge("weap_m4", "optic_acog", "optic_on_weapon")]);
        let optic_row = row("optic").unwrap();

        // primary picked → only the compatible ACOG offered.
        let p = picks(&[("primary", "weap_m4")]);
        let opts = row_options(optic_row, "", &p, &items, &idx, Some(&g));
        assert_eq!(
            opts.iter().map(|o| o.value.as_str()).collect::<Vec<_>>(),
            vec!["optic_acog"]
        );

        // an incompatible live pick stays visible, flagged.
        let opts = row_options(optic_row, "optic_eotech", &p, &items, &idx, Some(&g));
        assert!(opts
            .iter()
            .any(|o| o.value == "optic_eotech" && o.incompatible));

        // no primary → no options.
        assert!(row_options(optic_row, "", &HashMap::new(), &items, &idx, Some(&g)).is_empty());
    }

    #[test]
    fn kind_row_excludes_abstract_and_variants() {
        let mut base = item("rifle_base", "Rifle (base)", "gear_primary");
        base.r#abstract = Some(true);
        let mut variant = item("rifle_camo", "Rifle (camo)", "gear_primary");
        variant.variant_of = Some("rifle_m16".into());
        let items = vec![item("rifle_m16", "M16", "gear_primary"), base, variant];
        let idx = index_by_name(&items);
        let opts = row_options(
            row("primary").unwrap(),
            "",
            &HashMap::new(),
            &items,
            &idx,
            None,
        );
        assert_eq!(
            opts.iter().map(|o| o.value.as_str()).collect::<Vec<_>>(),
            vec!["rifle_m16"]
        );
    }

    #[test]
    fn validation_flags_stranded_and_orphan_edges() {
        let g = CompatGraph::from_edges(&[edge("weap_m4", "optic_acog", "optic_on_weapon")]);
        // valid: compatible optic on its weapon.
        let ok = picks(&[("primary", "weap_m4"), ("optic", "optic_acog")]);
        assert!(validate_loadout(&ok, Some(&g), CompatStatus::Ready).is_empty());
        // optic with no primary → "Requires a Primary pick".
        let orphan = picks(&[("optic", "optic_acog")]);
        let e = validate_loadout(&orphan, Some(&g), CompatStatus::Ready);
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].key, "optic");
        // incompatible optic → rejected.
        let bad = picks(&[("primary", "weap_m4"), ("optic", "optic_eotech")]);
        assert_eq!(
            validate_loadout(&bad, Some(&g), CompatStatus::Ready).len(),
            1
        );
        // unavailable feed → no edge validation.
        assert!(validate_loadout(&bad, Some(&g), CompatStatus::Unavailable).is_empty());
    }

    #[test]
    fn weight_is_honest_about_unknowns() {
        let mut m4 = item("weap_m4", "M4", "gear_primary");
        m4.weight_kg = Some(3.4);
        let helmet = item("helm", "Helmet", "gear_helmet"); // no weight
        let items = vec![m4, helmet];
        let idx = index_by_name(&items);
        let p = picks(&[("primary", "weap_m4"), ("headCover", "helm")]);
        let w = loadout_weight(&p, &idx);
        assert_eq!(w.item_count, 2);
        assert_eq!(w.unknown_count, 1);
        assert!((w.known_kg - 3.4).abs() < 1e-9);
        assert_eq!(
            format_loadout_weight(&w),
            "≥ 3.4 kg · 1 item without weight data"
        );

        // all known → plain readout.
        let p2 = picks(&[("primary", "weap_m4")]);
        assert_eq!(
            format_loadout_weight(&loadout_weight(&p2, &idx)),
            "3.4 kg · 1 item"
        );
    }

    // ---- T-068.15.2 cargo ----

    fn cargo_edge(item: &str, character: &str, target: &str, n: usize) -> Vec<RegistryCompatEdge> {
        (0..n)
            .map(|_| {
                let mut e = edge(item, character, "character_default_cargo");
                e.evidence = format!("TargetStorage={target}");
                e
            })
            .collect()
    }

    #[test]
    fn cargo_container_mapping_follows_spike_lock() {
        assert_eq!(
            cargo_container_from_evidence("TargetStorage=Pants/Pants_US_BDU.et"),
            Some("pants")
        );
        assert_eq!(
            cargo_container_from_evidence("TargetStorage=Jacket_US_BDU.et"),
            Some("jacket")
        );
        assert_eq!(
            cargo_container_from_evidence("TargetStorage=Vest_PASGT/MagPouch/x.et"),
            Some("vest")
        );
        assert_eq!(
            cargo_container_from_evidence("TargetStorage=Back/Backpack_ALICE.et"),
            Some("backpack")
        );
        // Unknown segment or non-TargetStorage evidence → skipped, never guessed.
        assert_eq!(
            cargo_container_from_evidence("TargetStorage=Helmet/x.et"),
            None
        );
        assert_eq!(cargo_container_from_evidence("LoadoutSlotInfo"), None);
    }

    #[test]
    fn cargo_defaults_aggregate_by_container_item() {
        let mut edges = cargo_edge("mag_stanag", "char_us_rfl", "Vest/Pouch1/x.et", 3);
        edges.extend(cargo_edge(
            "mag_stanag",
            "char_us_rfl",
            "Vest/Pouch2/x.et",
            2,
        ));
        edges.extend(cargo_edge("bandage", "char_us_rfl", "Pants/Pants.et", 1));
        edges.extend(cargo_edge("bandage", "char_other", "Pants/Pants.et", 1));
        edges.push(edge("mag_stanag", "rifle", "mag_in_weapon")); // other family ignored
        let map = cargo_defaults_by_character(&edges);
        assert_eq!(
            map["char_us_rfl"],
            vec![
                CargoRow {
                    container: "pants".into(),
                    item: "bandage".into(),
                    qty: 1
                },
                CargoRow {
                    container: "vest".into(),
                    item: "mag_stanag".into(),
                    qty: 5
                },
            ]
        );
        assert_eq!(map["char_other"].len(), 1);
    }

    #[test]
    fn seed_only_when_cargo_key_absent() {
        let defaults = vec![CargoRow {
            container: "vest".into(),
            item: "mag".into(),
            qty: 2,
        }];
        // No loadout at all → minimal V2 shell + cargo.
        let seeded = seed_cargo(None, &defaults).unwrap();
        let v: serde_json::Value = serde_json::from_str(&seeded).unwrap();
        assert_eq!(v["version"], 2);
        assert_eq!(v["wear"].as_object().unwrap().len(), WEAR_PICK_KEYS.len());
        assert_eq!(v["cargo"][0]["qty"], 2);
        // Loadout without the key → key added, rest preserved.
        let lo = r#"{"version":2,"wear":{"vest":"v1"},"weapons":[]}"#;
        let seeded = seed_cargo(Some(lo), &defaults).unwrap();
        let v: serde_json::Value = serde_json::from_str(&seeded).unwrap();
        assert_eq!(v["wear"]["vest"], "v1");
        assert_eq!(v["cargo"].as_array().unwrap().len(), 1);
        // Present key — populated, empty, or null — is user state: never reseeded.
        for user in [
            r#"{"version":2,"cargo":[{"container":"vest","item":"x","qty":9}]}"#,
            r#"{"version":2,"cargo":[]}"#,
            r#"{"version":2,"cargo":null}"#,
        ] {
            assert!(seed_cargo(Some(user), &defaults).is_none());
        }
        // No defaults → nothing to seed.
        assert!(seed_cargo(None, &[]).is_none());
    }

    #[test]
    fn cargo_roundtrip_and_budget() {
        let rows = vec![
            CargoRow {
                container: "vest".into(),
                item: "mag".into(),
                qty: 4,
            },
            CargoRow {
                container: "pants".into(),
                item: "bandage".into(),
                qty: 2,
            },
        ];
        let json = format!(r#"{{"version":2,"cargo":{}}}"#, cargo_rows_json(&rows));
        let (parsed, present) = cargo_from_loadout(Some(&json));
        assert!(present);
        assert_eq!(parsed, rows);
        // Malformed rows drop; qty < 1 drops.
        let (parsed, present) = cargo_from_loadout(Some(
            r#"{"cargo":[{"container":"vest"},{"container":"v","item":"i","qty":0}]}"#,
        ));
        assert!(present && parsed.is_empty());

        let mut mag = item("mag", "Mag", "magazine");
        mag.weight_kg = Some(0.5);
        mag.volume_cm3 = Some(60.0);
        let mut vest = item("vest_rn", "Vest", "gear_vest");
        vest.max_weight_kg = Some(5.0);
        vest.max_volume_cm3 = Some(200.0);
        let items = vec![mag, vest];
        let idx = index_by_name(&items);
        let vest_ref = *idx.get("vest_rn").unwrap();
        let b = cargo_budget(&idx, Some(vest_ref), &rows[..1]);
        assert!((b.weight - 2.0).abs() < 1e-9 && (b.volume - 240.0).abs() < 1e-9);
        assert!(b.over(), "240 cm³ > 200 cm³ capacity");
        // Absent capacity stays silent — never invented.
        let no_cap = item("nc", "NoCap", "gear_vest");
        let b2 = cargo_budget(&idx, Some(&no_cap), &rows[..1]);
        assert!(!b2.over());
    }

    /* ─────────────── T-240 — capacity as a fault, not a tint ─────────────── */

    /// Two magazines' worth of helpers for the capacity tests: a 0.5 kg / 60 cm³ magazine and a
    /// vest catalogued at 5 kg / 200 cm³.
    fn capacity_fixture() -> Vec<RegistryItem> {
        let mut mag = item("mag", "Mag", "magazine");
        mag.weight_kg = Some(0.5);
        mag.volume_cm3 = Some(60.0);
        let mut vest = item("vest_rn", "Plate Carrier", "gear_vest");
        vest.max_weight_kg = Some(5.0);
        vest.max_volume_cm3 = Some(200.0);
        let mut pack = item("pack_rn", "Rucksack", "gear_backpack");
        pack.max_weight_kg = Some(20.0);
        pack.max_volume_cm3 = Some(4000.0);
        vec![mag, vest, pack]
    }

    fn cargo(container: &str, item: &str, qty: i64) -> CargoRow {
        CargoRow {
            container: container.into(),
            item: item.into(),
            qty,
        }
    }

    #[test]
    fn cargo_over_capacity_is_a_fault_a_verdict_can_refuse_on() {
        let items = capacity_fixture();
        let idx = index_by_name(&items);
        let p = picks(&[("vest", "vest_rn"), ("backpack", "pack_rn")]);

        // 4 × 60 = 240 cm³ into a 200 cm³ vest, while the backpack is nowhere near its limit.
        let rows = vec![cargo("vest", "mag", 4), cargo("backpack", "mag", 4)];
        let errs = cargo_capacity_errors(&p, &rows, &idx);
        assert_eq!(errs.len(), 1, "only the overflowing container faults");
        assert_eq!(errs[0].key, "vest");
        let head = errs[0]
            .message
            .strip_suffix(CARGO_CAPACITY_CAVEAT)
            .expect("every capacity fault carries the caveat verbatim");
        // Names the offending dimension with both numbers, in the panel's own formatting …
        assert!(head.contains("240 / 200 cm³"), "{head}");
        assert!(head.contains("Plate Carrier"), "{head}");
        // … and stays quiet about the dimension inside budget (2.0 of 5 kg).
        assert!(!head.contains("kg"), "{head}");

        // One magazine fewer: 180 ≤ 200 → no fault at all. The block is a limit, not a mood.
        let ok = vec![cargo("vest", "mag", 3), cargo("backpack", "mag", 4)];
        assert!(cargo_capacity_errors(&p, &ok, &idx).is_empty());
    }

    #[test]
    fn cargo_fault_keys_on_the_row_the_author_must_change() {
        let mut brick = item("brick", "Brick", "gear_item");
        brick.weight_kg = Some(4.0);
        brick.volume_cm3 = Some(300.0);
        let mut av = item("av_rn", "Armored Vest", "gear_vest");
        av.max_weight_kg = Some(5.0);
        av.max_volume_cm3 = Some(200.0);
        let items = vec![brick, av];
        let idx = index_by_name(&items);

        // The `vest` CONTAINER is backed by the `armoredVest` ROW (the spike-locked alias), so
        // the fault must surface there — the row whose pick the author would change.
        let p = picks(&[("armoredVest", "av_rn")]);
        let errs = cargo_capacity_errors(&p, &[cargo("vest", "brick", 2)], &idx);
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].key, "armoredVest");
        // Both dimensions are over (8.0/5 kg, 600/200 cm³) → both are named.
        let head = errs[0].message.strip_suffix(CARGO_CAPACITY_CAVEAT).unwrap();
        assert!(head.contains("8.0 / 5 kg"), "{head}");
        assert!(head.contains("600 / 200 cm³"), "{head}");
    }

    #[test]
    fn cargo_capacity_never_invents_a_limit() {
        let items = capacity_fixture();
        let idx = index_by_name(&items);
        let heavy = vec![cargo("vest", "mag", 40)];

        // Garment worn, but the Workbench scan gave it no capacity → silent.
        let plain = {
            let mut v = capacity_fixture();
            v.push(item("plain_rn", "Uncatalogued Vest", "gear_vest"));
            v
        };
        let plain_idx = index_by_name(&plain);
        assert!(
            cargo_capacity_errors(&picks(&[("vest", "plain_rn")]), &heavy, &plain_idx).is_empty()
        );
        // No garment worn at all → silent; there is no container to overflow.
        assert!(cargo_capacity_errors(&picks(&[]), &heavy, &idx).is_empty());
        // A pick the catalog does not know → silent, never guessed.
        assert!(cargo_capacity_errors(&picks(&[("vest", "ghost")]), &heavy, &idx).is_empty());
    }

    #[test]
    fn cargo_fault_wording_refuses_without_overclaiming() {
        // The block rides stale-by-design data: `TBD_RegistryScan.c` `DeriveCargoGrid` (:896-909)
        // is a Workbench-time export the game never reads back, and the game has no runtime
        // capacity arithmetic to agree or disagree with it. So the wording must hedge the NUMBER
        // while staying blunt about the measured CONSEQUENCE (drop + IsComplete refuse at spawn).
        // Dropping either half fails here.
        let c = CARGO_CAPACITY_CAVEAT;
        assert!(c.contains("estimate, not a guarantee"), "{c}");
        assert!(c.contains("never reads back"), "{c}");
        assert!(c.contains("IsComplete"), "{c}");
        assert!(c.contains("refused"), "{c}");
        assert!(
            !c.contains("zero callers"),
            "T-415/T-541: IsComplete is consumed — caveat must not claim zero callers: {c}"
        );
        for overclaim in [
            "will not fit",
            "guaranteed",
            "cannot be delivered",
            "will be rejected",
        ] {
            assert!(
                !c.contains(overclaim),
                "capacity wording must not promise certainty it does not have: {overclaim}"
            );
        }
    }

    /* ───── T-504 — cargo with nowhere known to go ───── */

    /// The kit's catalogued default items (what `character_default_cargo` vouches for).
    fn kit(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn unworn_container_cargo_is_named_not_silent() {
        // Magazines and a brick into a vest, with no vest picked and a kit that is catalogued as
        // carrying neither. Before T-504 this produced no fault at all: the panel said "vest — no
        // garment worn", the export gate passed it, and the author first heard about it from a
        // server log they were never going to read.
        let rows = vec![
            cargo("vest", "mag", 4),
            cargo("vest", "brick", 1),
            cargo("backpack", "mag", 2),
        ];
        let p = picks(&[("backpack", "pack_rn")]);
        let errs = cargo_unworn_container_errors(&p, &rows, Some(&kit(&[])));
        assert_eq!(
            errs.len(),
            1,
            "only the unvouched container faults: {errs:?}"
        );
        // Keyed on the row the author would pick to fix it — the same convention the compat and
        // capacity faults use, so the message lands next to the control that resolves it.
        assert_eq!(errs[0].key, "vest");
        assert!(LOADOUT_ROWS.iter().any(|r| r.key == errs[0].key));
        let head = errs[0]
            .message
            .strip_suffix(CARGO_UNWORN_CAVEAT)
            .expect("every unworn-container fault carries the caveat verbatim");
        // Counts the undeliverable ROWS (2), not their units — the author fixes rows.
        assert!(head.contains("2 vest cargo row(s)"), "{head}");
        assert!(head.contains("nowhere known to go"), "{head}");
    }

    #[test]
    fn a_worn_container_is_silent_including_the_armored_vest_alias() {
        let rows = vec![cargo("vest", "mag", 4)];
        let none = kit(&[]);
        // Plain vest pick backs the container.
        let p = picks(&[("vest", "vest_rn")]);
        assert!(cargo_unworn_container_errors(&p, &rows, Some(&none)).is_empty());
        // …and so does `armoredVest`, which shares the `vest` container (the spike-locked alias).
        // Getting this wrong would fault every armoured loadout in the library.
        let p = picks(&[("armoredVest", "av")]);
        assert!(cargo_unworn_container_errors(&p, &rows, Some(&none)).is_empty());
        // An empty-string pick is not a pick (`cargo_garment` filters it) → still a fault.
        let p = picks(&[("vest", "")]);
        assert_eq!(
            cargo_unworn_container_errors(&p, &rows, Some(&none)).len(),
            1
        );
        // A container with nothing to deliver has nothing to warn about.
        assert!(cargo_unworn_container_errors(&picks(&[]), &[], Some(&none)).is_empty());
        assert_eq!(
            cargo_unworn_container_errors(&picks(&[]), &rows, Some(&none)).len(),
            1,
            "…but one row is enough"
        );
    }

    #[test]
    fn the_kits_own_default_cargo_is_never_faulted() {
        // THE false positive this rule exists to avoid. `character_default_cargo` is a scan of what
        // the character prefab already carries — 16k+ edges in the shipped registry, keyed
        // `TargetStorage=Vest/…` etc — and `seed_cargo` fills the Arsenal from it at open time. Those
        // containers are worn BY THE KIT, and the mod keeps a kit garment for any wear slot the
        // loadout leaves empty. Faulting them would put "N issue(s)" on every untouched slot in the
        // library, and a badge that is wrong that often is a badge nobody reads.
        let edges = vec![
            {
                let mut e = edge("mag", "kit:us_rifleman", "character_default_cargo");
                e.evidence = "TargetStorage=Vest/Mags".into();
                e
            },
            {
                let mut e = edge("bandage", "kit:us_rifleman", "character_default_cargo");
                e.evidence = "TargetStorage=Pants/Left".into();
                e
            },
        ];
        // The seed the Arsenal actually opens with…
        let seeded = cargo_defaults_by_character(&edges)
            .remove("kit:us_rifleman")
            .expect("the character has defaults");
        assert_eq!(seeded.len(), 2, "{seeded:?}");
        // …and the vouching set the UI derives from the same edge type, keyed on the character.
        let vouched: HashSet<String> = CompatGraph::from_edges(&edges)
            .items_for("kit:us_rifleman", CHARACTER_DEFAULT_CARGO_EDGE)
            .into_iter()
            .collect();
        assert!(
            vouched.contains("mag") && vouched.contains("bandage"),
            "{vouched:?}"
        );

        // An untouched, freshly seeded Arsenal picks no wear at all — and must be silent.
        assert!(
            cargo_unworn_container_errors(&picks(&[]), &seeded, Some(&vouched)).is_empty(),
            "a seeded loadout must not fault"
        );
        // Add one row the kit is not catalogued as carrying, and only THAT row is named.
        let mut rows = seeded.clone();
        rows.push(cargo("vest", "brick", 1));
        let errs = cargo_unworn_container_errors(&picks(&[]), &rows, Some(&vouched));
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert_eq!(errs[0].key, "vest");
        assert!(errs[0].message.contains("1 vest cargo row(s)"), "{errs:?}");
    }

    #[test]
    fn no_evidence_means_silence_not_a_guess() {
        // `None` = the compat feed never became ready, or the slot carries no `assetId`, so the
        // vouching set could not be built. Same degradation `validate_loadout` makes: a feed we
        // never received must not fail a loadout. Guessing here would fault every seeded slot
        // during the window before the registry lands.
        let rows = vec![cargo("vest", "mag", 4)];
        assert!(cargo_unworn_container_errors(&picks(&[]), &rows, None).is_empty());
        // An empty-but-present set is real evidence — a kit catalogued as carrying nothing
        // vouches for nothing — so it still faults.
        assert_eq!(
            cargo_unworn_container_errors(&picks(&[]), &rows, Some(&kit(&[]))).len(),
            1
        );
    }

    #[test]
    fn the_unworn_warning_never_becomes_an_export_refusal() {
        // The whole point of the warn/refuse split: `cargo_capacity_errors` gates the export, and
        // T-504 must not sneak into it. A container with no garment has no capacity to exceed, so
        // the block stays empty over the exact input the warning fires on. If a later slice folds
        // the unworn rule into the capacity call, Save/Export starts refusing loadouts the kit
        // prefab would have carried fine — and this goes red first.
        let items = capacity_fixture();
        let idx = index_by_name(&items);
        let rows = vec![cargo("vest", "mag", 400)];
        let bare = picks(&[]);
        assert_eq!(
            cargo_unworn_container_errors(&bare, &rows, Some(&kit(&[]))).len(),
            1
        );
        assert!(
            cargo_capacity_errors(&bare, &rows, &idx).is_empty(),
            "unworn containers must not reach the export refusal"
        );
    }

    #[test]
    fn unworn_wording_states_the_consequence_without_claiming_certainty() {
        let c = CARGO_UNWORN_CAVEAT;
        // The escape hatch that makes this a warning and not a refusal: the kit prefab's own
        // clothing is invisible to this editor, so "unworn here" is not "unworn at spawn".
        assert!(c.contains("kit prefab"), "{c}");
        assert!(c.contains("warns instead of refusing"), "{c}");
        // …and the measured consequence when the kit does NOT save it (helper Degrade →
        // IsComplete=0 → SpawnManager refuses). Dropping this half turns a real defect into a shrug.
        assert!(c.contains("IsComplete=0"), "{c}");
        assert!(c.contains("LOBBY/deploy"), "{c}");
        for overclaim in ["will be dropped", "cannot be delivered", "guaranteed"] {
            assert!(
                !c.contains(overclaim),
                "unworn wording must not promise a failure it cannot see: {overclaim}"
            );
        }
    }
}
