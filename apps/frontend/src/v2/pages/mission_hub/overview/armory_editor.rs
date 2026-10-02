//! Faction keys, the draft rows and the guards behind the armory editor.
//!
//! **Role:** the pure half of the Edit Armory dialog — where the faction keys it offers come
//! from, what a draft row is, what makes one unsavable, and the request body it builds.
//! **Position:** pure data and shared state, no markup; the page and the dialog both hold the
//! editor handle this file defines.
//! **Signals & state:** [`ArmoryEditor`] is a `Copy` bundle of signals — the open flag, the
//! mission id, the offered keys, the draft rows, the faction on screen, the new-row fields, the
//! busy latch and the saved counter — so it threads through the page and the dialog uncloned.
//! **Invariants:** the faction key is a join key compared byte for byte, so it is derived from the
//! mission's own order of battle and never typed. A key that could not be stored is still offered,
//! labelled as such, because the write is wholesale and a key the editor cannot represent is a row
//! the editor would silently delete.

use crate::v2::core::api::dto::MissionDetail;
use leptos::prelude::*;
use serde_json::Value;

/// Where a faction key offered by the editor came from, which is why the editor offers a choice
/// rather than a free text field.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum KeySource {
    /// From this mission's own order of battle. Attaching the mission to an event copies these
    /// exact bytes into each slot's faction column, and the event dossier groups its faction cards
    /// from that column — so an armory row carrying this key joins.
    Orbat,
    /// Present on a row already stored against this mission, and in no order-of-battle faction. It
    /// still renders here, because the tabs are built from the armory itself, but it matches no
    /// faction card. Offered anyway: the write is wholesale, so a key this editor cannot represent
    /// is a row this editor would silently delete.
    StoredOnly,
}

/// One faction key the editor may file rows under.
#[derive(Clone, PartialEq)]
pub(super) struct FactionKey {
    /// The bytes that go on the wire, never rewritten.
    pub(super) key: String,
    pub(super) source: KeySource,
}

/// The faction keys this mission's order of battle would produce, in the order it declares them.
///
/// The faction is a join key, not a label: the armory rows are grouped by their raw value and
/// matched against the list built from the slots' own faction column by exact byte equality. A key
/// that does not match renders a faction card with no items at all, while the write answers 200
/// and echoes the author's value back — which is why a free-text box would be a machine for
/// producing that failure, and why the key is derived instead.
///
/// The derivation mirrors the server's own template parser, the only producer of a slot's faction
/// through the API: an explicit non-empty top-level order of battle wins, and otherwise the keys
/// come from the editor graph's factions, copied verbatim. In practice the editor graph is the
/// live path.
///
/// Two details are load-bearing. A faction whose squads resolve to no slot is not offered, because
/// it would produce no slot rows and so no faction card — a key that joins to nothing is the bug
/// this function exists to prevent. And a malformed top-level order of battle falls through to the
/// editor graph exactly as the server does, which is what the shape checks below are for: taking
/// the other branch would offer keys from one source while the server materialises from the other.
///
/// What this cannot cover: slot rows already materialised from a superseded version, or inserted
/// directly by a seed. Those keys are not in the current payload, so the stored armory's own keys
/// are appended separately.
pub(super) fn orbat_faction_keys(payload: &Value) -> Vec<String> {
    fn push_unique(out: &mut Vec<String>, key: &str) {
        if !out.iter().any(|s| s == key) {
            out.push(key.to_string());
        }
    }
    /// The subset of a squad template's shape that decides whether the whole order-of-battle array
    /// would have decoded. Every field has a default, so only the types can reject it.
    fn squad_decodes(sq: &Value) -> bool {
        let Some(o) = sq.as_object() else {
            return false;
        };
        for (k, v) in o {
            let ok = match k.as_str() {
                "faction" | "callsign" | "squad" => v.is_string(),
                "slots" => v
                    .as_array()
                    .is_some_and(|s| s.iter().all(|sl| sl.is_object())),
                _ => true, // unknown fields are ignored by serde, not rejected
            };
            if !ok {
                return false;
            }
        }
        true
    }

    let mut out: Vec<String> = Vec::new();

    if let Some(orbat) = payload.get("orbat").and_then(Value::as_array) {
        if !orbat.is_empty() && orbat.iter().all(squad_decodes) {
            for sq in orbat {
                let has_slot = sq
                    .get("slots")
                    .and_then(Value::as_array)
                    .is_some_and(|s| !s.is_empty());
                if has_slot {
                    let faction = sq
                        .get("faction")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    push_unique(&mut out, faction);
                }
            }
            // The array won the precedence test, so the editor graph is never consulted,
            // even when the array contributed nothing. Returning here rather than falling
            // through is what keeps this in step with the server's own early return.
            return out;
        }
    }

    let Some(editor) = payload.get("editor") else {
        return out;
    };
    let Some(factions) = editor.get("factions").and_then(Value::as_array) else {
        return out;
    };
    let empty: Vec<Value> = Vec::new();
    let squads = editor
        .get("squads")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let slots = editor
        .get("slots")
        .and_then(Value::as_array)
        .unwrap_or(&empty);
    let id_of = |v: &Value| v.get("id").and_then(Value::as_str).map(str::to_owned);
    for f in factions {
        let Some(squad_ids) = f.get("squadIds").and_then(Value::as_array) else {
            continue;
        };
        let mut materialisable = 0usize;
        for sid in squad_ids.iter().filter_map(Value::as_str) {
            // Reversing mirrors the server, which collects into a map: a duplicate squad id
            // resolves to the last squad carrying it, not the first.
            let Some(sq) = squads
                .iter()
                .rev()
                .find(|s| id_of(s).as_deref() == Some(sid))
            else {
                continue;
            };
            let Some(slot_ids) = sq.get("slotIds").and_then(Value::as_array) else {
                continue;
            };
            materialisable += slot_ids
                .iter()
                .filter_map(Value::as_str)
                .filter(|id| slots.iter().any(|s| id_of(s).as_deref() == Some(*id)))
                .count();
        }
        if materialisable > 0 {
            let key = f.get("key").and_then(Value::as_str).unwrap_or_default();
            push_unique(&mut out, key);
        }
    }
    out
}

