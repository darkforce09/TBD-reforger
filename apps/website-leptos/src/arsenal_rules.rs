//! Smart-Arsenal domain core (T-167) — the Rust port of React `arsenalRules.ts` +
//! `arsenalDollModel.ts` (tag T-159.29.2). Pure, framework-free, native-tested: the 14 loadout
//! rows (incl. the compat **edge** rows optic/magazine), the compat edge graph + `items_for`,
//! per-row option building (abstract/variant filtered, stranded-pick preserved), loadout
//! validation, the paper-doll region model, and the honest weight readout.
//!
//! The UI (`arsenal.rs`) and the persisted `SlotLoadoutV2` shape (owned by `arsenal.rs`
//! `picks_to_loadout`) sit on top of this — this module holds only the decisions.
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

#[derive(Clone, PartialEq)]
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
}
