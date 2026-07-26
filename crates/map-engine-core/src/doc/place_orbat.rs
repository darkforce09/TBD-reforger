//! T-180.1 — place a character under a side faction, filing the slot into that side's current squad.
//!
//! **T-321 — one squad per side, not one squad per click.** T-180.1 minted a fresh squad on *every*
//! placement, so the ORBAT tree grew a squad per click and a five-man fireteam came out as five
//! one-man squads. That is also what made ">1 squad under a side" — the structure T-217's Apply
//! guard refused over — the ordinary state of any worked-on mission, which is the defect T-308 then
//! had to absorb downstream (see [`super::apply_faction`]). This module is the cause; T-308 was the
//! symptom.
//!
//! The model is an **implicit current squad**, resolved from the document on every call
//! ([`current_squad`]): a placement joins the side's bottom squad, and only starts a new one when
//! that squad holds authoring. Nothing is cached, so the target is always whatever the document
//! actually says — which is what makes it survive undo for free (undo rewinds `squadIds` /
//! `slotIds`, and the next placement simply re-reads them).
//!
//! The **explicit** alternative — an active-squad selector in the ORBAT panel that placement
//! honours — is the cleaner mental model, but it lives in `apps/website/frontend/src/orbat_manager.rs`
//! plus an `OpsCtx` signal in `editor_ops.rs`, neither of which this module can reach. The implicit
//! rule is a strict subset of it: if the panel ever grows a selector, it supplies the squad id and
//! [`current_squad`] becomes the fallback for "nothing selected".
//!
//! Pure helper over [`MissionDocCore`] so Class-R gates run as native `cargo test` (no wasm).

use serde_json::Value;

use super::MissionDocCore;

/// Valid Eden sides for T-180 place (no CIV).
const VALID_SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];

/// Error from [`place_character_under_side`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlaceOrbatError {
    /// `side` was not BLUFOR / OPFOR / INDFOR.
    InvalidSide(String),
}

impl std::fmt::Display for PlaceOrbatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSide(s) => write!(f, "invalid side {s:?}; expected BLUFOR|OPFOR|INDFOR"),
        }
    }
}

impl std::error::Error for PlaceOrbatError {}

/// Ensures `faction-{SIDE}` exists (`id` + `key` + `name` = SIDE), then files the slot into that
/// side's **current squad** — [`current_squad`], minting one only when there is no open squad to
/// join. The slot is appended at the end of `slotIds` and becomes `leaderSlotId` only if it is the
/// squad's first body, so a placement never steals the SL from a squad already led.
///
/// Returns `(faction_id, squad_id, slot_id)`. `squad_id` is now frequently an *existing* squad;
/// callers must not assume it names a squad this call created.
#[allow(clippy::too_many_arguments)]
pub fn place_character_under_side(
    doc: &MissionDocCore,
    side: &str,
    slot_id: &str,
    layer_id: &str,
    role: &str,
    tag: Option<String>,
    asset_id: Option<String>,
    x: f64,
    y: f64,
    z: f64,
    rotation: f64,
) -> Result<(String, String, String), PlaceOrbatError> {
    if !VALID_SIDES.contains(&side) {
        return Err(PlaceOrbatError::InvalidSide(side.to_string()));
    }

    let faction_id = format!("faction-{side}");
    ensure_side_faction(doc, side, &faction_id);

    let squad_id = match current_squad(doc, &faction_id) {
        Some(open) => open,
        None => {
            let id = mint_squad_id(doc, side);
            let squad_name = format!("Squad {}", squad_ordinal(doc, &faction_id) + 1);
            doc.add_squad(&id, &faction_id, &squad_name, None);
            id
        }
    };

    // Read the target's roster ONCE, before the write: it gives both the append index and the
    // "is this the first body?" leader test, and reading it after would answer neither.
    let existing = squad_slot_ids(doc, &squad_id);
    doc.add_slot(
        slot_id,
        &squad_id,
        layer_id,
        u32::try_from(existing.len()).unwrap_or(u32::MAX),
        role,
        tag,
        asset_id,
        x,
        y,
        z,
        rotation,
    );
    if existing.is_empty() {
        // First body in the squad — the same slot T-180.1 made leader, for the same reason. On an
        // append the squad already has one and `set_leader` would overwrite it (it does not check),
        // so this branch is what keeps clicking "SL, then four riflemen" from ending with the last
        // rifleman leading.
        doc.set_leader(&squad_id, slot_id);
    }
    Ok((faction_id, squad_id, slot_id.to_string()))
}