/// Whether the armory write will accept this faction key at all: non-blank, and byte-identical to
/// its own trimmed form.
///
/// Refusing rather than normalising is deliberate, because the other side of the join normalises
/// nothing either — a padded value on both sides renders correctly today, and a unilateral trim
/// here would break it. So this editor reports the key as unstorable and names the fix rather than
/// quietly rewriting a key into one that joins to nothing.
pub(super) fn key_storable(key: &str) -> bool {
    !key.trim().is_empty() && key == key.trim()
}

/// The picker label for a key, which makes an unstorable one visible rather than hiding it — rows
/// already filed under it still have to be findable and deletable.
pub(super) fn key_label(key: &str) -> String {
    if key.is_empty() {
        "(blank)".into()
    } else if key != key.trim() {
        // Quoted, so the padding the server refuses is on screen.
        format!("\u{201c}{key}\u{201d}")
    } else {
        key.to_string()
    }
}

/// One row of the draft armory.
///
/// Plain data rather than signals: the editor replaces whole rows, so a keystroke never rebuilds
/// the list.
#[derive(Clone, PartialEq)]
pub(super) struct DraftRow {
    /// Chosen from [`FactionKey`], never typed.
    pub(super) faction: String,
    pub(super) item_name: String,
    pub(super) category: String,
    /// Blank means unlimited, which is a null quantity on the wire.
    pub(super) quantity: String,
}

/// Parse a quantity field: a number is a limit, empty is unlimited, anything else is a refusal.
pub(super) fn parse_qty(s: &str) -> Option<Option<i64>> {
    let t = s.trim();
    if t.is_empty() {
        return Some(None);
    }
    t.parse::<i64>().ok().map(Some)
}

/// The Edit Armory dialog's state, `Copy` so the whole bundle threads into the page and the dialog
/// without clones.
#[derive(Clone, Copy)]
pub(super) struct ArmoryEditor {
    pub(super) open: RwSignal<bool>,
    /// The complete draft armory across every faction — the write replaces the lot.
    pub(super) rows: RwSignal<Vec<DraftRow>>,
    /// Which faction's rows are on screen. Always one of `keys`.
    pub(super) faction: RwSignal<String>,
    pub(super) keys: RwSignal<Vec<FactionKey>>,
    pub(super) busy: RwSignal<bool>,
    pub(super) new_name: RwSignal<String>,
    pub(super) new_category: RwSignal<String>,
    pub(super) new_qty: RwSignal<String>,
    /// The mission the draft was opened from, captured at that moment rather than re-read at save
    /// time: a resource keeps serving its last value while the next run is in flight, so reading
    /// the id late is how one mission's armory ends up under another mission's button.
    pub(super) mission_id: RwSignal<String>,
    /// Bumped after a successful write. The page's resource reads it, so the read view refreshes
    /// without this dialog holding a handle on it.
    pub(super) saved: RwSignal<u32>,
}

