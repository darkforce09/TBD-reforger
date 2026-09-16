//! Role: resolve a selection out of the document and phrase it for a clipboard.
//! Position: `editing/commands` in the map engine.
//! Signals & state: explicit inputs only; every function here is pure over what it is handed.
//! Invariants: the grid reference on every line comes from ONE formatter, so a digest and a grid export can never disagree about where something stands. A missing classname is spelled out, never left blank.

use crate::camera::grid_reference::grid_ref_3digit;

/* ───────────────────────── T-698 — clipboard exporters (3den E5) ─────────────────────────
 *
 * WOG's `wog3_3den` Log entries and 3den Enhanced both ship clipboard exporters, and the
 * milsim-relevant one is the grid reference: a mission maker reads a grid off the editor and types
 * it into a briefing or says it on the radio. Three exporters ship here — grid position, classnames,
 * and a selection summary — because the data is already in the document and the target is
 * `navigator.clipboard`.
 *
 * ## The grid format is NOT invented here
 *
 * T-667 already shipped grid-reference labels on the map-pane edges, and their text comes from
 * `eden_toolbelt::grid_ref_3digit` — the 3-digit hundreds-of-metres Arma ref, one axis at a time.
 * [`format_grid_ref`] CALLS that function rather than re-deriving the rule. This is the whole point:
 * if the furniture prints `032` down the top edge and this exporter put `0320` on the clipboard, the
 * two would disagree and the exporter would be a confident wrong answer — worse than no exporter,
 * because a briefing built on it looks authoritative. The agreement is pinned by
 * `the_exporter_grid_ref_is_the_map_furnitures_own_label_text`, which reads the labels
 * `edge_eastings` / `edge_northings` actually emit and compares them to this exporter's output for
 * an entity standing on that line. The separator is a single space — the `mortar.rs` "012 020"
 * convention this codebase already writes.
 *
 * ## Everything below the wasm boundary is a pure string function
 *
 * Resolution (ids → rows) and composition (rows → clipboard text) are ungated so native
 * `cargo test` pins them by VALUE, the [`compiled_export_text`] precedent. Only the two things that
 * genuinely need a browser — reading the live selection and the clipboard write itself — live in
 * [`imp`].
 */

/// T-698 — one selected entity, resolved out of the document into the four things a clipboard
/// exporter needs: which id, what kind of thing it is, what it is made of, and where it stands.
///
/// `kind` is the same three-way split `editor_ops::capture_selection_entities` uses, and for the
/// same reason: the document stores slots, vehicles and objects in three different maps with three
/// different classname keys, and a reader that guessed one would silently export nothing for the
/// other two.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectedEntity {
    /// The app-side entity id (the selection's own vocabulary).
    pub id: String,
    /// `"slot"` | `"vehicle"` | `"object"`.
    pub kind: &'static str,
    /// The Enfusion prefab path — a slot's `assetId`, a vehicle's or object's `resourceName`.
    /// Empty when the entity expresses no asset (a slot placed from the "+ button", which the
    /// compile resolves to a faction default later); an empty classname is REPORTED, never silently
    /// exported as a blank line.
    pub classname: String,
    /// The authored human label — a slot's role (or tag), an object's alias. Empty when unset.
    pub label: String,
    /// World easting, metres.
    pub x: f64,
    /// World northing, metres.
    pub y: f64,
}

/// T-698 — resolve selection ids against the document's own maps, in selection order.
///
/// `slots_json` is `MissionDocCore::slots_json` (an id→row map) and `small_maps_json` is
/// `MissionDocCore::small_maps_json`, whose `vehiclesById` / `entitiesById` hold the other two
/// kinds. The lookup order (slot → vehicle → object) mirrors `editor_ops::capture_selection_entities`
/// so the exporters and the composition capture agree about what an id IS.
///
/// Ids the document does not know are DROPPED rather than exported as a placeholder: a stale id in
/// the selection is not an entity, and inventing a `000 000` row for it would put a false grid in a
/// briefing. The caller compares `out.len()` against the id count when it needs to know.
pub fn resolve_selected_entities(
    slots_json: &str,
    small_maps_json: &str,
    ids: &[String],
) -> Vec<SelectedEntity> {
    let slots = serde_json::from_str::<serde_json::Value>(slots_json).unwrap_or_default();
    let small = serde_json::from_str::<serde_json::Value>(small_maps_json).unwrap_or_default();
    let vehicles = small.get("vehiclesById").cloned().unwrap_or_default();
    let entities = small.get("entitiesById").cloned().unwrap_or_default();

    let s = |row: &serde_json::Value, k: &str| {
        row.get(k)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    // The document stores world position under `position.{x,y}` for all three kinds (the shape
    // `capture_selection_entities` reads). `y` is the NORTHING — the map is north-up and the engine
    // carries elevation separately in `z`.
    let axis = |row: &serde_json::Value, k: &str| {
        row.get("position")
            .and_then(|p| p.get(k))
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0)
    };

    let mut out = Vec::new();
    for id in ids {
        let (kind, row) = if let Some(r) = slots.get(id) {
            ("slot", r)
        } else if let Some(r) = vehicles.get(id) {
            ("vehicle", r)
        } else if let Some(r) = entities.get(id) {
            ("object", r)
        } else {
            continue;
        };
        let classname = if kind == "slot" {
            s(row, "assetId")
        } else {
            s(row, "resourceName")
        };
        let label = match kind {
            "slot" => {
                let role = s(row, "role");
                if role.is_empty() { s(row, "tag") } else { role }
            }
            "object" => s(row, "alias"),
            _ => String::new(),
        };
        out.push(SelectedEntity {
            id: id.clone(),
            kind,
            classname,
            label,
            x: axis(row, "x"),
            y: axis(row, "y"),
        });
    }
    out
}

