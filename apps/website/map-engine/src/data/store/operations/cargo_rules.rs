//! Role: cargo rules.
//! Position: `doc/operations` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

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

/// One cargo row on `SlotLoadoutV2.cargo[]` (`{container, item, qty}` — the loadout-export v2 skeleton, volume/weight budget model, no grid cells).
#[derive(Clone, Debug, PartialEq)]
pub struct CargoRow {
    /// Container.
    pub container: String,

    /// Item.
    pub item: String,

    /// Qty.
    pub qty: i64,
}

/// Read `cargo[]` off a `SlotLoadoutV2` JSON → `(rows, key_present)`. A present key — `[]`, `null` (normalized to no rows), or rows — is **user state** (seed-ineligible); malformed rows are dropped, not errors.
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

/// Seed cargo using the supplied domain data.
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
