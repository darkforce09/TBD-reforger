//! The structural comparison of two stored mission payloads.
//!
//! **Role:** answers "what changed between these two versions" over the compiled mission JSON —
//! per-collection added / removed / moved / edited / unchanged counts plus a bounded list of
//! named rows, and the handful of scalars worth calling out one by one.
//! **Position:** pure data, no view. The version rail and the upload preview both render what
//! this produces.
//! **Signals & state:** none — every function here is a pure transform over borrowed JSON.
//! **Invariants:** rows are matched by their `id`, never by document order: the compiler re-emits
//! the collections from unordered maps, so a permuted array is not an edit. Nothing is cloned —
//! the index borrows straight out of both payloads — and the result is O(1) in the size of the
//! mission because only the sample lines grow, and they stop at [`DIFF_SAMPLE_CAP`].

use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// How many changed rows one collection names before it stops naming them.
///
/// Counts stay exact and uncapped; only this list is bounded, which is what keeps the size of a
/// comparison independent of the size of the mission.
pub(super) const DIFF_SAMPLE_CAP: usize = 6;

/// The id-keyed collections of a mission editor payload, as `(label, dotted path)`, in the order
/// an author reads them: the top-level arrays, the `loadouts` object keyed by row id, and the
/// four `editor.*` arrays.
const DIFF_COLLECTIONS: [(&str, &str); 9] = [
    ("Slots", "editor.slots"),
    ("Squads", "editor.squads"),
    ("Factions", "editor.factions"),
    ("Editor layers", "editor.editorLayers"),
    ("Objectives", "objectives"),
    ("Vehicles", "vehicles"),
    ("Entities", "entities"),
    ("Markers", "markers"),
    ("Loadouts", "loadouts"),
];
/// Payload scalars worth naming one by one, as `(label, dotted path)`.
///
/// `environment` is handled separately because its key set is open-ended.
const DIFF_SCALARS: [(&str, &str); 4] = [
    ("Title", "title"),
    ("Terrain", "map.terrain"),
    ("Map bounds", "map.bounds"),
    ("Schema version", "schemaVersion"),
];
/// Stand-in for a value that is absent on one side of the comparison.
const DIFF_ABSENT: &str = "—";

/// Resolve a dotted path such as `"editor.slots"` or `"map.terrain"` against a payload.
///
/// `None` means the path is not present, which is how a collection missing from one side is
/// represented.
fn payload_node<'a>(payload: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = payload;
    for seg in path.split('.') {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// Visit every row of a collection as `(id, row)`.
///
/// Handles both shapes the compiler emits: an array of rows that carry their own `id`, and the
/// `loadouts` object whose *key* is the id. Anything else — absent, null, a scalar — has no rows.
/// Takes a closure rather than returning an iterator so neither shape needs boxing or an
/// intermediate `Vec`; on a document with hundreds of thousands of rows that allocation is the
/// whole cost.
fn for_each_row<'a>(node: Option<&'a Value>, mut f: impl FnMut(Option<&'a str>, &'a Value)) {
    match node {
        Some(Value::Array(rows)) => {
            for row in rows {
                f(row.get("id").and_then(Value::as_str), row);
            }
        }
        Some(Value::Object(map)) => {
            for (id, row) in map {
                f(Some(id.as_str()), row);
            }
        }
        _ => {}
    }
}

/// Shorten a display string to `max` characters.
///
/// Counts characters rather than bytes: a mission title is author text and can be non-ASCII, and
/// slicing it by byte would panic in the middle of a codepoint.
fn ellipsize(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// One-line rendering of a payload scalar for the "from → to" column.
fn scalar_repr(v: Option<&Value>) -> String {
    let Some(v) = v else {
        return DIFF_ABSENT.to_string();
    };
    let rendered = match v {
        Value::Null => DIFF_ABSENT.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        // An empty string reads as emptiness, not as a value, so it takes the same glyph as an
        // absent one rather than a pair of quotes the reader has to decode.
        Value::String(s) if s.trim().is_empty() => DIFF_ABSENT.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(a) => {
            let inner: Vec<String> = a.iter().take(6).map(|e| scalar_repr(Some(e))).collect();
            let more = if a.len() > 6 { ", …" } else { "" };
            format!("[{}{}]", inner.join(", "), more)
        }
        Value::Object(o) => format!("{{{} keys}}", o.len()),
    };
    ellipsize(&rendered, 56)
}

/// The name an author would recognise a row by, falling back to its id.
fn row_label(id: &str, row: &Value) -> String {
    for key in ["name", "title", "label", "callsign", "role"] {
        if let Some(s) = row.get(key).and_then(Value::as_str) {
            if !s.trim().is_empty() {
                return ellipsize(s.trim(), 40);
            }
        }
    }
    ellipsize(id, 40)
}

/// How one row changed between two versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RowChange {
    Same,
    /// Only `position` differs. Worth its own bucket: dragging a squad across the map is the most
    /// common edit in this editor, and it is not the same event as re-roling a slot.
    Moved,
    Edited,
}