impl ArmoryEditor {
    /// A closed editor with an empty draft.
    pub(super) fn new() -> Self {
        Self {
            open: RwSignal::new(false),
            rows: RwSignal::new(Vec::new()),
            faction: RwSignal::new(String::new()),
            keys: RwSignal::new(Vec::new()),
            busy: RwSignal::new(false),
            new_name: RwSignal::new(String::new()),
            new_category: RwSignal::new(String::new()),
            new_qty: RwSignal::new(String::new()),
            mission_id: RwSignal::new(String::new()),
            saved: RwSignal::new(0),
        }
    }

    /// Snapshot the mission into the draft and open.
    ///
    /// The dialog is loaded from the mission it was opened on, so nothing it sends can be carrying
    /// another mission's data.
    pub(super) fn open_for(self, m: &MissionDetail) {
        let rows: Vec<DraftRow> = m
            .armory
            .iter()
            .map(|a| {
                let s = |k: &str| {
                    a.get(k)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                DraftRow {
                    faction: s("faction"),
                    item_name: s("item_name"),
                    category: s("category"),
                    quantity: a
                        .get("quantity")
                        .and_then(Value::as_i64)
                        .map(|q| q.to_string())
                        .unwrap_or_default(),
                }
            })
            .collect();
        let mut keys: Vec<FactionKey> = m
            .current_version
            .as_ref()
            .map(|v| orbat_faction_keys(&v.json_payload))
            .unwrap_or_default()
            .into_iter()
            .map(|key| FactionKey {
                key,
                source: KeySource::Orbat,
            })
            .collect();
        // Anything the stored armory already uses has to be representable, or saving would
        // delete it. Appended after the derived keys, so the joining ones are what the dialog
        // opens on.
        for r in &rows {
            if !keys.iter().any(|k| k.key == r.faction) {
                keys.push(FactionKey {
                    key: r.faction.clone(),
                    source: KeySource::StoredOnly,
                });
            }
        }
        self.faction
            .set(keys.first().map(|k| k.key.clone()).unwrap_or_default());
        self.keys.set(keys);
        self.rows.set(rows);
        self.new_name.set(String::new());
        self.new_category.set(String::new());
        self.new_qty.set(String::new());
        self.mission_id.set(m.id.clone());
        self.open.set(true);
    }
}

/// The first reason the armory write would refuse this draft, or `None`.
///
/// A mirror of the server's per-item guard plus the one thing the deserialiser decides before that
/// guard runs: a non-numeric quantity fails decoding, and the refusal that comes back talks about
/// missing fields the request plainly has. Catching it here is the difference between a useful
/// sentence and a misleading one.
///
/// The rows this editor creates cannot trip any of these — the picker supplies the key and the
/// quantity field is validated as it is typed — but a row loaded from a stored armory can.
///
/// Worth blocking loudly: the write is wholesale, so a refusal leaves the whole armory as it was.
pub(super) fn draft_problem(rows: &[DraftRow]) -> Option<String> {
    for r in rows {
        if r.item_name.trim().is_empty() {
            return Some(format!(
                "An item under {} has no name. The armory endpoint rejects a blank item_name.",
                key_label(&r.faction)
            ));
        }
        if r.faction.trim().is_empty() {
            return Some(format!(
                "\u{201c}{}\u{201d} has a blank faction key. That key matches no Event Hub faction \
                 card and the endpoint rejects it — remove the item.",
                r.item_name.trim()
            ));
        }
        if r.faction != r.faction.trim() {
            return Some(format!(
                "Faction key {} is padded with whitespace. The endpoint refuses it rather than \
                 trimming it, because the ORBAT side of the join is not trimmed either (T-346) — \
                 fix the faction key in the Mission Creator, then re-attach the mission.",
                key_label(&r.faction)
            ));
        }
        if parse_qty(&r.quantity).is_none() {
            return Some(format!(
                "\u{201c}{}\u{201d} has a non-numeric quantity. Leave it blank for unlimited.",
                r.item_name.trim()
            ));
        }
    }
    None
}

/// The body of the armory write.
///
/// `items` is always present, including when it is empty: an empty list is how the endpoint is
/// told to clear the armory, whereas an absent one fails to decode on purpose, because the
/// handler's first statement is an unconditional delete.
///
/// The faction and the item name both go on the wire verbatim, for opposite reasons — the server
/// trims the label and refuses a padded key, and both of those decisions are its to make. Each
/// row's position is recorded because the read path orders by it, and leaving every row at the
/// default would make the rendered order arbitrary.
pub(super) fn armory_body(rows: &[DraftRow]) -> Value {
    let items: Vec<Value> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| {
            serde_json::json!({
                "faction": r.faction,
                "category": r.category,
                "item_name": r.item_name,
                "quantity": parse_qty(&r.quantity).flatten(),
                "sort_order": i as i64,
            })
        })
        .collect();
    serde_json::json!({ "items": items })
}