/// The squad a placement on `faction_id` joins: the side's **bottom** squad in `squadIds`, but only
/// while it is still open for placement. `None` ⇒ mint a fresh one.
///
/// **Why the bottom squad.** `add_squad` appends, so `squadIds.last()` is both the most recently
/// created squad and the one the operator sees at the bottom of the side in the ORBAT tree. That
/// makes the rule one sentence they can hold in their head — *placements land in the bottom squad
/// of the side* — and it is derived from the document rather than remembered beside it, so undo,
/// redo, hydrate and a reload all get the right answer with no extra machinery. (`reorder_squads`
/// moves the target, deliberately: the rule is about the tree the operator is looking at.)
///
/// **Why "open" and not simply "the last one".** A squad the operator authored is not scratch space.
/// Growing an Apply-applied template squad, or one they named and gave a callsign, would silently
/// change structure they built — and, for the applied case, quietly hand those bodies to the next
/// re-apply's surplus-slot trim. So placement steps around authoring instead: it starts a new squad
/// at the bottom and accumulates there.
///
/// The signals are exactly `apply_faction::squad_authorship`'s squad-level ones — name, callsign,
/// vehicles — **minus its slot count**, which cannot be a signal here: slots accumulating in one
/// squad is the whole of this ticket. That asymmetry is the intended one. It also means the bug
/// cannot return: a squad this function mints is open by construction, so at most one new squad is
/// created per authoring event, never one per click.
fn current_squad(doc: &MissionDocCore, faction_id: &str) -> Option<String> {
    let root = serde_json::from_str::<Value>(&doc.small_maps_json()).ok()?;
    let last = root
        .get("factionsById")?
        .get(faction_id)?
        .get("squadIds")?
        .as_array()?
        .last()?
        .as_str()?;
    let squad = root.get("squadsById")?.get(last)?;
    is_open_for_placement(squad).then(|| last.to_string())
}

/// Is this squad still scratch space a placement may join, or has the operator made it theirs?
fn is_open_for_placement(squad: &Value) -> bool {
    let name = squad
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let callsign = squad
        .get("callsign")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let vehicles = squad
        .get("vehicleIds")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    is_minted_squad_name(name) && callsign.is_empty() && vehicles == 0
}

/// `true` while `name` is still a machine-minted `Squad {n}` label — i.e. nobody renamed it.
///
/// Matches what this module and `editor_ops::orbat_add_squad` both write,
/// `format!("Squad {}", ordinal + 1)`; an empty name counts as minted, being the absence of a
/// choice rather than one. `apply_faction` holds its own copy of this predicate for the far side of
/// the same contract (it has to recognise what this module mints). The two must agree — a name this
/// file treats as scratch and that file treats as authoring would let a placement grow a squad
/// Apply then refuses over. `minted_names_match_what_placement_writes` below pins this copy against
/// real placement output; `apply_faction::minted_squad_names_are_recognised` pins the other.
fn is_minted_squad_name(name: &str) -> bool {
    let n = name.trim();
    n.is_empty()
        || n.strip_prefix("Squad ")
            .is_some_and(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
}

/// Slot ids of a squad, in `slotIds` order.
fn squad_slot_ids(doc: &MissionDocCore, squad_id: &str) -> Vec<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return Vec::new();
    };
    root.get("squadsById")
        .and_then(|m| m.get(squad_id))
        .and_then(|sq| sq.get("slotIds"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn ensure_side_faction(doc: &MissionDocCore, side: &str, faction_id: &str) {
    if faction_exists(doc, faction_id) {
        return;
    }
    doc.add_faction(faction_id, side, side);
}

fn faction_exists(doc: &MissionDocCore, faction_id: &str) -> bool {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return false;
    };
    root.get("factionsById")
        .and_then(|v| v.as_object())
        .is_some_and(|m| m.contains_key(faction_id))
}

fn squad_ordinal(doc: &MissionDocCore, faction_id: &str) -> usize {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return 0;
    };
    root.get("factionsById")
        .and_then(|v| v.get(faction_id))
        .and_then(|f| f.get("squadIds"))
        .and_then(|a| a.as_array())
        .map(|a| a.len())
        .unwrap_or(0)
}

