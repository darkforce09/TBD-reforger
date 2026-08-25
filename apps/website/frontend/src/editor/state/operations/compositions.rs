//! T-934.7 — saved compositions (T-650): save / rows / rename / mint and the capture
//! helpers. Split from `operations.rs`; the façade re-exports keep paths stable.

use crate::editor::state::history as mission_history;
use map_engine_core::doc::MissionDocCore;

#[allow(unused_imports)]
use super::{attrs::*, cargo::*, context::*, entity::*, transform::*};

/* ═══════════════════════════════ T-650 — saved compositions ═══════════════════════════════ */
//
// Save (COMP-SAVE-001): capture the current selection → a self-contained composition row. Place
// (COMP-PLACE-001): the `Pending::Composition` arm above re-anchors + stamps every entity as one
// undo step. Edit (COMP-EDIT-001): rename / recategorize / delete the row. The three metadata
// fields ATTR-FIELD-COMP-{TITLE,AUTHOR,CATEGORY} are the row's own title/author/category.
//
// The capture shapes REUSE the clipboard capture (`copy_selection` / `paste_at_cursor`): a slot's
// role/tag/asset/stance/loadout come off `slots_json` exactly as the paste reads them, and a
// vehicle's heading/crew SHAPE come off `small_maps_json` exactly as `vehicle_rows` reads them. The
// only transform is absolute→relative: each entry stores `(dx, dz)` from the selection centroid, so
// a later place re-anchors the centroid at the cursor.
//
// T-781 widened the capture twice, and both halves are documented on `capture_selection_entities`:
// a selected COMMENT is captured (the composable clause of `PLACE-COMMENT-001`) and is stamped back
// into the editor-only `commentsById` root that never compiles; and every placeable entry carries
// its AUTHORED elevation through the shared `slot_z` reader, so a composition of rooftop entities
// no longer comes back on the ground.

/// One saved composition as the palette needs it: identity, metadata, and an entity count for the
/// row summary. The `entities` payload itself stays in the doc (the dock never needs to unpack it —
/// only the count and the three metadata fields are shown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionRow {
    pub id: String,
    pub title: String,
    pub author: String,
    pub category: String,
    pub entity_count: usize,
}

/// T-650 (COMP-SAVE-001) — capture the current selection into a new saved composition, titled
/// `title` under `category`, authored by `author` (the current user's display string as-authored).
/// Returns the new composition id, or `None` when the selection is empty / captured nothing.
///
/// Each selected id is classified as a slot, vehicle, object, or comment (whichever map holds it)
/// and emitted as a RELATIVE-OFFSET entry from the selection centroid. A slot carries
/// role/tag/asset/stance and its loadout blob; a vehicle carries resourceName/heading/crewed + the
/// crew SHAPE; an object carries alias/resourceName/faction; a comment (T-781) carries its
/// title/tooltip and nothing else. Runs the shared dirty tail (one undo step for the save).
#[must_use]
pub fn save_composition(title: String, category: String, author: String) -> Option<String> {
    let new_id = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let ctx = guard.as_ref()?;
        let sel: Vec<String> = ctx.selection.borrow().clone();
        if sel.is_empty() {
            return None;
        }
        let d = ctx.doc.borrow();
        let core = d.as_ref()?;
        let entities = capture_selection_entities(core, &sel);
        if entities.is_empty() {
            return None;
        }
        let comp_id = mint_composition_id(ctx, core);
        let row = serde_json::json!({
            "id": comp_id,
            "title": title,
            "author": author,
            "category": category,
            "entities": entities,
        });
        core.add_composition(&comp_id, &row.to_string());
        Some(comp_id)
    });
    if new_id.is_some() {
        mission_history::after_local_edit();
    }
    new_id
}

