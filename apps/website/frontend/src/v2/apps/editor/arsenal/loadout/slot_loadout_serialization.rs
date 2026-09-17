//! Read and write the persisted slot loadout JSON.

use super::*;

/// `loadoutToPicks` — read the slot's `SlotLoadoutV2` JSON into a per-key `resource_name` map. An
/// absent loadout → all-empty picks. Weapons resolve by `slotIndex`; wear by key.
pub fn loadout_to_picks(loadout_json: Option<&str>) -> std::collections::HashMap<String, String> {
    let mut picks = std::collections::HashMap::new();
    let Some(json) = loadout_json else {
        return picks;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else {
        return picks;
    };
    if let Some(wear) = v.get("wear").and_then(|w| w.as_object()) {
        for (k, val) in wear {
            if let Some(s) = val.as_str() {
                if !s.is_empty() {
                    picks.insert(k.clone(), s.to_string());
                }
            }
        }
    }
    if let Some(weapons) = v.get("weapons").and_then(|w| w.as_array()) {
        for wp in weapons {
            let idx = wp.get("slotIndex").and_then(serde_json::Value::as_i64);
            let weapon = wp.get("weapon").and_then(|x| x.as_str());
            if let (Some(idx), Some(weapon)) = (idx, weapon) {
                if let Some(row) = ROWS.iter().find(|r| r.weapon.map(|(i, _)| i) == Some(idx)) {
                    picks.insert(row.key.to_string(), weapon.to_string());
                    // Primary carries the Smart-Forge sub-fields (`w.optic`/`w.magazine`) — capture
                    // them as sticky picks so a re-save from the dumb Forge never drops them (React
                    // `loadoutToPicks` reads them identically; the rows themselves fold forward).
                    if row.key == "primary" {
                        for sub in ["optic", "magazine"] {
                            if let Some(s) = wp.get(sub).and_then(|x| x.as_str()) {
                                if !s.is_empty() {
                                    picks.insert(sub.to_string(), s.to_string());
                                }
                            }
                        }
                    }
                    // `attachments[]` belongs to every v2 weapon row.
                    //
                    // The separator is safe for registry nodes, but this array is untrusted JSON.
                    // `ATTACHMENT_SEP` is safe for anything the compat graph produced (its nodes
                    // are pinned to a pattern that admits no control character), but this array is
                    // untrusted JSON: `loadout-export.schema.json:83` types `attachments` as
                    // `{"type":"string"}` items with no pattern, so a hand-edited or mod-authored
                    // document may legally carry a value containing U+001F. Packing that value
                    // would make it unpack as TWO attachments — a silent, invented pick. Such a
                    // value cannot be a real registry node, so it is dropped here rather than
                    // sanitized: the read path is the only door into the packed key, so no
                    // downstream consumer (weight, validation, persist, export) can ever see one.
                    let atts: Vec<String> = wp
                        .get("attachments")
                        .and_then(|a| a.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|v| v.as_str())
                                .filter(|s| !s.is_empty() && !s.contains(ATTACHMENT_SEP))
                                .map(str::to_string)
                                .collect()
                        })
                        .unwrap_or_default();
                    if !atts.is_empty() {
                        picks.insert(attachments_key(row.key), pack_attachments(&atts));
                    }
                }
            }
        }
    }
    picks
}

