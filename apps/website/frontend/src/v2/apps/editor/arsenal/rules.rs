//! The Smart-Arsenal domain core — every decision the loadout editor makes, and no rendering.
//!
//! **Role:** owns the 14 loadout rows (including the compatibility **edge** rows, optic and
//! magazine), the compatibility edge graph behind `items_for`, per-row option building (abstract
//! and variant rows filtered out, a stranded pick preserved so it can still be seen and
//! cleared), loadout validation, the paper-doll region model and the weight readout.
//! **Position:** the pure floor of `v2::apps::editor::arsenal`. The panels and the persisted
//! `SlotLoadoutV2` serialization sit on top of it; it depends on neither, so it is framework-free
//! and tested on the native shell.
//! **Signals & state:** none. Every entry point is a function of the registry rows, the
//! compatibility edges and the current picks.
//! **Invariants:** the blanket `allow(dead_code)` below is earned by exactly three items in the
//! shipping wasm32 build — `PRIMARY_SUB_REGIONS`, `DollRegion::kind` and `DOLL_REGIONS`, the
//! paper-doll region model that currently has no consumer. A native `cargo check` lists more,
//! but those are consumers behind `cfg(target_arch = "wasm32")` rather than real deadness.
//! While the `allow` is on, the compiler cannot report an unwired rule, so remove it the moment
//! those three find a caller or go — and do not delete them to get there.
#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap, HashSet};
pub use website_map_engine::data::store::operations::cargo_rules::cargo_from_loadout;
pub use website_map_engine::data::store::operations::cargo_rules::cargo_rows_json;

pub use website_map_engine::data::store::operations::cargo_rules::CargoRow;

use crate::v2::core::api::dto::{RegistryCompatEdge, RegistryItem};

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

/// One row of the loadout editor: the pick slot's stable key, the caption beside it, and where
/// its options come from.
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
    /// Build the adjacency from the registry's raw compatibility edges.
    ///
    /// Every edge is inserted in both directions under its own `edge_type`, which is what makes
    /// the seed's from/to convention irrelevant to [`CompatGraph::items_for`]. Evidence and
    /// quantity are dropped here — cargo defaults need them, so they are read separately from the
    /// same rows by [`cargo_defaults_by_character`].
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
/// One loadout row's refusal: the row's key and the sentence shown against it.
///
/// `Debug` because a fault is a refusal reason a caller propagates through a `Result`, and
/// `expect` / `unwrap_err` on that `Result` has to be able to print what it refused on.
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

/// Which half of the paper doll a region belongs to: a carried weapon, or worn equipment.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RegionKind {
    Weapon,
    Wear,
}

/// One clickable region of the paper doll: the pick key it edits and the half it belongs to.
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

/// Total the picked items' weights against `catalog_by_name`, walking the canonical row order so
/// the result does not depend on the pick map's iteration order.
///
/// An item the catalog has no weight for is counted as unknown rather than as zero, so the
/// readout can say the total is incomplete instead of quietly understating it. Empty picks
/// contribute nothing at all.
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