fn mint_squad_id(doc: &MissionDocCore, side: &str) -> String {
    let existing = existing_squad_ids(doc);
    let mut n: u32 = 1;
    loop {
        // Format never equals the retired dump id `squad-1` (A1 / A-L2).
        let id = format!("squad-{side}-{n}");
        if !existing.contains(&id) {
            return id;
        }
        n = n.saturating_add(1);
    }
}

fn existing_squad_ids(doc: &MissionDocCore) -> std::collections::HashSet<String> {
    let Ok(root) = serde_json::from_str::<Value>(&doc.small_maps_json()) else {
        return std::collections::HashSet::new();
    };
    root.get("squadsById")
        .and_then(|v| v.as_object())
        .map(|m| m.keys().cloned().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn layer(doc: &MissionDocCore) {
        doc.add_editor_layer("lyr", "Layer 1", None);
    }

    fn place(
        doc: &MissionDocCore,
        side: &str,
        slot: &str,
    ) -> Result<(String, String, String), PlaceOrbatError> {
        place_character_under_side(
            doc,
            side,
            slot,
            "lyr",
            "Rifleman",
            None,
            Some("asset/Rifleman.et".into()),
            100.0,
            200.0,
            0.0,
            0.0,
        )
    }

    fn small(doc: &MissionDocCore) -> Value {
        serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json")
    }

    fn slots(doc: &MissionDocCore) -> Value {
        serde_json::from_str(&doc.slots_json()).expect("slots_json")
    }

    /// A1 — OPFOR place mints faction-OPFOR + new squad (≠ squad-1) with leader = slot.
    #[test]
    fn place_character_under_side_opfor() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let (fid, sid, slot) = place(&doc, "OPFOR", "n0").expect("place");
        assert_eq!(fid, "faction-OPFOR");
        assert_eq!(slot, "n0");
        assert_ne!(sid, "squad-1", "must not dump into squad-1");

        let root = small(&doc);
        let faction = &root["factionsById"]["faction-OPFOR"];
        assert_eq!(faction["key"], "OPFOR");
        assert_eq!(faction["name"], "OPFOR");
        let squad_ids = faction["squadIds"].as_array().expect("squadIds");
        assert_eq!(squad_ids.len(), 1);
        assert_eq!(squad_ids[0], sid);

        let squad = &root["squadsById"][&sid];
        let slot_ids = squad["slotIds"].as_array().expect("slotIds");
        assert_eq!(slot_ids, &vec![Value::String("n0".into())]);
        assert_eq!(squad["leaderSlotId"], "n0");
    }

    /// A2 — callsign + rank round-trip via update_slot_identity + slots_json.
    #[test]
    fn slot_callsign_rank_roundtrip() {
        let doc = MissionDocCore::new();
        layer(&doc);
        place(&doc, "BLUFOR", "n1").expect("place");
        doc.update_slot_identity("n1", Some("Alpha-1".into()), Some("Sergeant".into()));
        let v = slots(&doc);
        assert_eq!(v["n1"]["callsign"], "Alpha-1");
        assert_eq!(v["n1"]["rank"], "Sergeant");

        // Clear both keys.
        doc.update_slot_identity("n1", None, None);
        let v = slots(&doc);
        assert!(v["n1"].get("callsign").is_none(), "{v}");
        assert!(v["n1"].get("rank").is_none(), "{v}");
    }

    /// A4 (T-321 inverted) — two places same side ⇒ **one** squad holding both.
    ///
    /// T-180.1's A4 asserted the opposite (`squad_ids.len() == 2`, ids distinct); minting per click
    /// is the defect this ticket removes, so the criterion is re-pinned rather than kept. The
    /// second body appends: `index` 1, and the first keeps the SL.
    #[test]
    fn two_places_one_squad_same_side() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let (_, s1, a) = place(&doc, "BLUFOR", "a").expect("p1");
        let (_, s2, b) = place(&doc, "BLUFOR", "b").expect("p2");
        assert_eq!(s1, s2, "second placement joined the first's squad");

        let root = small(&doc);
        let squad_ids = root["factionsById"]["faction-BLUFOR"]["squadIds"]
            .as_array()
            .expect("squadIds");
        assert_eq!(squad_ids.len(), 1, "one squad, not one per click");
        assert_eq!(squad_ids[0], s1);

        let squad = &root["squadsById"][&s1];
        assert_eq!(
            squad["slotIds"].as_array().expect("slotIds"),
            &vec![Value::String(a.clone()), Value::String(b.clone())]
        );
        assert_eq!(squad["leaderSlotId"], a, "second place must not steal SL");
        let s = slots(&doc);
        assert_eq!(s[&a]["index"], 0);
        assert_eq!(s[&b]["index"], 1, "appended, not overwriting index 0");
    }

    /// F4 — three placements refiled into one squad ⇒ 1 squad, 2 leader→member segments.
    ///
    /// T-321 keeps the refile by forcing three squads the way the new model actually produces them:
    /// renaming the bottom squad makes it authored, so the next placement starts a fresh one. That
    /// exercises the mint-around-authoring branch and the original F4 render invariant at once.
    #[test]
    fn refile_merge_two_link_segments() {
        use crate::squad_links::build_squad_link_segments;
        use std::collections::HashMap;

        let doc = MissionDocCore::new();
        layer(&doc);
        let (_, s1, a) = place_character_under_side(
            &doc, "BLUFOR", "a", "lyr", "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
        )
        .expect("p1");
        doc.rename_squad(&s1, "Alpha");
        let (_, s2, b) = place_character_under_side(
            &doc, "BLUFOR", "b", "lyr", "Rifleman", None, None, 10.0, 0.0, 0.0, 0.0,
        )
        .expect("p2");
        doc.rename_squad(&s2, "Bravo");
        let (_, s3, c) = place_character_under_side(
            &doc, "BLUFOR", "c", "lyr", "Rifleman", None, None, 20.0, 0.0, 0.0, 0.0,
        )
        .expect("p3");
        assert_ne!(s1, s2, "a renamed squad is not grown");
        assert_ne!(s2, s3, "a renamed squad is not grown");

        doc.move_slot_to_squad(&b, &s1);
        doc.move_slot_to_squad(&c, &s1);

        let root = small(&doc);
        let squad_ids = root["factionsById"]["faction-BLUFOR"]["squadIds"]
            .as_array()
            .expect("squadIds");
        assert_eq!(squad_ids.len(), 1, "merged to one squad: {squad_ids:?}");
        assert_eq!(squad_ids[0], s1);
        assert!(root["squadsById"].get(&s2).is_none(), "s2 GC'd");
        assert!(root["squadsById"].get(&s3).is_none(), "s3 GC'd");

        let mut xy = HashMap::new();
        xy.insert(a.clone(), (0.0_f32, 0.0_f32));
        xy.insert(b.clone(), (10.0_f32, 0.0_f32));
        xy.insert(c.clone(), (20.0_f32, 0.0_f32));
        let verts = build_squad_link_segments(&doc.squad_link_inputs(), &xy);
        assert_eq!(
            verts.len() / 12,
            2,
            "size-3 squad ⇒ 2 segments; verts={}",
            verts.len()
        );
    }

    /// G5 — add_squad under a side increases squadsById count for that faction.
    #[test]
    fn orbat_add_squad_increases_count_under_side() {
        let doc = MissionDocCore::new();
        layer(&doc);
        doc.add_faction("faction-OPFOR", "OPFOR", "OPFOR");
        let before = small(&doc)["factionsById"]["faction-OPFOR"]["squadIds"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0);
        doc.add_squad("squad-OPFOR-1", "faction-OPFOR", "Squad 1", None);
        let after = small(&doc)["factionsById"]["faction-OPFOR"]["squadIds"]
            .as_array()
            .expect("squadIds")
            .len();
        assert_eq!(after, before + 1);
        assert!(small(&doc)["squadsById"].get("squad-OPFOR-1").is_some());
    }

    /// G6 — add_slot into an existing squad increases slotIds (not a new squad via place).
    #[test]
    fn orbat_add_role_increases_squad_slot_ids() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let (_, squad_id, first) = place_character_under_side(
            &doc, "BLUFOR", "n0", "lyr", "Rifleman", None, None, 0.0, 0.0, 0.0, 0.0,
        )
        .expect("place");
        let before = small(&doc)["squadsById"][&squad_id]["slotIds"]
            .as_array()
            .expect("slotIds")
            .len();
        doc.add_slot(
            "n1",
            &squad_id,
            "lyr",
            before as u32,
            "Medic",
            Some("MED".into()),
            None,
            1.0,
            0.0,
            0.0,
            0.0,
        );
        let root = small(&doc);
        let ids = root["squadsById"][&squad_id]["slotIds"]
            .as_array()
            .expect("slotIds");
        assert_eq!(ids.len(), before + 1);
        assert!(ids.iter().any(|v| v == "n1"));
        assert_eq!(
            root["squadsById"][&squad_id]["leaderSlotId"], first,
            "add role must not steal SL"
        );
        // Still one squad under BLUFOR.
        assert_eq!(
            root["factionsById"]["faction-BLUFOR"]["squadIds"]
                .as_array()
                .expect("squadIds")
                .len(),
            1
        );
    }

    /// A5 — invalid side ⇒ Err and no mutation.
    #[test]
    fn place_rejects_invalid_side() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let before_small = small(&doc);
        let before_slots = slots(&doc);
        for bad in ["CIV", "nope"] {
            let err = place(&doc, bad, "x").expect_err("must reject");
            assert!(matches!(err, PlaceOrbatError::InvalidSide(_)), "{err:?}");
        }
        // Compare parsed Values — yrs `to_json` map key order is not stable across calls.
        assert_eq!(small(&doc), before_small);
        assert_eq!(slots(&doc), before_slots);
        assert!(
            before_small["factionsById"]
                .as_object()
                .is_some_and(|m| m.is_empty()),
            "no faction minted on reject"
        );
        assert!(
            before_slots.as_object().is_some_and(|m| m.is_empty()),
            "no slot minted on reject"
        );
    }

    // ── T-321 — one squad per side, not one per click ────────────────────────────────────────────

    fn squad_ids(doc: &MissionDocCore, side: &str) -> Vec<String> {
        small(doc)["factionsById"][format!("faction-{side}")]["squadIds"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[allow(clippy::too_many_arguments)]
    fn place_at(doc: &MissionDocCore, side: &str, slot: &str, x: f64, y: f64) -> String {
        let (_, sid, _) = place_character_under_side(
            doc,
            side,
            slot,
            "lyr",
            "Rifleman",
            None,
            Some("asset/Rifleman.et".into()),
            x,
            y,
            0.0,
            0.0,
        )
        .expect("place");
        sid
    }

    /// The headline: five clicks build a five-man squad, not five one-man squads — and every body
    /// keeps the id, map position, rank and callsign it was placed with, at a dense `index`.
    #[test]
    fn five_places_build_one_squad_and_keep_every_body() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let mut ids = Vec::new();
        for i in 0..5 {
            let slot = format!("n{i}");
            place_at(&doc, "OPFOR", &slot, 1000.0 + 10.0 * f64::from(i), 2000.0);
            ids.push(slot);
        }
        doc.update_slot_identity("n3", Some("A-4".into()), Some("Corporal".into()));

        let squads = squad_ids(&doc, "OPFOR");
        assert_eq!(squads.len(), 1, "five clicks, one squad: {squads:?}");
        let squad = &small(&doc)["squadsById"][&squads[0]];
        assert_eq!(
            squad["slotIds"].as_array().expect("slotIds").len(),
            5,
            "all five bodies filed into it"
        );
        assert_eq!(squad["leaderSlotId"], "n0", "the first click leads");
        assert_eq!(squad["name"], "Squad 1");

        let s = slots(&doc);
        for (i, id) in ids.iter().enumerate() {
            assert_eq!(s[id]["index"], i as u64, "dense index for {id}");
            assert_eq!(s[id]["position"]["x"], 1000.0 + 10.0 * i as f64, "{id} x");
            assert_eq!(s[id]["position"]["y"], 2000.0, "{id} y");
            assert_eq!(s[id]["squadId"], squads[0], "{id} squad");
        }
        assert_eq!(s["n3"]["callsign"], "A-4", "identity survives later places");
        assert_eq!(s["n3"]["rank"], "Corporal");
    }

    /// Sides do not share a current squad — BLUFOR's bottom squad is not OPFOR's.
    #[test]
    fn each_side_keeps_its_own_current_squad() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let b1 = place_at(&doc, "BLUFOR", "b1", 1.0, 1.0);
        let o1 = place_at(&doc, "OPFOR", "o1", 2.0, 2.0);
        let b2 = place_at(&doc, "BLUFOR", "b2", 3.0, 3.0);
        assert_eq!(b1, b2, "BLUFOR kept its own squad across an OPFOR place");
        assert_ne!(b1, o1);
        assert_eq!(squad_ids(&doc, "BLUFOR").len(), 1);
        assert_eq!(squad_ids(&doc, "OPFOR").len(), 1);
    }

    /// A squad the operator authored is stepped around, not grown: placement starts a fresh squad
    /// at the bottom of the side. Then — the property that keeps the old defect from returning —
    /// every further placement accumulates in that new squad instead of minting again.
    ///
    /// One case per authoring signal. Each closure leaves the side with an authored **bottom**
    /// squad and returns `(its id, the roster it must still have afterwards)`.
    #[test]
    fn placement_starts_a_new_squad_rather_than_growing_an_authored_one() {
        type Author = fn(&MissionDocCore, &str) -> (String, Vec<Value>);
        let authors: [(&str, Author); 3] = [
            ("renamed", |doc, first| {
                doc.rename_squad(first, "Alpha");
                (first.to_string(), vec![Value::String("a".into())])
            }),
            ("vehicle", |doc, first| {
                doc.add_vehicle("veh-1", "{V}Truck.et", Some(1.0), Some(2.0), None, None);
                doc.attach_vehicle(first, "veh-1");
                (first.to_string(), vec![Value::String("a".into())])
            }),
            ("callsign", |doc, _first| {
                // `add_squad` is the only writer of squad-level `callsign`, so this shape reaches a
                // live document through `hydrate` — a loaded mission whose bottom squad carries one.
                doc.add_squad(
                    "squad-BLUFOR-cs",
                    "faction-BLUFOR",
                    "Squad 2",
                    Some("A-1".into()),
                );
                ("squad-BLUFOR-cs".to_string(), Vec::new())
            }),
        ];

        for (label, author) in authors {
            let doc = MissionDocCore::new();
            layer(&doc);
            let first = place_at(&doc, "BLUFOR", "a", 10.0, 20.0);
            let (authored, roster) = author(&doc, &first);
            let before = squad_ids(&doc, "BLUFOR");
            assert_eq!(
                before.last(),
                Some(&authored),
                "{label}: authored is bottom"
            );

            let second = place_at(&doc, "BLUFOR", "b", 30.0, 40.0);
            assert_ne!(
                second, authored,
                "{label}: authored squad must not be grown"
            );
            let after = squad_ids(&doc, "BLUFOR");
            assert_eq!(
                after.len(),
                before.len() + 1,
                "{label}: exactly one new squad"
            );
            assert_eq!(after.last(), Some(&second), "{label}: minted at the bottom");

            // The authored squad is untouched — same roster it had before the placement.
            assert_eq!(
                small(&doc)["squadsById"][&authored]["slotIds"]
                    .as_array()
                    .expect("slotIds"),
                &roster,
                "{label}: authored squad kept its roster"
            );

            // …and the new squad now accumulates, so this mints ONE extra squad, not one per click.
            let third = place_at(&doc, "BLUFOR", "c", 50.0, 60.0);
            let fourth = place_at(&doc, "BLUFOR", "d", 70.0, 80.0);
            assert_eq!(third, second, "{label}: accumulates in the new squad");
            assert_eq!(fourth, second, "{label}: accumulates in the new squad");
            assert_eq!(
                squad_ids(&doc, "BLUFOR").len(),
                before.len() + 1,
                "{label}: one squad per authoring event, never one per click"
            );
            assert_eq!(
                small(&doc)["squadsById"][&second]["leaderSlotId"],
                "b",
                "{label}: the new squad's first body leads it"
            );
        }
    }

    /// The ORBAT panel's "add squad" (`editor_ops::orbat_add_squad`) mints an EMPTY `Squad {n}`.
    /// That is the operator saying "fill this next", so the next placement joins it — and, being
    /// its first body, leads it — rather than minting a third squad beside an empty husk.
    #[test]
    fn an_empty_panel_minted_squad_is_filled_by_the_next_placement() {
        let doc = MissionDocCore::new();
        layer(&doc);
        let first = place_at(&doc, "BLUFOR", "a", 10.0, 20.0);
        doc.rename_squad(&first, "Alpha"); // close it, the way an operator would
        doc.add_squad("squad-BLUFOR-99", "faction-BLUFOR", "Squad 2", None);

        let target = place_at(&doc, "BLUFOR", "b", 30.0, 40.0);
        assert_eq!(target, "squad-BLUFOR-99", "filled the husk");
        assert_eq!(squad_ids(&doc, "BLUFOR").len(), 2, "no third squad");
        let squad = &small(&doc)["squadsById"]["squad-BLUFOR-99"];
        assert_eq!(squad["slotIds"].as_array().expect("slotIds").len(), 1);
        assert_eq!(
            squad["leaderSlotId"], "b",
            "first body into an empty squad leads"
        );
    }

    /// The target is re-derived from the document on every call, never remembered beside it — so
    /// undo needs no cooperation from this module. Undoing the second placement rewinds `slotIds`,
    /// and the next placement reads the rewound state and lands in the same squad again.
    ///
    /// It also gets cheaper: an appending placement is ONE tracked transaction (`add_slot`), where
    /// the T-180.1 mint was three (`add_squad` + `add_slot` + `set_leader`) — with
    /// `capture_timeout_millis = 0` every transaction is its own undo step, so one Ctrl+Z used to
    /// strip a placement's leader and leave the body behind.
    #[test]
    fn the_current_squad_is_re_derived_after_undo() {
        let mut doc = MissionDocCore::new();
        layer(&doc);
        let s1 = place_at(&doc, "BLUFOR", "a", 10.0, 20.0);
        let depth_after_first = doc.undo_depth();
        let s2 = place_at(&doc, "BLUFOR", "b", 30.0, 40.0);
        assert_eq!(s1, s2);
        assert_eq!(
            doc.undo_depth() - depth_after_first,
            1,
            "an appending placement is one undo step"
        );

        assert!(doc.undo(), "undo the second placement");
        let root = small(&doc);
        assert_eq!(
            squad_ids(&doc, "BLUFOR"),
            vec![s1.clone()],
            "squad survives"
        );
        assert_eq!(
            root["squadsById"][&s1]["slotIds"]
                .as_array()
                .expect("slotIds"),
            &vec![Value::String("a".into())],
            "the undone body is gone, the first is not"
        );
        assert_eq!(root["squadsById"][&s1]["leaderSlotId"], "a", "SL intact");

        // Re-place: the rewound document is read fresh, so it lands in the same squad at index 1.
        let s3 = place_at(&doc, "BLUFOR", "c", 50.0, 60.0);
        assert_eq!(s3, s1, "no stale pointer to an undone squad");
        assert_eq!(slots(&doc)["c"]["index"], 1);
        assert_eq!(squad_ids(&doc, "BLUFOR").len(), 1);
    }

    /// [`is_minted_squad_name`] has a twin in `apply_faction`; they must agree or a placement could
    /// grow a squad Apply then refuses over. Pin this copy against the strings *and* against what a
    /// real placement actually writes.
    #[test]
    fn minted_names_match_what_placement_writes() {
        for minted in ["Squad 1", "Squad 2", "Squad 17", "Squad 0", "", "  "] {
            assert!(is_minted_squad_name(minted), "{minted:?}");
        }
        for authored in ["Alpha", "Squad", "Squad A", "1st Squad", "Squad 1a"] {
            assert!(!is_minted_squad_name(authored), "{authored:?}");
        }

        // The real thing: every squad a placement mints must read as minted to its own predicate,
        // or the second click would refuse to join the first click's squad.
        let doc = MissionDocCore::new();
        layer(&doc);
        let first = place_at(&doc, "BLUFOR", "a", 1.0, 2.0);
        doc.rename_squad(&first, "Alpha");
        let second = place_at(&doc, "BLUFOR", "b", 3.0, 4.0);
        let root = small(&doc);
        assert_eq!(root["squadsById"][&second]["name"], "Squad 2", "ordinal");
        assert!(is_minted_squad_name(
            root["squadsById"][&second]["name"].as_str().unwrap_or("")
        ));
        assert!(is_open_for_placement(&root["squadsById"][&second]));
        assert!(!is_open_for_placement(&root["squadsById"][&first]));
    }
}
