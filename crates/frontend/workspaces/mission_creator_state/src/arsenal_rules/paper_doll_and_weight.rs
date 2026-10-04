//! Paper doll regions and catalogued loadout weight.

use super::*;

/* ───────────────────────────── paper-doll region model ───────────────────────────── */

/// One clickable region of the paper doll: the pick key it edits.
pub struct DollRegion {
    /// The pick key the region edits.
    pub key: &'static str,
}

/// `RAIL_REGIONS` — the A3 slot-rail order (all 14 keys; weapons + rifle attachments first, then
/// head-to-toe wear). **Differs from `LOADOUT_ROWS` order**: vest/armoredVest/backpack pulled up
/// after helmet/jacket, pants/boots dropped to the end.
pub const RAIL_REGIONS: &[DollRegion] = &[
    DollRegion { key: "primary" },
    DollRegion { key: "optic" },
    DollRegion { key: "magazine" },
    DollRegion { key: "launcher" },
    DollRegion { key: "handgun" },
    DollRegion { key: "throwable" },
    DollRegion { key: "headCover" },
    DollRegion { key: "jacket" },
    DollRegion { key: "vest" },
    DollRegion { key: "armoredVest" },
    DollRegion { key: "backpack" },
    DollRegion { key: "handwear" },
    DollRegion { key: "pants" },
    DollRegion { key: "boots" },
];

/// `DOLL_REGIONS` — the SVG doll's clickable regions (12; optic/magazine excluded — they ride the
/// rifle).
#[cfg(test)]
pub const DOLL_REGIONS: &[DollRegion] = &[
    DollRegion { key: "headCover" },
    DollRegion { key: "jacket" },
    DollRegion { key: "vest" },
    DollRegion { key: "armoredVest" },
    DollRegion { key: "backpack" },
    DollRegion { key: "handwear" },
    DollRegion { key: "pants" },
    DollRegion { key: "boots" },
    DollRegion { key: "primary" },
    DollRegion { key: "launcher" },
    DollRegion { key: "handgun" },
    DollRegion { key: "throwable" },
];

/* ───────────────────────────── weight ───────────────────────────── */

/// Honest loadout weight (React `loadoutWeight`): sum numeric `weight_kg`; a `None` weight is a
/// counted-but-unknown item (engine class default), NEVER guessed as 0.
#[derive(Clone, Copy, Default, PartialEq)]
pub struct LoadoutWeight {
    /// The summed weight of the items whose weight is known, in kilograms.
    pub known_kg: f64,
    /// How many items carry no known weight.
    pub unknown_count: u32,
    /// How many items the loadout holds.
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