/// Sum one container group's `rows` into a [`CargoBudget`] and pair the totals with the picked
/// `garment`'s capacity.
///
/// A row naming an item `idx` does not hold contributes nothing, and an item with no recorded
/// weight or volume contributes zero on that axis — a missing figure must not inflate a total
/// that decides whether the operator is over capacity.
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
/// Workbench-time scan (`TBD_RegistryScan.c` `DeriveCargoGrid` — `cells =
/// Ceil(maxVolume/50)`, grid width hardcoded to 4), and **the game never reads that export
/// back**. There is no runtime capacity arithmetic under `Scripts/Game/` at all: the equip
/// helper pushes at the engine and reads a bool. So this model is a heuristic over data that
/// is stale by design, and the wording must not pretend otherwise.
///
/// What *is* measured, and what justifies blocking rather than tinting: on a cargo unit the
/// authored container rejects, `TBD_LoadoutEquipHelper.c` retries into *any* storage on the
/// character, and if nothing accepts it deletes the entity and `break`s the row's qty loop —
/// dropping **the whole remaining quantity of that row**, not one item.
///
/// **T-605 CORRECTED THE CONSEQUENCE, AND THIS TEXT WITH IT.** T-415 had `ReportVerdict`
/// ERROR-refuse any pass that was not `IsComplete()`, and T-541 wired that same answer to the
/// SPAWN BOUNDARY — so one dropped magazine kept *every* client in LOADING, not just its owner.
/// That was the defect, and it is fixed in the mod: the spawn boundary now refuses only a body
/// that is UNPLAYABLE (an asset that does not exist, a storage or weapon slot that does not
/// exist, a garment that would not go on — `TBD_LoadoutApplication.HasBlockingFailure`). A
/// character that simply ran out of room is a SHORTFALL: the session opens, that player plays,
/// and the mod reports the missing items on the console and in the admin trail.
///
/// So this caveat must no longer threaten a refusal — that would be a false threat, and the next
/// author to hit it would learn the warning lies. The real cost is the one it always described:
/// the player does not get the items, and the author here is the only person who can fix it.
pub const CARGO_CAPACITY_CAVEAT: &str = "Capacity is a build-time catalogue figure the game never reads back, so treat it as an estimate, not a guarantee. The failure it points at is real: at spawn, cargo the character cannot hold is moved to another container or dropped — the rest of that row goes with it. The round still starts and the player is still playable, but they will not be carrying these items and only you can fix that, here.";

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
/// → `SCR_CharacterInventoryStorageComponent.GetClothFromArea(…)` — i.e. from what the
/// body is *actually wearing*. Nothing worn there ⇒ `InsertCargo` raises
/// `Degrade("cargo:<container>", …, "this slot's kit wears no <container> — mission/kit authoring
/// mismatch, NOT a mod fault")` and the any-storage fallback re-homes the units
/// somewhere else on the body.
///
/// **T-605 — WHAT THAT DEGRADE COSTS, CORRECTED.** This block used to end "…and
/// `TBD_SpawnManager` consumes that at the spawn boundary — `LOBBY refused … (IsComplete=0)`",
/// which was true and was the bug: `IsComplete()` is
/// `m_aFailures.IsEmpty() && m_aDegraded.IsEmpty()`, so ONE such row on ONE slot kept
/// EVERY client in LOADING. T-504 (this rule) was right that the authoring side must not refuse,
/// and T-605 established that the spawn side must not refuse either: a degrade means the item is
/// on the body in a different container, which is a player who can play. The boundary now reads
/// `HasBlockingFailure()` and this row is not one. The session opens; the mod logs the row and
/// records it in the admin trail; the placement the author asked for is still not what they get.
///
/// So the author is now the ONLY person who will ever act on this, which raises the stakes on
/// this message rather than lowering them — nothing downstream will stop the mission any more.
///
/// **Why it warns instead of refusing** (the export gate is deliberately not extended): "the
/// loadout picks no garment here" is **not** the same claim as "nothing will be worn here".
/// `TBD_LoadoutEquipHelper.IssueEquip` returns early on an empty gear field — *"absent gear slot —
/// kit garment (if any) is deliberately retained"* — so a slot whose kit prefab ships a
/// vest satisfies a `vest` cargo row with no `vest` pick anywhere in this editor. The website
/// cannot see inside the kit prefab, so it cannot tell those two apart, and a refusal would stop
/// authoring dead on loadouts that deliver perfectly. Weigh the two failures: a wrong refusal
/// blocks work that would have shipped, while an ignored warning lands on a row the mod delivers
/// somewhere else and reports. Warn, count it, never block.
pub const CARGO_UNWORN_CAVEAT: &str = "The Arsenal only sees the wear this loadout picks, so if the slot's kit prefab wears one of its own the cargo still lands correctly — which is why this warns instead of refusing. If nothing wears it, the mod re-homes the items into whatever storage will take them and reports the row as degraded on the server and in the admin trail. The round still starts, so nothing downstream will catch this for you: if the placement matters, fix it here.";

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

/* ═════ T-686 — checking a document against the SHIPPED `loadout-export.schema.json` ═════ */

/// The bytes of `packages/tbd-schema/schema/loadout-export.schema.json`, compiled in.
///
/// `include_str!` and **not** a transcription of the rules, for the same reason
/// `arsenal::tests::export_schema` reads the file rather than restating it: the defect T-199 fixed
/// was a writer checked against somebody's *reading* of the schema. An importer carrying its own
/// hand-copied rule list would reproduce that failure one layer down, and would drift silently the
/// first time the schema gains a required key or closes another object. This is a **read** of a
/// file this module does not own — nothing here writes it, and the build breaks loudly if it moves.
///
/// Compiled in rather than fetched because the check has to work in the browser, where there is no
/// filesystem: the schema the importer enforces and the schema the repo ships are then the same
/// bytes by construction, not by deployment discipline.
pub const LOADOUT_EXPORT_SCHEMA_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/tbd-schema/schema/loadout-export.schema.json"
));

/// Every JSON Schema keyword this checker implements, plus the annotations it may ignore.
///
/// The list is a **guard, not documentation**: a keyword outside it is reported as a refusal
/// (see [`validate_against_loadout_export_schema`]), so the day the schema grows a `pattern`,
/// `maxItems` or `uniqueItems` the importer stops accepting documents it only partly checked
/// instead of quietly waving them through. That "reports success over an input it never examined"
/// failure is this repo's signature defect; here it is designed out.
const SUPPORTED_SCHEMA_KEYWORDS: &[&str] = &[
    // annotations — no effect on validity
    "$schema",
    "$id",
    "title",
    "description",
    "$defs",
    // implemented assertions
    "$ref",
    "oneOf",
    "type",
    "const",
    "enum",
    "required",
    "properties",
    "patternProperties",
    "additionalProperties",
    "items",
    "minLength",
    "minimum",
];

/// The keywords whose value is a map of **name → subschema**. Their keys are author-chosen names,
/// never keywords, so a walk descends into the values and must not read the keys as assertions.
///
/// This constant, [`SCHEMA_SUBSCHEMA_KEYWORDS`] and `oneOf` are what make [`audit_schema_support`]
/// STRUCTURAL. The guard it replaces asked "is this node a schema?" by looking for a keyword it
/// recognised, so a node carrying only keywords it did NOT recognise answered "not a schema" and
/// was skipped — the one shape the guard existed to catch (T-735).
const SCHEMA_NAMED_SUBSCHEMAS: &[&str] = &["properties", "patternProperties", "$defs"];

/// The keywords whose value is a single subschema (where it is not a boolean).
const SCHEMA_SUBSCHEMA_KEYWORDS: &[&str] = &["items", "additionalProperties"];