/// Build the relative-offset `entities` array for a selection. Slots come off `slots_json`
/// (the exact-f64 dicts the clipboard capture reads); vehicles, objects and comments come off
/// `small_maps_json`. The centroid is the mean of every captured entry's world position, in
/// selection order (a stable f64 sum), so the offsets recenter cleanly on place.
///
/// **T-781 Part A — a COMMENT is a capture source.** `PLACE-COMMENT-001` asks for composable
/// comments; before this, `commentsById` was simply not read here, so a selected comment was
/// dropped without a word and the composition came back one entity short. A comment entry carries
/// `title`/`tooltip` and no `resourceName`, because the thing it stamps is an editor-only
/// annotation that `MissionDocCore::place_composition` writes into the root `comments` map — the
/// collection `mission::flatten::EditorPayload` does not declare and serde therefore drops before
/// the mod document exists.
///
/// **T-781 Part B — every placeable entry carries its AUTHORED elevation**, resolved through the
/// shared [`slot_z`] reader against the exact-f64 rows above. Not the materialized SoA: its `zs`
/// column is f32 and it omits hidden-layer slots (T-665), so a careful operator's tucked-away
/// rooftop would read back as ground. `slot_z` is written against `{id: {position: {z}}}`, which is
/// the shape of all three source maps, so this is the one z-resolution vocabulary and not a third.
/// The elevation is written as a FIELD of the entry that produced it — never a parallel vector — so
/// no index can drift and give an entity somebody else's height.
pub(super) fn capture_selection_entities(
    core: &MissionDocCore,
    sel: &[String],
) -> Vec<serde_json::Value> {
    let slots = serde_json::from_str::<serde_json::Value>(&core.slots_json()).unwrap_or_default();
    let small =
        serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).unwrap_or_default();
    let vehicles = small.get("vehiclesById").cloned().unwrap_or_default();
    let entities = small.get("entitiesById").cloned().unwrap_or_default();
    let comments = small.get("commentsById").cloned().unwrap_or_default();

    // First pass: resolve each id to (kind, world position, source row) so the centroid is over the
    // SAME set the entries are built from.
    struct Captured {
        kind: &'static str,
        x: f64,
        y: f64,
        rotation: f64,
        /// T-781 Part B — the row's authored `position.z`. Zero for a comment, which has no third
        /// component to carry.
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
    // T-781 Part B — the shared z reader, applied to whichever source map holds the id. Resolved in
    // the SAME iteration that reads x/y/rotation off that row, so the elevation cannot be paired
    // with a different entity's offsets.
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
            // T-781 — a comment row's position is `{x, z}`: TWO HORIZONTALS, the `$defs/marker`
            // vocabulary, and no height at all. Its `z` is therefore the second WORLD axis and
            // belongs in `y` here beside every other kind's `position.y` — reading it as an
            // elevation would file the note's northing as its altitude and place it at the origin.
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
                // A note has no heading and no height; both are absent from the row by design.
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
            // T-781 Part B — the authored height, spelled `elevation` because `dz` is already spent
            // on the second HORIZONTAL offset in this same map. `place_composition` reads it back by
            // that name. Omitted for a comment, whose row has no such component to restore.
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
                    // The loadout blob VERBATIM (the paste-copies-loadout contract); omit when absent.
                    if let Some(l) = c.row.get("loadout").filter(|l| !l.is_null()) {
                        e.insert("loadout".into(), l.clone());
                    }
                }
                "vehicle" => {
                    e.insert(
                        "resourceName".into(),
                        serde_json::json!(s(&c.row, "resourceName")),
                    );
                    // `crewed` omit idiom: only carry `false` (absence = the with-crew default).
                    if c.row.get("crewed") == Some(&serde_json::Value::Bool(false)) {
                        e.insert("crewed".into(), serde_json::json!(false));
                    }
                    // The crew SHAPE verbatim (`{seat_id: slot_id}`), when the vehicle is crewed.
                    if let Some(crew) = c.row.get("crew").filter(|v| v.is_object()) {
                        e.insert("crew".into(), crew.clone());
                    }
                }
                "comment" => {
                    // T-781 — the two ATTR-FIELD-CMT-* text fields, verbatim and uncapped, exactly
                    // as the mutators store them (they reach no consumer that could reject them, so
                    // there is nothing to normalise FOR). The third field, position, is already the
                    // entry's `(dx, dz)`.
                    e.insert("title".into(), serde_json::json!(s(&c.row, "title")));
                    e.insert("tooltip".into(), serde_json::json!(s(&c.row, "tooltip")));
                }
                _ => {
                    // object
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

/// Mint an unused composition id (`comp-{n}`), proven unique against the live compositions map.
fn mint_composition_id(ctx: &OpsCtx, core: &MissionDocCore) -> String {
    let existing: std::collections::HashSet<String> =
        serde_json::from_str::<serde_json::Value>(&core.compositions_json())
            .ok()
            .and_then(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default();
    loop {
        let id = format!("comp-{}", ctx.next_id.get());
        ctx.next_id.set(ctx.next_id.get().saturating_add(1));
        if !existing.contains(&id) {
            return id;
        }
    }
}

/// T-650 — read every saved composition for the palette list, sorted by (category, title) so the
/// dock can group them. Off [`MissionDocCore::compositions_json`].
#[must_use]
pub fn composition_rows() -> Vec<CompositionRow> {
    OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return Vec::new();
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return Vec::new();
        };
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
    })
}

