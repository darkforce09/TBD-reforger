//! Role: answer "which ids does the live document hold?" for the selection prune, and derive the
//! render-only views that hang off that question.
//! Position: `editing` in the map engine.
//! Signals & state: none; every answer is read from the document JSON handed in, never from a
//! cached snapshot.
//! Invariants: membership is KEY PRESENCE in the post-change document, so a live id survives a
//! prune and a removed id still falls out; id universes come off the raw by-id maps, never off the
//! materialized SoA, which drops rows for being hidden rather than for being gone.

use crate::data::store::MissionDocCore;
use crate::data::store::SlotSoa;

/// **The ids the live document holds that a selection may name.** Slots off the raw `slots_json`
/// key set (hidden rows INCLUDED), plus the `vehiclesById` / `entitiesById` / `commentsById` key
/// sets off `small_maps_json`.
///
/// Both halves are read from the POST-change document, which is what keeps the two directions the
/// prune has to get right: a LIVE id survives because its key is in one of the four maps, and a
/// GONE id still falls out because it left those maps before the prune ran. Sourcing either half
/// from a snapshot would spend the staleness guarantee.
///
/// The slot half comes off `slots_json` and NOT off the materialized SoA: the SoA drops slots on a
/// hidden layer and slots carrying the editor-hidden flag, so an SoA-sourced prune would deselect
/// a slot for being HIDDEN rather than for being GONE — a visibility policy nobody wrote, and one
/// that makes the hide toggle unable to toggle back.
///
/// ZONES AND MARKERS ARE DELIBERATELY ABSENT. A zone is selected in the Zones panel and never
/// enters the editor selection; a marker has no selection route at all. Admitting either would
/// widen the universe past anything that can appear in the set it prunes, which is how a prune
/// stops being able to say "this id is gone".
#[must_use]
pub fn selectable_ids(
    slots_json: &str,
    small_maps_json: &str,
) -> std::collections::HashSet<String> {
    let mut live: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(slots_json)
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    if let Ok(small) = serde_json::from_str::<serde_json::Value>(small_maps_json) {
        for key in ["vehiclesById", "entitiesById", "commentsById"] {
            if let Some(obj) = small.get(key).and_then(|v| v.as_object()) {
                live.extend(obj.keys().cloned());
            }
        }
    }
    live
}

/// **Slot ids currently referenced by any placed vehicle's crew map.**
///
/// Derived state only: the crew assignment IS the hide. No document flag records "this slot is
/// crewed", so unassigning a seat, deleting the vehicle or undoing the board removes the id from
/// this set and the figure returns by itself. Read off `vehiclesById.*.crew` in `small_maps_json`,
/// never off a filtered SoA.
#[must_use]
pub fn crewed_slot_ids(small_maps_json: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let Ok(small) = serde_json::from_str::<serde_json::Value>(small_maps_json) else {
        return out;
    };
    let Some(vehicles) = small.get("vehiclesById").and_then(|v| v.as_object()) else {
        return out;
    };
    for v in vehicles.values() {
        let Some(crew) = v.get("crew").and_then(|c| c.as_object()) else {
            continue;
        };
        for slot in crew.values() {
            if let Some(id) = slot.as_str().filter(|s| !s.is_empty()) {
                out.insert(id.to_string());
            }
        }
    }
    out
}

/// **Which SoA rows stay on the map** after the derived crew hide.
///
/// Returns the KEEP indices into `ids` and every parallel column. A crewed slot leaves the map
/// render SoA — figure and label — but this filter is a VIEW over the crew assignment, not a drop
/// inside the document: compile, outliner and selection still see every slot. An empty `crewed`
/// keeps every row.
#[must_use]
pub fn map_render_keep_indices(
    ids: &[String],
    crewed: &std::collections::HashSet<String>,
) -> Vec<usize> {
    if crewed.is_empty() {
        return (0..ids.len()).collect();
    }
    ids.iter()
        .enumerate()
        .filter(|(_, id)| !crewed.contains(id.as_str()))
        .map(|(i, _)| i)
        .collect()
}

/// The map-render SoA: the materialized slots minus every slot referenced by a vehicle crew list.
///
/// **Not** a change to what the document materializes. Operator-hidden rows are dropped inside the
/// core; crewed slots must still materialize and compile. This is the RENDER shape only — feed the
/// symbology bind and the map picks with it, and leave outliner, selection and id minting on the
/// raw maps and the unfiltered SoA.
#[must_use]
pub fn map_render_slot_soa(core: &MissionDocCore) -> SlotSoa {
    let soa = core.materialize();
    let crewed = crewed_slot_ids(&core.small_maps_json());
    filter_slot_soa_excluding(&soa, &crewed)
}