/// How many refusals to carry back. A malformed document can fail every key it has; the author
/// needs the first handful to act, not a wall.
const MAX_SCHEMA_FAULTS: usize = 12;

/// A verdict under construction, split by **kind** — and the split is one half of the T-735 fix.
///
/// A **fault** is the document's problem: it breaks a rule this build implements. A **refusal** is
/// this build's problem: the schema says something it cannot evaluate, so answering "valid" would
/// be a claim about bytes it never examined.
///
/// Pooling the two is what made [`check_schema_one_of`] fail open. That function rightly discards
/// the complaints of branches the document did not claim — but a refusal is not a complaint about
/// the document. It is true whichever branch the document took, and discarding it turned an
/// unimplemented keyword behind a `$ref` in a losing branch into an ACCEPTED document.
#[derive(Default)]
struct SchemaFaults {
    /// The document broke a rule this build implements.
    faults: Vec<String>,
    /// This build could not read part of the rule set. Survives branch selection.
    refusals: Vec<String>,
}

impl SchemaFaults {
    fn fault(&mut self, msg: String) {
        self.faults.push(msg);
    }

    /// Deduplicated: one `$defs` node reached from both `oneOf` branches is one refusal, not two.
    fn refuse(&mut self, msg: String) {
        if !self.refusals.contains(&msg) {
            self.refusals.push(msg);
        }
    }

    /// Nothing to report — neither a rule broken nor a rule unread.
    fn clean(&self) -> bool {
        self.faults.is_empty() && self.refusals.is_empty()
    }

    /// Refusals first: "this build cannot check X" outranks "your document got Y wrong", because
    /// an author can fix every Y and still be looking at an X nobody checked.
    fn into_messages(self) -> Vec<String> {
        let mut all = self.refusals;
        all.extend(self.faults);
        all
    }
}

/// Trim a verdict to [`MAX_SCHEMA_FAULTS`], saying how many were dropped.
fn cap_schema_messages(mut msgs: Vec<String>) -> Vec<String> {
    let total = msgs.len();
    if total > MAX_SCHEMA_FAULTS {
        msgs.truncate(MAX_SCHEMA_FAULTS);
        msgs.push(format!(
            "…and {} more schema fault(s).",
            total - MAX_SCHEMA_FAULTS
        ));
    }
    msgs
}

/// Validate `doc` against the shipped `loadout-export.schema.json`.
///
/// `Ok(())` means the document satisfies exactly one of the schema's two `oneOf` branches. `Err`
/// carries the reasons, in author-readable form, each pointing at the path that failed.
///
/// **Fails closed, and T-735 is what that sentence cost.** A `$ref` that cannot be resolved, a key
/// pattern the matcher cannot evaluate ([`anchored_pattern_matches`]), a keyword outside
/// [`SUPPORTED_SCHEMA_KEYWORDS`] and a keyword whose VALUE has a form this checker does not
/// implement are all *refusals*, never skipped checks — because the alternative is an importer that
/// says "valid" about a constraint it never read. That claim was made before T-735 and was false in
/// three ways; see [`validate_against_schema`] for what makes it true now.
pub fn validate_against_loadout_export_schema(doc: &serde_json::Value) -> Result<(), Vec<String>> {
    let root: serde_json::Value = match serde_json::from_str(LOADOUT_EXPORT_SCHEMA_JSON) {
        Ok(v) => v,
        Err(e) => {
            return Err(vec![format!(
                "the shipped loadout-export schema did not parse ({e}) — refusing to import against a rule set this build cannot read"
            )])
        }
    };
    validate_against_schema(&root, doc)
}

/// The checker proper, against an arbitrary parsed schema `root`. **Two passes, and the order is
/// the point.**
///
/// 1. [`audit_schema_support`] — *can this build read the rules at all?* Document-INDEPENDENT by
///    construction, and that is not a stylistic preference. Every pre-T-735 refusal was raised from
///    inside the document walk, so it fired only where a document happened to reach: a `oneOf`
///    branch the document did not claim, an `items` array over an empty list, an
///    `additionalProperties` subschema with no extra key to apply it to were all unexamined and
///    silent. A rule set this build cannot fully read is refused for EVERY document, including the
///    ones that would never have visited the part it cannot read.
/// 2. The document walk, which by then is only ever answering "does this document obey rules this
///    build understands".
///
/// Split out of [`validate_against_loadout_export_schema`] by T-735 so the traps can be PROVEN: the
/// public entry hardcodes the shipped file, the shipped file is clean, and that is precisely why
/// three fail-open forms sat here unmeasured. Tests hand this function the shipped schema plus one
/// `$defs` edit — the whole distance between today and a green suite over unexamined documents.
fn validate_against_schema(
    root: &serde_json::Value,
    doc: &serde_json::Value,
) -> Result<(), Vec<String>> {
    let mut support = SchemaFaults::default();
    audit_schema_support(root, root, "#", &mut Vec::new(), &mut support);
    if !support.clean() {
        // No document verdict at all. "Your `qty` is wrong" alongside "and I could not read four
        // other rules" reads as a normal validation failure, and the author fixes `qty` and ships.
        return Err(cap_schema_messages(support.into_messages()));
    }
    let mut out = SchemaFaults::default();
    check_schema_node(root, root, doc, "", &mut out);
    if out.clean() {
        return Ok(());
    }
    Err(cap_schema_messages(out.into_messages()))
}