/// The readable leaf of an Enfusion prefab path:
/// `{ABCDEF0123456789}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et` → `UAZ469`. Anything that is not a
/// prefab path comes back unchanged, so this can never turn a name into a wrong one.
pub fn prefab_leaf(classname: &str) -> String {
    let after_guid = classname.rsplit('}').next().unwrap_or(classname);
    let file = after_guid.rsplit('/').next().unwrap_or(after_guid);
    file.strip_suffix(".et").unwrap_or(file).to_string()
}

/// What to CALL an entity in a human-facing line: the authored label if there is one, else the
/// prefab leaf, else the raw id. Never empty — a nameless row in a summary is a row the reader
/// cannot match back to the map.
pub fn entity_display_name(e: &SelectedEntity) -> String {
    if !e.label.is_empty() {
        return e.label.clone();
    }
    let leaf = prefab_leaf(&e.classname);
    if leaf.is_empty() { e.id.clone() } else { leaf }
}

/// `1 slot` / `3 slots` — the pluralising counter the exporter messages share.
pub fn count_noun(n: usize, singular: &str, plural: &str) -> String {
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {plural}")
    }
}

/// T-698 — the six-figure grid reference of a world position, in the map furniture's own format.
///
/// **Both halves come from [`crate::editor::panels::toolbelt::grid_ref_3digit`]** — the T-667 formatter whose
/// output is literally the text printed on the map-pane edge labels. Do not re-derive the rule here:
/// a second convention that disagreed with the on-screen labels would be the confident-wrong-answer
/// defect this exporter exists to avoid. Separator is one space (`mortar.rs`'s "012 020").
pub fn format_grid_ref(x: f64, y: f64) -> String {
    format!("{} {}", grid_ref_3digit(x), grid_ref_3digit(y))
}

/// T-698 exporter 1 — **grid position**, the milsim-relevant one.
///
/// A single selection yields the BARE reference (`"032 048"`) and nothing else, because that is what
/// gets typed into a briefing line or read over the radio — a decorated string would have to be
/// hand-edited at the paste site every time. A multi-selection yields one line per entity,
/// `"<grid>  <name>"`, so the list stays attributable.
pub fn grid_position_text(entities: &[SelectedEntity]) -> String {
    match entities {
        [] => String::new(),
        [one] => format_grid_ref(one.x, one.y),
        many => many
            .iter()
            .map(|e| format!("{}  {}", format_grid_ref(e.x, e.y), entity_display_name(e)))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// T-698 exporter 2 — **classnames**, one prefab path per line in selection order.
///
/// Returns `(text, skipped)`. Duplicates are KEPT: two of the same prefab is two entities, and a
/// silent dedupe would change the count a reader pastes into a config. Entities with no classname
/// contribute no line — and are COUNTED into `skipped` so the caller can say so, rather than
/// emitting a blank line that reads as a real (empty) classname.
pub fn classnames_text(entities: &[SelectedEntity]) -> (String, usize) {
    let mut lines: Vec<&str> = Vec::new();
    let mut skipped = 0usize;
    for e in entities {
        if e.classname.is_empty() {
            skipped += 1;
        } else {
            lines.push(&e.classname);
        }
    }
    (lines.join("\n"), skipped)
}

/// T-698 exporter 3 — **selection summary**, a human-readable digest.
///
/// A headline (`"3 entities selected — 2 slots, 1 vehicle"`) plus one `- name (kind) at grid —
/// classname` line per entity. The grid on every row is [`format_grid_ref`]'s, so the summary and
/// the grid exporter can never disagree about where something stands. A missing classname is
/// spelled `(no classname)` rather than left blank — the digest says what it does not know.
pub fn selection_summary_text(entities: &[SelectedEntity]) -> String {
    if entities.is_empty() {
        return String::new();
    }
    let mut parts: Vec<String> = Vec::new();
    for (kind, singular, plural) in [
        ("slot", "slot", "slots"),
        ("vehicle", "vehicle", "vehicles"),
        ("object", "object", "objects"),
    ] {
        let n = entities.iter().filter(|e| e.kind == kind).count();
        if n > 0 {
            parts.push(count_noun(n, singular, plural));
        }
    }
    let mut out = format!(
        "{} selected — {}",
        count_noun(entities.len(), "entity", "entities"),
        parts.join(", ")
    );
    for e in entities {
        let class = if e.classname.is_empty() {
            "(no classname)"
        } else {
            &e.classname
        };
        out.push_str(&format!(
            "\n- {} ({}) at {} — {}",
            entity_display_name(e),
            e.kind,
            format_grid_ref(e.x, e.y),
            class
        ));
    }
    out
}

#[cfg(test)]
#[path = "tests/selection_digest.rs"]
mod tests;