/// Two JSON numbers are equal when they denote the same value, not the same representation.
///
/// `serde_json::Number` compares representationally: `100` and `100.0` deserialise to different
/// variants and compare unequal, so a representation flip between two stored payloads would make
/// untouched rows report as edited. Integers are compared exactly and never widened through
/// `f64`, which would make two distinct integers above 2^53 compare equal — a false "unchanged"
/// is the worse of the two failures, so the widening happens only when a float is involved.
fn number_eq(x: &serde_json::Number, y: &serde_json::Number) -> bool {
    if let (Some(p), Some(q)) = (x.as_i64(), y.as_i64()) {
        return p == q;
    }
    if let (Some(p), Some(q)) = (x.as_u64(), y.as_u64()) {
        return p == q;
    }
    match (x.as_f64(), y.as_f64()) {
        (Some(p), Some(q)) => p == q,
        _ => x == y,
    }
}

/// Deep equality that uses [`number_eq`] at every numeric leaf.
///
/// Structural for arrays and objects; falls through to `Value`'s own equality for null, bool and
/// string, where the representation is the value.
fn json_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => number_eq(x, y),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| json_eq(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| json_eq(v, w)))
        }
        _ => a == b,
    }
}

/// Classify a row that exists on both sides.
///
/// [`RowChange::Moved`] requires identical key sets and equality on every key except `position` —
/// the key the editor writes for slots, entities, vehicles and markers alike. A row that moved
/// *and* changed something else is [`RowChange::Edited`], because that is the stronger and less
/// dismissable claim.
pub(super) fn classify_row(a: &Value, b: &Value) -> RowChange {
    if json_eq(a, b) {
        return RowChange::Same;
    }
    match (a.as_object(), b.as_object()) {
        (Some(ao), Some(bo)) => {
            let same_keys = ao.len() == bo.len() && ao.keys().all(|k| bo.contains_key(k));
            let only_position_differs = same_keys
                && ao
                    .iter()
                    .all(|(k, v)| k == "position" || bo.get(k).is_some_and(|w| json_eq(v, w)));
            if only_position_differs {
                RowChange::Moved
            } else {
                RowChange::Edited
            }
        }
        _ => RowChange::Edited,
    }
}

/// What happened to one collection between two versions.
///
/// `a_rows` and `b_rows` count everything present on each side, so a "142 → 150" headline stays
/// exact even when some rows could not be keyed. The invariant the tests pin is
/// `b_rows == added + moved + edited + unchanged + unkeyed_b`, and its mirror for `a_rows`, which
/// is what makes a silently dropped row impossible to hide.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct CollectionDelta {
    pub(super) label: &'static str,
    pub(super) a_rows: usize,
    pub(super) b_rows: usize,
    pub(super) added: usize,
    pub(super) removed: usize,
    pub(super) moved: usize,
    pub(super) edited: usize,
    pub(super) unchanged: usize,
    /// Rows with no usable string `id`. They cannot be matched to anything, so they are counted
    /// and reported rather than quietly dropped.
    pub(super) unkeyed_a: usize,
    pub(super) unkeyed_b: usize,
    /// The same id appearing twice within one side. The compiler builds its arrays from an
    /// id-keyed map so this cannot arise from the editor, but an imported payload can carry it
    /// and the counts would be off by the duplicate.
    pub(super) duplicate_ids: usize,
    pub(super) samples: Vec<String>,
}

impl CollectionDelta {
    /// How many rows landed in a bucket that means something changed.
    fn changed(&self) -> usize {
        self.added + self.removed + self.moved + self.edited
    }

    /// True when nothing about this collection differs between the two sides.
    pub(super) fn is_unchanged(&self) -> bool {
        self.changed() == 0
    }

    /// Rows this comparison could not account for — the caveat the view has to surface.
    pub(super) fn unreadable(&self) -> usize {
        self.unkeyed_a + self.unkeyed_b + self.duplicate_ids
    }
}

/// One scalar that differs between two versions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FieldChange {
    pub(super) label: String,
    pub(super) from: String,
    pub(super) to: String,
}

/// The full answer to "what changed between these two versions".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct MissionDiff {
    pub(super) fields: Vec<FieldChange>,
    /// Every collection, changed or not — the view filters. Keeping the unchanged ones is what
    /// lets the version census reuse this struct instead of counting rows a second way.
    pub(super) collections: Vec<CollectionDelta>,
}

impl MissionDiff {
    /// True when neither a scalar nor a collection differs.
    pub(super) fn is_empty(&self) -> bool {
        self.fields.is_empty() && self.collections.iter().all(CollectionDelta::is_unchanged)
    }