/// Walk **every schema position reachable in `root`** and refuse anything this build cannot fully
/// evaluate: an unsupported keyword, a keyword whose value has an unimplemented form, an
/// unresolvable `$ref`, a `$ref` carrying siblings, a `$ref` CHAIN (a `$ref` whose target is itself
/// a `$ref` — [`schema_deref`] resolves one hop only), a `patternProperties` key this matcher cannot
/// parse.
///
/// Structural, never keyword-sniffing. It descends into [`SCHEMA_NAMED_SUBSCHEMAS`] values,
/// `oneOf` entries and [`SCHEMA_SUBSCHEMA_KEYWORDS`], so whatever sits at one of those positions is
/// audited as a schema *whatever it contains*. `{"maxItems": 1}` in `$defs` is therefore reported,
/// where the guard this replaces skipped it for carrying no keyword the guard recognised.
///
/// `visited` holds the `$ref` strings already followed: a `$def` is audited once, and one that
/// reaches itself through a subschema (`{"items": {"$ref": "#/$defs/self"}}`) terminates instead of
/// recursing forever. A `$ref` that points STRAIGHT at another `$ref` never gets that far — it is
/// refused above as a chain, which is also what closes the plain `$ref`-to-itself cycle.
fn audit_schema_support(
    root: &serde_json::Value,
    node: &serde_json::Value,
    path: &str,
    visited: &mut Vec<String>,
    out: &mut SchemaFaults,
) {
    let Some(map) = node.as_object() else {
        out.refuse(format!(
            "{path}: the schema puts {} where a subschema belongs — this importer implements only object subschemas, so it refuses rather than skipping the check",
            schema_type_of(node)
        ));
        return;
    };

    for key in map.keys() {
        if !SUPPORTED_SCHEMA_KEYWORDS.contains(&key.as_str()) {
            out.refuse(format!(
                "{path}: the shipped schema uses `{key}`, which this importer does not implement — refusing rather than accepting a document it only partly checked"
            ));
        }
    }

    // A 2020-12 `$ref` applies *alongside* its siblings; this checker replaces the node with its
    // target, so a sibling assertion would be dropped. Refuse, and stop — the node IS its target.
    if map.contains_key("$ref") {
        let extra: Vec<&str> = map
            .keys()
            .map(String::as_str)
            .filter(|k| !matches!(*k, "$ref" | "description" | "title"))
            .collect();
        if !extra.is_empty() {
            out.refuse(format!(
                "{path}: the schema puts {} beside a $ref, which this importer would drop — refusing rather than skipping the check",
                extra.join(", ")
            ));
            return;
        }
        match (map["$ref"].as_str(), schema_deref(root, node)) {
            (Some(r), Some(target)) => {
                // A `$ref` whose TARGET is itself a `$ref` is a CHAIN — or, when it comes back
                // round, a CYCLE. [`schema_deref`] resolves exactly ONE hop and the document walk
                // never re-derefs, so the walk replaces this node with a target that is nothing but
                // another pointer and then checks NOTHING. This audit, meanwhile, follows the chain
                // to the assertions at the end of it and passes it clean — the guard calling a
                // schema fully supported while the walk reads none of it, which is the exact
                // fail-open shape T-735 exists to close.
                //
                // Refuse rather than implement chain-following: "this importer does not follow $ref
                // chains" is honest and fails CLOSED, where a chain-follower invites cycle-detection
                // bugs in the one code path whose whole job is not to lie.
                if target.get("$ref").is_some() {
                    out.refuse(format!(
                        "{path}: `$ref` points at `{r}`, which is itself a $ref — this importer follows exactly one hop, so it refuses rather than skipping every assertion behind the chain"
                    ));
                    return;
                }
                if !visited.iter().any(|seen| seen == r) {
                    visited.push(r.to_string());
                    audit_schema_support(root, target, r, visited, out);
                }
            }
            _ => out.refuse(format!(
                "{path}: the schema uses a $ref this importer cannot resolve — refusing rather than skipping the check"
            )),
        }
        return;
    }

    // KEYWORD FORMS. Every one of these is a shape the document walk would read as `None` and skip
    // in silence, which is a skipped check wearing the costume of a satisfied one.
    for (key, want, ok) in [
        (
            "type",
            "a type name or a list of type names",
            map.get("type").is_none_or(|v| match v {
                serde_json::Value::String(_) => true,
                serde_json::Value::Array(a) => !a.is_empty() && a.iter().all(|n| n.is_string()),
                _ => false,
            }),
        ),
        (
            "required",
            "a list of key names",
            map.get("required").is_none_or(|v| {
                v.as_array()
                    .is_some_and(|a| a.iter().all(|k| k.is_string()))
            }),
        ),
        (
            "enum",
            "a non-empty list of allowed values",
            map.get("enum")
                .is_none_or(|v| v.as_array().is_some_and(|a| !a.is_empty())),
        ),
        (
            "oneOf",
            "a non-empty list of branch schemas",
            map.get("oneOf")
                .is_none_or(|v| v.as_array().is_some_and(|a| !a.is_empty())),
        ),
        (
            "items",
            "a single subschema (the tuple form is not implemented)",
            map.get("items").is_none_or(serde_json::Value::is_object),
        ),
        (
            "additionalProperties",
            "a boolean or a subschema",
            map.get("additionalProperties")
                .is_none_or(|v| v.is_boolean() || v.is_object()),
        ),
        (
            "minLength",
            "a number",
            map.get("minLength")
                .is_none_or(serde_json::Value::is_number),
        ),
        (
            "minimum",
            "a number",
            map.get("minimum").is_none_or(serde_json::Value::is_number),
        ),
    ] {
        if !ok {
            out.refuse(format!(
                "{path}: `{key}` is given as {} where this importer implements only {want} — refusing rather than skipping the check",
                schema_type_of(&map[key])
            ));
        }
    }
    for named in SCHEMA_NAMED_SUBSCHEMAS {
        if map.get(*named).is_some_and(|v| !v.is_object()) {
            out.refuse(format!(
                "{path}: `{named}` is given as {} where this importer implements only a map of names to subschemas — refusing rather than skipping the check",
                schema_type_of(&map[*named])
            ));
        }
    }
    // A pattern the matcher cannot parse is a refusal HERE, against the schema alone. Raised only
    // from the document walk it needed a document key to land on, so an empty `wear: {}` met an
    // unevaluable pattern with silence.
    for pattern in map
        .get("patternProperties")
        .and_then(serde_json::Value::as_object)
        .into_iter()
        .flatten()
        .map(|(p, _)| p)
    {
        if parse_anchored_pattern(pattern).is_none() {
            out.refuse(format!(
                "{path}: the schema's key pattern `{pattern}` uses a construct this importer cannot evaluate — refusing rather than skipping the check"
            ));
        }
    }

    for named in SCHEMA_NAMED_SUBSCHEMAS {
        let children = map.get(*named).and_then(serde_json::Value::as_object);
        for (name, child) in children.into_iter().flatten() {
            audit_schema_support(root, child, &format!("{path}/{named}/{name}"), visited, out);
        }
    }
    let branches = map.get("oneOf").and_then(serde_json::Value::as_array);
    for (i, branch) in branches.into_iter().flatten().enumerate() {
        audit_schema_support(root, branch, &format!("{path}/oneOf/{i}"), visited, out);
    }
    for single in SCHEMA_SUBSCHEMA_KEYWORDS {
        // Only objects descend, and the two keywords get there differently. `additionalProperties`
        // accepts the boolean schemas `true`/`false`, so its form check passes them and the
        // DOCUMENT WALK implements them ("admits anything" / "closes the object"); they are simply
        // not subschemas to recurse into. `items` accepts no boolean at all — its form check above
        // already REFUSED one, along with the tuple form. So a non-object here is either handled
        // elsewhere or already refused; neither is a check skipped in silence.
        if let Some(child) = map.get(*single).filter(|v| v.is_object()) {
            audit_schema_support(root, child, &format!("{path}/{single}"), visited, out);
        }
    }
}