/// T-650 — saved-composition count (backs the palette header count).
#[must_use]
pub fn composition_count() -> usize {
    OPS_CTX.with(|c| {
        c.borrow()
            .as_ref()
            .and_then(|ctx| {
                ctx.doc
                    .borrow()
                    .as_ref()
                    .map(MissionDocCore::composition_count)
            })
            .unwrap_or(0)
    })
}

/// T-650 (COMP-EDIT-001 / ATTR-FIELD-COMP-TITLE) — rename a saved composition (inline edit). Blank
/// titles are allowed at the doc layer; the dock declines to write an all-whitespace title.
pub fn rename_composition(id: String, title: String) -> bool {
    edit_composition(|core| core.set_composition_title(&id, &title))
}

/// T-650 (COMP-EDIT-001 / ATTR-FIELD-COMP-CATEGORY) — recategorize a saved composition (inline).
pub fn recategorize_composition(id: String, category: String) -> bool {
    edit_composition(|core| core.set_composition_category(&id, &category))
}

/// T-650 (ATTR-FIELD-COMP-AUTHOR) — set a saved composition's author display string (inline).
pub fn set_composition_author(id: String, author: String) -> bool {
    edit_composition(|core| core.set_composition_author(&id, &author))
}

/// T-650 (COMP-EDIT-001) — delete a saved composition (inline). Clears the place arm if it was armed
/// on the row being deleted, so a release cannot commit a composition that no longer exists.
pub fn delete_composition(id: String) -> bool {
    let did = edit_composition(|core| core.remove_composition(&id));
    if did {
        OPS_CTX.with(|c| {
            if let Some(ctx) = c.borrow().as_ref() {
                let clear =
                    matches!(&*ctx.pending.borrow(), Some(Pending::Composition(p)) if *p == id);
                if clear {
                    *ctx.pending.borrow_mut() = None;
                }
            }
        });
    }
    did
}

/// Shared edit tail for the composition mutators: run `f` against the core, then the dirty tail
/// (one undo step). Returns `false` when there is no doc.
pub(super) fn edit_composition(f: impl FnOnce(&MissionDocCore)) -> bool {
    let did = OPS_CTX.with(|c| {
        let guard = c.borrow();
        let Some(ctx) = guard.as_ref() else {
            return false;
        };
        let d = ctx.doc.borrow();
        let Some(core) = d.as_ref() else {
            return false;
        };
        f(core);
        true
    });
    if did {
        mission_history::after_local_edit();
    }
    did
}

/// T-650 — the `entities` array (as a JSON string) of composition `id`, or `None` when the id is
/// absent. Read off the narrow [`MissionDocCore::compositions_json`] getter.
pub(super) fn composition_entities_json(core: &MissionDocCore, id: &str) -> Option<String> {
    let map = serde_json::from_str::<serde_json::Value>(&core.compositions_json()).ok()?;
    let entities = map.get(id)?.get("entities")?;
    Some(entities.to_string())
}

/// T-809 wave-203 — composition `id`'s title (the label the recently-placed list shows for a stamp),
/// or the id itself when the row carries no title. Read off the same [`MissionDocCore::compositions_json`]
/// getter as [`composition_entities_json`] so the stamp path resolves the label without a second
/// borrow of `OPS_CTX` (which the caller already holds).
pub(super) fn composition_title(core: &MissionDocCore, id: &str) -> String {
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

/// T-650 — how many entities an `entities` JSON array carries (0 for a non-array).
pub(super) fn composition_entity_count(entities_json: &str) -> usize {
    serde_json::from_str::<serde_json::Value>(entities_json)
        .ok()
        .and_then(|v| v.as_array().map(Vec::len))
        .unwrap_or(0)
}