/// Writes canonical `SlotLoadoutV2` JSON from selected rows and cargo.
/// A present empty cargo list records an intentional clear and prevents default reseeding. Attachments belong only to selected weapons.
pub fn picks_to_loadout(
    picks: &std::collections::HashMap<String, String>,
    names: &std::collections::HashMap<String, String>,
    cargo: Option<&[rules::CargoRow]>,
) -> Option<String> {
    if cargo.is_none_or(|c| c.is_empty())
        && ROWS
            .iter()
            .all(|r| picks.get(r.key).map(String::is_empty).unwrap_or(true))
    {
        return None;
    }
    let sticky = |k: &str| {
        picks
            .get(k)
            .filter(|s| !s.is_empty())
            .map(|s| serde_json::Value::String(s.clone()))
            .unwrap_or(serde_json::Value::Null)
    };
    let mut weapons = Vec::new();
    for row in ROWS.iter().filter(|r| r.weapon.is_some()) {
        let Some(w) = picks.get(row.key).filter(|s| !s.is_empty()) else {
            continue;
        };
        let (slot_index, slot_type) = row.weapon.unwrap();
        let mut obj = serde_json::json!({
            "slotIndex": slot_index,
            "slotType": slot_type,
            "weapon": w,
        });
        let attachments = attachments_of(picks, row.key);
        if row.key == "primary" {
            obj["optic"] = sticky("optic");
            obj["magazine"] = sticky("magazine");
            // Primary keeps emitting the key even when empty: `attachments: []` is the byte shape
            // every already-persisted loadout carries, and dropping it would rewrite every mission
            // on disk on its next save for no gain.
            obj["attachments"] = serde_json::json!(attachments);
        } else if !attachments.is_empty() {
            // The other three weapons never carried the key, so they only grow one when there is
            // something to say, preserving the empty-set row's wire shape.
            obj["attachments"] = serde_json::json!(attachments);
        }
        weapons.push(obj);
    }
    let mut wear = serde_json::Map::new();
    for row in ROWS.iter().filter(|r| r.weapon.is_none()) {
        wear.insert(row.key.to_string(), sticky(row.key));
    }
    // `buildLoadoutSummary` — display names of primary/optic/magazine/launcher, non-empty, ` · `.
    let summary = ["primary", "optic", "magazine", "launcher"]
        .into_iter()
        .filter_map(|k| picks.get(k).filter(|s| !s.is_empty()))
        .map(|rn| names.get(rn).cloned().unwrap_or_else(|| rn.clone()))
        .collect::<Vec<_>>()
        .join(" · ");
    let mut loadout = serde_json::json!({
        "version": 2,
        "wear": wear,
        "weapons": weapons,
    });
    if let Some(rows) = cargo {
        loadout["cargo"] = rules::cargo_rows_json(rows);
    }
    if !summary.is_empty() {
        loadout["summary"] = serde_json::Value::String(summary);
    }
    Some(loadout.to_string())
}

/* ───────────── downloaded loadout export document ───────────── */

/// Writes the v2 export document, including all selected weapons and wear.
/// The legacy gear block is derived from those picks for readers of that vocabulary.
pub fn picks_to_export(
    picks: &std::collections::HashMap<String, String>,
    cargo: &[rules::CargoRow],
    modpack_id: &str,
) -> String {
    let pick = |k: &str| picks.get(k).filter(|s| !s.is_empty()).map(String::as_str);
    // `#/$defs/slot` — a ResourceName or null. Never `""`: the schema's own vocabulary for
    // "empty slot" is null, and the mod reader treats "" and absent identically anyway.
    let slot = |k: &str| pick(k).map_or(serde_json::Value::Null, |s| serde_json::json!(s));

    let mut wear = serde_json::Map::new();
    for row in ROWS.iter().filter(|r| r.weapon.is_none()) {
        wear.insert(row.key.to_string(), slot(row.key));
    }

    // `weapons[]` is slot-indexed, not positional: only picked rows appear, each naming the engine
    // slot it belongs in. The reader matches on (slotIndex, slotType).
    let mut weapons = Vec::new();
    for row in ROWS.iter().filter(|r| r.weapon.is_some()) {
        let Some(weapon) = pick(row.key) else {
            continue;
        };
        let (slot_index, slot_type) = row.weapon.unwrap();
        let mut obj = serde_json::json!({
            "slotIndex": slot_index,
            "slotType": slot_type,
            "weapon": weapon,
        });
        if row.key == "primary" {
            obj["optic"] = slot("optic");
            obj["magazine"] = slot("magazine");
        }
        // Every weapon carries the key, empty or not: unlike the doc field there is no
        // already-persisted byte shape to preserve here, and a uniform row is easier to read.
        // `attachments_of` unpacks the packed picks key, so no value here can contain
        // `ATTACHMENT_SEP` (see the guard in `loadout_to_picks`).
        obj["attachments"] = serde_json::json!(attachments_of(picks, row.key));
        weapons.push(obj);
    }

    let primary = pick("primary");
    let doc = serde_json::json!({
        "loadoutVersion": "2",
        "modpackId": modpack_id,
        "wear": wear,
        "weapons": weapons,
        "cargo": rules::cargo_rows_json(cargo),
        "gear": {
            "primary": slot("primary"),
            "uniform": slot("jacket"),
            "vest": pick("armoredVest").or_else(|| pick("vest"))
                .map_or(serde_json::Value::Null, |s| serde_json::json!(s)),
            "helmet": slot("headCover"),
            "optic": if primary.is_some() { slot("optic") } else { serde_json::Value::Null },
            "magazine": if primary.is_some() { slot("magazine") } else { serde_json::Value::Null },
        },
    });
    // Pretty: the file's job is to be dropped into `$profile:` and read by a human debugging a
    // spawn. `to_string_pretty` only fails on non-string map keys, which this document has none of.
    serde_json::to_string_pretty(&doc).unwrap_or_else(|_| doc.to_string())
}