/// `#/a/b`-style local pointer resolution, **one hop and one hop only** — the returned value is
/// whatever sits at the pointer, `$ref` and all, and no caller re-derefs it. That is why
/// [`audit_schema_support`] refuses a target that is itself a `$ref`: following the chain here would
/// mean owning cycle detection, and the walk would still be reading the first hop.
///
/// `None` = a `$ref` this checker will not follow (remote, a pointer into nothing, a `$ref` that
/// is not even a string, or an RFC 6901 ESCAPED segment), which the caller turns into a refusal.
/// The `as_str()?` is deliberate: `{"$ref": 5}` used to answer "there is no $ref here", which made a
/// node with no other keyword a node with nothing to check.
///
/// **Escaped tokens are refused, not unescaped.** RFC 6901 spells `/` inside a key as `~1` and `~`
/// as `~0`, so `#/$defs/a~1b` means the key `a/b`. This resolver splits on `/` and looks the segment
/// up verbatim, so it would ask for a key literally named `a~1b`. That is not merely a miss: if the
/// schema happens to carry BOTH `a/b` and a literal `a~1b`, the pointer silently resolves to the
/// WRONG node and the document is validated against a schema its author never wrote. Refusing keeps
/// the one-hop contract honest — the same reason a `$ref` chain is refused rather than followed.
/// Unescaping properly would be a handful of lines; it is not done because nothing in the shipped
/// schema needs it, and an unused unescaper is one more thing that can be wrong in silence.
fn schema_deref<'a>(
    root: &'a serde_json::Value,
    node: &'a serde_json::Value,
) -> Option<&'a serde_json::Value> {
    let Some(raw) = node.get("$ref") else {
        return Some(node);
    };
    let mut cur = root;
    for seg in raw.as_str()?.strip_prefix("#/")?.split('/') {
        if seg.contains('~') {
            return None;
        }
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// The path label a fault is reported under — `""` is the document itself.
fn schema_at(path: &str) -> &str {
    if path.is_empty() {
        "document"
    } else {
        path
    }
}

/// The JSON type name of a value, in the schema's own vocabulary.
fn schema_type_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "integer"
            } else {
                "number"
            }
        }
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

/// Does `doc` satisfy a single `type` name? `number` admits integers (JSON Schema's rule); the
/// reverse is deliberately NOT true — `qty: 1.5` is not an integer here and must not pass.
fn schema_type_matches(name: &str, doc: &serde_json::Value) -> bool {
    match name {
        "number" => doc.is_number(),
        other => schema_type_of(doc) == other,
    }
}