/// Drop SoA rows whose ids are in `exclude`, keeping the dictionaries so the remaining `*_idx`
/// values stay valid. Pure column filter — it does not touch the document.
#[must_use]
pub fn filter_slot_soa_excluding(
    soa: &SlotSoa,
    exclude: &std::collections::HashSet<String>,
) -> SlotSoa {
    let keep = map_render_keep_indices(&soa.ids, exclude);
    if keep.len() == soa.ids.len() {
        return soa.clone();
    }
    let mut out = SlotSoa {
        roles: soa.roles.clone(),
        tags: soa.tags.clone(),
        squads: soa.squads.clone(),
        layers: soa.layers.clone(),
        ..SlotSoa::default()
    };
    out.ids.reserve(keep.len());
    out.xs.reserve(keep.len());
    out.ys.reserve(keep.len());
    out.xy.reserve(keep.len() * 2);
    out.zs.reserve(keep.len());
    out.rotations.reserve(keep.len());
    out.stance.reserve(keep.len());
    out.role_idx.reserve(keep.len());
    out.tag_idx.reserve(keep.len());
    out.squad_idx.reserve(keep.len());
    out.layer_idx.reserve(keep.len());
    out.side_keys.reserve(keep.len());
    for i in keep {
        out.ids.push(soa.ids[i].clone());
        out.xs.push(soa.xs[i]);
        out.ys.push(soa.ys[i]);
        out.xy.push(soa.xy[i * 2]);
        out.xy.push(soa.xy[i * 2 + 1]);
        out.zs.push(soa.zs[i]);
        out.rotations.push(soa.rotations[i]);
        out.stance.push(soa.stance[i]);
        out.role_idx.push(soa.role_idx[i]);
        out.tag_idx.push(soa.tag_idx[i]);
        out.squad_idx.push(soa.squad_idx[i]);
        out.layer_idx.push(soa.layer_idx[i]);
        out.side_keys.push(soa.side_keys[i].clone());
    }
    out
}

/// A zone's geometric centre in world metres — a circle's centre, or a polygon's vertex mean.
/// `None` for a shapeless row (a draw that was never committed), which is exactly the row a click
/// cannot centre on and therefore must not advertise.
///
/// The shape vocabulary is the document's: `shape.circle {x, z, r}` and
/// `shape.polygon [[x, z], …]`, where **the map's world `y` IS that `z`**. Read off the JSON so
/// the answer is available wherever the document's own row types are not.
pub(crate) fn zone_centre(zone: &serde_json::Value) -> Option<(f64, f64)> {
    let shape = zone.get("shape")?;
    if let Some(c) = shape.get("circle")
        && let (Some(x), Some(z)) = (
            c.get("x").and_then(serde_json::Value::as_f64),
            c.get("z").and_then(serde_json::Value::as_f64),
        )
    {
        return Some((x, z));
    }
    let ring = shape.get("polygon")?.as_array()?;
    let verts: Vec<(f64, f64)> = ring
        .iter()
        .filter_map(|p| {
            let a = p.as_array()?;
            Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
        })
        .collect();
    if verts.is_empty() {
        return None;
    }
    let n = verts.len() as f64;
    let (sx, sz) = verts
        .iter()
        .fold((0.0, 0.0), |(ax, az), (x, z)| (ax + x, az + z));
    Some((sx / n, sz / n))
}

/// Where a PLAIN paste anchors when the map cursor is not available.
///
/// The paste verb takes an OPTIONAL anchor, and "no anchor" means one thing only: paste every slot
/// on its source coordinates. So the plain arm has to answer the off-map question here rather than
/// fall through to a branch named for something else.
///
/// An off-map plain paste anchors on the CENTRE OF THE VISIBLE MAP. The cursor is `None` whenever
/// the pointer sits over any chrome panel, which is exactly where it is after a click and then a
/// paste chord, so a silent no-op would strand a frequent gesture. Pasting into the middle of the
/// view keeps the promise plain paste makes, lands inside the terrain clamp, and leaves the result
/// selected and visible. What it deliberately does NOT do is quietly become paste-at-original:
/// that is a separate command with a separate chord.
///
/// `None` out means "do not paste" and is reachable only when there is no camera to take a centre
/// from — the engine has not booted, or its matrix is singular. There is nowhere to put the slots
/// and no view to put them in, so the keypress falls through unhandled rather than inventing a
/// coordinate.
pub fn plain_paste_anchor(
    cursor: Option<(f64, f64)>,
    view_centre: Option<(f64, f64)>,
) -> Option<(f64, f64)> {
    cursor.or(view_centre)
}
