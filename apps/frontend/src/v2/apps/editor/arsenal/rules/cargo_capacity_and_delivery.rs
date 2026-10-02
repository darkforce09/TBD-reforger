//! Cargo defaults, capacity budgets, and delivery warnings.

use super::*;

// Cargo in the persisted slot loadout and export document.

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
    /// Reports whether either known capacity is exceeded.
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

/* ─────────────── cargo capacity refusals ─────────────── */

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

/// Explains why catalogued capacity is an estimate and why overflow blocks export.
/// The build-time registry is not read back by the game. At spawn, rejected cargo moves to other storage or is dropped, while the round continues.
pub const CARGO_CAPACITY_CAVEAT: &str = "Capacity is a build-time catalogue figure the game never reads back, so treat it as an estimate, not a guarantee. The failure it points at is real: at spawn, cargo the character cannot hold is moved to another container or dropped — the rest of that row goes with it. The round still starts and the player is still playable, but they will not be carrying these items and only you can fix that, here.";

/// Returns overflow refusals keyed to the worn garment row.
/// Unknown capacity and unworn containers do not imply overflow; unworn cargo is reported separately as a warning.
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

/* ───── cargo with no known destination garment ───── */

/// Explains the warning for cargo whose destination garment is not picked.
/// A kit may supply its own garment, so the editor cannot prove this cargo will be misplaced and does not block export.
pub const CARGO_UNWORN_CAVEAT: &str = "The Arsenal only sees the wear this loadout picks, so if the slot's kit prefab wears one of its own the cargo still lands correctly — which is why this warns instead of refusing. If nothing wears it, the mod re-homes the items into whatever storage will take them and reports the row as degraded on the server and in the admin trail. The round still starts, so nothing downstream will catch this for you: if the placement matters, fix it here.";

/// The edge type carrying a character prefab's own carried items — the seed source
/// ([`cargo_defaults_by_character`]) and the vouching evidence [`cargo_unworn_container_errors`]
/// uses. `from_node` is the item, `to_node` the character, so a [`CompatGraph`] lookup keyed on the
/// character returns its default items.
pub const CHARACTER_DEFAULT_CARGO_EDGE: &str = "character_default_cargo";

/// Warns about authored cargo with no picked garment or kit-default evidence.
/// A character default item vouches for its prefab container. Missing feed evidence produces no warning because the editor cannot determine where that cargo lands.
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