fn check_schema_node(
    root: &serde_json::Value,
    node: &serde_json::Value,
    doc: &serde_json::Value,
    path: &str,
    out: &mut SchemaFaults,
) {
    // A 2020-12 `$ref` applies *alongside* its siblings. This checker replaces the node with its
    // target, so a `$ref` carrying a real assertion beside it would have that assertion dropped —
    // an unexamined constraint, i.e. the failure mode this whole module is built to refuse.
    if let Some(map) = node.as_object() {
        if map.contains_key("$ref") {
            let extra: Vec<&str> = map
                .keys()
                .map(String::as_str)
                .filter(|k| !matches!(*k, "$ref" | "description" | "title"))
                .collect();
            if !extra.is_empty() {
                out.refuse(format!(
                    "{}: the schema puts {} beside a $ref, which this importer would drop — refusing rather than skipping the check",
                    schema_at(path),
                    extra.join(", ")
                ));
                return;
            }
        }
    }
    let Some(node) = schema_deref(root, node) else {
        out.refuse(format!(
            "{}: the schema uses a $ref this importer cannot resolve — refusing rather than skipping the check",
            schema_at(path)
        ));
        return;
    };
    // T-735: this `else` used to be a bare `return` — the single quietest line in the module. A
    // subschema position holding anything but an object (tuple-form `items`, a boolean schema) was
    // dropped here having recorded NOTHING, so `[123]` satisfied `[{"type": "string"}]`.
    let Some(map) = node.as_object() else {
        out.refuse(format!(
            "{}: the schema puts {} where a subschema belongs — this importer implements only object subschemas, so it refuses rather than skipping the check",
            schema_at(path),
            schema_type_of(node)
        ));
        return;
    };

    for key in map.keys() {
        if !SUPPORTED_SCHEMA_KEYWORDS.contains(&key.as_str()) {
            out.refuse(format!(
                "{}: the shipped schema uses `{key}`, which this importer does not implement — refusing rather than accepting a document it only partly checked",
                schema_at(path)
            ));
        }
    }

    // A type mismatch makes every other keyword at this node noise, so it short-circuits.
    if let Some(t) = map.get("type") {
        let names: Vec<&str> = match t {
            serde_json::Value::String(s) => vec![s.as_str()],
            serde_json::Value::Array(a) => a.iter().filter_map(serde_json::Value::as_str).collect(),
            _ => Vec::new(),
        };
        if !names.is_empty() && !names.iter().any(|n| schema_type_matches(n, doc)) {
            out.fault(format!(
                "{}: expected {}, found {}",
                schema_at(path),
                names.join(" or "),
                schema_type_of(doc)
            ));
            return;
        }
    }
    if let Some(c) = map.get("const") {
        if doc != c {
            out.fault(format!("{}: must be {c}, found {doc}", schema_at(path)));
        }
    }
    if let Some(e) = map.get("enum").and_then(serde_json::Value::as_array) {
        if !e.contains(doc) {
            out.fault(format!(
                "{}: {doc} is outside the closed vocabulary {}",
                schema_at(path),
                serde_json::Value::Array(e.clone())
            ));
        }
    }
    if let (Some(m), Some(s)) = (
        map.get("minLength").and_then(serde_json::Value::as_u64),
        doc.as_str(),
    ) {
        if (s.chars().count() as u64) < m {
            out.fault(format!(
                "{}: must be at least {m} character(s), found {}",
                schema_at(path),
                s.chars().count()
            ));
        }
    }
    if let (Some(m), Some(n)) = (
        map.get("minimum").and_then(serde_json::Value::as_f64),
        doc.as_f64(),
    ) {
        if n < m {
            out.fault(format!(
                "{}: must be at least {m}, found {n}",
                schema_at(path)
            ));
        }
    }
    if let Some(branches) = map.get("oneOf").and_then(serde_json::Value::as_array) {
        check_schema_one_of(root, branches, doc, path, out);
    }
    if let Some(items) = map.get("items") {
        // The single-subschema form is the only one implemented. The tuple form is refused at the
        // KEYWORD, not per element: a fix that fired inside the loop would still be silent over the
        // empty array, i.e. over the document that exercises the constraint least.
        if items.is_object() {
            for (i, v) in doc.as_array().into_iter().flatten().enumerate() {
                check_schema_node(root, items, v, &format!("{path}/{i}"), out);
            }
        } else {
            out.refuse(format!(
                "{}: `items` is given as {} where this importer implements only a single subschema (the tuple form is not implemented) — refusing rather than skipping the check",
                schema_at(path),
                schema_type_of(items)
            ));
        }
    }
    if let Some(fields) = doc.as_object() {
        check_schema_object(root, map, fields, path, out);
    }
}