    /// The changed collections only, in payload order.
    pub(super) fn changed_collections(&self) -> impl Iterator<Item = &CollectionDelta> {
        self.collections.iter().filter(|c| !c.is_unchanged())
    }
}

/// Append a sample line while there is room.
///
/// The cap is what keeps the memory a comparison holds independent of the mission's size.
fn push_sample(samples: &mut Vec<String>, line: String) {
    if samples.len() < DIFF_SAMPLE_CAP {
        samples.push(line);
    }
}

/// Compare one id-keyed collection: three linear passes over borrowed data.
fn diff_collection(label: &'static str, path: &str, a: &Value, b: &Value) -> CollectionDelta {
    let mut d = CollectionDelta {
        label,
        ..Default::default()
    };
    let (a_node, b_node) = (payload_node(a, path), payload_node(b, path));

    // Pass 1 — index the older side by id.
    let mut left: HashMap<&str, &Value> = HashMap::new();
    {
        let dr = &mut d;
        for_each_row(a_node, |id, row| {
            dr.a_rows += 1;
            match id.filter(|s| !s.is_empty()) {
                Some(id) => {
                    if left.insert(id, row).is_some() {
                        dr.duplicate_ids += 1;
                    }
                }
                None => dr.unkeyed_a += 1,
            }
        });
    }

    // Pass 2 — walk the newer side, classifying each row against that index.
    let mut seen: HashSet<&str> = HashSet::with_capacity(left.len());
    {
        let dr = &mut d;
        let index = &left;
        for_each_row(b_node, |id, row| {
            dr.b_rows += 1;
            let Some(id) = id.filter(|s| !s.is_empty()) else {
                dr.unkeyed_b += 1;
                return;
            };
            if !seen.insert(id) {
                dr.duplicate_ids += 1;
            }
            match index.get(id) {
                None => {
                    dr.added += 1;
                    push_sample(&mut dr.samples, format!("+ {}", row_label(id, row)));
                }
                Some(prev) => match classify_row(prev, row) {
                    RowChange::Same => dr.unchanged += 1,
                    RowChange::Moved => {
                        dr.moved += 1;
                        push_sample(&mut dr.samples, format!("~ {} moved", row_label(id, row)));
                    }
                    RowChange::Edited => {
                        dr.edited += 1;
                        push_sample(&mut dr.samples, format!("~ {} edited", row_label(id, row)));
                    }
                },
            }
        });
    }

    // Pass 3 — removals, walked in the older document's own order rather than by draining the
    // index, because hash-map iteration order is not stable and a sample that reshuffles between
    // runs makes a flaky test, which is how a comparison stops being trusted.
    {
        let dr = &mut d;
        let index = &mut left;
        for_each_row(a_node, |id, row| {
            let Some(id) = id.filter(|s| !s.is_empty()) else {
                return;
            };
            // Removing from the index makes this exactly once per unique id, even when the
            // older side repeats one.
            if !seen.contains(id) && index.remove(id).is_some() {
                dr.removed += 1;
                push_sample(&mut dr.samples, format!("− {}", row_label(id, row)));
            }
        });
    }
    d
}

/// What changed between mission editor payload `a` (older) and `b` (newer).
pub(super) fn diff_mission_payloads(a: &Value, b: &Value) -> MissionDiff {
    let mut fields = Vec::new();
    for (label, path) in DIFF_SCALARS {
        let (l, r) = (payload_node(a, path), payload_node(b, path));
        if l != r {
            fields.push(FieldChange {
                label: label.to_string(),
                from: scalar_repr(l),
                to: scalar_repr(r),
            });
        }
    }
    // `environment` is an open map (weather, timeOfDay, wind…), so walk the union of both sides'
    // keys: iterating only the newer side would make a deleted environment key invisible, and
    // that is exactly the class of change an author most needs told.
    let (ea, eb) = (
        payload_node(a, "environment"),
        payload_node(b, "environment"),
    );
    let mut env_keys: Vec<&str> = Vec::new();
    for node in [ea, eb] {
        if let Some(Value::Object(o)) = node {
            for k in o.keys() {
                if !env_keys.contains(&k.as_str()) {
                    env_keys.push(k.as_str());
                }
            }
        }
    }
    env_keys.sort_unstable();
    for k in env_keys {
        let (l, r) = (ea.and_then(|o| o.get(k)), eb.and_then(|o| o.get(k)));
        if l != r {
            fields.push(FieldChange {
                label: format!("Environment · {k}"),
                from: scalar_repr(l),
                to: scalar_repr(r),
            });
        }
    }
    let collections = DIFF_COLLECTIONS
        .iter()
        .map(|(label, path)| diff_collection(label, path, a, b))
        .collect();
    MissionDiff {
        fields,
        collections,
    }
}
