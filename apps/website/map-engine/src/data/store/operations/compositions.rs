//! Role: compositions.
//! Position: `doc/operations` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::attrs::slot_z;
use crate::data::store::MissionDocCore;

/// One saved composition as the palette needs it: identity, metadata, and an entity count for the row summary. The `entities` payload itself stays in the doc (the dock never needs to unpack it — only the count and the three metadata fields are shown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionRow {
    /// Id.
    pub id: String,

    /// Title.
    pub title: String,

    /// Author.
    pub author: String,

    /// Category.
    pub category: String,

    /// Entity count.
    pub entity_count: usize,
}

/// Build the relative-offset `entities` array for a selection. Slots come off `slots_json` (the exact-f64 dicts the clipboard capture reads); vehicles, objects and comments come off `small_maps_json`. The centroid is the mean of every captured entry's world position, in selection order (a stable f64 sum), so the offsets recenter cleanly on place.
pub fn capture_selection_entities(core: &MissionDocCore, sel: &[String]) -> Vec<serde_json::Value> {
    let slots = serde_json::from_str::<serde_json::Value>(&core.slots_json()).unwrap_or_default();
    let small =
        serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).unwrap_or_default();
    let vehicles = small.get("vehiclesById").cloned().unwrap_or_default();
    let entities = small.get("entitiesById").cloned().unwrap_or_default();
    let comments = small.get("commentsById").cloned().unwrap_or_default();

    struct Captured {
        kind: &'static str,
        x: f64,
        y: f64,
        rotation: f64,

        elevation: f64,
        row: serde_json::Value,
    }
    let pos = |row: &serde_json::Value| -> (f64, f64, f64) {
        let p = row.get("position");
        (
            p.and_then(|p| p.get("x"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            p.and_then(|p| p.get("y"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
            p.and_then(|p| p.get("rotation"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0),
        )
    };

    let elev_of = |src: &serde_json::Value, id: &str| -> f64 {
        src.as_object().and_then(|m| slot_z(m, id)).unwrap_or(0.0)
    };
    let mut captured: Vec<Captured> = Vec::new();
    for id in sel {
        if let Some(row) = slots.get(id) {
            let (x, y, r) = pos(row);
            captured.push(Captured {
                kind: "slot",
                x,
                y,
                rotation: r,
                elevation: elev_of(&slots, id),
                row: row.clone(),
            });
        } else if let Some(row) = vehicles.get(id) {
            let (x, y, r) = pos(row);
            captured.push(Captured {
                kind: "vehicle",
                x,
                y,
                rotation: r,
                elevation: elev_of(&vehicles, id),
                row: row.clone(),
            });
        } else if let Some(row) = entities.get(id) {
            let (x, y, r) = pos(row);
            captured.push(Captured {
                kind: "object",
                x,
                y,
                rotation: r,
                elevation: elev_of(&entities, id),
                row: row.clone(),
            });
        } else if let Some(row) = comments.get(id) {
            let p = row.get("position");
            let axis = |k: &str| {
                p.and_then(|p| p.get(k))
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0)
            };
            captured.push(Captured {
                kind: "comment",
                x: axis("x"),
                y: axis("z"),

                rotation: 0.0,
                elevation: 0.0,
                row: row.clone(),
            });
        }
    }
    if captured.is_empty() {
        return Vec::new();
    }
    let n = captured.len() as f64;
    let cx = captured.iter().map(|c| c.x).sum::<f64>() / n;
    let cy = captured.iter().map(|c| c.y).sum::<f64>() / n;

    let s = |row: &serde_json::Value, k: &str| {
        row.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    captured
        .into_iter()
        .map(|c| {
            let mut e = serde_json::Map::new();
            e.insert("kind".into(), serde_json::json!(c.kind));
            e.insert("dx".into(), serde_json::json!(c.x - cx));
            e.insert("dz".into(), serde_json::json!(c.y - cy));
            e.insert("rotation".into(), serde_json::json!(c.rotation));

            if c.kind != "comment" {
                e.insert("elevation".into(), serde_json::json!(c.elevation));
            }
            match c.kind {
                "slot" => {
                    e.insert("role".into(), serde_json::json!(s(&c.row, "role")));
                    e.insert("tag".into(), serde_json::json!(s(&c.row, "tag")));
                    e.insert("assetId".into(), serde_json::json!(s(&c.row, "assetId")));
                    let stance = s(&c.row, "stance");
                    e.insert(
                        "stance".into(),
                        serde_json::json!(if stance.is_empty() {
                            "stand".to_string()
                        } else {
                            stance
                        }),
                    );

                    if let Some(l) = c.row.get("loadout").filter(|l| !l.is_null()) {
                        e.insert("loadout".into(), l.clone());
                    }
                }
                "vehicle" => {
                    e.insert(
                        "resourceName".into(),
                        serde_json::json!(s(&c.row, "resourceName")),
                    );

                    if c.row.get("crewed") == Some(&serde_json::Value::Bool(false)) {
                        e.insert("crewed".into(), serde_json::json!(false));
                    }

                    if let Some(crew) = c.row.get("crew").filter(|v| v.is_object()) {
                        e.insert("crew".into(), crew.clone());
                    }
                }
                "comment" => {
                    e.insert("title".into(), serde_json::json!(s(&c.row, "title")));
                    e.insert("tooltip".into(), serde_json::json!(s(&c.row, "tooltip")));
                }
                _ => {
                    e.insert("alias".into(), serde_json::json!(s(&c.row, "alias")));
                    e.insert(
                        "resourceName".into(),
                        serde_json::json!(s(&c.row, "resourceName")),
                    );
                    e.insert("faction".into(), serde_json::json!(s(&c.row, "faction")));
                }
            }
            serde_json::Value::Object(e)
        })
        .collect()
}

/// Composition entities json using the supplied domain data.
pub fn composition_entities_json(core: &MissionDocCore, id: &str) -> Option<String> {
    let map = serde_json::from_str::<serde_json::Value>(&core.compositions_json()).ok()?;
    let entities = map.get(id)?.get("entities")?;
    Some(entities.to_string())
}

/// Composition title using the supplied domain data.
pub fn composition_title(core: &MissionDocCore, id: &str) -> String {
    serde_json::from_str::<serde_json::Value>(&core.compositions_json())
        .ok()
        .and_then(|map| {
            map.get(id)?
                .get("title")?
                .as_str()
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| id.to_string())
}

/// Composition entity count using the supplied domain data.
pub fn composition_entity_count(entities_json: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(entities_json)
        .ok()
        .and_then(|v| v.as_array().map(Vec::len))
        .unwrap_or(0)
}

/// Apply composition_rows to explicit document state.
pub fn composition_rows(core: &MissionDocCore) -> Vec<CompositionRow> {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.compositions_json()) else {
        return Vec::new();
    };
    let Some(obj) = map.as_object() else {
        return Vec::new();
    };
    let s = |v: &serde_json::Value, k: &str| {
        v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
    };
    let mut rows: Vec<CompositionRow> = obj
        .iter()
        .map(|(id, v)| CompositionRow {
            id: id.clone(),
            title: s(v, "title"),
            author: s(v, "author"),
            category: s(v, "category"),
            entity_count: v
                .get("entities")
                .and_then(|e| e.as_array())
                .map_or(0, Vec::len),
        })
        .collect();
    rows.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    });
    rows
}

/// Document operation over explicit authored state.
pub fn mint_composition_id(core: &MissionDocCore, next_id: &std::cell::Cell<u32>) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.compositions_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    loop {
        let id = format!("comp-{}", next_id.get());
        next_id.set(next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// Apply save_composition to explicit document state.
pub fn save_composition(
    core: &MissionDocCore,
    title: String,
    category: String,
    author: String,
    sel: Vec<String>,
    next_id: &std::cell::Cell<u32>,
) -> Option<String> {
    let entities = capture_selection_entities(core, &sel);
    if entities.is_empty() {
        return None;
    }
    let comp_id = mint_composition_id(core, next_id);
    let row = serde_json::json!({
        "id": comp_id,
        "title": title,
        "author": author,
        "category": category,
        "entities": entities,
    });
    core.add_composition(&comp_id, &row.to_string());
    Some(comp_id)
}
