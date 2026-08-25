//! The pure loadout core — `SlotLoadoutV2` serialization ([`loadout_to_picks`] /
//! [`picks_to_loadout`]), the export/import gates ([`try_export`] / [`try_import`]), the
//! T-699 loadout buffer (plan / commit / receipts) and the refusal vocabulary.
//!
//! Split out of `arsenal/mod.rs` at T-934.8 with bodies unchanged; `mod.rs` re-exports every
//! public item, so the `crate::editor::arsenal::X` paths external callers use are stable.

use std::collections::{HashMap, HashSet};

use crate::core::dto::RegistryItem;
use crate::editor::arsenal::arsenal_rules::{
    self as rules, index_by_name, validate_loadout, CompatFeed,
};

/// A loadout row: the pick key (matches `arsenalRules` `LoadoutKey`), its label, the registry kind
/// it sources from, and whether it is a weapon slot (→ `weapons[]`) or wear (→ `wear{}`).
struct Row {
    key: &'static str,
    label: &'static str,
    kind: &'static str,
    /// `Some((slot_index, slot_type))` for weapon rows; `None` for wear rows.
    weapon: Option<(i64, &'static str)>,
}

/// `LOADOUT_ROWS` minus the two compat `edge` rows (optic / magazine) — the kind-sourced set.
/// Order mirrors the React ACE layout.
const ROWS: &[Row] = &[
    Row {
        key: "primary",
        label: "Primary",
        kind: "gear_primary",
        weapon: Some((0, "primary")),
    },
    Row {
        key: "launcher",
        label: "Launcher",
        kind: "gear_launcher",
        weapon: Some((1, "primary")),
    },
    Row {
        key: "handgun",
        label: "Handgun",
        kind: "gear_handgun",
        weapon: Some((2, "secondary")),
    },
    Row {
        key: "throwable",
        label: "Throwable",
        kind: "gear_throwable",
        weapon: Some((3, "grenade")),
    },
    Row {
        key: "headCover",
        label: "Helmet",
        kind: "gear_helmet",
        weapon: None,
    },
    Row {
        key: "jacket",
        label: "Jacket",
        kind: "gear_jacket",
        weapon: None,
    },
    Row {
        key: "pants",
        label: "Pants",
        kind: "gear_pants",
        weapon: None,
    },
    Row {
        key: "boots",
        label: "Boots",
        kind: "gear_boots",
        weapon: None,
    },
    Row {
        key: "vest",
        label: "Vest (chest rig)",
        kind: "gear_vest",
        weapon: None,
    },
    Row {
        key: "armoredVest",
        label: "Armored Vest",
        kind: "gear_armored_vest",
        weapon: None,
    },
    Row {
        key: "backpack",
        label: "Backpack",
        kind: "gear_backpack",
        weapon: None,
    },
    Row {
        key: "handwear",
        label: "Gloves",
        kind: "gear_gloves",
        weapon: None,
    },
];

/* ─────────────────────── T-197 — weapon attachments (the pick SET) ─────────────────────── */

/// The compat family that links an attachment to the weapon accepting it. **241 such edges ship in
/// the vanilla export and nothing read them before this slice.** They have no `LOADOUT_ROWS` entry
/// because an attachment slot is not one-of-N: a rifle takes a handguard AND a stock AND a muzzle
/// device at once, so the pick is a **set**, not a value — and `LoadoutRow` models a value.
pub(super) const ATTACHMENT_EDGE: &str = "attachment_on_weapon";

/// Separator for the packed attachment set. U+001F (ASCII US) is safe **by contract, not by luck**:
/// `registry-compat.schema.json#/$defs/resourceName` pins every node to
/// `^\{[0-9A-F]{16}\}[A-Za-z0-9/_.\- ()']+$` — a pattern that admits no control character — so a
/// join can never produce a string that splits back into something else.
const ATTACHMENT_SEP: &str = "\u{1f}";

/// The `picks` key holding `weapon_key`'s attachment set.
///
/// The set rides a **synthetic key** rather than widening `picks` to `HashMap<String, Vec<String>>`
/// because that map is the argument type of three [`crate::editor::arsenal::arsenal_rules`] entry points
/// (`row_options`, `validate_loadout`, `loadout_weight`) and this slice does not own that module.
/// The `@` infix cannot collide with a row key, and each of those consumers iterates `LOADOUT_ROWS`
/// **by key** — so the synthetic entry is invisible to them by construction, not by convention.
pub(super) fn attachments_key(weapon_key: &str) -> String {
    format!("attachments@{weapon_key}")
}

/// `weapon_key`'s picked attachments, in pick order.
pub(super) fn attachments_of(picks: &HashMap<String, String>, weapon_key: &str) -> Vec<String> {
    picks
        .get(&attachments_key(weapon_key))
        .map(|packed| {
            packed
                .split(ATTACHMENT_SEP)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Pack a set back into its `picks` value. An empty set packs to `""`, which the `pick_item` path
/// treats as "remove the key" — so clearing the last attachment leaves no residue in the map.
pub(super) fn pack_attachments(list: &[String]) -> String {
    list.join(ATTACHMENT_SEP)
}

/// Attachments stranded by a weapon swap — the same authoring hazard `validate_loadout` already
/// flags for optic/magazine, checked here because the set rides a key `arsenal_rules` cannot see.
/// Keyed on the **weapon** row so the message lands on the row the author must actually change,
/// and worded to mirror the two `validate_loadout` cases (hostless / rejected).
///
/// Degrades exactly like `validate_loadout`: a feed we never received must never fail a loadout.
fn attachment_errors(picks: &HashMap<String, String>, feed: &CompatFeed) -> Vec<rules::RowError> {
    let Some(g) = feed.ready_graph() else {
        return Vec::new();
    };
    let mut errs = Vec::new();
    for &(key, _, _) in rules::WEAPON_SLOTS {
        let host = picks.get(key).filter(|s| !s.is_empty());
        let label = rules::row(key).map_or(key, |r| r.label);
        for rn in attachments_of(picks, key) {
            // The message names `rn` because the key cannot. `refusal_line` separates two stranded
            // *rows* by prefixing their labels, but a weapon row can strand two *attachments* at
            // once — same row, same key, same prefix — so without the resource name the author is
            // handed the identical sentence twice and told what is wrong but not *which of their
            // attachments* is at fault. That is exactly the defect T-737 removed, one level down.
            let message = match host {
                None => format!("Attachment `{rn}` requires a {label} pick"),
                Some(h) if !g.accepts(h, &rn, ATTACHMENT_EDGE) => {
                    format!("Attachment `{rn}` not compatible with the selected {label}")
                }
                Some(_) => continue,
            };
            errs.push(rules::RowError { key, message });
        }
    }
    errs
}

/// T-240 — every fault on this loadout, in one list: the compat edge rows
/// ([`validate_loadout`]), the stranded attachments ([`attachment_errors`]), the over-capacity
/// cargo containers ([`rules::cargo_capacity_errors`]) and — T-504 — the cargo authored against a
/// container this loadout wears nothing in ([`rules::cargo_unworn_container_errors`]).
///
/// This is what the verdict badge counts and what the per-row error line reads. Every source is
/// keyed on the row whose pick the author must change, and the feed-fed ones degrade to empty when
/// the compat feed never arrived — a feed we did not receive must never fail a loadout. (Capacity
/// does not need the feed at all; it reads the registry. The unworn check needs neither: worn-or-not
/// is a fact about `picks`.)
///
/// T-504 — the "Loadout valid" badge was the tool reporting success over an input it had never
/// examined. Undeliverable cargo produced no fault anywhere, so a loadout whose mags were headed
/// for a vest nobody wears was badged valid, exported clean, and only failed on a server the author
/// does not read. The fault belongs **here** and not in [`try_export`]: this list warns, that one
/// refuses, and [`rules::CARGO_UNWORN_CAVEAT`] sets out why refusing would be wrong. `kit_defaults`
/// is the vouching evidence — [`kit_default_items`] builds it, `None` keeps the rule silent.
pub(super) fn loadout_faults(
    picks: &HashMap<String, String>,
    cargo: &[rules::CargoRow],
    feed: &CompatFeed,
    idx: &HashMap<String, &RegistryItem>,
    kit_defaults: Option<&HashSet<String>>,
) -> Vec<rules::RowError> {
    let mut errs = validate_loadout(picks, feed.ready_graph(), feed.status);
    errs.extend(attachment_errors(picks, feed));
    errs.extend(rules::cargo_capacity_errors(picks, cargo, idx));
    errs.extend(rules::cargo_unworn_container_errors(
        picks,
        cargo,
        kit_defaults,
    ));
    errs
}

/// T-504 — what the slot's character prefab is catalogued as already carrying, keyed on its
/// `assetId`. This is the evidence [`rules::cargo_unworn_container_errors`] needs to tell a seeded
/// row (delivered by the kit) from one the author aimed at nothing.
///
/// The `character_default_cargo` edges are already in the feed the Arsenal holds — `CompatGraph`
/// keeps their adjacency in both directions, so a lookup keyed on the character returns its items.
/// (The *containers* are in the edges' `evidence`, which the graph drops, which is why the vouching
/// is by item.)
///
/// `None` — the honest "no evidence" answer — whenever the feed is not `Ready` or the slot has no
/// `assetId` to key on, so the rule stays silent rather than guessing.
pub(super) fn kit_default_items(
    feed: &CompatFeed,
    asset_id: Option<&str>,
) -> Option<HashSet<String>> {
    let graph = feed.ready_graph()?;
    let rn = asset_id?;
    Some(
        graph
            .items_for(rn, rules::CHARACTER_DEFAULT_CARGO_EDGE)
            .into_iter()
            .collect(),
    )
}

/// The slot's `assetId` (its character prefab) straight off the live document.
///
/// Read through the existing public `editor_ops::slots_json` rather than a new accessor — this
/// slice does not own `editor_ops`. Native has no hosted document, so there is no `assetId` and
/// [`kit_default_items`] answers `None`.
pub(super) fn slot_asset_id(slot_id: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let json = crate::editor::state::operations::slots_json()?;
        let map: serde_json::Value = serde_json::from_str(&json).ok()?;
        map.get(slot_id)?
            .get("assetId")?
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = slot_id;
        None
    }
}

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
                    // T-197 — `attachments[]` is a per-weapon field on the v2 `weapon` def
                    // (`loadout-export.schema.json`), not a primary-only sub-slot, so it is read
                    // for EVERY weapon row: a mod that ships `attachment_on_weapon` edges for a
                    // launcher round-trips without a second code path.
                    //
                    // T-199 — THIS IS WHERE THE SEPARATOR HAZARD LIVES, so this is where it dies.
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

/// `picksToLoadout` — build the canonical `SlotLoadoutV2` from the picks. All-empty (picks AND
/// cargo) → `None` (clear the doc field). Wear map + weapons array; primary re-emits its sticky
/// `optic`/`magazine` (String or null) plus its `attachments[]` — the T-197 wire-through that
/// replaced the hardcoded `[]` this line carried since the dumb Forge. An attachment set only ever
/// rides a weapon that is actually picked, so a set stranded by a cleared weapon is flagged in the
/// UI (see [`attachment_errors`]) but never reaches the doc. `cargo`: `Some(rows)` re-emits verbatim
/// (the commit fires on each pick change; dropping it would wipe seeded rows) — `Some(&[])`
/// included, since key presence is the T-068.15.2 "user state" marker that stops re-seeding a
/// cleared list. `None` = the slot never had the key and cargo was untouched: stay key-less so a
/// later seed can still fire. `names` resolves `resource_name` → `display_name` for the `summary`.
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
            // something to say — an empty-set row stays byte-identical to its pre-T-197 self.
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

/* ───────────── T-199 — the downloaded FILE (`loadout-export.schema.json`) ───────────── */

/// Build the **exported document** — the bytes behind "Download loadout JSON".
///
/// THE BUG THIS REPLACES. The button used to hand the user [`picks_to_loadout`]'s output, i.e.
/// the editor's own persisted `SlotLoadoutV2` dict (`mission.schema.json` `slot.loadout`). Those
/// are two different contracts that merely look alike, and the doc field fails **both** `oneOf`
/// branches of `loadout-export.schema.json`: it has no `loadoutVersion`, no `modpackId` and no
/// `gear`, and it carries `version` + `summary` against `additionalProperties: false`. The one
/// consumer of the file — `TBD_LoadoutEquipComponent` reading `$profile:TBD_LoadoutTest.json` —
/// reads `loadoutVersion` off it and refuses anything it does not recognise, so the download
/// produced a file that the only thing that reads it rejected on sight.
///
/// WHY v2, NOT v1. The v1 branch is `{loadoutVersion, modpackId, gear}` with
/// `additionalProperties: false`, so choosing it would mean deleting the launcher, the sidearm,
/// the throwable, pants/boots/gloves/backpack, attachments and every cargo row from the file —
/// exactly the content T-182 widened the compiled gear block to carry. v2 is the branch written
/// for this producer, and it keeps the derived legacy `gear` block for the v1-shaped reader.
///
/// The derived `gear` block uses the **locked** rule, kept byte-identical to the compiler's
/// `mission/flatten.rs::mod_slot_loadout` so the file and the compiled mission describe the same
/// soldier: `jacket`→uniform, `armoredVest` else `vest`→vest, `headCover`→helmet, and the weapon
/// at `(slotIndex 0, slotType "primary")`→primary (+ its optic/magazine). `optic`/`magazine` ride
/// the primary alone — deriving them when no primary is picked would describe a scope mounted on
/// nothing.
///
/// `equipment` is omitted deliberately: it is optional in v2 and the Arsenal has no equipment
/// rows yet (binoculars/wristwatch land with the equipment slice), so emitting an all-null block
/// would claim authored state that does not exist. `wear` and `cargo` are always emitted, because
/// "no cargo" and "this slot is bare" are things the file should say out loud; the doc field's
/// key-presence subtleties are an anti-reseed marker for the editor, not part of this contract.
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
    // slot it belongs in. That pair — (slotIndex, slotType) — is what the T-182 reader matches on.
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

/// The modpack the picks were authored against — `modpackId` on the exported file.
///
/// Every registry row is scoped to one modpack (`GET /registry` filters by it), so the catalog the
/// Arsenal was handed IS the answer; there is no second source to disagree with. An empty catalog
/// yields `""`, which the schema permits (`{"type":"string"}`, no `minLength`) and which the mod
/// reader turns into a named `modpackId … != expected` warning rather than a silent mismatch —
/// the honest outcome when the registry fetch failed and we genuinely do not know.
pub(super) fn export_modpack_id(items: &[RegistryItem]) -> String {
    items
        .first()
        .map(|it| it.modpack_id.clone())
        .unwrap_or_default()
}

/// T-240 — the export gate. `Ok` is the `loadout-export.schema.json` document; on `Err` there are
/// **no bytes at all**, only the refusals, so a refusal cannot be half-downloaded.
///
/// Refuses on **capacity faults only** ([`rules::cargo_capacity_errors`]), deliberately not on the
/// whole [`loadout_faults`] list. The compat / stranded-attachment faults predate this ticket, have
/// never blocked an export, and making them blocking is a separate behaviour change nobody has
/// measured — the badge still counts them so the author sees them. Capacity is different in kind:
/// what it flags is kit the game will silently drop on the way to the field, so the file is a lie
/// about the soldier it describes.
///
/// T-504 — the unworn-container fault ([`rules::cargo_unworn_container_errors`]) is deliberately
/// **not** added to this gate either, and for a stronger reason than "nobody measured it": this
/// module cannot see the slot's kit prefab, whose own clothing is what the mod actually resolves
/// the container against, so a refusal here would block Save/Export on loadouts that deliver
/// perfectly — including freshly seeded ones nobody has touched. It warns in [`loadout_faults`]
/// instead. Full argument on [`rules::CARGO_UNWORN_CAVEAT`]; pinned by
/// `tests::t504::undeliverable_cargo_fails_the_verdict_but_never_the_export`.
///
/// Structured to lift: the `Err` arm is already a list of independent findings, the same shape
/// `validate_mission_editor_payload` returns, for when this rule moves server-side.
pub fn try_export(
    picks: &HashMap<String, String>,
    cargo: &[rules::CargoRow],
    items: &[RegistryItem],
    modpack_id: &str,
) -> Result<String, Vec<rules::RowError>> {
    let idx = index_by_name(items);
    let refusals = rules::cargo_capacity_errors(picks, cargo, &idx);
    if !refusals.is_empty() {
        return Err(refusals);
    }
    Ok(picks_to_export(picks, cargo, modpack_id))
}

/* ───────── T-686 — the INGEST half: reading a `loadout-export.schema.json` doc back ───────── */

/// The row key every *document-level* refusal is filed under.
///
/// [`rules::RowError::key`] normally names the loadout row whose pick the author must change, and
/// the rule-derived refusals below keep doing exactly that. A malformed file has no row to blame —
/// the fault is the document — so it gets its own key rather than being pinned on an innocent row.
const IMPORT_DOC_KEY: &str = "document";

/// T-737 — render one [`rules::RowError`] as the line the author actually reads.
///
/// A `RowError` is a **pair**: the row whose pick must change, and the reason it must. Both refusal
/// lists in this panel used to print `e.message` alone and drop the key on the floor — and the
/// reason on its own is not an instruction. One weapon swap strands the optic *and* the magazine,
/// and both rows then say the identical sentence ("Not compatible with the selected Primary"), so
/// an author handed two of them learns only that something is wrong twice. The row label is the
/// address; without it a refusal names what is required but never *which of their rows* is at
/// fault. Prefixing it puts the distinguishing token at the left margin of every line, which is
/// where a list is scanned.
///
/// **[`IMPORT_DOC_KEY`] is exempt, by the same argument.** A document-level fault has no row to
/// blame — that is why it has its own key — and the schema checker's message already carries the
/// JSON pointer, which is a *better* address than any row label. So the rule is not "the doc key is
/// special", it is "prefix the label when there is a row"; [`rules::row`] answering `None` is
/// exactly the condition, and any future non-row key inherits the same handling for free.
///
/// The reason is never rewritten, only prefixed, so a caller that has already framed the message
/// (Apply's "Buffered loadout from `<id>` — …") keeps its framing intact underneath.
#[must_use]
pub fn refusal_line(e: &rules::RowError) -> String {
    match rules::row(e.key) {
        Some(r) => format!("{} — {}", r.label, e.message),
        None => e.message.clone(),
    }
}

/// What an accepted import *would* apply. Nothing in here has touched the live mission document:
/// [`try_import`] returns a value, the caller applies it, and that separation is what makes
/// "a document that does not validate applies nothing" true by construction rather than by care.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportedLoadout {
    /// The picks map, in the same shape [`loadout_to_picks`] produces (incl. the packed
    /// `attachments@<weapon>` keys).
    pub picks: HashMap<String, String>,
    pub cargo: Vec<rules::CargoRow>,
    /// Did the document carry a `cargo` key at all?
    ///
    /// `cargo` is optional in the v2 branch and absent from v1 entirely, and key presence is the
    /// T-068.15.2 "user state" marker: present-and-empty means *the author cleared it* (never
    /// re-seed), absent means *nobody has said* (a later seed may still fire). A file that never
    /// mentions cargo has not authored an empty cargo list, so importing one must not claim it did.
    pub cargo_present: bool,
    /// `modpackId` off the document. Reported, never a refusal — see [`try_import`].
    pub modpack_id: String,
    /// `"1"` or `"2"` — which `oneOf` branch the document satisfied.
    pub loadout_version: String,
}

/// Read an accepted document into picks + cargo.
///
/// **v2** is the inverse of [`picks_to_export`] and re-uses [`loadout_to_picks`] +
/// [`rules::cargo_from_loadout`] verbatim rather than growing a second reader: the export file's
/// `wear` / `weapons` / `cargo` blocks are the same byte shape as the persisted `SlotLoadoutV2`
/// doc field (that is *why* T-199 could reuse the wear/weapon vocabulary), so the code that already
/// reads one reads the other. The derived legacy `gear` block is deliberately IGNORED on a v2
/// document: it is a lossy projection of `wear`/`weapons` written for the v1 mod reader, and
/// preferring it would silently discard the launcher, the sidearm and half the wear rows.
///
/// **v1** has only the four fixed gear slots, so the mapping is the documented derivation run
/// backwards: `uniform`→jacket, `helmet`→headCover, `primary`(+optic/magazine)→the primary weapon.
/// `vest` lands on the **`vest`** row and not `armoredVest`, because v1 has one vest key and the
/// two are one-way collapsible — choosing `armoredVest` would invent armour the file never claimed.
fn import_doc_to_picks(
    raw: &str,
    doc: &serde_json::Value,
) -> (HashMap<String, String>, Vec<rules::CargoRow>, bool) {
    let version = doc
        .get("loadoutVersion")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    if version == "2" {
        let picks = loadout_to_picks(Some(raw));
        let (cargo, present) = rules::cargo_from_loadout(Some(raw));
        return (picks, cargo, present);
    }
    let gear = doc.get("gear");
    let mut picks = HashMap::new();
    for (doc_key, pick_key) in [
        ("primary", "primary"),
        ("uniform", "jacket"),
        ("vest", "vest"),
        ("helmet", "headCover"),
        ("optic", "optic"),
        ("magazine", "magazine"),
    ] {
        let value = gear
            .and_then(|g| g.get(doc_key))
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty());
        if let Some(v) = value {
            picks.insert(pick_key.to_string(), v.to_string());
        }
    }
    (picks, Vec::new(), false)
}

/// T-686 — the import gate, and the exact mirror of [`try_export`]. `Ok` is the state to apply; on
/// `Err` there are **no picks at all**, only the refusals, so a refusal cannot be half-applied.
///
/// TBD shipped the export half of this round-trip and none of the ingest half: `try_export` +
/// `download_json` wrote a `loadout-export.schema.json` v2 file that nothing in the SPA could read
/// back. This is the door in. No new format — the same shipped schema, the same reader
/// ([`loadout_to_picks`]), the same [`rules::RowError`] refusal vocabulary.
///
/// **Three gates, in order, and the first one that speaks stops the import:**
/// 1. **It is JSON.** A parse failure is the document's fault, so it is filed under
///    [`IMPORT_DOC_KEY`].
/// 2. **It satisfies the SHIPPED schema** ([`rules::validate_against_loadout_export_schema`], which
///    checks against the `include_str!`-compiled bytes of the file itself, not a transcription).
///    This is the gate that makes the OFCRA class of bug unrepresentable rather than merely
///    unlikely: a misspelled wear key, a cargo container outside the closed vocabulary, `qty: 0`, a
///    string where a slot wants a ResourceName-or-null — all of them are *schema* errors, and all
///    of them were silent data in a hand-maintained `.sqf` (ofcra_omtk.md 5.9, 14.1).
/// 3. **The picks obey the loadout rules** — [`validate_loadout`] (compat edges),
///    [`attachment_errors`] (the packed set `arsenal_rules` cannot see) and
///    [`rules::cargo_capacity_errors`]. A schema-valid document can still describe a scope on no
///    rifle or forty magazines in a chest rig; importing it without this check would re-import
///    exactly the silent data bugs the schema gate cannot see.
///
/// **What is deliberately NOT a refusal, and why:**
/// * [`rules::cargo_unworn_container_errors`] — T-504's argument holds unchanged on the way in:
///   this module cannot see the slot's kit prefab, whose own clothing is what the mod resolves the
///   container against, so refusing here would block imports of loadouts that deliver perfectly.
///   It stays a warning in [`loadout_faults`], where the author sees it after the import lands.
/// * A **`modpackId` mismatch.** The document carries the modpack it was authored against and the
///   Arsenal knows its own ([`export_modpack_id`]), but the honest answer to a mismatch is "these
///   resource names may not resolve", not "you may not do this" — and the compat/registry checks
///   above already fail on names this catalog genuinely does not have. The value is returned so the
///   caller can say so.
///
/// Note the asymmetry this creates with [`try_export`], which refuses on capacity **only**: a
/// loadout with a stranded optic can be downloaded but not re-imported. That is intended. The
/// export gate's job is to not write a file that lies about a soldier; this one's job is to not let
/// an outside document put the editor into a state the author did not author. A document that
/// fails here describes a loadout the Arsenal would badge as broken the moment it landed, and the
/// author is better served being told before it lands than after. (Both feed-fed checks degrade to
/// empty when the compat feed is not `Ready` — a feed we never received must never fail a loadout,
/// on the way in or out.)
pub fn try_import(
    raw: &str,
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> Result<ImportedLoadout, Vec<rules::RowError>> {
    let doc: serde_json::Value = match serde_json::from_str(raw) {
        Ok(v) => v,
        Err(e) => {
            return Err(vec![rules::RowError {
                key: IMPORT_DOC_KEY,
                message: format!("This file is not valid JSON — {e}."),
            }])
        }
    };
    if let Err(faults) = rules::validate_against_loadout_export_schema(&doc) {
        return Err(faults
            .into_iter()
            .map(|message| rules::RowError {
                key: IMPORT_DOC_KEY,
                message,
            })
            .collect());
    }
    let (picks, cargo, cargo_present) = import_doc_to_picks(raw, &doc);

    // The rule pass, BEFORE anything is applied. Same three sources the verdict badge counts,
    // minus the one T-504 proved must never block.
    //
    // T-699 — the three checks moved BODILY into `loadout_rule_refusals` and this line now calls it,
    // because T-699's Apply needs the identical pass and "identical" has to be structural. Nothing
    // about this gate's behaviour changed; what changed is that there is now exactly one of it.
    let refusals = loadout_rule_refusals(&picks, &cargo, items, feed);
    if !refusals.is_empty() {
        return Err(refusals);
    }

    Ok(ImportedLoadout {
        picks,
        cargo,
        cargo_present,
        modpack_id: doc
            .get("modpackId")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        loadout_version: doc
            .get("loadoutVersion")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
    })
}

/// The one-line receipt an accepted import prints: what actually landed, counted off the applied
/// state rather than off the file, so it cannot claim more than was applied.
pub(super) fn import_summary(name: &str, doc: &ImportedLoadout, catalog_modpack: &str) -> String {
    let weapons = ROWS
        .iter()
        .filter(|r| r.weapon.is_some())
        .filter(|r| doc.picks.get(r.key).is_some_and(|v| !v.is_empty()))
        .count();
    let wear = ROWS
        .iter()
        .filter(|r| r.weapon.is_none())
        .filter(|r| doc.picks.get(r.key).is_some_and(|v| !v.is_empty()))
        .count();
    let mut line = format!(
        "Imported {name} (v{}) — {weapons} weapon(s), {wear} wear row(s), {} cargo row(s). One Ctrl+Z undoes the whole import.",
        doc.loadout_version,
        doc.cargo.len(),
    );
    // Warn-only, and only when both sides actually know what they are: an empty modpackId is the
    // honest "we do not know" the export writes when the registry fetch failed, not a mismatch.
    if !doc.modpack_id.is_empty()
        && !catalog_modpack.is_empty()
        && doc.modpack_id != catalog_modpack
    {
        line.push_str(&format!(
            " Note: this file was authored against modpack {}, and this mission's catalog is {} — check the picks resolved to what you expected.",
            doc.modpack_id, catalog_modpack
        ));
    }
    line
}

/* ═════ T-699 (3DEN-LOAD-001 / -002 / -010) — the loadout BUFFER: Copy · Apply · Remove Everything ═════ */

/// **A buffer, not an inheritance hierarchy — and that is the whole design.**
///
/// T-687 proposed OFCRA-style loadout INHERITANCE (parent kits, defaults-by-role, children that
/// *resolve* against a template) and the operator cancelled it outright; it is filed REJECTED, not
/// deferred, precisely so a later synthesis pass cannot revive it without asking again. T-699 is
/// the practical half that survives, and the difference is structural rather than a matter of
/// taste: **nothing in this module stores a relationship.** `editor_ops::copy_loadouts_from_selection`
/// snapshots bytes that already exist on the sources, [`plan_apply`] writes them onto other entities, and
/// from that instant the two documents are strangers — editing the source later changes nothing,
/// because no target holds a reference to it. There is no parent, no template, no default-by-role
/// and no resolution step, and the [`BufferedLoadout`] type below is the evidence: a source id kept
/// for the receipt, and a `String` of JSON. Add a field pointing the other way and you have built
/// the cancelled ticket.
///
/// **Three verbs, and the exclusions are as load-bearing as the inclusions.** Copy, Apply and
/// Remove Everything ship. The nine per-category strip variants that 3den E7 also lists (remove
/// NVGs / vests / goggles / headgear / weapons / …) are marked `maybe` upstream and are deliberately
/// **not** here: each would be a second, narrower writer over the same document field, and nine of
/// them is nine chances for `wear`-key vocabulary to drift out of step with [`ROWS`]. Remove
/// Everything needs no vocabulary at all — see [`stripped_loadout`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BufferedLoadout {
    /// The entity the bytes were copied off. Reported in the receipt and in refusal messages so a
    /// rejected Apply names *which* buffered loadout is unusable; it is never resolved or followed.
    pub source_id: String,
    /// The source's `SlotLoadoutV2` JSON, verbatim. `None` when the source carried no `loadout` key
    /// at all — a **bare** entity, which is a legitimate thing to buffer and to apply (it is how you
    /// say "make these look like that empty one"), and which is not the same value as
    /// [`stripped_loadout`]; see there for why the two differ.
    pub loadout_json: Option<String>,
}

/// One document write an accepted plan wants to make: exactly one `editor_ops::set_loadout`, which
/// is exactly one core transaction, which is exactly one undo step. The plan is a `Vec` of these
/// **because that is the honest shape of the operation** — see [`commit_writes`] for the undo
/// arithmetic and why it is reported rather than papered over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadoutWrite {
    pub target_id: String,
    /// Which buffered entity this loadout was drawn from; `None` for Remove Everything, which has
    /// no source.
    pub source_id: Option<String>,
    pub loadout_json: Option<String>,
}

