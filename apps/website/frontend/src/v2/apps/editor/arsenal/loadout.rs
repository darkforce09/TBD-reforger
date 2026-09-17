//! The pure loadout core — `SlotLoadoutV2` serialization ([`loadout_to_picks`] /
//! [`picks_to_loadout`]), the export/import gates ([`try_export`] / [`try_import`]), the
//! T-699 loadout buffer (plan / commit / receipts) and the refusal vocabulary.
//!
//! Split out of `arsenal/mod.rs` at T-934.8 with bodies unchanged; `mod.rs` re-exports every
//! public item, so the `crate::v2::apps::editor::arsenal::X` paths external callers use are stable.

use std::collections::{HashMap, HashSet};
pub use website_map_engine::data::store::operations::cargo::commit_writes;
pub use website_map_engine::data::store::operations::cargo::BufferedLoadout;
pub use website_map_engine::data::store::operations::cargo::LoadoutWrite;

use crate::v2::apps::editor::arsenal::rules::{self, index_by_name, validate_loadout, CompatFeed};
use crate::v2::core::api::dto::RegistryItem;

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
pub(crate) const ATTACHMENT_EDGE: &str = "attachment_on_weapon";

/// Separator for the packed attachment set. U+001F (ASCII US) is safe **by contract, not by luck**:
/// `registry-compat.schema.json#/$defs/resourceName` pins every node to
/// `^\{[0-9A-F]{16}\}[A-Za-z0-9/_.\- ()']+$` — a pattern that admits no control character — so a
/// join can never produce a string that splits back into something else.
const ATTACHMENT_SEP: &str = "\u{1f}";

/// The `picks` key holding `weapon_key`'s attachment set.
///
/// The set rides a **synthetic key** rather than widening `picks` to `HashMap<String, Vec<String>>`
/// because that map is the argument type of three [`crate::v2::apps::editor::arsenal::rules`] entry points
/// (`row_options`, `validate_loadout`, `loadout_weight`) and this slice does not own that module.
/// The `@` infix cannot collide with a row key, and each of those consumers iterates `LOADOUT_ROWS`
/// **by key** — so the synthetic entry is invisible to them by construction, not by convention.
pub(crate) fn attachments_key(weapon_key: &str) -> String {
    format!("attachments@{weapon_key}")
}

/// `weapon_key`'s picked attachments, in pick order.
pub(crate) fn attachments_of(picks: &HashMap<String, String>, weapon_key: &str) -> Vec<String> {
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
pub(crate) fn pack_attachments(list: &[String]) -> String {
    list.join(ATTACHMENT_SEP)
}

/// Attachments stranded by a weapon swap — the same authoring hazard `validate_loadout` already
/// flags for optic/magazine, checked here because the set rides a key `rules` cannot see.
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
/// Read through the existing public `editor_context::slots_json` rather than a new accessor — this
/// slice does not own `editor_ops`. Native has no hosted document, so there is no `assetId` and
/// [`kit_default_items`] answers `None`.
pub(super) fn slot_asset_id(slot_id: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let json = crate::v2::apps::editor::bridge::host_state::editor_context::slots_json()?;
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
///    [`attachment_errors`] (the packed set `rules` cannot see) and
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

/// **T-779 — the single-write sibling of [`commit_writes`]: the history tail fires only if the
/// document acknowledged the write.**
///
/// T-770 gave `MissionDocCore::update_slot_loadout` a `bool` return and taught the *batch* path to
/// count it. The *single* path — `loadout_commands::set_loadout`, the one every Arsenal pick and every
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
#[path = "tests/loadout/buffer_operations.rs"]
mod buffer_operations_tests;
#[cfg(test)]
#[path = "tests/loadout/import_round_trip.rs"]
mod import_round_trip_tests;
#[cfg(test)]
#[path = "tests/loadout/refusal_messages.rs"]
mod refusal_messages_tests;
#[cfg(test)]
#[path = "tests/loadout/serialization_and_export.rs"]
mod serialization_and_export_tests;
#[cfg(test)]
#[path = "tests/loadout/write_acknowledgment.rs"]
mod write_acknowledgment_tests;