/// `required` + `properties` + `patternProperties` + the `additionalProperties` applicator.
fn check_schema_object(
    root: &serde_json::Value,
    schema: &serde_json::Map<String, serde_json::Value>,
    doc: &serde_json::Map<String, serde_json::Value>,
    path: &str,
    out: &mut SchemaFaults,
) {
    for req in schema
        .get("required")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(k) = req.as_str() else { continue };
        if !doc.contains_key(k) {
            out.fault(format!("{}: missing required key `{k}`", schema_at(path)));
        }
    }
    let props = schema
        .get("properties")
        .and_then(serde_json::Value::as_object);
    let patterns = schema
        .get("patternProperties")
        .and_then(serde_json::Value::as_object);
    // `additionalProperties` is a SUBSCHEMA keyword, not a boolean flag — `false` is simply the
    // subschema nothing satisfies. Reading it as `== false` (the pre-T-735 shape) turned
    // `{"additionalProperties": {"type": "string"}}` into a no-op that asserted nothing whatsoever,
    // so `{"x": 123}` came back Ok against a schema that plainly forbids it.
    let additional = schema.get("additionalProperties");
    for (k, v) in doc {
        let child = format!("{path}/{k}");
        let mut matched = false;
        if let Some(spec) = props.and_then(|p| p.get(k)) {
            matched = true;
            check_schema_node(root, spec, v, &child, out);
        }
        for (pattern, spec) in patterns.into_iter().flatten() {
            match anchored_pattern_matches(pattern, k) {
                Some(true) => {
                    matched = true;
                    check_schema_node(root, spec, v, &child, out);
                }
                Some(false) => {}
                None => out.refuse(format!(
                    "{child}: the schema's key pattern `{pattern}` uses a construct this importer cannot evaluate — refusing rather than skipping the check"
                )),
            }
        }
        if matched {
            continue;
        }
        match additional {
            // Absent, or the `true` boolean schema: every remaining key is allowed.
            None | Some(serde_json::Value::Bool(true)) => {}
            Some(serde_json::Value::Bool(false)) => out.fault(format!(
                "{}: `{k}` is not in the schema and additionalProperties is false",
                schema_at(path)
            )),
            Some(sub) if sub.is_object() => check_schema_node(root, sub, v, &child, out),
            Some(other) => out.refuse(format!(
                "{}: `additionalProperties` is given as {} where this importer implements only a boolean or a subschema — refusing rather than skipping the check",
                schema_at(path),
                schema_type_of(other)
            )),
        }
    }
}

/// `oneOf` with the schema's own `loadoutVersion` const as the discriminator.
///
/// A generic `oneOf` failure ("none of 2 branches matched") is useless to an author, because it
/// hands back both branches' complaints and half of them are about the version they did not write.
/// So when every branch fails, this reports the branch whose `loadoutVersion` const the document
/// actually claims — and when it claims none of them, says exactly that instead.
fn check_schema_one_of(
    root: &serde_json::Value,
    branches: &[serde_json::Value],
    doc: &serde_json::Value,
    path: &str,
    out: &mut SchemaFaults,
) {
    let per_branch: Vec<SchemaFaults> = branches
        .iter()
        .map(|b| {
            let mut errs = SchemaFaults::default();
            check_schema_node(root, b, doc, path, &mut errs);
            errs
        })
        .collect();
    // **T-735.** Refusals leave this function no matter which branch won. Discarding a branch's
    // FAULTS is the whole point of `oneOf` — the document did not claim that branch, so its
    // complaints are noise. A REFUSAL is not a complaint about the document: it is this build
    // saying it cannot read part of the rule set, which is equally true whichever branch matched.
    // The `passing == 1` return below used to throw both away together, so an unimplemented keyword
    // hiding behind a `$ref` in a losing branch never surfaced and the document was ACCEPTED.
    for branch in &per_branch {
        for refusal in &branch.refusals {
            out.refuse(refusal.clone());
        }
    }
    // A branch this build could not fully evaluate has not "passed" — it is unexamined, and
    // counting it as a match is the same error one level down.
    let passing = per_branch.iter().filter(|e| e.clean()).count();
    if passing == 1 {
        return;
    }
    if passing > 1 {
        // Not reachable with the shipped schema (the version const separates the branches), but a
        // schema edit could make it so, and "matched two mutually exclusive shapes" is a real fault.
        out.fault(format!(
            "{}: the document satisfies {passing} mutually exclusive schema branches",
            schema_at(path)
        ));
        return;
    }
    let claimed = doc.get("loadoutVersion");
    let hit = branches.iter().position(|b| {
        claimed.is_some() && b.pointer("/properties/loadoutVersion/const") == claimed
    });
    if let Some(i) = hit {
        for f in &per_branch[i].faults {
            out.fault(f.clone());
        }
        return;
    }
    let versions: Vec<String> = branches
        .iter()
        .filter_map(|b| b.pointer("/properties/loadoutVersion/const"))
        .map(|v| v.to_string())
        .collect();
    if versions.is_empty() {
        // The schema stopped discriminating on version — report everything rather than nothing.
        for f in per_branch.into_iter().flat_map(|b| b.faults) {
            out.fault(f);
        }
        return;
    }
    out.fault(format!(
        "{}: `loadoutVersion` must be one of {} — this is not a loadout-export document",
        schema_at(path),
        versions.join(" / ")
    ));
}

/* ───── the tiny anchored-pattern matcher `patternProperties` needs ───── */

/// One term of a parsed pattern: a character class with a repetition range.
struct PatTerm {
    negated: bool,
    ranges: Vec<(char, char)>,
    min: usize,
    max: usize,
}

/// Terms beyond this, and the backtracking below stops being obviously cheap. The shipped schema
/// uses two; a pattern needing nine is a pattern this matcher should refuse rather than run.
const MAX_PATTERN_TERMS: usize = 8;

/// Keys longer than this are not evaluated at all. A 512-character JSON key is not loadout data,
/// and refusing beats spending unbounded backtracking on it.
const MAX_PATTERN_INPUT: usize = 512;