/// The odd 64-bit constant SplitMix64 advances its state by (the odd-gamma Weyl sequence from
/// Steele/Lea/Flood 2014). Used both as the per-Apply seed step and to decorrelate the ordinal.
const APPLY_SEED_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

/// SplitMix64's finalizer — an avalanche mix, not a source of entropy. It exists so that seeds and
/// ordinals that differ by 1 produce draws that differ everywhere, which is what makes
/// [`buffer_draw`] behave like a fair die rather than like a counter.
const fn splitmix64(seed: u64) -> u64 {
    let mut z = seed;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// **WHAT "RANDOM" MEANS HERE**, because the ticket calls the randomisation the novel part and a
/// hand-wave would make it unreviewable. Apply draws one buffered loadout **per target entity**, and
/// the draw is:
///
/// * **Uniform** over the buffer. The mix is scaled by a widening multiply
///   (`(r × len) >> 64`) rather than `r % len`, so the buckets are equal-sized by construction
///   instead of equal-to-within-a-modulo-bias.
/// * **Independent per entity.** `ordinal` is the target's index in the selection, mixed into the
///   stream separately, so ten entities get ten draws — not one draw applied ten times. Two targets
///   landing on the same source is a legitimate outcome of a fair die, not a bug.
/// * **Deterministic given `(seed, ordinal, len)`, and therefore reproducible.** This is the half
///   that makes the feature reasonable to reason about: an assignment is a pure function of a
///   number, so a test can assert an exact distribution, and a bug report that says "the third
///   Apply of the session" replays exactly. `editor_ops` advances the session seed by
///   [`APPLY_SEED_GAMMA`] once per Apply, so pressing the button twice re-rolls (which is what an
///   author means by random) while the *sequence* stays fixed (which is what a reviewer means by
///   reproducible). Deliberately NO wall clock and no JS RNG: a clock would make the behaviour
///   untestable natively and irreproducible in a bug report, and would buy nothing an author can
///   perceive.
/// * **Degenerate at `len == 1`.** One buffered loadout means every target gets it, with no draw at
///   all — the single-source Copy→Apply case is plain deterministic behaviour, and randomness must
///   not be able to make it surprising.
#[must_use]
pub fn buffer_draw(seed: u64, ordinal: u64, len: usize) -> usize {
    if len <= 1 {
        return 0;
    }
    let r = splitmix64(seed ^ splitmix64(ordinal.wrapping_add(APPLY_SEED_GAMMA)));
    let wide = u128::from(r) * u128::try_from(len).unwrap_or(1);
    usize::try_from(wide >> 64).unwrap_or(0)
}

/// **The T-686 gate, extracted — not a second one.**
///
/// Wave 112's `try_import` established the rule for putting an OUTSIDE loadout onto an entity: run
/// the compat pass, the stranded-attachment pass and the cargo-capacity pass *before* anything is
/// committed, and refuse the whole document rather than half-applying it. Apply has exactly the same
/// hazard from the other direction — a buffered loadout written onto an entity that cannot carry it
/// is the same silent data bug — so it runs exactly the same three checks, and the way to guarantee
/// "exactly the same" is for there to be one function. [`try_import`] now calls this too; if a later
/// slice adds a fourth check here, both doors get it or neither does.
///
/// **[`rules::cargo_unworn_container_errors`] is deliberately absent, matching T-686's T-504 call,
/// and the argument is *stronger* here than it was for import.** T-504's reason was that this module
/// cannot see the slot's kit prefab, whose own clothing is what the mod resolves a container
/// against, so a refusal would block loadouts that deliver perfectly. Apply adds a second reason on
/// top: the check is a property of the *target* entity's character, not of the loadout bytes, so
/// wiring it in would make a buffered loadout acceptable for one selection and refused for another —
/// and, because Apply picks its source at random, refused *intermittently* for the same selection.
/// A gate that flips on a die roll is worse than no gate. It stays a warning in [`loadout_faults`],
/// which the author sees on the entity after the Apply lands.
fn loadout_rule_refusals(
    picks: &HashMap<String, String>,
    cargo: &[rules::CargoRow],
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> Vec<rules::RowError> {
    let mut refusals = validate_loadout(picks, feed.ready_graph(), feed.status);
    refusals.extend(attachment_errors(picks, feed));
    refusals.extend(rules::cargo_capacity_errors(
        picks,
        cargo,
        &index_by_name(items),
    ));
    refusals
}

/// Run [`loadout_rule_refusals`] over **every** buffered loadout, before a single die is rolled.
///
/// This ordering is the point. Validating only the loadouts that happen to be *drawn* would make
/// the gate's verdict depend on the draw: the same buffer over the same selection would be accepted
/// on one press and refused on the next, and a broken loadout could sit in the buffer indefinitely
/// waiting to ambush an author on the press where it finally came up. Validating the buffer makes
/// the answer a property of what the author copied, which is a thing they can act on. Each refusal
/// is prefixed with the source entity, because "which of the four things I copied is bad" is the
/// first question a refusal has to answer.
#[must_use]
pub fn buffer_refusals(
    buffer: &[BufferedLoadout],
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> Vec<rules::RowError> {
    let mut out = Vec::new();
    for entry in buffer {
        let picks = loadout_to_picks(entry.loadout_json.as_deref());
        let (cargo, _present) = rules::cargo_from_loadout(entry.loadout_json.as_deref());
        out.extend(
            loadout_rule_refusals(&picks, &cargo, items, feed)
                .into_iter()
                .map(|e| rules::RowError {
                    key: e.key,
                    message: format!("Buffered loadout from {} — {}", entry.source_id, e.message),
                }),
        );
    }
    out
}

/// **Apply.** Plan the writes for `targets`, drawing one buffered loadout per target. `Ok` is the
/// exact set of writes to commit; on `Err` there are **no writes at all**, only the refusals — the
/// same all-or-nothing contract [`try_import`] has, for the same reason: a partly-applied Apply
/// leaves a selection in a state the author neither authored nor can name.
///
/// An empty selection or an empty buffer is `Ok(no writes)`, not an error. Neither is a fault; there
/// is simply nothing to do, and a refusal list would be a lie about a state the author can see.
///
/// The buffered bytes are copied through **verbatim**. They are already a `SlotLoadoutV2` document
/// that this editor wrote, so re-deriving one through [`picks_to_loadout`] would be a lossy
/// round-trip for nothing: it would drop any key this module does not model (and the `cargo` key's
/// present-but-empty state, the T-068.15.2 anti-reseed marker, is exactly such a subtlety).
pub fn plan_apply(
    targets: &[String],
    buffer: &[BufferedLoadout],
    seed: u64,
    items: &[RegistryItem],
    feed: &CompatFeed,
) -> Result<Vec<LoadoutWrite>, Vec<rules::RowError>> {
    if targets.is_empty() || buffer.is_empty() {
        return Ok(Vec::new());
    }
    let refusals = buffer_refusals(buffer, items, feed);
    if !refusals.is_empty() {
        return Err(refusals);
    }
    let mut writes = Vec::with_capacity(targets.len());
    for (ordinal, target) in targets.iter().enumerate() {
        let src = &buffer[buffer_draw(seed, ordinal as u64, buffer.len())];
        writes.push(LoadoutWrite {
            target_id: target.clone(),
            source_id: Some(src.source_id.clone()),
            loadout_json: src.loadout_json.clone(),
        });
    }
    Ok(writes)
}

/// **Remove Everything** — the canonical stripped `SlotLoadoutV2`: every wear key null, no weapons,
/// and an explicitly **empty `cargo` array**.
///
/// The `cargo: []` is the load-bearing part and it is why this is not simply `set_loadout(None)`.
/// Clearing the doc field entirely would leave the slot with **no `cargo` key**, and no `cargo` key
/// is precisely the T-068.15.2 condition under which [`rules::seed_cargo`] re-seeds the character's
/// engine defaults — so the next time anyone opened the Arsenal on that entity, the magazines and
/// medical the author just removed would quietly come back. A strip verb that undoes itself on the
/// next panel open is not a strip verb. Emitting the key states "the author cleared this", which is
/// the marker the seed rule already respects, so Remove Everything sticks.
///
/// The wear vocabulary comes from [`ROWS`] rather than a second hand-written key list, so this
/// document and [`picks_to_loadout`]'s cannot drift apart.
#[must_use]
pub fn stripped_loadout() -> String {
    let mut wear = serde_json::Map::new();
    for row in ROWS.iter().filter(|r| r.weapon.is_none()) {
        wear.insert(row.key.to_string(), serde_json::Value::Null);
    }
    serde_json::json!({
        "version": 2,
        "wear": wear,
        "weapons": [],
        "cargo": [],
    })
    .to_string()
}

/// Plan a Remove Everything over `targets`. No gate: the stripped document is the one document that
/// cannot fail [`loadout_rule_refusals`] — no picks means no compat edge to violate, no attachment
/// to strand and no cargo to overflow — so running the rules over it would be a check whose answer
/// is a constant. (Pinned as behaviour by `tests::t699::the_stripped_document_passes_every_rule`,
/// not asserted in a comment, because "constant" is a claim about the rules module and the rules
/// module can change.)
#[must_use]
pub fn plan_remove(targets: &[String]) -> Vec<LoadoutWrite> {
    targets
        .iter()
        .map(|id| LoadoutWrite {
            target_id: id.clone(),
            source_id: None,
            loadout_json: Some(stripped_loadout()),
        })
        .collect()
}

/// Push a plan into the document, and **return how many writes actually reached it**.
///
/// ⚠️ **THE UNDO ARITHMETIC, STATED HONESTLY (T-732).** Every `commit` here is one
/// `editor_ops::set_loadout` → one `MissionDocCore::update_slot_loadout` → one Yrs transaction, and
/// the store runs with `capture_timeout_millis = 0`, which makes **every transaction its own undo
/// step**. So an Apply over N entities costs **N** Ctrl+Z presses, not one. That is not a choice
/// this slice made; it is the absence of an atomic multi-entity loadout write in the core, filed as
/// **T-732** and hit before by wave 111's T-645 and by the position lane's `commit_positions`. The
/// core *does* have per-entity one-txn batches where somebody built one — `set_slots_editor_hidden`
/// is exactly that shape for `editorHidden` — but there is none for `loadout`, and `store.rs` is not
/// this slice's to change. T-686 got one undo step for free because an import is one entity; Apply
/// is not, and pretending otherwise would be the lie this comment exists to refuse.
///
/// What this function does about it: it **counts sink acknowledgements**, and
/// [`apply_receipt`] reports that number to the author rather than the number that was planned. A
/// commit path that silently dropped writes would produce a receipt that says so, instead of a
/// receipt that says "applied 6" over 4 landed documents. The sink returns `bool` (T-770) — the
/// production path is `MissionDocCore::update_slot_loadout`, which returns `false` on an unknown
/// id — so the WARNING arm is reachable when the document refused a write, not merely when the
/// loop itself was skipped. `commit` is a parameter and not a direct `set_loadout` call for the
/// same reason: it makes the arithmetic testable natively, where `editor_ops` (a wasm32-only
/// module) cannot be reached at all. This is also the single seam a future T-732 fix touches —
/// when a batch API exists, this loop becomes one call, the returned count becomes 1, and the
/// receipt starts telling the truth about *that* without another edit.
pub fn commit_writes(
    writes: &[LoadoutWrite],
    mut commit: impl FnMut(&str, Option<String>) -> bool,
) -> usize {
    let mut done = 0usize;
    for w in writes {
        if commit(&w.target_id, w.loadout_json.clone()) {
            done += 1;
        }
    }
    done
}

/// **T-779 — the single-write sibling of [`commit_writes`]: the history tail fires only if the
/// document acknowledged the write.**
///
/// T-770 gave `MissionDocCore::update_slot_loadout` a `bool` return and taught the *batch* path to
/// count it. The *single* path — `editor_ops::set_loadout`, the one every Arsenal pick and every
/// cargo edit goes through — kept a hardcoded `true` directly under the mutator call and threw the
/// answer away. The consequence was not cosmetic: `mission_history::after_local_edit` fired whenever
/// the ops context and the document merely existed, so a pick against a slot id the mission no
/// longer held still dirtied the mission and minted an undo step over a document that had not
/// changed. Ctrl+Z then had a step in it that restored nothing.
///
/// This exists as a **parameterised** function in `arsenal` rather than as an `if` inside
/// `editor_ops` for the same reason [`commit_writes`] does: `editor_ops` is `cfg(target_arch =
/// "wasm32")` from its first line and cannot be reached by a native test at all, so a gate written
/// there is provable only by reading source. Here the gate can be driven — a sink that refuses, the
/// production shape for an unknown id, must produce zero tails — and `tests::t779` does exactly
/// that. The `bool` comes back out so the caller can tell the operator; silence over a refused
/// write is the defect this whole line of tickets exists to remove.
pub fn commit_one_write(commit: impl FnOnce() -> bool, tail: impl FnOnce()) -> bool {
    let took = commit();
    if took {
        tail();
    }
    took
}

/// The Copy receipt. Counts the bare sources out loud: buffering an entity with no loadout is legal
/// and useful, but an author who selected forty soldiers and copied forty bare kits should be told
/// before they Apply, not after.
#[must_use]
pub fn copy_receipt(buffer: &[BufferedLoadout]) -> String {
    let bare = buffer.iter().filter(|b| b.loadout_json.is_none()).count();
    let mut line = format!(
        "Copied {} loadout(s) to the buffer. Apply writes one of them to each selected entity, picked at random.",
        buffer.len()
    );
    if bare > 0 {
        line.push_str(&format!(
            " {bare} of them carry no loadout at all — applying one of those leaves that entity bare."
        ));
    }
    line
}

/// The Apply receipt — built from `commits` (what the document took), never from the plan length.
/// It states the undo cost in the same breath, because N-presses-to-undo is a thing the author is
/// about to need and the only place they can learn it is here.
#[must_use]
pub fn apply_receipt(planned: usize, buffer_len: usize, commits: usize) -> String {
    let mut line = format!(
        "Applied {commits} loadout(s), drawn at random from a {buffer_len}-loadout buffer. \
         That is {commits} undo step(s) — one per entity, because there is no atomic multi-entity \
         loadout write (T-732), so Ctrl+Z {commits} times to put it back."
    );
    if commits != planned {
        line.push_str(&format!(
            " WARNING: {planned} write(s) were planned and {commits} reached the document."
        ));
    }
    line
}

/// The Remove Everything receipt. Says the anti-reseed half out loud — an author who strips cargo
/// needs to know it will not silently return, and that promise is the whole reason
/// [`stripped_loadout`] emits `cargo: []`.
#[must_use]
pub fn remove_receipt(planned: usize, commits: usize) -> String {
    let mut line = format!(
        "Stripped {commits} entity(ies) — every wear row, weapon and cargo row cleared, and cargo \
         stays cleared (no default re-seed). That is {commits} undo step(s), one per entity (T-732)."
    );
    if commits != planned {
        line.push_str(&format!(
            " WARNING: {planned} write(s) were planned and {commits} reached the document."
        ));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn names() -> HashMap<String, String> {
        [
            ("res://rifle_m16", "M16A2"),
            ("res://helmet_pasgt", "PASGT Helmet"),
            ("res://acog", "ACOG"),
            ("res://mag_stanag", "STANAG 30rd"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
    }

    fn picks(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// A ready compat feed carrying only `attachment_on_weapon` edges.
    fn attachment_feed(edges: &[(&str, &str)]) -> CompatFeed {
        let rows: Vec<crate::core::dto::RegistryCompatEdge> = edges
            .iter()
            .enumerate()
            .map(|(i, (from, to))| crate::core::dto::RegistryCompatEdge {
                id: i.to_string(),
                modpack_id: "m".into(),
                from_node: (*from).into(),
                to_node: (*to).into(),
                edge_type: ATTACHMENT_EDGE.into(),
                evidence: String::new(),
                qty: 1,
                created_at: String::new(),
                updated_at: String::new(),
            })
            .collect();
        CompatFeed {
            status: rules::CompatStatus::Ready,
            graph: rules::CompatGraph::from_edges(&rows),
        }
    }

    #[test]
    fn all_empty_picks_clear_the_field() {
        assert!(picks_to_loadout(&HashMap::new(), &names(), None).is_none());
        // An unknown (non-row) key alone still counts as empty — no row is set.
        assert!(picks_to_loadout(&picks(&[("optic", "res://acog")]), &names(), None).is_none());
        // A present-but-empty cargo key alone is still "all empty" → clear.
        assert!(picks_to_loadout(&HashMap::new(), &names(), Some(&[])).is_none());
        // Non-empty cargo alone keeps the loadout alive (wear all-null shell).
        let rows = vec![rules::CargoRow {
            container: "vest".into(),
            item: "res://mag_stanag".into(),
            qty: 3,
        }];
        let lo = picks_to_loadout(&HashMap::new(), &names(), Some(&rows)).expect("cargo-only");
        let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
        assert_eq!(v["cargo"][0]["container"], "vest");
        assert_eq!(v["cargo"][0]["qty"], 3);
        assert_eq!(v["wear"].as_object().unwrap().len(), 8);
    }

    #[test]
    fn cargo_key_presence_follows_user_state() {
        let p = picks(&[("primary", "res://rifle_m16")]);
        // Untouched cargo (None) → no key: a later seed may still fire.
        let lo = picks_to_loadout(&p, &names(), None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
        assert!(v.get("cargo").is_none());
        // Touched-but-cleared (Some empty) → key persists as [] and round-trips as
        // present (the anti-reseed marker).
        let lo = picks_to_loadout(&p, &names(), Some(&[])).unwrap();
        let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
        assert_eq!(v["cargo"], serde_json::json!([]));
        let (rows, present) = rules::cargo_from_loadout(Some(&lo));
        assert!(present && rows.is_empty());
        // Seeded rows survive a pick-edit persist verbatim.
        let seeded = rules::seed_cargo(
            Some(&picks_to_loadout(&p, &names(), None).unwrap()),
            &[rules::CargoRow {
                container: "pants".into(),
                item: "res://mag_stanag".into(),
                qty: 2,
            }],
        )
        .expect("seeds");
        let (rows, present) = rules::cargo_from_loadout(Some(&seeded));
        assert!(present);
        let resaved = picks_to_loadout(&loadout_to_picks(Some(&seeded)), &names(), Some(&rows))
            .expect("resave");
        let v: serde_json::Value = serde_json::from_str(&resaved).unwrap();
        assert_eq!(v["cargo"][0]["item"], "res://mag_stanag");
        assert_eq!(v["cargo"][0]["qty"], 2);
    }

    #[test]
    fn canonical_v2_shape_matches_react() {
        // primary weapon + a wear row → the exact `picksToLoadout` superset.
        let lo = picks_to_loadout(
            &picks(&[
                ("primary", "res://rifle_m16"),
                ("headCover", "res://helmet_pasgt"),
            ]),
            &names(),
            None,
        )
        .expect("non-empty");
        let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
        assert_eq!(v["version"], 2);
        // weapons[0]: slotIndex 0 / slotType primary / attachments [] / null optic+magazine.
        let w0 = &v["weapons"][0];
        assert_eq!(w0["slotIndex"], 0);
        assert_eq!(w0["slotType"], "primary");
        assert_eq!(w0["weapon"], "res://rifle_m16");
        assert!(w0["optic"].is_null());
        assert!(w0["magazine"].is_null());
        assert_eq!(w0["attachments"], serde_json::json!([]));
        // wear carries EVERY wear key (present-or-null), headCover set.
        assert_eq!(v["wear"]["headCover"], "res://helmet_pasgt");
        assert!(v["wear"]["jacket"].is_null());
        assert_eq!(v["wear"].as_object().unwrap().len(), 8);
        // summary = display names of primary/optic/magazine/launcher.
        assert_eq!(v["summary"], "M16A2");
    }

    #[test]
    fn round_trips_through_the_doc_field() {
        let p = picks(&[
            ("primary", "res://rifle_m16"),
            ("launcher", "res://rpg"),
            ("headCover", "res://helmet_pasgt"),
            ("vest", "res://vest_m88"),
        ]);
        let lo = picks_to_loadout(&p, &names(), None).unwrap();
        let back = loadout_to_picks(Some(&lo));
        for k in ["primary", "launcher", "headCover", "vest"] {
            assert_eq!(back.get(k), p.get(k), "key {k} lost on round-trip");
        }
    }

    #[test]
    fn attachments_ride_their_own_weapon_and_round_trip() {
        // T-197 — `attachments[]` is a per-weapon field, not a primary-only sub-slot: a set on the
        // handgun must land on the handgun's `weapons[]` entry and come back on the handgun.
        let mut p = picks(&[("primary", "res://rifle_m16"), ("handgun", "res://m9")]);
        p.insert(
            attachments_key("primary"),
            pack_attachments(&["res://handguard".into(), "res://stock".into()]),
        );
        p.insert(
            attachments_key("handgun"),
            pack_attachments(&["res://supp".into()]),
        );
        let lo = picks_to_loadout(&p, &names(), None).expect("non-empty");
        let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
        // `weapons[]` is ROWS order — primary (slotIndex 0), then handgun (slotIndex 2).
        assert_eq!(v["weapons"][0]["slotIndex"], 0);
        assert_eq!(
            v["weapons"][0]["attachments"],
            serde_json::json!(["res://handguard", "res://stock"])
        );
        assert_eq!(v["weapons"][1]["slotIndex"], 2);
        assert_eq!(
            v["weapons"][1]["attachments"],
            serde_json::json!(["res://supp"])
        );
        let back = loadout_to_picks(Some(&lo));
        assert_eq!(
            attachments_of(&back, "primary"),
            ["res://handguard", "res://stock"]
        );
        assert_eq!(attachments_of(&back, "handgun"), ["res://supp"]);
        assert!(attachments_of(&back, "launcher").is_empty());
    }

    #[test]
    fn an_empty_attachment_set_keeps_the_pre_t197_byte_shape() {
        // Primary keeps emitting `attachments: []` (what every persisted loadout already carries);
        // the other three weapon rows still emit no key at all. A mission with no attachments must
        // serialize byte-identically to its pre-T-197 self, or every save rewrites every slot.
        let p = picks(&[("primary", "res://rifle_m16"), ("launcher", "res://rpg")]);
        let lo = picks_to_loadout(&p, &names(), None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&lo).unwrap();
        assert_eq!(v["weapons"][0]["attachments"], serde_json::json!([]));
        assert!(v["weapons"][1].get("attachments").is_none());
        // A set on a weapon that is NOT picked never reaches the doc (it is flagged in the UI).
        let mut orphan = picks(&[("primary", "res://rifle_m16")]);
        orphan.insert(
            attachments_key("handgun"),
            pack_attachments(&["res://supp".into()]),
        );
        let v: serde_json::Value =
            serde_json::from_str(&picks_to_loadout(&orphan, &names(), None).unwrap()).unwrap();
        assert_eq!(v["weapons"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn the_packed_separator_survives_the_resource_name_charset() {
        // The pack/split round-trip is only safe because `registry-compat.schema.json`'s
        // `resourceName` pattern admits no control character. Pin that with a node using every
        // other character the pattern allows.
        let a = "{0123456789ABCDEF}Prefabs/Weapons/A-b_c.1 (x)'y.et";
        let b = "{FEDCBA9876543210}Prefabs/B.et";
        let mut m = HashMap::new();
        m.insert(
            attachments_key("primary"),
            pack_attachments(&[a.to_string(), b.to_string()]),
        );
        assert_eq!(attachments_of(&m, "primary"), [a, b]);
        // An emptied set packs to "", which the pick path reads as "remove the key".
        assert_eq!(pack_attachments(&[]), "");
        assert!(attachments_of(&HashMap::new(), "primary").is_empty());
    }

    #[test]
    fn stranded_attachments_are_flagged_and_an_outage_never_condemns_one() {
        let feed = attachment_feed(&[("res://handguard", "res://rifle_m16")]);
        let mut p = picks(&[("primary", "res://rifle_m16")]);
        p.insert(attachments_key("primary"), "res://handguard".into());
        assert!(attachment_errors(&p, &feed).is_empty());
        // Swap the rifle: the handguard now hangs off a host that does not accept it.
        p.insert("primary".into(), "res://rifle_vz58".into());
        let errs = attachment_errors(&p, &feed);
        assert_eq!(errs.len(), 1);
        // Keyed on the row the author must change — and since that key can only ever say
        // "primary", the message is the only place the offending attachment can be named.
        assert_eq!(errs[0].key, "primary");
        assert!(errs[0].message.contains("not compatible"));
        assert!(errs[0].message.contains("res://handguard"), "{errs:?}");
        // No weapon at all → the wording `validate_loadout` gives a hostless optic, likewise named.
        p.remove("primary");
        let hostless = &attachment_errors(&p, &feed)[0].message;
        assert!(hostless.contains("requires a Primary"), "{hostless}");
        assert!(hostless.contains("res://handguard"), "{hostless}");
        // An outage must not fail a loadout we never got compat data for.
        let dead = CompatFeed {
            status: rules::CompatStatus::Unavailable,
            ..feed
        };
        assert!(attachment_errors(&p, &dead).is_empty());
    }

    #[test]
    fn optic_magazine_survive_a_dumb_forge_resave() {
        // A Smart-Forge loadout (optic+magazine on weapons[0]) opened + re-saved from the dumb tab
        // must keep the sticky sub-fields — the regression this pass-through guards.
        let smart = serde_json::json!({
            "version": 2,
            "wear": { "headCover": null, "jacket": null, "pants": null, "boots": null,
                      "vest": null, "armoredVest": null, "backpack": null, "handwear": null },
            "weapons": [ { "slotIndex": 0, "slotType": "primary", "weapon": "res://rifle_m16",
                           "optic": "res://acog", "magazine": "res://mag_stanag", "attachments": [] } ],
        })
        .to_string();
        let back = loadout_to_picks(Some(&smart));
        assert_eq!(back.get("optic").map(String::as_str), Some("res://acog"));
        assert_eq!(
            back.get("magazine").map(String::as_str),
            Some("res://mag_stanag")
        );
        let resaved = picks_to_loadout(&back, &names(), None).unwrap();
        let v: serde_json::Value = serde_json::from_str(&resaved).unwrap();
        assert_eq!(v["weapons"][0]["optic"], "res://acog");
        assert_eq!(v["weapons"][0]["magazine"], "res://mag_stanag");
        // summary resolves display names of primary · optic · magazine (launcher absent).
        assert_eq!(v["summary"], "M16A2 · ACOG · STANAG 30rd");
    }

    /* ─────────────── T-199 — the exported FILE vs `loadout-export.schema.json` ─────────────── */

    /// The repo's real `loadout-export.schema.json`, read at test time.
    ///
    /// Deliberately the FILE and not a transcription of it: the bug this ticket fixes was a writer
    /// checked against somebody's reading of the schema, so a test that embeds its own copy of the
    /// rules would reproduce the same failure mode one layer down. Reading it here means the day
    /// the schema gains a required key or closes another object, this test goes red.
    fn export_schema() -> serde_json::Value {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../packages/tbd-schema/schema/loadout-export.schema.json");
        serde_json::from_str(&std::fs::read_to_string(&p).expect("read loadout-export.schema.json"))
            .expect("parse loadout-export.schema.json")
    }

    /// Resolve a local `{"$ref": "#/$defs/x"}` against the root schema (one hop is all this
    /// schema uses).
    fn deref<'a>(
        root: &'a serde_json::Value,
        node: &'a serde_json::Value,
    ) -> &'a serde_json::Value {
        match node.get("$ref").and_then(|r| r.as_str()) {
            Some(r) => r
                .trim_start_matches("#/")
                .split('/')
                .fold(root, |acc, seg| &acc[seg]),
            None => node,
        }
    }

    /// Assert `doc` satisfies `sub`'s `required` list and its `additionalProperties: false`
    /// closure, recursing into `properties` that are objects with their own contract.
    fn assert_object_contract(
        root: &serde_json::Value,
        sub: &serde_json::Value,
        doc: &serde_json::Value,
        label: &str,
    ) {
        let obj = doc
            .as_object()
            .unwrap_or_else(|| panic!("{label}: not an object"));
        for req in sub["required"].as_array().into_iter().flatten() {
            let k = req.as_str().unwrap();
            assert!(obj.contains_key(k), "{label}: missing required key `{k}`");
        }
        if sub["additionalProperties"] == serde_json::Value::Bool(false) {
            let props = sub["properties"].as_object();
            for k in obj.keys() {
                assert!(
                    props.is_some_and(|p| p.contains_key(k)),
                    "{label}: key `{k}` is not in the schema and additionalProperties is false"
                );
            }
        }
        for (k, spec) in sub["properties"].as_object().into_iter().flatten() {
            let Some(v) = obj.get(k) else { continue };
            // `const` is how this schema pins the version discriminator.
            if let Some(c) = spec.get("const") {
                assert_eq!(v, c, "{label}: `{k}` must be {c}");
            }
            // Recurse into every nested object that carries its own contract — `gear` reaches
            // `#/$defs/gear` this way, through the schema's own pointer rather than a path we
            // guessed.
            let spec = deref(root, spec);
            if spec.get("required").is_some() && v.is_object() {
                assert_object_contract(root, spec, v, &format!("{label}/{k}"));
            }
        }
    }

    /// The full-kit picks a real author produces: all four weapon slots, every wear row, a
    /// sticky optic/magazine and an attachment set.
    fn full_picks() -> HashMap<String, String> {
        let mut p = picks(&[
            ("primary", "res://rifle_m16"),
            ("launcher", "res://m72"),
            ("handgun", "res://m9"),
            ("throwable", "res://m67"),
            ("optic", "res://acog"),
            ("magazine", "res://mag_stanag"),
            ("headCover", "res://helmet_pasgt"),
            ("jacket", "res://bdu_blouse"),
            ("pants", "res://bdu_pants"),
            ("boots", "res://jungle_boots"),
            ("vest", "res://chest_rig"),
            ("armoredVest", "res://pasgt_vest"),
            ("backpack", "res://alice_pack"),
            ("handwear", "res://gloves"),
        ]);
        p.insert(
            attachments_key("primary"),
            pack_attachments(&["res://supp".into(), "res://grip".into()]),
        );
        p
    }

    #[test]
    fn exported_file_satisfies_the_v2_branch_of_the_real_schema() {
        let rows = vec![rules::CargoRow {
            container: "vest".into(),
            item: "res://mag_stanag".into(),
            qty: 6,
        }];
        // Both ends of the range a real author hits: a fully kitted soldier, and the empty
        // Arsenal that used to fall through to a hand-written literal.
        let docs = [
            (
                "full kit",
                picks_to_export(&full_picks(), &rows, "00000000-0000-4000-a000-000000000001"),
            ),
            ("empty arsenal", picks_to_export(&HashMap::new(), &[], "")),
        ];
        let schema = export_schema();
        let v2 = schema["oneOf"]
            .as_array()
            .expect("oneOf")
            .iter()
            .find(|b| b["properties"]["loadoutVersion"]["const"] == "2")
            .cloned()
            .expect("a v2 branch");

        for (label, raw) in &docs {
            // The exact bytes the download button writes. `cargo test -p website-frontend
            // exported_file -- --nocapture` re-dumps them for an external schema run.
            println!("─── {label} ───\n{raw}");
            let doc: serde_json::Value = serde_json::from_str(raw).expect("valid JSON");
            assert_object_contract(&schema, &v2, &doc, label);

            // wear keys must match the schema's own pattern (open map, mod-added areas allowed).
            for k in doc["wear"].as_object().unwrap().keys() {
                let mut c = k.chars();
                assert!(
                    c.next().is_some_and(|f| f.is_ascii_alphabetic())
                        && c.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
                        && k.len() <= 64,
                    "{label}: wear key `{k}` fails the schema pattern"
                );
                assert!(
                    doc["wear"][k].is_string() || doc["wear"][k].is_null(),
                    "{label}: wear/{k} must be a ResourceName or null"
                );
            }
            // Array items: each element against the schema's own `items` subschema. `gear` needs
            // no line here — `assert_object_contract` already recursed into it.
            let weapon_def = deref(&schema, &v2["properties"]["weapons"]["items"]);
            for w in doc["weapons"].as_array().unwrap() {
                assert_object_contract(&schema, weapon_def, w, &format!("{label}/weapons[]"));
                assert!(!w["weapon"].as_str().unwrap().is_empty()); // minLength 1
                assert!(w["slotIndex"].as_i64().unwrap() >= 0); // minimum 0
            }
            let cargo_def = deref(&schema, &v2["properties"]["cargo"]["items"]);
            let containers = deref(&schema, &cargo_def["properties"]["container"])["enum"]
                .as_array()
                .expect("cargoContainer enum")
                .clone();
            for row in doc["cargo"].as_array().unwrap() {
                assert_object_contract(&schema, cargo_def, row, &format!("{label}/cargo[]"));
                assert!(
                    containers.contains(&row["container"]),
                    "{label}: cargo container `{}` is outside the closed vocabulary",
                    row["container"]
                );
                assert!(row["qty"].as_i64().unwrap() >= 1); // minimum 1
                assert!(!row["item"].as_str().unwrap().is_empty()); // minLength 1
            }
        }
    }

    #[test]
    fn export_carries_all_four_weapon_slots_and_the_locked_gear_derivation() {
        let raw = picks_to_export(&full_picks(), &[], "mp");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        // T-182's four slots, each naming its engine slot — the pairs `mod_slot_loadout` matches.
        let slots: Vec<(i64, &str, &str)> = v["weapons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| {
                (
                    w["slotIndex"].as_i64().unwrap(),
                    w["slotType"].as_str().unwrap(),
                    w["weapon"].as_str().unwrap(),
                )
            })
            .collect();
        assert_eq!(
            slots,
            [
                (0, "primary", "res://rifle_m16"),
                (1, "primary", "res://m72"),
                (2, "secondary", "res://m9"),
                (3, "grenade", "res://m67"),
            ]
        );
        assert_eq!(
            v["weapons"][0]["attachments"],
            serde_json::json!(["res://supp", "res://grip"])
        );
        // Derived gear: jacket→uniform, armoredVest beats vest, headCover→helmet, primary triple.
        assert_eq!(v["gear"]["uniform"], "res://bdu_blouse");
        assert_eq!(v["gear"]["vest"], "res://pasgt_vest");
        assert_eq!(v["gear"]["helmet"], "res://helmet_pasgt");
        assert_eq!(v["gear"]["primary"], "res://rifle_m16");
        assert_eq!(v["gear"]["optic"], "res://acog");
        assert_eq!(v["gear"]["magazine"], "res://mag_stanag");
        // vest falls back when no armoredVest is worn (the compiler's own single-vest rule).
        let mut p = full_picks();
        p.remove("armoredVest");
        let v: serde_json::Value = serde_json::from_str(&picks_to_export(&p, &[], "mp")).unwrap();
        assert_eq!(v["gear"]["vest"], "res://chest_rig");
    }

    #[test]
    fn an_empty_arsenal_still_exports_a_conforming_document() {
        // `picks_to_loadout` returns None here (clear the doc field) — a FILE has no such option,
        // and the literal the button used to fall back to was itself non-conforming.
        let raw = picks_to_export(&HashMap::new(), &[], "mp");
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["loadoutVersion"], "2");
        assert_eq!(v["modpackId"], "mp");
        assert_eq!(v["weapons"], serde_json::json!([]));
        assert_eq!(v["cargo"], serde_json::json!([]));
        assert_eq!(v["wear"].as_object().unwrap().len(), 8);
        assert!(v["wear"].as_object().unwrap().values().all(|x| x.is_null()));
        // The four required gear keys exist and are honestly null — not omitted, not "".
        for k in ["primary", "uniform", "vest", "helmet"] {
            assert!(v["gear"][k].is_null(), "gear/{k}");
        }
        // Nothing from the doc-field shape leaks into the file.
        for k in ["version", "summary", "equipment"] {
            assert!(v.get(k).is_none(), "`{k}` must not be in the export");
        }
        // A sticky optic with no rifle describes nothing — the gear block says so.
        let v: serde_json::Value = serde_json::from_str(&picks_to_export(
            &picks(&[("optic", "res://acog")]),
            &[],
            "mp",
        ))
        .unwrap();
        assert!(v["gear"]["optic"].is_null());
    }

    #[test]
    fn a_separator_bearing_attachment_never_reaches_the_export() {
        // `loadout-export.schema.json` types `attachments` items as unconstrained strings, so a
        // hand-edited document may legally carry U+001F inside one. Packed, it would unpack as two
        // picks and the export would then emit an attachment nobody chose.
        let hostile = format!("res://supp{ATTACHMENT_SEP}res://invented");
        let doc = serde_json::json!({
            "version": 2,
            "wear": {},
            "weapons": [ { "slotIndex": 0, "slotType": "primary", "weapon": "res://rifle_m16",
                           "attachments": [hostile, "res://grip"] } ],
        })
        .to_string();
        let back = loadout_to_picks(Some(&doc));
        assert_eq!(attachments_of(&back, "primary"), ["res://grip"]);
        let v: serde_json::Value =
            serde_json::from_str(&picks_to_export(&back, &[], "mp")).unwrap();
        assert_eq!(
            v["weapons"][0]["attachments"],
            serde_json::json!(["res://grip"])
        );
        assert!(v["weapons"][0]["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| !a.as_str().unwrap().contains(ATTACHMENT_SEP)));
    }

    #[test]
    fn the_modpack_id_comes_from_the_catalog_the_picks_were_made_against() {
        assert_eq!(export_modpack_id(&[]), "");
        let it = crate::core::dto::RegistryItem {
            id: "1".into(),
            modpack_id: "00000000-0000-4000-a000-000000000001".into(),
            resource_name: "res://rifle_m16".into(),
            display_name: "M16A2".into(),
            category: "WEAPONS".into(),
            icon_url: None,
            kind: "gear_primary".into(),
            r#abstract: None,
            arsenal_type: None,
            weight_kg: None,
            volume_cm3: None,
            max_weight_kg: None,
            max_volume_cm3: None,
            cargo_grid_w: None,
            cargo_grid_h: None,
            addon: None,
            variant_of: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        };
        assert_eq!(
            export_modpack_id(std::slice::from_ref(&it)),
            "00000000-0000-4000-a000-000000000001"
        );
    }

    /* ─────────── T-240 — the export button refuses over-capacity cargo ─────────── */

    fn gear(rn: &str, name: &str, kind: &str) -> RegistryItem {
        RegistryItem {
            id: String::new(),
            modpack_id: "mp".into(),
            resource_name: rn.into(),
            display_name: name.into(),
            category: String::new(),
            icon_url: None,
            kind: kind.into(),
            r#abstract: None,
            arsenal_type: None,
            weight_kg: None,
            volume_cm3: None,
            max_weight_kg: None,
            max_volume_cm3: None,
            cargo_grid_w: None,
            cargo_grid_h: None,
            addon: None,
            variant_of: None,
            sort_order: 0,
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    fn row(container: &str, item: &str, qty: i64) -> rules::CargoRow {
        rules::CargoRow {
            container: container.into(),
            item: item.into(),
            qty,
        }
    }

    /// A 0.5 kg / 60 cm³ magazine and a chest rig catalogued at 5 kg / 200 cm³.
    fn capacity_catalog() -> Vec<RegistryItem> {
        let mut mag = gear("res://mag_stanag", "STANAG 30rd", "magazine");
        mag.weight_kg = Some(0.5);
        mag.volume_cm3 = Some(60.0);
        let mut vest = gear("res://chest_rig", "Chest Rig", "gear_vest");
        vest.max_weight_kg = Some(5.0);
        vest.max_volume_cm3 = Some(200.0);
        vec![mag, vest]
    }

    #[test]
    fn over_capacity_cargo_cannot_be_exported_and_legitimate_cargo_still_can() {
        let items = capacity_catalog();
        let p = picks(&[("vest", "res://chest_rig")]);

        // 4 × 60 = 240 cm³ into a 200 cm³ rig. The export is REFUSED, and a refusal carries
        // reasons instead of bytes — there is no document to half-download.
        let over = vec![row("vest", "res://mag_stanag", 4)];
        let reasons = try_export(&p, &over, &items, "mp")
            .expect_err("over-capacity cargo must not reach a file");
        assert_eq!(reasons.len(), 1);
        assert_eq!(reasons[0].key, "vest");
        assert!(
            reasons[0].message.contains("240 / 200 cm³"),
            "{}",
            reasons[0].message
        );
        assert!(
            reasons[0].message.ends_with(rules::CARGO_CAPACITY_CAVEAT),
            "the refusal must carry its own estimate caveat: {}",
            reasons[0].message
        );

        // The same author, one magazine lighter: 180 ≤ 200. They still get their file, and it
        // is the real document — the gate refuses or gets out of the way, it never degrades.
        let ok = vec![row("vest", "res://mag_stanag", 3)];
        let json = try_export(&p, &ok, &items, "mp").expect("legitimate cargo must still export");
        assert_eq!(
            json,
            picks_to_export(&p, &ok, "mp"),
            "an accepted export must be byte-identical to the unguarded one"
        );
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["loadoutVersion"], "2");
        assert_eq!(v["cargo"][0]["qty"], 3);
    }

    #[test]
    fn the_export_gate_never_refuses_on_capacity_it_does_not_have() {
        // An uncatalogued garment, no garment at all, and a bare Arsenal must all still export.
        // A gate that refuses everything is indistinguishable from a broken button.
        let mut items = capacity_catalog();
        items.push(gear("res://unknown_rig", "Uncatalogued Rig", "gear_vest"));
        let heavy = vec![row("vest", "res://mag_stanag", 40)];

        for (label, p) in [
            (
                "garment with no catalogued capacity",
                picks(&[("vest", "res://unknown_rig")]),
            ),
            ("no garment worn", picks(&[])),
            (
                "garment the catalog does not know",
                picks(&[("vest", "res://ghost")]),
            ),
        ] {
            assert!(
                try_export(&p, &heavy, &items, "mp").is_ok(),
                "must still export — {label}"
            );
        }
        // And the pre-T-240 baseline: a full loadout over a catalog with no capacity columns
        // at all exports exactly as it did before this ticket.
        assert!(try_export(&full_picks(), &[], &[], "mp").is_ok());
    }

    #[test]
    fn the_verdict_counts_capacity_beside_compat_and_attachment_faults() {
        let items = capacity_catalog();
        let idx = index_by_name(&items);
        // A ready feed with no edges → the packed attachment on the primary is stranded.
        let feed = attachment_feed(&[]);
        let mut p = picks(&[("vest", "res://chest_rig"), ("primary", "res://rifle_m16")]);
        p.insert(
            attachments_key("primary"),
            pack_attachments(&["res://supp".into()]),
        );

        let kit = kit(&[]);
        let faults = loadout_faults(
            &p,
            &[row("vest", "res://mag_stanag", 4)],
            &feed,
            &idx,
            Some(&kit),
        );
        assert_eq!(
            faults.len(),
            2,
            "one stranded attachment + one over-capacity vest"
        );
        let keys: Vec<&str> = faults.iter().map(|e| e.key).collect();
        assert!(keys.contains(&"primary"), "{keys:?}");
        assert!(keys.contains(&"vest"), "{keys:?}");

        // Empty the cargo and the capacity fault goes with it — the attachment one stays.
        let faults = loadout_faults(&p, &[], &feed, &idx, Some(&kit));
        assert_eq!(faults.len(), 1);
        assert_eq!(faults[0].key, "primary");
    }

    /* ═════════ T-504 — cargo with nowhere known to go ═════════ */

    /// The kit-default vouching set, as [`kit_default_items`] would build it.
    fn kit(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn undeliverable_cargo_fails_the_verdict_but_never_the_export() {
        let items = capacity_catalog();
        let idx = index_by_name(&items);
        let feed = attachment_feed(&[]);
        // Three magazines into a vest: no vest picked, and a kit not catalogued as carrying them.
        // 180 cm³ is comfortably inside any rig, so capacity has nothing to say — and before T-504
        // neither did anything else: the badge read "Loadout valid" over cargo it had never checked
        // was deliverable.
        let bare = picks(&[]);
        let rows = vec![row("vest", "res://mag_stanag", 3)];
        let empty_kit = kit(&[]);
        assert!(
            rules::cargo_capacity_errors(&bare, &rows, &idx).is_empty(),
            "capacity must stay out of this — that is the point"
        );

        let faults = loadout_faults(&bare, &rows, &feed, &idx, Some(&empty_kit));
        assert_eq!(faults.len(), 1, "the verdict must count it: {faults:?}");
        assert_eq!(faults[0].key, "vest", "keyed on the row that fixes it");
        assert!(
            faults[0].message.contains("nowhere known to go"),
            "{faults:?}"
        );
        assert!(
            faults[0].message.ends_with(rules::CARGO_UNWORN_CAVEAT),
            "the warning must carry its own kit-prefab caveat: {faults:?}"
        );

        // …and it must NOT reach the export gate. The kit prefab this editor cannot see may wear
        // the vest itself, so refusing would block a loadout that delivers perfectly.
        assert!(
            try_export(&bare, &rows, &items, "mp").is_ok(),
            "a warning must never become a refusal"
        );

        // Pick a vest and the fault goes; the rule reads picks, not the registry, so an
        // uncatalogued rig satisfies it exactly as well as a catalogued one.
        for rn in ["res://chest_rig", "res://ghost_rig"] {
            let worn = picks(&[("vest", rn)]);
            assert!(
                loadout_faults(&worn, &rows, &feed, &idx, Some(&empty_kit)).is_empty(),
                "a worn {rn} must clear it"
            );
        }

        // A kit catalogued as carrying that magazine vouches for the container — this is the seeded
        // path, and faulting it would put an issue on essentially every untouched slot.
        assert!(
            loadout_faults(&bare, &rows, &feed, &idx, Some(&kit(&["res://mag_stanag"]))).is_empty(),
            "the kit's own default cargo must never fault"
        );
    }

    #[test]
    fn the_kit_evidence_comes_off_the_live_compat_feed() {
        // `kit_default_items` is the seam between the UI and the pure rule, so it gets its own
        // test: the vouching set must come from the character's `character_default_cargo` edges,
        // and must answer `None` — "no evidence", the silent case — whenever it cannot.
        let edges: Vec<crate::core::dto::RegistryCompatEdge> =
            ["res://mag_stanag", "res://bandage"]
                .iter()
                .enumerate()
                .map(|(i, item)| crate::core::dto::RegistryCompatEdge {
                    id: i.to_string(),
                    modpack_id: "mp".into(),
                    from_node: (*item).into(),
                    to_node: "kit:us_rifleman".into(),
                    edge_type: rules::CHARACTER_DEFAULT_CARGO_EDGE.into(),
                    evidence: "TargetStorage=Vest/Mags".into(),
                    qty: 1,
                    created_at: String::new(),
                    updated_at: String::new(),
                })
                .collect();
        let ready = CompatFeed {
            status: rules::CompatStatus::Ready,
            graph: rules::CompatGraph::from_edges(&edges),
        };

        let found = kit_default_items(&ready, Some("kit:us_rifleman")).expect("ready + assetId");
        assert!(found.contains("res://mag_stanag"), "{found:?}");
        assert!(found.contains("res://bandage"), "{found:?}");
        // A character with no edges is real evidence (an empty set), not an absence of it.
        assert_eq!(
            kit_default_items(&ready, Some("kit:unknown")),
            Some(HashSet::new())
        );
        // No assetId → no key to look up → no evidence.
        assert_eq!(kit_default_items(&ready, None), None);
        // Feed not ready → no evidence, so the rule stays silent instead of faulting every slot
        // in the window before the registry lands.
        for status in [
            rules::CompatStatus::Loading,
            rules::CompatStatus::Unavailable,
        ] {
            let pending = CompatFeed {
                status,
                graph: rules::CompatGraph::from_edges(&edges),
            };
            assert_eq!(kit_default_items(&pending, Some("kit:us_rifleman")), None);
        }
        // Native has no hosted document, so there is no assetId to read.
        assert_eq!(slot_asset_id("slot-1"), None);
    }

    /* ═══════════ T-686 — the import half of the round-trip ═══════════ */

    mod t686 {
        use super::*;

        /// A schema-valid v2 document, as the download button writes it.
        fn v2_file(p: &HashMap<String, String>, cargo: &[rules::CargoRow]) -> serde_json::Value {
            serde_json::from_str(&picks_to_export(p, cargo, "mp")).expect("export is JSON")
        }

        fn refuse(raw: &str) -> Vec<rules::RowError> {
            try_import(raw, &[], &CompatFeed::default())
                .expect_err("this document must not be applied")
        }

        /// **The claim in the ticket title.** Download a loadout, hand the file back, and the
        /// Arsenal is in the state it started in — picks, cargo and all. Nothing in between
        /// invents, drops or renames a value.
        #[test]
        fn the_round_trip_closes() {
            let rows = vec![
                row("vest", "res://mag_stanag", 3),
                row("backpack", "res://mag_stanag", 2),
            ];
            let raw = picks_to_export(&full_picks(), &rows, "mp");
            // A `Loading` feed: no compat data, so no edge validation — the round-trip claim is
            // about the serialization, and a feed we never received must not colour it.
            let back = try_import(&raw, &[], &CompatFeed::default()).expect("its own export");
            assert_eq!(
                back.picks,
                full_picks(),
                "picks must survive the round-trip"
            );
            assert_eq!(back.cargo, rows, "cargo must survive the round-trip");
            assert!(back.cargo_present);
            assert_eq!(back.loadout_version, "2");
            assert_eq!(back.modpack_id, "mp");
            // And the empty end of the range: a bare-soldier document is a legal import.
            let bare = picks_to_export(&HashMap::new(), &[], "");
            let back = try_import(&bare, &[], &CompatFeed::default()).expect("bare soldier");
            assert!(back.picks.is_empty() && back.cargo.is_empty());
        }

        /// The importer must enforce the file the repo ships, not a copy of it that can drift.
        #[test]
        fn the_compiled_in_schema_is_the_shipped_file() {
            let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../../packages/tbd-schema/schema/loadout-export.schema.json");
            let on_disk = std::fs::read_to_string(&p).expect("read the shipped schema");
            assert_eq!(
                rules::LOADOUT_EXPORT_SCHEMA_JSON,
                on_disk,
                "the importer must validate against the shipped schema, byte for byte"
            );
            // And it is the v2 producer's own schema — the one `picks_to_export` writes against.
            let schema: serde_json::Value = serde_json::from_str(&on_disk).unwrap();
            assert_eq!(
                schema["$id"],
                "https://schema.tbdevent.eu/loadout-export/v2.json"
            );
        }

        /// **The refusal contract.** Every one of these is a document the OFCRA-class silent data
        /// bug looks like in JSON, and every one of them must apply NOTHING and say why.
        #[test]
        fn a_document_that_does_not_validate_applies_nothing() {
            let base = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);

            let mut unknown_key = base.clone();
            unknown_key["equipmentt"] = serde_json::json!({});

            let mut no_gear = base.clone();
            no_gear.as_object_mut().unwrap().remove("gear");

            let mut bad_container = base.clone();
            bad_container["cargo"] =
                serde_json::json!([{"container": "rucksack", "item": "res://mag", "qty": 1}]);

            let mut zero_qty = base.clone();
            zero_qty["cargo"] =
                serde_json::json!([{"container": "vest", "item": "res://mag", "qty": 0}]);

            let mut bad_wear_key = base.clone();
            bad_wear_key["wear"]["chest rig"] = serde_json::json!("res://x");

            let mut wear_not_a_slot = base.clone();
            wear_not_a_slot["wear"]["jacket"] = serde_json::json!(7);

            let mut weapon_missing_slot_type = base.clone();
            weapon_missing_slot_type["weapons"][0]
                .as_object_mut()
                .unwrap()
                .remove("slotType");

            let mut empty_weapon = base.clone();
            empty_weapon["weapons"][0]["weapon"] = serde_json::json!("");

            let mut future_version = base.clone();
            future_version["loadoutVersion"] = serde_json::json!("3");

            let cases: Vec<(&str, String, &str)> = vec![
                ("not JSON at all", "{ nope".to_string(), "not valid JSON"),
                ("an empty object", "{}".to_string(), "loadoutVersion"),
                (
                    "a key outside the closed envelope",
                    unknown_key.to_string(),
                    "additionalProperties is false",
                ),
                (
                    "a v2 document with no gear block",
                    no_gear.to_string(),
                    "missing required key `gear`",
                ),
                (
                    "a cargo container outside the closed vocabulary",
                    bad_container.to_string(),
                    "outside the closed vocabulary",
                ),
                (
                    "a zero-quantity cargo row",
                    zero_qty.to_string(),
                    "at least 1",
                ),
                (
                    "a wear key that fails the schema pattern",
                    bad_wear_key.to_string(),
                    "additionalProperties is false",
                ),
                (
                    "a wear slot that is neither a ResourceName nor null",
                    wear_not_a_slot.to_string(),
                    "expected string or null",
                ),
                (
                    "a weapon with no slotType",
                    weapon_missing_slot_type.to_string(),
                    "missing required key `slotType`",
                ),
                (
                    "an empty weapon ResourceName",
                    empty_weapon.to_string(),
                    "at least 1 character",
                ),
                (
                    "a loadoutVersion nobody ships",
                    future_version.to_string(),
                    "loadoutVersion",
                ),
            ];

            for (label, raw, needle) in cases {
                let faults = refuse(&raw);
                assert!(
                    faults.iter().all(|f| f.key == IMPORT_DOC_KEY),
                    "{label}: a malformed document blames the document, not a row"
                );
                let joined = faults
                    .iter()
                    .map(|f| f.message.as_str())
                    .collect::<Vec<_>>()
                    .join(" | ");
                assert!(
                    joined.contains(needle),
                    "{label}: refusal must say why — wanted `{needle}`, got `{joined}`"
                );
            }
            // The one that must NOT refuse, or the gate is indistinguishable from a broken button.
            assert!(try_import(&base.to_string(), &[], &CompatFeed::default()).is_ok());
        }

        /// The T-686 requirement in as many words: the imported picks go through the SAME loadout
        /// rules the panel uses, before anything is committed. A schema-valid document can still
        /// describe a scope on no rifle or forty magazines in a chest rig.
        #[test]
        fn imported_picks_go_through_the_loadout_rules_before_commit() {
            // 1. Capacity — a hand-authored file the export gate would never have written.
            let items = capacity_catalog();
            let over = picks_to_export(
                &picks(&[("vest", "res://chest_rig")]),
                &[row("vest", "res://mag_stanag", 4)],
                "mp",
            );
            let faults = try_import(&over, &items, &CompatFeed::default())
                .expect_err("over-capacity cargo must not be imported");
            assert_eq!(faults.len(), 1);
            assert_eq!(faults[0].key, "vest");
            assert!(faults[0].message.contains("240 / 200 cm³"), "{faults:?}");

            // 2. Compat — a ready feed carrying no `optic_on_weapon` edge rejects the optic.
            let optic_doc = picks_to_export(
                &picks(&[("primary", "res://rifle_m16"), ("optic", "res://acog")]),
                &[],
                "mp",
            );
            let ready = attachment_feed(&[]);
            let faults = try_import(&optic_doc, &items, &ready)
                .expect_err("an incompatible optic must not be imported");
            assert!(faults.iter().any(|f| f.key == "optic"), "{faults:?}");

            // 3. Attachments — the packed set `arsenal_rules` cannot see is checked too.
            let mut p = picks(&[("primary", "res://rifle_m16")]);
            p.insert(
                attachments_key("primary"),
                pack_attachments(&["res://supp".into()]),
            );
            let att_doc = picks_to_export(&p, &[], "mp");
            let faults = try_import(&att_doc, &items, &ready)
                .expect_err("a stranded attachment must not be imported");
            assert!(faults.iter().any(|f| f.key == "primary"), "{faults:?}");

            // And all three land on a live import once the feed vouches for them.
            let feed = CompatFeed::default();
            assert!(try_import(&optic_doc, &items, &feed).is_ok());
            assert!(try_import(&att_doc, &items, &feed).is_ok());
        }

        /// T-504's argument survives the trip in: undeliverable cargo WARNS, it never blocks.
        /// The website cannot see the slot's kit prefab, so a refusal here would stop an author
        /// importing a loadout the mod delivers perfectly.
        #[test]
        fn undeliverable_cargo_does_not_block_an_import() {
            // Three magazines aimed at a vest this document does not wear.
            let raw = picks_to_export(&HashMap::new(), &[row("vest", "res://mag_stanag", 3)], "mp");
            let back = try_import(&raw, &capacity_catalog(), &CompatFeed::default())
                .expect("an unworn container must not refuse an import");
            assert_eq!(back.cargo.len(), 1);
            // …and the verdict badge still counts it once it has landed.
            let faults = loadout_faults(
                &back.picks,
                &back.cargo,
                &CompatFeed::default(),
                &index_by_name(&capacity_catalog()),
                Some(&kit(&[])),
            );
            assert_eq!(faults.len(), 1, "{faults:?}");
            assert_eq!(faults[0].key, "vest");
        }

        /// The v1 branch: the locked `gear` derivation, run backwards.
        #[test]
        fn the_v1_branch_imports_through_the_locked_derivation_backwards() {
            let raw = serde_json::json!({
                "loadoutVersion": "1",
                "modpackId": "legacy",
                "gear": {
                    "primary": "res://rifle_m16",
                    "uniform": "res://bdu_blouse",
                    "vest": "res://chest_rig",
                    "helmet": "res://helmet_pasgt",
                    "optic": "res://acog",
                    "magazine": serde_json::Value::Null,
                },
            })
            .to_string();
            let back = try_import(&raw, &[], &CompatFeed::default()).expect("a v1 document");
            assert_eq!(back.loadout_version, "1");
            assert_eq!(back.picks.get("primary").unwrap(), "res://rifle_m16");
            assert_eq!(back.picks.get("jacket").unwrap(), "res://bdu_blouse");
            assert_eq!(back.picks.get("headCover").unwrap(), "res://helmet_pasgt");
            assert_eq!(back.picks.get("optic").unwrap(), "res://acog");
            // v1 has ONE vest key, and the two Arsenal rows collapse into it one-way. It lands on
            // `vest`; claiming `armoredVest` would invent armour the file never described.
            assert_eq!(back.picks.get("vest").unwrap(), "res://chest_rig");
            assert!(back.picks.get("armoredVest").is_none());
            assert!(back.picks.get("magazine").is_none(), "null is not a pick");
            // v1 carries no cargo at all, so the key is absent — a later seed may still fire.
            assert!(back.cargo.is_empty() && !back.cargo_present);
            // A v2 document's DERIVED gear block must not be read in its place: this file's gear
            // names a different rifle, and the v2 fields win.
            let mut lying = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);
            lying["gear"]["primary"] = serde_json::json!("res://not_the_rifle");
            let back = try_import(&lying.to_string(), &[], &CompatFeed::default()).unwrap();
            assert_eq!(back.picks.get("primary").unwrap(), "res://rifle_m16");
        }

        /// `cargo` key PRESENCE is the T-068.15.2 anti-reseed marker, and an import must not
        /// invent it: a file that never mentions cargo has not authored an empty cargo list.
        #[test]
        fn the_cargo_key_marker_follows_the_document() {
            let mut silent = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);
            silent.as_object_mut().unwrap().remove("cargo");
            let back = try_import(&silent.to_string(), &[], &CompatFeed::default()).unwrap();
            assert!(
                !back.cargo_present,
                "a file with no cargo key must stay seed-eligible"
            );
            // Present-and-empty is the author having cleared it — that must stick.
            let cleared = v2_file(&picks(&[("primary", "res://rifle_m16")]), &[]);
            let back = try_import(&cleared.to_string(), &[], &CompatFeed::default()).unwrap();
            assert!(back.cargo_present && back.cargo.is_empty());
        }

        /// The receipt counts what was APPLIED, and the modpack note warns without blocking.
        #[test]
        fn the_receipt_reports_what_landed_and_warns_on_a_foreign_modpack() {
            let raw = picks_to_export(
                &full_picks(),
                &[row("vest", "res://mag_stanag", 1)],
                "alpha",
            );
            let doc = try_import(&raw, &[], &CompatFeed::default()).unwrap();
            let line = import_summary("kit.json", &doc, "alpha");
            assert!(line.contains("4 weapon(s)"), "{line}");
            assert!(line.contains("8 wear row(s)"), "{line}");
            assert!(line.contains("1 cargo row(s)"), "{line}");
            assert!(line.contains("Ctrl+Z"), "{line}");
            assert!(!line.contains("modpack"), "matching modpack: {line}");
            // A foreign modpack is a note, not a refusal.
            let line = import_summary("kit.json", &doc, "bravo");
            assert!(line.contains("authored against modpack alpha"), "{line}");
            // "We do not know" on either side is not a mismatch.
            let unknown = try_import(
                &picks_to_export(&HashMap::new(), &[], ""),
                &[],
                &CompatFeed::default(),
            )
            .unwrap();
            assert!(!import_summary("k.json", &unknown, "alpha").contains("modpack"));
        }
    }

    /* ═════════ T-699 — the loadout buffer: Copy · Apply (random) · Remove Everything ═════════ */

    mod t699 {
        use super::*;

        fn buf(source: &str, json: Option<&str>) -> BufferedLoadout {
            BufferedLoadout {
                source_id: source.to_string(),
                loadout_json: json.map(str::to_string),
            }
        }

        fn ids(v: &[&str]) -> Vec<String> {
            v.iter().map(|s| (*s).to_string()).collect()
        }

        /// A `SlotLoadoutV2` document as the Arsenal persists one, distinguishable by its primary.
        fn kit_doc(primary: &str) -> String {
            picks_to_loadout(&picks(&[("primary", primary)]), &names(), None)
                .expect("a picked primary is not an empty loadout")
        }

        /// **What "random" means here**, asserted rather than described.
        ///
        /// Four properties, and every one of them is load-bearing: uniform (no source is
        /// systematically favoured), independent per entity (N entities get N draws, not one draw
        /// N times), reproducible from `(seed, ordinal, len)` (so a bug report replays and this very
        /// test can exist), and degenerate at `len == 1` (a single-source Copy→Apply must be plain
        /// deterministic behaviour).
        #[test]
        fn the_draw_is_uniform_independent_and_reproducible() {
            const N: u64 = 30_000;
            let len = 3usize;
            let mut hits = [0usize; 3];
            for ordinal in 0..N {
                hits[buffer_draw(0xA5A5_A5A5, ordinal, len)] += 1;
            }
            // Uniform: a fair 3-way split of 30k is 10k each; ±5% is far outside anything a
            // correct mix produces by chance and far inside anything a biased one does.
            for (i, h) in hits.iter().enumerate() {
                assert!(
                    (9_500..=10_500).contains(h),
                    "index {i} came up {h} times in {N} draws — not a uniform draw: {hits:?}"
                );
            }
            // Independent per entity: consecutive ordinals must not walk the buffer in lockstep.
            let walk: Vec<usize> = (0..12).map(|o| buffer_draw(7, o, len)).collect();
            let cyclic: Vec<usize> = (0..12).map(|o| (o as usize) % len).collect();
            assert_ne!(walk, cyclic, "the draw is a counter, not a die: {walk:?}");

            // Reproducible: same seed → same assignment, every time.
            for ordinal in 0..50 {
                assert_eq!(
                    buffer_draw(1234, ordinal, len),
                    buffer_draw(1234, ordinal, len)
                );
            }
            // …and a different seed genuinely re-rolls.
            let a: Vec<usize> = (0..40).map(|o| buffer_draw(1, o, len)).collect();
            let b: Vec<usize> = (0..40).map(|o| buffer_draw(2, o, len)).collect();
            assert_ne!(a, b, "advancing the seed must change the assignment");

            // Degenerate at one: randomness must not be able to surprise the single-source case.
            for ordinal in 0..100 {
                assert_eq!(buffer_draw(ordinal * 7919, ordinal, 1), 0);
            }
            // …and an empty buffer never indexes anything (plan_apply refuses to call it, but the
            // function must not be a landmine for the next caller either).
            assert_eq!(buffer_draw(9, 9, 0), 0);
        }

        /// Apply writes ONE buffered loadout per entity, drawn from the buffer, and every write is
        /// a full document — never a merge of two sources, which is the shape that would quietly
        /// invent a soldier nobody authored.
        #[test]
        fn apply_gives_every_entity_exactly_one_buffered_loadout() {
            let sources = [
                buf("s1", Some(&kit_doc("res://rifle_m16"))),
                buf("s2", Some(&kit_doc("res://rifle_ak"))),
                buf("s3", None), // a bare soldier is a legitimate thing to copy and to apply
            ];
            let targets = ids(&["t1", "t2", "t3", "t4", "t5", "t6"]);
            let writes = plan_apply(&targets, &sources, 42, &[], &CompatFeed::default())
                .expect("a clean buffer applies");

            assert_eq!(writes.len(), targets.len(), "one write per selected entity");
            for (w, t) in writes.iter().zip(&targets) {
                assert_eq!(
                    &w.target_id, t,
                    "writes stay index-aligned with the selection"
                );
                let src = sources
                    .iter()
                    .find(|s| Some(&s.source_id) == w.source_id.as_ref())
                    .expect("every write names a buffered source");
                assert_eq!(
                    w.loadout_json, src.loadout_json,
                    "a write is one source's document verbatim, never a blend"
                );
            }
            // Over six entities and three sources the draw must actually vary — a plan that gave
            // everyone source #1 would satisfy every assertion above and be the bug.
            let drawn: HashSet<Option<String>> =
                writes.iter().map(|w| w.source_id.clone()).collect();
            assert!(drawn.len() > 1, "the draw did not vary: {drawn:?}");

            // THE ANTI-INHERITANCE PROPERTY (T-687 was cancelled): the plan carries BYTES, so it is
            // complete without the sources. Drop them and every write still describes its loadout.
            drop(sources);
            assert!(writes.iter().all(|w| w.target_id.starts_with('t')));
        }

        /// One buffered loadout ⇒ everybody gets it, with no draw involved.
        #[test]
        fn a_single_buffered_loadout_needs_no_die() {
            let only = kit_doc("res://rifle_m16");
            let writes = plan_apply(
                &ids(&["a", "b", "c"]),
                &[buf("s", Some(&only))],
                0xDEAD_BEEF,
                &[],
                &CompatFeed::default(),
            )
            .expect("a clean buffer applies");
            assert_eq!(writes.len(), 3);
            assert!(writes
                .iter()
                .all(|w| w.loadout_json.as_deref() == Some(only.as_str())));
        }

        /// Nothing selected, or nothing buffered, is **not** a refusal — there is simply no work.
        #[test]
        fn an_empty_selection_or_buffer_plans_nothing_and_refuses_nothing() {
            let full = [buf("s", Some(&kit_doc("res://rifle_m16")))];
            assert!(plan_apply(&[], &full, 1, &[], &CompatFeed::default())
                .expect("no targets is not a fault")
                .is_empty());
            assert!(
                plan_apply(&ids(&["t"]), &[], 1, &[], &CompatFeed::default())
                    .expect("no buffer is not a fault")
                    .is_empty()
            );
            assert!(plan_remove(&[]).is_empty());
        }

        /// **The gate is the one T-686 built, not a second one that merely resembles it.** The same
        /// bytes refused on the way IN through `try_import` are refused on the way ACROSS through
        /// `plan_apply`, reason for reason — because both call `loadout_rule_refusals`.
        ///
        /// RED (a second gate): drop the `cargo_capacity_errors` line from `loadout_rule_refusals`
        /// and both sides go quiet together, which is what makes this an equivalence and not a
        /// transcription.
        #[test]
        fn the_apply_gate_is_the_import_gate() {
            let items = capacity_catalog();
            // 4 × 60 cm³ of magazine into a 200 cm³ chest rig — a schema-valid document describing
            // kit the game would silently drop.
            let raw = picks_to_export(
                &picks(&[("vest", "res://chest_rig")]),
                &[row("vest", "res://mag_stanag", 4)],
                "mp",
            );
            let on_the_way_in = try_import(&raw, &items, &CompatFeed::default())
                .expect_err("over-capacity cargo must not be importable");
            let across = plan_apply(
                &ids(&["t1"]),
                &[buf("s1", Some(&raw))],
                7,
                &items,
                &CompatFeed::default(),
            )
            .expect_err("…nor applicable");

            assert_eq!(
                on_the_way_in.len(),
                across.len(),
                "the two doors must find the same faults: {on_the_way_in:?} vs {across:?}"
            );
            for (i, a) in on_the_way_in.iter().zip(&across) {
                assert_eq!(i.key, a.key, "same row blamed");
                assert!(
                    a.message.ends_with(&i.message),
                    "same reason, differing only by which buffered source it names: {a:?}"
                );
                assert!(
                    a.message.starts_with("Buffered loadout from s1"),
                    "a refusal must say WHICH copied loadout is unusable: {a:?}"
                );
            }
        }

        /// **The verdict must not depend on the die.** A buffer holding one unusable loadout is
        /// refused for every seed — including the seeds on which the bad entry would never have been
        /// drawn. Validating only what came up would leave a broken loadout lurking in the buffer to
        /// ambush the author on some later press.
        ///
        /// RED: move the gate below the draw loop and validate `src` instead of the buffer → seeds
        /// on which the good source wins go green and this test names them.
        #[test]
        fn a_bad_entry_refuses_the_apply_whatever_the_die_says() {
            let items = capacity_catalog();
            let good = picks_to_export(
                &picks(&[("vest", "res://chest_rig")]),
                &[row("vest", "res://mag_stanag", 1)],
                "mp",
            );
            let bad = picks_to_export(
                &picks(&[("vest", "res://chest_rig")]),
                &[row("vest", "res://mag_stanag", 4)],
                "mp",
            );
            let buffer = [buf("good", Some(&good)), buf("bad", Some(&bad))];
            for seed in 0..64u64 {
                let out = plan_apply(&ids(&["t"]), &buffer, seed, &items, &CompatFeed::default());
                let refusals = out.expect_err(&format!("seed {seed} must refuse the whole apply"));
                assert!(
                    refusals.iter().all(|r| r.message.contains("from bad")),
                    "seed {seed}: {refusals:?}"
                );
            }
            // The same buffer without the bad entry applies cleanly — so the refusal is about the
            // loadout, not about the shape of the test.
            assert!(plan_apply(
                &ids(&["t"]),
                &buffer[..1],
                0,
                &items,
                &CompatFeed::default()
            )
            .is_ok());
        }

        /// T-504, matched deliberately to T-686's choice: cargo authored against a container the
        /// loadout wears nothing in is a **warning on the entity**, never a refusal at the door.
        /// Apply has a second reason on top of T-686's — the fault is a property of the target's
        /// character rather than of the bytes, so wiring it in would make the gate's answer depend
        /// on which entity the die picked.
        #[test]
        fn undeliverable_cargo_warns_but_never_blocks_an_apply() {
            let items = capacity_catalog();
            let idx = index_by_name(&items);
            // Three mags into a vest with no vest picked: 180 cm³, so capacity has nothing to say.
            let raw = picks_to_export(&picks(&[]), &[row("vest", "res://mag_stanag", 3)], "mp");
            let doc_picks = loadout_to_picks(Some(&raw));
            let (cargo, _) = rules::cargo_from_loadout(Some(&raw));

            let faults = loadout_faults(
                &doc_picks,
                &cargo,
                &attachment_feed(&[]),
                &idx,
                Some(&kit(&[])),
            );
            assert_eq!(faults.len(), 1, "the badge must still count it: {faults:?}");
            assert!(faults[0].message.contains("nowhere known to go"));

            assert!(
                buffer_refusals(&[buf("s", Some(&raw))], &items, &CompatFeed::default()).is_empty(),
                "a warning must never become an Apply refusal"
            );
            assert!(
                try_import(&raw, &items, &CompatFeed::default()).is_ok(),
                "…on either door"
            );
        }

        /// **Remove Everything must stay removed.** The strip writes an explicit empty document
        /// rather than clearing the field, because a cleared field has no `cargo` key and no `cargo`
        /// key is exactly the condition on which `seed_cargo` puts the character's default magazines
        /// back — a strip verb that undoes itself the next time the panel opens.
        ///
        /// RED: drop `"cargo": []` from `stripped_loadout` → "the strip must mark cargo as
        /// user-cleared".
        #[test]
        fn remove_everything_strips_the_kit_and_stops_the_cargo_reseed() {
            let stripped = stripped_loadout();
            assert!(
                loadout_to_picks(Some(&stripped)).is_empty(),
                "no wear row and no weapon survives a strip"
            );
            let (rows, present) = rules::cargo_from_loadout(Some(&stripped));
            assert!(rows.is_empty(), "no cargo survives a strip");
            assert!(present, "the strip must mark cargo as user-cleared");
            // Every wear row the persist path emits is present and null — the vocabulary comes from
            // ROWS, so this document and `picks_to_loadout`'s cannot drift apart.
            let v: serde_json::Value = serde_json::from_str(&stripped).expect("valid JSON");
            let wear = v["wear"].as_object().expect("a wear block");
            assert_eq!(
                wear.len(),
                ROWS.iter().filter(|r| r.weapon.is_none()).count()
            );
            assert!(wear.values().all(serde_json::Value::is_null));
            assert_eq!(v["weapons"], serde_json::json!([]));

            // THE HAZARD, demonstrated rather than asserted about: the seed rule really does fire on
            // a cleared field, and really does not fire on this document.
            let defaults = vec![row("vest", "res://mag_stanag", 3)];
            assert!(
                rules::seed_cargo(None, &defaults).is_some(),
                "a cleared loadout field re-seeds — this is what the strip must not leave behind"
            );
            assert!(
                rules::seed_cargo(Some(&stripped), &defaults).is_none(),
                "the stripped document must be seed-ineligible, or Remove Everything undoes itself"
            );

            // And the plan: one write per target, each carrying that document.
            let writes = plan_remove(&ids(&["a", "b"]));
            assert_eq!(writes.len(), 2);
            assert!(writes.iter().all(|w| w.source_id.is_none()));
            assert!(writes
                .iter()
                .all(|w| w.loadout_json.as_deref() == Some(stripped.as_str())));
        }

        /// `plan_remove` runs no gate because the stripped document cannot fail one. That is a claim
        /// about the RULES module, which can change, so it is a test and not a comment.
        #[test]
        fn the_stripped_document_passes_every_rule() {
            let items = capacity_catalog();
            for feed in [CompatFeed::default(), attachment_feed(&[])] {
                assert!(
                    buffer_refusals(&[buf("s", Some(&stripped_loadout()))], &items, &feed)
                        .is_empty(),
                    "a stripped loadout must be unconditionally applicable"
                );
            }
        }

        /// **The undo arithmetic, measured — not counted in the source text.**
        ///
        /// Wave 112's one-commit pin counted the literal `persist(` and stayed green under an N-step
        /// perturbation (T-736). This one runs the commit path against a sink that records what it
        /// was actually handed, so the number in the receipt is the number of documents written. An
        /// Apply over N entities is N transactions and therefore N undo steps — the core has no
        /// atomic multi-entity loadout write (T-732) — and the receipt says so out loud rather than
        /// claiming an atomicity nothing here provides.
        ///
        /// RED (a dropped write): force the sink to return `false` for one id →
        /// "3 writes planned, sink took 2" and the receipt WARNING arm.
        /// RED (count invocations again): `done += 1` unconditionally → miss path no longer red.
        /// RED (a fake one-step claim): report `1` instead of the commit count → the receipt no
        /// longer names the real number of Ctrl+Z presses.
        #[test]
        fn the_receipt_counts_the_writes_the_document_actually_took() {
            let writes = plan_remove(&ids(&["a", "b", "c"]));
            let mut sink: Vec<(String, Option<String>)> = Vec::new();
            let commits = commit_writes(&writes, |id, json| {
                sink.push((id.to_string(), json));
                true
            });

            assert_eq!(commits, 3, "one commit per planned write");
            assert_eq!(
                sink.len(),
                writes.len(),
                "{} writes planned, sink took {}",
                writes.len(),
                sink.len()
            );
            let seen: Vec<&str> = sink.iter().map(|(id, _)| id.as_str()).collect();
            assert_eq!(
                seen,
                ["a", "b", "c"],
                "every target is written exactly once"
            );

            let line = remove_receipt(writes.len(), commits);
            assert!(line.contains("3 undo step(s)"), "{line}");
            assert!(
                line.contains("T-732"),
                "the receipt must cite the gap: {line}"
            );
            assert!(!line.contains("WARNING"), "{line}");

            // The honesty property: a sink that refuses one id (the production shape when
            // `update_slot_loadout` returns false) must shrink the counted commits and light the
            // WARNING arm. Counting loop invocations would keep commits==3 and hide the miss.
            let mut miss_sink: Vec<(String, Option<String>)> = Vec::new();
            let miss_commits = commit_writes(&writes, |id, json| {
                if id == "b" {
                    return false;
                }
                miss_sink.push((id.to_string(), json));
                true
            });
            assert_eq!(miss_commits, 2, "refused ack must not count as a commit");
            assert_eq!(
                miss_sink.len(),
                2,
                "{} writes planned, sink took {}",
                writes.len(),
                miss_sink.len()
            );
            let dropped = remove_receipt(writes.len(), miss_commits);
            assert!(dropped.contains("WARNING"), "{dropped}");
            assert!(
                dropped.contains("3 write(s) were planned and 2 reached the document"),
                "{dropped}"
            );

            // Apply says the same three things: how many landed, that it is one step each, and why.
            let apply = apply_receipt(5, 2, 5);
            assert!(apply.contains("Applied 5 loadout(s)"), "{apply}");
            assert!(apply.contains("2-loadout buffer"), "{apply}");
            assert!(apply.contains("5 undo step(s)"), "{apply}");
            assert!(apply.contains("Ctrl+Z 5 times"), "{apply}");
            assert!(apply.contains("T-732"), "{apply}");
            assert!(apply_receipt(5, 2, 4).contains("WARNING"));
        }

        /// The Copy receipt counts the bare kits out loud — buffering forty empty soldiers and
        /// discovering it only after Apply is exactly the surprise a receipt exists to prevent.
        #[test]
        fn the_copy_receipt_reports_what_was_buffered_including_the_bare_ones() {
            let doc = kit_doc("res://rifle_m16");
            let line = copy_receipt(&[buf("a", Some(&doc)), buf("b", Some(&doc))]);
            assert!(line.contains("Copied 2 loadout(s)"), "{line}");
            assert!(line.contains("at random"), "{line}");
            assert!(!line.contains("no loadout at all"), "{line}");

            let mixed = copy_receipt(&[buf("a", Some(&doc)), buf("b", None), buf("c", None)]);
            assert!(mixed.contains("Copied 3 loadout(s)"), "{mixed}");
            assert!(
                mixed.contains("2 of them carry no loadout at all"),
                "{mixed}"
            );
        }
    }

    /// **T-737 — a refusal has to say which row.**
    ///
    /// The defect is not "the message is wrong"; every message here is true. The defect is that
    /// two *different* rows produce the *same* true sentence, so the list the author is shown
    /// cannot be acted on. Every test below therefore uses **two** stranded rows — one row cannot
    /// observe this defect at all, and a test written with one would have stayed green through it.
    mod t737 {
        use super::*;

        /// A ready feed carrying arbitrary typed edges. `attachment_feed` only speaks
        /// `attachment_on_weapon`; the two rows this defect is about (`optic`, `magazine`) are
        /// `RowSource::Edge` rows on two *other* edge types, so they need their own feed.
        fn typed_feed(edges: &[(&str, &str, &str)]) -> CompatFeed {
            let rows: Vec<crate::core::dto::RegistryCompatEdge> = edges
                .iter()
                .enumerate()
                .map(|(i, (from, to, ty))| crate::core::dto::RegistryCompatEdge {
                    id: i.to_string(),
                    modpack_id: "m".into(),
                    from_node: (*from).into(),
                    to_node: (*to).into(),
                    edge_type: (*ty).into(),
                    evidence: String::new(),
                    qty: 1,
                    created_at: String::new(),
                    updated_at: String::new(),
                })
                .collect();
            CompatFeed {
                status: rules::CompatStatus::Ready,
                graph: rules::CompatGraph::from_edges(&rows),
            }
        }

        /// `mod t699`'s buffer helpers, re-declared rather than borrowed: they are private to that
        /// module, and a sibling reaching into it would not compile.
        fn buf(source: &str, json: Option<&str>) -> BufferedLoadout {
            BufferedLoadout {
                source_id: source.to_string(),
                loadout_json: json.map(str::to_string),
            }
        }

        fn ids(v: &[&str]) -> Vec<String> {
            v.iter().map(|s| (*s).to_string()).collect()
        }

        /// One weapon swap, two stranded rows: an ACOG and a STANAG that this catalog knows —
        /// on the *other* rifle. Both edge rows are refused, and refused for the same reason.
        fn two_stranded_rows() -> (String, CompatFeed) {
            let raw = picks_to_export(
                &picks(&[
                    ("primary", "res://rifle_m16"),
                    ("optic", "res://acog"),
                    ("magazine", "res://mag_stanag"),
                ]),
                &[],
                "mp",
            );
            let feed = typed_feed(&[
                ("res://rifle_ak", "res://acog", "optic_on_weapon"),
                ("res://rifle_ak", "res://mag_stanag", "mag_in_weapon"),
            ]);
            (raw, feed)
        }

        /// **The claim in the ticket title.** Two stranded rows must render as two lines the
        /// author can tell apart, each naming its own row — while the reason each carries survives
        /// intact underneath.
        ///
        /// RED (the shipped defect): render the list with `.map(|e| e.message)` again — i.e. make
        /// `refusal_line` return `e.message.clone()` unconditionally → "two stranded rows must not
        /// print the same line".
        #[test]
        fn two_stranded_rows_render_as_two_distinguishable_refusals() {
            let (raw, feed) = two_stranded_rows();
            let refusals = try_import(&raw, &[], &feed).expect_err("a stranded loadout is refused");
            assert_eq!(refusals.len(), 2, "two rows are stranded: {refusals:?}");

            // The premise, stated as a fact about the data rather than assumed: the two REASONS
            // are byte-identical. Everything that distinguishes the rows lives in `key`, which is
            // exactly what the old rendering threw away.
            assert_eq!(
                refusals[0].message, refusals[1].message,
                "the premise of this test — the reason alone cannot tell the rows apart"
            );
            assert_eq!(
                [refusals[0].key, refusals[1].key],
                ["optic", "magazine"],
                "…and the key is where the difference is: {refusals:?}"
            );

            let lines: Vec<String> = refusals.iter().map(refusal_line).collect();
            assert_ne!(
                lines[0], lines[1],
                "two stranded rows must not print the same line: {lines:?}"
            );
            assert!(lines[0].starts_with("Optic — "), "{lines:?}");
            assert!(lines[1].starts_with("Magazine — "), "{lines:?}");
            for (line, e) in lines.iter().zip(&refusals) {
                assert!(
                    line.ends_with(&e.message),
                    "naming the row must not cost the reason: {line}"
                );
            }
        }

        /// The same two rows down the **Apply** door, which shares the refusal contract and shared
        /// the defect. `buffer_refusals` names which copied loadout is bad; `refusal_line` names
        /// which row inside it — and both are needed, because one buffered loadout can strand two
        /// rows at once.
        #[test]
        fn apply_refusals_name_the_row_as_well_as_the_source() {
            let (raw, feed) = two_stranded_rows();
            let refusals = plan_apply(&ids(&["t1"]), &[buf("s1", Some(&raw))], 7, &[], &feed)
                .expect_err("…nor applicable");
            assert_eq!(refusals.len(), 2, "{refusals:?}");
            assert_eq!(
                refusals[0].message, refusals[1].message,
                "the source prefix alone cannot tell two rows of ONE loadout apart"
            );

            let lines: Vec<String> = refusals.iter().map(refusal_line).collect();
            assert_ne!(lines[0], lines[1], "{lines:?}");
            for line in &lines {
                assert!(
                    line.contains("Buffered loadout from s1"),
                    "which copied loadout is still the first question: {line}"
                );
            }
            assert!(lines[0].starts_with("Optic — "), "{lines:?}");
            assert!(lines[1].starts_with("Magazine — "), "{lines:?}");
        }

        /// Schema and parse faults are left exactly as they were: their messages already carry the
        /// JSON pointer, which is a better address than any row label, and there is no row to name.
        /// The exemption is `rules::row` answering `None` — not a hard-coded key comparison.
        #[test]
        fn document_faults_keep_their_own_address() {
            for raw in ["{ not json", r#"{"loadoutVersion":"2"}"#] {
                let refusals = try_import(raw, &[], &CompatFeed::default())
                    .expect_err("a malformed document is refused");
                assert!(!refusals.is_empty());
                for e in &refusals {
                    assert_eq!(e.key, IMPORT_DOC_KEY, "no row is to blame: {e:?}");
                    assert_eq!(
                        refusal_line(e),
                        e.message,
                        "a document fault must not grow a row prefix it cannot justify"
                    );
                }
            }
        }

        /// **The T-686 asymmetry is not this ticket's to change, and this pins that it did not.**
        /// Export refuses on capacity ONLY, so a loadout with a stranded optic can still be
        /// downloaded; the import gate additionally refuses compat, so the same bytes cannot come
        /// back in. That is intended — the import gate's job is to not let an outside document put
        /// the editor into a state the author did not author — and it is precisely the case where
        /// naming the row matters most, so the naming is asserted on the very bytes that prove it.
        #[test]
        fn the_export_import_asymmetry_still_holds() {
            let (raw, feed) = two_stranded_rows();
            let doc_picks = loadout_to_picks(Some(&raw));
            assert!(
                try_export(&doc_picks, &[], &[], "mp").is_ok(),
                "export refuses on capacity only — a stranded optic must still download"
            );
            let refusals = try_import(&raw, &[], &feed)
                .expect_err("…and the import gate must still refuse those same bytes on compat");
            let lines: Vec<String> = refusals.iter().map(refusal_line).collect();
            assert_ne!(
                lines[0], lines[1],
                "the exportable-but-not-importable case is where naming the row matters most"
            );
        }

        /// **The same defect one level down — the case `refusal_line` structurally cannot reach.**
        ///
        /// Two stranded ROWS differ in their `key`, so prefixing the row label separates them. Two
        /// stranded ATTACHMENTS hang off the SAME weapon row: same key, therefore same prefix, and
        /// the reason was identical too — so an author who swapped one rifle was handed
        /// "Primary — Attachment not compatible with the selected Primary" **twice** and learned
        /// what was wrong but not which of their two attachments to pull. The only place left to
        /// carry the difference is the message, so the message names the attachment.
        ///
        /// One attachment cannot observe this, exactly as one row could not observe T-737's.
        ///
        /// RED (the shipped defect): drop `` `{rn}` `` from both message arms in
        /// `attachment_errors` → "two stranded attachments must not print the same line".
        #[test]
        fn two_stranded_attachments_on_one_row_render_as_two_distinguishable_refusals() {
            // Both attachments are known to this catalog — on the OTHER rifle. One swap strands
            // both at once, which is the whole point: it is a single authoring mistake.
            let feed = attachment_feed(&[
                ("res://handguard", "res://rifle_ak"),
                ("res://supp", "res://rifle_ak"),
            ]);
            let mut p = picks(&[("primary", "res://rifle_m16")]);
            p.insert(
                attachments_key("primary"),
                pack_attachments(&["res://handguard".into(), "res://supp".into()]),
            );

            let errs = attachment_errors(&p, &feed);
            assert_eq!(errs.len(), 2, "both attachments are stranded: {errs:?}");
            // The premise, as a fact about the data: `refusal_line` has nothing to work with here.
            // One row, one key — the difference cannot live where T-737 put it.
            assert_eq!(
                [errs[0].key, errs[1].key],
                ["primary", "primary"],
                "{errs:?}"
            );

            let lines: Vec<String> = errs.iter().map(refusal_line).collect();
            assert_ne!(
                lines[0], lines[1],
                "two stranded attachments must not print the same line: {lines:?}"
            );
            // …and each line names *its own* attachment, not merely some attachment.
            assert!(
                lines[0].contains("res://handguard") && !lines[0].contains("res://supp"),
                "{lines:?}"
            );
            assert!(
                lines[1].contains("res://supp") && !lines[1].contains("res://handguard"),
                "{lines:?}"
            );
            // Naming the item is not paid for with the row prefix T-737 added.
            for line in &lines {
                assert!(line.starts_with("Primary — "), "{lines:?}");
            }

            // The hostless arm carries the same burden: no Primary at all, two attachments still
            // stranded, still two lines an author can tell apart.
            p.remove("primary");
            let hostless: Vec<String> = attachment_errors(&p, &feed)
                .iter()
                .map(refusal_line)
                .collect();
            assert_eq!(hostless.len(), 2, "{hostless:?}");
            assert_ne!(hostless[0], hostless[1], "{hostless:?}");
            assert!(hostless[0].contains("res://handguard"), "{hostless:?}");
            assert!(hostless[1].contains("res://supp"), "{hostless:?}");
        }
    }

    /* ═══════════ T-779 — the single write path must not fake its acknowledgement ═══════════ */

    /// The behaviour half of the T-779 pin — [`commit_one_write`] driven natively with a
    /// refusing sink, the exact production shape for an unknown id. The wiring half (the
    /// live `editor_ops` and panel scrub pins) stays in `arsenal/mod.rs::tests::t779`.
    mod t779 {
        use super::*;

        /// **The acceptance test: a write the document REFUSES mints no history tail.**
        ///
        /// `update_slot_loadout` returns `false` for an id the document does not hold — an entity
        /// deleted, or undone away, while the Arsenal sat open over it. A test that only ever used
        /// a valid id could not observe this defect at all: with the hardcoded `true` in place, the
        /// valid-id path behaved identically before and after the fix.
        #[test]
        fn a_refused_write_mints_no_tail_and_does_not_dirty_the_mission() {
            // The refusal. `tails` stands in for `mission_history::after_local_edit` — which both
            // sets `HistoryCtx::dirty` and mints the undo step, so one counter answers both halves
            // of the acceptance: no tail fired means nothing was dirtied and no step was minted.
            let mut tails = 0usize;
            let took = commit_one_write(|| false, || tails += 1);
            assert!(
                !took,
                "T-779: a refused write must report itself refused, not report success"
            );
            assert_eq!(
                tails, 0,
                "T-779: a refused write must mint no history tail — the document did not change, \
                 so there is nothing to dirty and nothing for Ctrl+Z to restore"
            );

            // The accepted write, so the pin cannot pass by never firing the tail at all.
            let mut tails = 0usize;
            let took = commit_one_write(|| true, || tails += 1);
            assert!(took, "T-779: an acknowledged write must report success");
            assert_eq!(
                tails, 1,
                "T-779: an acknowledged write is exactly one tail — one undo step per pick (T-732)"
            );

            // The gate must read the SINK, not the fact that a commit closure ran. Counting
            // invocations is precisely the T-770 defect, one layer down.
            let mut ran = 0usize;
            let mut tails = 0usize;
            let took = commit_one_write(
                || {
                    ran += 1;
                    false
                },
                || tails += 1,
            );
            assert_eq!(ran, 1, "the commit closure must still be called");
            assert!(!took);
            assert_eq!(
                tails, 0,
                "T-779: the tail is gated on the ACK, not on the closure having been invoked"
            );
        }
    }
}
