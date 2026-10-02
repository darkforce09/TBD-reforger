//! Role: clipboard.
//! Position: `doc/operations/entity` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::MissionDocCore;
use super::comment_details;
use super::composition_entities_json;
use super::composition_entity_count;
use super::composition_title;
use super::mint_id;
use super::mint_ids;
use super::slot_attrs_exists;
use super::slot_z;
use super::terrain_bounds_of;

/// Apply delete_selection to explicit document state.
pub fn delete_selection(core: &MissionDocCore, ids: Vec<String>) {
    let comment_ids: std::collections::HashSet<String> =
        comment_details(core).into_iter().map(|c| c.id).collect();
    let (comments, ids): (Vec<String>, Vec<String>) =
        ids.into_iter().partition(|id| comment_ids.contains(id));
    for id in &comments {
        core.remove_comment(id);
    }

    for id in &ids {
        let _ = core.remove_connections_touching(id);
    }

    if !ids.is_empty() {
        core.remove_slots(ids);
    }
}

/// Document operation over explicit authored state.
pub fn paste_at_cursor(
    core: &MissionDocCore,
    clip: Vec<serde_json::Value>,
    layer_id: String,
    next_id: &std::cell::Cell<u32>,
    cx: Option<f64>,
    cy: Option<f64>,
) -> Vec<String> {
    let terrain = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
        .ok()
        .and_then(|v| v.get("meta")?.get("terrain")?.as_str().map(str::to_string))
        .unwrap_or_default();
    let b = crate::data::scenario::compile::terrain_bounds(&terrain);

    let n = clip.len();
    let mut ids = Vec::with_capacity(n);
    let (mut sx, mut sy, mut srot, mut zs) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut squad_ids, mut layer_ids) = (Vec::new(), Vec::new());
    let (mut roles, mut tags, mut asset_ids, mut stances, mut loadouts) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut extras = Vec::with_capacity(n);
    let g = |v: &serde_json::Value, k: &str| {
        v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string()
    };
    let gp = |v: &serde_json::Value, k: &str| {
        v.get("position")
            .and_then(|p| p.get(k))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    };

    let z_rows: serde_json::Map<String, serde_json::Value> = clip
        .iter()
        .filter_map(|s| Some((s.get("id")?.as_str()?.to_string(), s.clone())))
        .collect();

    const PASTE_KNOWN: &[&str] = &[
        "id",
        "squadId",
        "index",
        "role",
        "tag",
        "assetId",
        "stance",
        "loadoutId",
        "loadout",
    ];
    for slot in &clip {
        ids.push(mint_id(core, next_id));
        sx.push(gp(slot, "x"));
        sy.push(gp(slot, "y"));
        srot.push(gp(slot, "rotation"));

        zs.push(slot_z(&z_rows, &g(slot, "id")).unwrap_or(0.0));

        squad_ids.push(g(slot, "squadId"));
        layer_ids.push(layer_id.clone());
        roles.push(g(slot, "role"));
        tags.push(g(slot, "tag"));
        asset_ids.push(g(slot, "assetId"));
        let st = g(slot, "stance");
        stances.push(if st.is_empty() {
            "stand".to_string()
        } else {
            st
        });
        loadouts.push(
            slot.get("loadout")
                .filter(|l| !l.is_null())
                .map(std::string::ToString::to_string)
                .unwrap_or_default(),
        );
        let mut extra = serde_json::Map::new();
        if let Some(obj) = slot.as_object() {
            for (k, v) in obj {
                if PASTE_KNOWN.contains(&k.as_str()) {
                    continue;
                }
                if k == "position" {
                    if let Some(pos) = v.as_object() {
                        let mut pos_extra = serde_json::Map::new();
                        for (pk, pv) in pos {
                            if !matches!(pk.as_str(), "x" | "y" | "z" | "rotation") {
                                pos_extra.insert(pk.clone(), pv.clone());
                            }
                        }
                        if !pos_extra.is_empty() {
                            extra.insert("position".into(), serde_json::Value::Object(pos_extra));
                        }
                    }
                    continue;
                }
                extra.insert(k.clone(), v.clone());
            }
        }
        extras.push(if extra.is_empty() {
            String::new()
        } else {
            serde_json::Value::Object(extra).to_string()
        });
    }
    core.paste_slots(
        ids.clone(),
        squad_ids,
        layer_ids,
        sx,
        sy,
        srot,
        zs,
        roles,
        tags,
        asset_ids,
        stances,
        loadouts,
        extras,
        cx,
        cy,
        b[2],
        b[3],
    );
    ids
}

/// Document operation over explicit authored state.
pub fn copy_selection(
    core: &MissionDocCore,
    sel: &std::collections::HashSet<String>,
) -> Option<Vec<serde_json::Value>> {
    let Ok(map) = serde_json::from_str::<serde_json::Value>(&core.slots_json()) else {
        return None;
    };
    let clip: Vec<serde_json::Value> = map
        .as_object()
        .map(|o| {
            o.values()
                .filter(|v| {
                    v.get("id")
                        .and_then(|i| i.as_str())
                        .is_some_and(|i| sel.contains(i))
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    if clip.is_empty() {
        return None;
    }
    Some(clip)
}

/// Document operation over explicit authored state.
pub fn place_saved_composition(
    core: &MissionDocCore,
    comp_id: &str,
    side: &str,
    x: f64,
    y: f64,
    next_id: &std::cell::Cell<u32>,
    ensure_layer: impl FnOnce(&MissionDocCore) -> String,
) -> Option<(Vec<String>, String)> {
    let entities = composition_entities_json(core, comp_id)?;
    let count = composition_entity_count(&entities);
    if count == 0 {
        return None;
    }
    let ids = mint_ids(core, next_id, count);
    let layer_id = ensure_layer(core);
    let b = terrain_bounds_of(core);
    let written = core.place_composition(&entities, &ids, side, &layer_id, x, y, b[2], b[3]);
    if written.is_empty() {
        return None;
    }

    let title = composition_title(core, comp_id);

    let slot_ids: Vec<String> = written
        .into_iter()
        .filter(|w| slot_attrs_exists(core, w))
        .collect();
    Some((slot_ids, title))
}