/// Match `text` against an **anchored** regex from the subset `loadout-export.schema.json` uses:
/// `^`, `$`, character classes (`[a-zA-Z0-9_]`, `[^…]`), single literal characters, and the
/// quantifiers `{m,n}` / `{m,}` / `{m}` / `*` / `+` / `?`.
///
/// * `Some(true)` / `Some(false)` — the pattern was fully evaluated.
/// * **`None` — the pattern uses a construct this matcher does not implement, and the caller must
///   REFUSE.** That is the whole point of the third return value: a pattern we cannot evaluate is
///   not a pattern we may ignore. Alternation, groups, `.`, backslash escapes and unanchored
///   patterns all land here.
///
/// A full regex engine is not the right answer for one `patternProperties` entry, and pretending
/// the pattern is a rubber stamp is how mod-added wear keys would smuggle themselves past a closed
/// object. This is the honest middle: evaluate what it can, refuse what it cannot.
pub fn anchored_pattern_matches(pattern: &str, text: &str) -> Option<bool> {
    let terms = parse_anchored_pattern(pattern)?;
    let chars: Vec<char> = text.chars().collect();
    if chars.len() > MAX_PATTERN_INPUT {
        return None;
    }
    Some(match_pattern_terms(&terms, &chars))
}

fn parse_anchored_pattern(pattern: &str) -> Option<Vec<PatTerm>> {
    let cs: Vec<char> = pattern.chars().collect();
    if cs.first() != Some(&'^') || cs.last() != Some(&'$') || cs.len() < 2 {
        return None; // unanchored — this matcher makes no claim about partial matches.
    }
    let end = cs.len() - 1;
    let mut i = 1usize;
    let mut terms: Vec<PatTerm> = Vec::new();
    while i < end {
        let (negated, ranges) = match cs[i] {
            '[' => {
                let mut j = i + 1;
                let negated = cs.get(j) == Some(&'^');
                if negated {
                    j += 1;
                }
                let mut ranges: Vec<(char, char)> = Vec::new();
                loop {
                    let c = *cs.get(j)?; // unterminated class
                    if c == ']' {
                        break;
                    }
                    if c == '\\' {
                        return None; // escapes: not implemented
                    }
                    if cs.get(j + 1) == Some(&'-') && cs.get(j + 2).is_some_and(|e| *e != ']') {
                        let hi = *cs.get(j + 2)?;
                        if hi == '\\' {
                            return None;
                        }
                        ranges.push((c, hi));
                        j += 3;
                    } else {
                        ranges.push((c, c));
                        j += 1;
                    }
                }
                if ranges.is_empty() {
                    return None;
                }
                i = j + 1;
                (negated, ranges)
            }
            c if c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | ' ' | '/' | ':') => {
                i += 1;
                (false, vec![(c, c)])
            }
            // `.`, `(`, `|`, `\`, `^`, `$` mid-pattern, and anything else: not implemented.
            _ => return None,
        };
        let (min, max) = match cs.get(i) {
            Some('{') => {
                let mut j = i + 1;
                let mut inner = String::new();
                while j < end && cs[j] != '}' {
                    inner.push(cs[j]);
                    j += 1;
                }
                if cs.get(j) != Some(&'}') {
                    return None;
                }
                i = j + 1;
                match inner.split_once(',') {
                    None => {
                        let n: usize = inner.parse().ok()?;
                        (n, n)
                    }
                    Some((lo, "")) => (lo.parse().ok()?, usize::MAX),
                    Some((lo, hi)) => (lo.parse().ok()?, hi.parse().ok()?),
                }
            }
            Some('*') => {
                i += 1;
                (0, usize::MAX)
            }
            Some('+') => {
                i += 1;
                (1, usize::MAX)
            }
            Some('?') => {
                i += 1;
                (0, 1)
            }
            _ => (1, 1),
        };
        if min > max || terms.len() == MAX_PATTERN_TERMS {
            return None;
        }
        terms.push(PatTerm {
            negated,
            ranges,
            min,
            max,
        });
    }
    Some(terms)
}

fn pat_class_matches(t: &PatTerm, ch: char) -> bool {
    t.ranges.iter().any(|(lo, hi)| ch >= *lo && ch <= *hi) != t.negated
}

/// Greedy match with backtracking. Bounded by [`MAX_PATTERN_TERMS`] and [`MAX_PATTERN_INPUT`].
fn match_pattern_terms(terms: &[PatTerm], text: &[char]) -> bool {
    let Some((t, rest)) = terms.split_first() else {
        return text.is_empty();
    };
    let mut n = 0usize;
    while n < text.len() && n < t.max && pat_class_matches(t, text[n]) {
        n += 1;
    }
    loop {
        if n >= t.min && match_pattern_terms(rest, &text[n..]) {
            return true;
        }
        if n == 0 || n - 1 < t.min {
            return false;
        }
        n -= 1;
    }
}

#[cfg(test)]
#[path = "tests/rules/cargo_rules.rs"]
mod cargo_rules_tests;
#[cfg(test)]
#[path = "tests/rules/export_schema_rules.rs"]
mod export_schema_rules_tests;
#[cfg(test)]
#[path = "tests/rules/fixtures.rs"]
mod fixtures;
#[cfg(test)]
#[path = "tests/rules/loadout_row_rules.rs"]
mod loadout_row_rules_tests;
