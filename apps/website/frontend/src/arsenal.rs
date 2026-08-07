//! Arsenal tab — the **Smart Forge** (ArsenalTab.tsx + arsenalRules.ts + SoldierSilhouette.tsx
//! port, T-159.27 → T-167). A doc-backed loadout editor: the 14 loadout rows (incl. the compat
//! `edge` rows optic/magazine keyed off the picked weapon), the **attachment set** each weapon
//! accepts (T-197), a clickable **SVG paper-doll**, an honest **weight** readout, and per-row
//! **compat validation** — persisted on the slot via `editor_ops::set_loadout` (one undo step per
//! pick) as the canonical `SlotLoadoutV2` shape (the same `picksToLoadout` output the mod equip
//! reads), so a pick round-trips through Save/Export.
//!
//! The domain decisions (rows, compat graph, option building, validation, doll regions, weight)
//! live in [`crate::arsenal_rules`] (pure, native-tested). This module is the UI + the persisted
//! serialization ([`picks_to_loadout`] / [`loadout_to_picks`]: optic/magazine ride `weapons[0]` as
//! sticky sub-fields; attachments ride their own weapon's `attachments[]`).
//!
//! # Persistence — there is no Save button here, and that is the design (T-503)
//!
//! Every pick and every cargo edit calls [`crate::editor_ops::set_loadout`] the moment it happens.
//! Nothing stages. T-503 asked whether that is a bug — whether the Arsenal should grow an explicit
//! Save with a dirty indicator and a discard path — and the answer from the rest of the SPA is no,
//! twice over:
//!
//! * **Every other mission-document editor commits on the spot.** `editor_ops.rs` funnels 26 call
//!   sites into `mission_history::after_local_edit` — measured 2026-07-31; this line said 28 and
//!   28 is the SPA-wide total. The other two are direct calls from `mission_hydrate.rs:496` and
//!   `mission_editor.rs:1316`, neither of them an editor commit point, so the argument below is
//!   unaffected by the correction. The Arsenal's `set_loadout`
//!   (`editor_ops.rs:777`) is one of them. Its own siblings in this very modal are the clearest
//!   case: Transform X/Y/Z/rotation (`attributes.rs:265`) and Identity role/tag/stance
//!   (`attributes.rs:335`) commit on blur/Enter with no Save of their own — `attributes.rs:7` states
//!   the contract in as many words ("rebind + persist + one undo step per commit"). Same for the
//!   outliner, the ORBAT manager (`orbat_manager.rs:1301`) and the top-strip title
//!   (`eden_chrome.rs:1057`). A Save button in the Arsenal would make it the only editor in the
//!   application with a second commit point, and would break the one-undo-step-per-pick contract
//!   the module header above is built on.
//! * **The editor already has exactly one commit point, and it is not per-panel.**
//!   `after_local_edit` sets `HistoryCtx::dirty` (`mission_history.rs:62`), a debounced IDB persist
//!   keeps the work across a reload, `register_unload_guard` (T-189) refuses to let the tab close
//!   over it, and **Save Version** publishes it to the server and clears the flag. Undo is Ctrl+Z,
//!   not a per-panel discard button.
//!
//! What *was* wrong is that the author could not tell any of this from inside the Arsenal. The one
//! platform-wide "your work is not saved yet" signal is the `•` next to the mission title
//! (`eden_chrome.rs:1066`) — and this tab renders under a full-viewport `bg-black/50
//! backdrop-blur-sm` scrim (`attributes.rs:88`) that dims and blurs precisely that indicator while
//! the Arsenal is open. So the fix is not a Save button; it is saying it here, in the panel, next
//! to the verdict badge — see the `data-arsenal-persist` line at the bottom of [`ArsenalTab`]. The
//! wiring is pinned by `tests::t503`, so a future slice that quietly introduces staging goes red.
#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use leptos::prelude::*;

use crate::arsenal_rules::{
    self as rules, format_loadout_weight, index_by_name, loadout_weight, row_options,
    validate_loadout, CompatFeed,
};
use crate::dto::RegistryItem;

const CONTROL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";

/// T-503 — the persistence contract as the **author** reads it, not as the module doc reads it.
///
/// The Arsenal has no Save button (see the module header for why the rest of the SPA says it should
/// not), and until this line existed nothing in the panel said so: an author who made a pick and
/// closed the modal had no way to tell whether the pick had been kept. This is the answer, and it
/// is unconditional because the behaviour is.
const PERSIST_ALWAYS: &str = "Every pick and cargo edit here is written to the mission document the moment you make it — the Arsenal has no Save button by design, and Ctrl+Z undoes one pick.";

/// The half of the persistence line that reads the live `mission_history` dirty flag: the mission
/// itself has nothing waiting for the server. Paired with [`PERSIST_UNSAVED`].
const PERSIST_CLEAN: &str = "The mission has no unsaved changes.";

/// The dirty half: the doc holds work no server version carries yet. This is the same state the top
/// strip's `•` reports — which this modal's backdrop is busy blurring, hence the repeat here.
const PERSIST_UNSAVED: &str =
    "The mission has unsaved changes — Save Version publishes them to the server.";

/// Does the live mission document hold work the server has not seen?
///
/// `mission_history` is `cfg(target_arch = "wasm32")` (it drives the hosted doc), so the native view
/// shell answers `false`: there is no editor mounted there and therefore nothing unsaved. The read
/// itself is `try_get_untracked`, so the persistence line below tracks a local commit counter to
/// re-run — the modal scrim means an Arsenal commit is the only edit that can happen while this is
/// on screen.
fn mission_has_unsaved_work() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        crate::mission_history::is_dirty()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        false
    }
}

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
const ATTACHMENT_EDGE: &str = "attachment_on_weapon";

/// Separator for the packed attachment set. U+001F (ASCII US) is safe **by contract, not by luck**:
/// `registry-compat.schema.json#/$defs/resourceName` pins every node to
/// `^\{[0-9A-F]{16}\}[A-Za-z0-9/_.\- ()']+$` — a pattern that admits no control character — so a
/// join can never produce a string that splits back into something else.
const ATTACHMENT_SEP: &str = "\u{1f}";

/// The `picks` key holding `weapon_key`'s attachment set.
///
/// The set rides a **synthetic key** rather than widening `picks` to `HashMap<String, Vec<String>>`
/// because that map is the argument type of three [`crate::arsenal_rules`] entry points
/// (`row_options`, `validate_loadout`, `loadout_weight`) and this slice does not own that module.
/// The `@` infix cannot collide with a row key, and each of those consumers iterates `LOADOUT_ROWS`
/// **by key** — so the synthetic entry is invisible to them by construction, not by convention.
fn attachments_key(weapon_key: &str) -> String {
    format!("attachments@{weapon_key}")
}

/// `weapon_key`'s picked attachments, in pick order.
fn attachments_of(picks: &HashMap<String, String>, weapon_key: &str) -> Vec<String> {
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
fn pack_attachments(list: &[String]) -> String {
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
            let message = match host {
                None => format!("Attachment requires a {label} pick"),
                Some(h) if !g.accepts(h, &rn, ATTACHMENT_EDGE) => {
                    format!("Attachment not compatible with the selected {label}")
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
fn loadout_faults(
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
fn kit_default_items(feed: &CompatFeed, asset_id: Option<&str>) -> Option<HashSet<String>> {
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
fn slot_asset_id(slot_id: &str) -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let json = crate::editor_ops::slots_json()?;
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
fn export_modpack_id(items: &[RegistryItem]) -> String {
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
    let mut refusals = validate_loadout(&picks, feed.ready_graph(), feed.status);
    refusals.extend(attachment_errors(&picks, feed));
    refusals.extend(rules::cargo_capacity_errors(
        &picks,
        &cargo,
        &index_by_name(items),
    ));
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
fn import_summary(name: &str, doc: &ImportedLoadout, catalog_modpack: &str) -> String {
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

/// The Smart Arsenal tab — mounted in the Attributes modal (T-159.26 seam). `registry` is the flat
/// catalog; `compat` the edge feed (both fetched once by the editor); `slot_id` + `loadout_json`
/// come from the modal's re-read.
#[component]
pub fn ArsenalTab(
    slot_id: String,
    /// The slot's current `loadout` JSON (from `editor_ops::read_loadout`).
    loadout_json: Option<String>,
    /// The flat registry gear rows, `None` while loading.
    registry: RwSignal<Option<Vec<RegistryItem>>>,
    /// The compat edge feed (optic/magazine rows + validation).
    compat: RwSignal<CompatFeed>,
) -> impl IntoView {
    // T-068.15.2 — open-time cargo seed for pre-existing slots (place/apply already
    // seed at their own hooks): only fires when the loadout has no `cargo` key and
    // the character has `character_default_cargo` defaults; returns the seeded JSON
    // so this render uses it without a re-read.
    #[cfg(target_arch = "wasm32")]
    let loadout_json = crate::editor_ops::seed_slot_cargo(&slot_id).or(loadout_json);
    // T-504 — the slot's character prefab, read once: it cannot change while the modal is open, and
    // it keys the kit-default evidence the undeliverable-cargo rule needs.
    let asset_id = StoredValue::new(slot_asset_id(&slot_id));
    let id = StoredValue::new(slot_id);
    // Reactive picks so the doll, weight, validation, and dependent edge rows all re-render live.
    let picks = RwSignal::new(loadout_to_picks(loadout_json.as_deref()));
    // Cargo rows + whether the loadout carries the `cargo` key (the "user state" marker —
    // absent means a later seed may still fire, so persists stay key-less until touched).
    let (cargo0, cargo_present0) = rules::cargo_from_loadout(loadout_json.as_deref());
    let cargo = RwSignal::new(cargo0);
    let cargo_present = RwSignal::new(cargo_present0);
    // The rail/doll active region (highlighted row + hotspot). Default to the primary weapon.
    let active_key = RwSignal::new("primary".to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, cargo_present);

    // T-503 — commits made in this tab, purely so the persistence line below can re-run: the dirty
    // flag it reads is `try_get_untracked` and therefore not reactive on its own.
    let commits = RwSignal::new(0u32);
    // Persist the current picks + cargo as the canonical V2 loadout (one undo step). wasm-only.
    //
    // T-503 — this is THE commit, and it runs on every mutation with nothing staged in between.
    // That is deliberate and matches every other mission-document editor in the SPA; the module
    // header sets out the evidence, and `tests::t503` pins the wiring.
    let persist = move |map: &HashMap<String, String>, items: &[RegistryItem]| {
        #[cfg(target_arch = "wasm32")]
        {
            let names: HashMap<String, String> = items
                .iter()
                .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                .collect();
            let rows = cargo.get_untracked();
            let rows = cargo_present.get_untracked().then_some(rows.as_slice());
            crate::editor_ops::set_loadout(&id.get_value(), picks_to_loadout(map, &names, rows));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (map, items);
        commits.update(|n| *n = n.wrapping_add(1));
    };
    // Cargo edits mark the key present, then persist through the same path.
    let persist_cargo = move |items: &[RegistryItem]| {
        cargo_present.set(true);
        persist(&picks.get_untracked(), items);
    };

    // T-686 — the import outcome, said in the panel next to the button that produced it. Two
    // signals and not one `Result` because they render differently and never both: a receipt is a
    // quiet line, a refusal is a list the author has to read.
    let import_status = RwSignal::new(String::new());
    let import_refusals = RwSignal::new(Vec::<String>::new());

    // T-172 B10 — full screen-04 Smart Forge layout (operator-confirmed scope): region icon
    // rail · filtered item list · 3D doll (DollEngine; SVG paper-doll only as the create-error
    // fallback, the T-154 contract) · compat panel · COMPAT/VALID badges · Download loadout JSON.
    // Data flow unchanged: picks/active_key drive everything; persist writes SlotLoadoutV2.
    let doll_unavailable = RwSignal::new(false);
    let filter = RwSignal::new(String::new());
    // Switching regions clears the list filter (each region gets a fresh search).
    Effect::new(move |prev: Option<String>| {
        let k = active_key.get();
        if prev.as_deref().is_some_and(|p| p != k) {
            filter.set(String::new());
        }
        k
    });
    view! {
        <div class="flex flex-col gap-2">
            {move || match registry.get() {
                None => view! {
                    <p class="text-label-sm normal-case text-outline">"Loading catalog…"</p>
                }.into_any(),
                Some(items) => {
                    let names: HashMap<String, String> = items
                        .iter()
                        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                        .collect();
                    let items = StoredValue::new(items);
                    let names = StoredValue::new(names);
                    let pick_item = move |key: String, value: String| {
                        picks.update(|m| {
                            if value.is_empty() { m.remove(key.as_str()); }
                            else { m.insert(key.clone(), value.clone()); }
                        });
                        persist(&picks.get_untracked(), &items.get_value());
                    };
                    // T-686 — apply an ACCEPTED import. **This is the one-undo-step contract.**
                    //
                    // The three `set`s are signal writes and commit nothing; the single `persist`
                    // that follows is the only document mutation, and `persist` is one
                    // `editor_ops::set_loadout` is one `mission_history::after_local_edit`
                    // (`editor_ops.rs:1611`) is one undo step. So Ctrl+Z after an import restores
                    // the whole loadout the author had before it — not the last wear row of it.
                    // No new atomic-batch API was needed: the Arsenal's existing commit already
                    // takes the entire `SlotLoadoutV2` document in one call, which is exactly the
                    // shape an import wants. `tests::t686::the_import_applies_in_one_commit` pins it.
                    let apply_import = move |doc: ImportedLoadout, items: &[RegistryItem]| {
                        picks.set(doc.picks);
                        cargo.set(doc.cargo);
                        cargo_present.set(doc.cargo_present);
                        persist(&picks.get_untracked(), items);
                    };
                    // T-686 — the file picker. Same off-DOM programmatic idiom as the mission
                    // upload (`missions.rs:1875`) and the CMS hero upload (`content.rs:632`): a
                    // one-shot `<input type=file>` that never sits in the DOM, so there is no dead
                    // control in a panel most authors will never import into.
                    //
                    // Read → parse → validate all happen before a single signal is written; the
                    // apply above is the only writer, and it only ever sees an `Ok`.
                    let import_loadout = move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::closure::Closure;
                            use wasm_bindgen::JsCast;

                            let picker = web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| d.create_element("input").ok())
                                .and_then(|el| el.dyn_into::<web_sys::HtmlInputElement>().ok());
                            let Some(input) = picker else {
                                import_status.set(String::new());
                                import_refusals
                                    .set(vec!["Could not open the file picker.".to_string()]);
                                return;
                            };
                            input.set_type("file");
                            input.set_accept("application/json,.json");

                            let input_for_cb = input.clone();
                            let on_change = Closure::once(move |_ev: web_sys::Event| {
                                let Some(file) =
                                    input_for_cb.files().and_then(|list| list.item(0))
                                else {
                                    return;
                                };
                                let name = file.name();
                                import_refusals.set(Vec::new());
                                import_status.set(format!("Reading {name}…"));
                                leptos::task::spawn_local(async move {
                                    // `Blob::text()` is a Promise — the browser reads off disk on
                                    // its own thread and the tab stays interactive through it.
                                    let text = match wasm_bindgen_futures::JsFuture::from(
                                        file.text(),
                                    )
                                    .await
                                    {
                                        Ok(v) => v.as_string().unwrap_or_default(),
                                        Err(_) => {
                                            import_status.set(String::new());
                                            import_refusals.set(vec![format!(
                                                "Could not read {name}."
                                            )]);
                                            return;
                                        }
                                    };
                                    let its = items.get_value();
                                    match try_import(&text, &its, &compat.get_untracked()) {
                                        Ok(doc) => {
                                            let line = import_summary(
                                                &name,
                                                &doc,
                                                &export_modpack_id(&its),
                                            );
                                            apply_import(doc, &its);
                                            import_refusals.set(Vec::new());
                                            import_status.set(line);
                                        }
                                        Err(refusals) => {
                                            // The refusal contract, said first and said plainly:
                                            // a document that does not validate applies NOTHING.
                                            import_status.set(String::new());
                                            import_refusals.set(
                                                std::iter::once(format!(
                                                    "{name} was not applied — this loadout is unchanged.",
                                                ))
                                                .chain(refusals.into_iter().map(|e| e.message))
                                                .collect(),
                                            );
                                        }
                                    }
                                });
                            });
                            let _ = input.add_event_listener_with_callback(
                                "change",
                                on_change.as_ref().unchecked_ref(),
                            );
                            // One-shot listener outlives this frame — the picker is
                            // fire-and-forget (the `content.rs` contract).
                            on_change.forget();
                            input.click();
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // No DOM, no file picker, and no hosted document to import into.
                            let _ = (apply_import, import_status, import_refusals, items);
                        }
                    };
                    view! {
                        // Top badges: compat status (left) + live weight (right).
                        <div class="flex items-center justify-between">
                            {move || {
                                let s = compat.get().status;
                                let (cls, label) = match s {
                                    rules::CompatStatus::Ready => (
                                        "rounded border border-success/40 bg-success/10 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-success",
                                        "Compat active",
                                    ),
                                    rules::CompatStatus::Loading => (
                                        "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-on-surface-variant",
                                        "Compat loading…",
                                    ),
                                    rules::CompatStatus::Unavailable => (
                                        "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-outline",
                                        "Compat unavailable",
                                    ),
                                };
                                view! { <span class=cls data-compat-badge>{label}</span> }
                            }}
                            <div class="flex items-center gap-3">
                                // T-068.15.2 — per-container capacity readout (registry-only:
                                // max kg + grid W×H; absent values simply don't render).
                                {move || {
                                    let key = active_key.get();
                                    if !rules::CAPACITY_KEYS.contains(&key.as_str()) {
                                        return ().into_any();
                                    }
                                    let rn = picks.with(|m| m.get(key.as_str()).cloned()).filter(|v| !v.is_empty());
                                    let Some(rn) = rn else { return ().into_any() };
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let Some(it) = idx.get(rn.as_str()) else { return ().into_any() };
                                    let mut parts: Vec<String> = Vec::new();
                                    if let Some(kg) = it.max_weight_kg {
                                        parts.push(format!("max {kg} kg"));
                                    }
                                    if let (Some(w), Some(h)) = (it.cargo_grid_w, it.cargo_grid_h) {
                                        parts.push(format!("{w}\u{00d7}{h} grid"));
                                    }
                                    if parts.is_empty() {
                                        return ().into_any();
                                    }
                                    view! {
                                        <span
                                            data-capacity-badge
                                            class="rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm tabular-nums normal-case text-on-surface-variant"
                                        >
                                            {parts.join(" · ")}
                                        </span>
                                    }.into_any()
                                }}
                                {move || {
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let map = picks.get();
                                    let mut w = loadout_weight(&map, &idx);
                                    // T-197 — attachments hang off a weapon, not off a row, so
                                    // `loadout_weight` (which walks LOADOUT_ROWS) cannot see them.
                                    // A suppressor is 0.68 kg of real carried mass; omitting it
                                    // would make an "honest weight" readout quietly dishonest.
                                    // Scoped to weapons that are actually picked — that is exactly
                                    // the set `picks_to_loadout` persists attachments for.
                                    for &(key, _, _) in rules::WEAPON_SLOTS {
                                        if map.get(key).is_none_or(String::is_empty) {
                                            continue;
                                        }
                                        for rn in attachments_of(&map, key) {
                                            w.item_count += 1;
                                            match idx.get(rn.as_str()).and_then(|it| it.weight_kg) {
                                                Some(kg) => w.known_kg += kg,
                                                None => w.unknown_count += 1,
                                            }
                                        }
                                    }
                                    let w = format_loadout_weight(&w);
                                    view! {
                                        <p class="font-mono text-label-sm tabular-nums normal-case text-on-surface-variant">{w}</p>
                                    }
                                }}
                            </div>
                        </div>
                        <div class="grid h-[52vh] min-h-0 grid-cols-[44px_230px_minmax(0,1fr)_230px] gap-3">
                            // Region icon rail (14, RAIL order).
                            <div class="custom-scrollbar flex flex-col items-center gap-1 overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 py-1.5">
                                {rules::RAIL_REGIONS.iter().map(|r| {
                                    let key = r.key;
                                    view! {
                                        <button
                                            type="button"
                                            data-arsenal-rail=key
                                            aria-label=region_title(key)
                                            title=region_title(key)
                                            class=move || {
                                                let active = active_key.get() == key;
                                                let equipped = picks.with(|m| m.get(key).is_some_and(|v| !v.is_empty()));
                                                if active {
                                                    "flex size-8 items-center justify-center rounded-md bg-primary/25 text-primary"
                                                } else if equipped {
                                                    "flex size-8 items-center justify-center rounded-md text-primary/80 transition-colors hover:bg-white/10"
                                                } else {
                                                    "flex size-8 items-center justify-center rounded-md text-outline transition-colors hover:bg-white/10 hover:text-on-surface"
                                                }
                                            }
                                            on:click=move |_| active_key.set(key.to_string())
                                        >
                                            <span class="material-symbols-outlined text-[18px]">{region_icon(key)}</span>
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                            // Item list for the active region (filter + None + grouped options).
                            <div class="custom-scrollbar flex min-h-0 flex-col overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2">
                                {move || {
                                    let feed = compat.get();
                                    let map = picks.get();
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let key = active_key.get();
                                    let Some(row) = rules::LOADOUT_ROWS.iter().find(|r| r.key == key) else {
                                        return view! { <p class="text-label-sm text-outline">"—"</p> }.into_any();
                                    };
                                    let current = map.get(row.key).cloned().unwrap_or_default();
                                    let opts = row_options(row, &current, &map, &its, &idx, feed.ready_graph());
                                    let q = filter.get().trim().to_lowercase();
                                    let opts: Vec<_> = opts
                                        .into_iter()
                                        .filter(|o| q.is_empty() || o.label.to_lowercase().contains(&q))
                                        .collect();
                                    let count = opts.len();
                                    // Group by registry category (screen 04's WEAPONS/… headers).
                                    let mut groups: Vec<(String, Vec<rules::RowOption>)> = Vec::new();
                                    for o in opts {
                                        let cat = idx
                                            .get(o.value.as_str())
                                            .map(|it| it.category.to_uppercase())
                                            .unwrap_or_else(|| "OTHER".to_string());
                                        match groups.last_mut() {
                                            Some((c, list)) if *c == cat => list.push(o),
                                            _ => groups.push((cat, vec![o])),
                                        }
                                    }
                                    // T-197 — attachment faults are keyed on the WEAPON row, so
                                    // they surface on the row whose pick the author must change.
                                    // T-240 — over-capacity cargo joins them, keyed on the garment
                                    // row backing the container.
                                    // T-504 — and cargo with nowhere known to go, keyed on the
                                    // container's own wear row, which is the pick that fixes it.
                                    let kit = kit_default_items(&feed, asset_id.get_value().as_deref());
                                    let err = loadout_faults(&map, &cargo.get(), &feed, &idx, kit.as_ref())
                                        .into_iter()
                                        .find(|e| e.key == row.key)
                                        .map(|e| e.message);
                                    let row_key = row.key;
                                    let none_cls = if current.is_empty() {
                                        "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm bg-primary/15 text-primary"
                                    } else {
                                        "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                    };
                                    view! {
                                        <div class="flex items-center justify-between px-1 pb-1">
                                            <span class="text-label-sm font-semibold uppercase tracking-wider text-on-surface">{row.label}</span>
                                            <span class="font-mono text-label-sm text-outline">{count}</span>
                                        </div>
                                        <input
                                            type="search"
                                            aria-label=format!("Filter {}", row.label)
                                            placeholder=format!("Filter {}…", row.label.to_lowercase())
                                            prop:value=move || filter.get()
                                            on:input=move |ev| filter.set(event_target_value(&ev))
                                            class="mb-1.5 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface outline-none placeholder:text-outline focus:border-primary/60"
                                        />
                                        <button
                                            type="button"
                                            class=none_cls
                                            on:click=move |_| pick_item(row_key.to_string(), String::new())
                                        >
                                            <span>"— None —"</span>
                                            {current.is_empty().then(|| view! { <MaterialCheck /> })}
                                        </button>
                                        {groups.into_iter().map(|(cat, list)| view! {
                                            <p class="mt-1.5 px-1 font-mono text-[10px] tracking-widest text-outline uppercase">{cat}</p>
                                            {list.into_iter().map(|o| {
                                                let is_current = o.value == current;
                                                let cls = if is_current {
                                                    "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm bg-primary/15 text-primary"
                                                } else if o.incompatible {
                                                    "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm text-error transition-colors hover:bg-white/10"
                                                } else {
                                                    "flex w-full items-center justify-between rounded px-2 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                                };
                                                let value = o.value.clone();
                                                let data_value = o.value.clone();
                                                view! {
                                                    <button
                                                        type="button"
                                                        data-value=data_value
                                                        class=cls
                                                        on:click=move |_| pick_item(row_key.to_string(), value.clone())
                                                    >
                                                        <span class="truncate normal-case">{o.label.clone()}</span>
                                                        {is_current.then(|| view! { <MaterialCheck /> })}
                                                    </button>
                                                }
                                            }).collect_view()}
                                        }).collect_view()}
                                        {err.map(|m| view! {
                                            <p class="mt-1.5 px-1 text-label-sm normal-case text-error">{m}</p>
                                        })}
                                    }
                                        .into_any()
                                }}
                            </div>
                            // Center: the 3D doll (SVG paper-doll on create failure) + caption.
                            <div class="relative flex min-h-0 flex-col overflow-hidden rounded-lg bg-[#858fa1]">
                                <div class="relative min-h-0 flex-1">
                                    {move || doll_view(picks, active_key, names, doll_unavailable)}
                                </div>
                                <p class="pointer-events-none absolute inset-x-0 bottom-1 text-center font-mono text-label-sm text-surface-container-lowest">
                                    {move || {
                                        let key = active_key.get();
                                        let label = rules::LOADOUT_ROWS.iter().find(|r| r.key == key).map_or("", |r| r.label);
                                        let name = picks.with(|m| m.get(key.as_str()).cloned()).filter(|v| !v.is_empty())
                                            .map(|rn| names.with_value(|n| n.get(&rn).cloned().unwrap_or(rn)))
                                            .unwrap_or_else(|| "empty".to_string());
                                        format!("{label} — {name}")
                                    }}
                                </p>
                            </div>
                            // Compat panel: the active item + its dependent edge slots.
                            <div class="custom-scrollbar flex min-h-0 flex-col overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2.5">
                                {move || compat_panel(picks, active_key, compat, names, items, pick_item)}
                            </div>
                        </div>
                        // T-068.15.2 — container cargo editor (SlotLoadoutV2.cargo[]; seeded from
                        // character_default_cargo; warn-only weight/volume budget).
                        <div
                            data-cargo-editor
                            class="custom-scrollbar max-h-[22vh] overflow-y-auto rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2.5"
                        >
                            {move || cargo_panel(cargo, picks, items, names, persist_cargo)}
                        </div>
                        // Bottom: validation verdict + loadout download.
                        <div class="flex items-center justify-between gap-2">
                            {move || {
                                let feed = compat.get();
                                let map = picks.get();
                                let its = items.get_value();
                                // T-197 — a stranded attachment is a real loadout fault; the
                                // verdict badge counts it alongside the edge-row faults.
                                // T-240 — and over-capacity cargo alongside both.
                                // T-504 — and cargo the kit has nowhere to put, so the badge stops
                                // saying "Loadout valid" over rows nothing was going to deliver.
                                let kit = kit_default_items(&feed, asset_id.get_value().as_deref());
                                let errs = loadout_faults(&map, &cargo.get(), &feed, &index_by_name(&its), kit.as_ref());
                                if errs.is_empty() {
                                    view! {
                                        <span
                                            data-loadout-valid
                                            class="rounded border border-success/40 bg-success/10 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-success"
                                        >
                                            "Loadout valid"
                                        </span>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <span
                                            data-loadout-valid
                                            class="rounded border border-error-alert/40 bg-error/10 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-error-alert"
                                        >
                                            {format!("{} issue(s)", errs.len())}
                                        </span>
                                    }
                                        .into_any()
                                }
                            }}
                            // T-240 — the export refusal, said out loud next to the button that
                            // stopped working. The per-container reason (with its estimate
                            // caveat) is on the garment row and on this control's tooltip.
                            <div class="flex min-w-0 items-center gap-2">
                                {move || {
                                    let its = items.get_value();
                                    let refusals = rules::cargo_capacity_errors(
                                        &picks.get(), &cargo.get(), &index_by_name(&its),
                                    );
                                    if refusals.is_empty() {
                                        return ().into_any();
                                    }
                                    let n = refusals.len();
                                    let why = refusals
                                        .iter()
                                        .map(|e| e.message.as_str())
                                        .collect::<Vec<_>>()
                                        .join("\n\n");
                                    view! {
                                        <span
                                            data-export-blocked=n.to_string()
                                            title=why
                                            class="truncate text-label-sm normal-case text-error-alert"
                                        >
                                            {format!(
                                                "Export blocked — {n} container(s) over the catalogued capacity",
                                            )}
                                        </span>
                                    }
                                        .into_any()
                                }}
                                // T-686 — the other half of the round-trip. Never disabled: the
                                // gate is `try_import`, and an author with a bad file needs to be
                                // told WHY, which requires letting them pick it.
                                <button
                                    type="button"
                                    data-loadout-import
                                    class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10"
                                    on:click=import_loadout
                                >
                                    <span class="material-symbols-outlined text-[16px]">"upload"</span>
                                    "Import loadout JSON"
                                </button>
                                <button
                                    type="button"
                                    prop:disabled=move || {
                                        let its = items.get_value();
                                        !rules::cargo_capacity_errors(
                                            &picks.get(), &cargo.get(), &index_by_name(&its),
                                        )
                                            .is_empty()
                                    }
                                    class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:border-outline-variant/20 disabled:text-outline disabled:hover:bg-transparent"
                                    on:click=move |_| {
                                        // T-199 — the FILE contract, not the doc field. `picks_to_export`
                                        // writes `loadout-export.schema.json` v2; the old call wrote the
                                        // editor's `SlotLoadoutV2` dict, which fails both `oneOf` branches
                                        // and which the mod reader refuses. An empty Arsenal still exports:
                                        // a bare-soldier document is valid and says so (all-null wear, no
                                        // weapons), where the old "clear the field" `None` had to be papered
                                        // over with a hand-written literal that was itself non-conforming.
                                        //
                                        // T-240 — through `try_export`, not `picks_to_export`. The
                                        // `disabled` attribute is the affordance; THIS is the gate. A
                                        // refusal produces no bytes, so there is nothing to download.
                                        #[cfg(target_arch = "wasm32")]
                                        if let Ok(json) = try_export(
                                            &picks.get_untracked(),
                                            &cargo.get_untracked(),
                                            &items.get_value(),
                                            &export_modpack_id(&items.get_value()),
                                        ) {
                                            let _ = crate::mission_commands::download_json("loadout-export.json", &json);
                                        }
                                    }
                                >
                                    <span class="material-symbols-outlined text-[16px]">"download"</span>
                                    "Download loadout JSON"
                                </button>
                            </div>
                        </div>
                        // T-686 — the import outcome. A refusal lists EVERY reason and applied
                        // nothing, so there is no half-applied state to explain and no "partially
                        // imported" wording anywhere in it. An acceptance prints what landed.
                        {move || {
                            let refusals = import_refusals.get();
                            if !refusals.is_empty() {
                                let n = refusals.len() - 1; // the lead line is not a reason
                                return view! {
                                    <div
                                        data-import-refused=n.to_string()
                                        class="rounded-lg border border-error-alert/40 bg-error/10 p-2 text-label-sm normal-case text-error-alert"
                                    >
                                        <ul class="flex list-none flex-col gap-1">
                                            {refusals
                                                .into_iter()
                                                .map(|m| view! { <li>{m}</li> })
                                                .collect::<Vec<_>>()}
                                        </ul>
                                    </div>
                                }
                                    .into_any();
                            }
                            let status = import_status.get();
                            if status.is_empty() {
                                return ().into_any();
                            }
                            view! {
                                <p
                                    data-import-status
                                    class="text-label-sm normal-case text-on-surface-variant"
                                >
                                    {status}
                                </p>
                            }
                                .into_any()
                        }}
                        // T-503 — the persistence contract, said in the panel. The platform's one
                        // "not saved yet" signal is the `•` beside the mission title, and this tab
                        // renders under a full-viewport blur scrim that dims exactly that. So the
                        // Arsenal repeats it here rather than leaving the author to guess whether a
                        // pick stuck. `data-arsenal-persist` carries the state for the gate harness.
                        {move || {
                            commits.track();
                            let unsaved = mission_has_unsaved_work();
                            let (marker, cls, state) = if unsaved {
                                (
                                    "unsaved",
                                    "flex items-start gap-1.5 text-label-sm normal-case text-tactical-yellow",
                                    PERSIST_UNSAVED,
                                )
                            } else {
                                (
                                    "saved",
                                    "flex items-start gap-1.5 text-label-sm normal-case text-outline",
                                    PERSIST_CLEAN,
                                )
                            };
                            view! {
                                <p data-arsenal-persist=marker class=cls>
                                    <span class="material-symbols-outlined shrink-0 text-[14px]">
                                        {if unsaved { "cloud_upload" } else { "check_circle" }}
                                    </span>
                                    <span>{PERSIST_ALWAYS} " " {state}</span>
                                </p>
                            }
                        }}
                        <p class="text-label-sm normal-case text-outline">
                            "Weapon attachments are multi-select in the compat panel — pick a weapon region on the rail to see what it accepts. Container cargo (mags, medical, throwables) lives in the Cargo panel above — seeded from the character's engine defaults. Dedicated equipment wear rows (binoculars, radios, glasses) come with the equipment slice."
                        </p>
                    }.into_any()
                }
            }}
        </div>
    }
}

/// Registry kinds offered by the cargo "add" picker (worn/held gear stays on the wear
/// and weapon rows — cargo is what goes *inside* containers).
const CARGO_ADD_KINDS: &[&str] = &[
    "magazine",
    "ammo",
    "gear_item",
    "gear_throwable",
    "gear_explosive",
];

/// T-068.15.2 — the per-container cargo editor: rows (name × qty, stepper, remove),
/// an add picker, and the budget vs the garment's registry capacity.
///
/// T-240 — the container→worn-garment alias used to live here as a second copy of the rule
/// `arsenal_rules` already documents; it is now [`rules::cargo_garment`] alone, so the readout
/// and the block can never disagree about which garment backs a container.
fn cargo_panel(
    cargo: RwSignal<Vec<rules::CargoRow>>,
    picks: RwSignal<HashMap<String, String>>,
    items: StoredValue<Vec<RegistryItem>>,
    names: StoredValue<HashMap<String, String>>,
    on_change: impl Fn(&[RegistryItem]) + Copy + 'static,
) -> AnyView {
    let its = items.get_value();
    let idx = index_by_name(&its);
    let rows_now = cargo.get();
    let picks_now = picks.get();

    // Add-picker options: eligible kinds, concrete (non-abstract, non-variant), name-sorted.
    let mut addable: Vec<(String, String)> = its
        .iter()
        .filter(|it| CARGO_ADD_KINDS.contains(&it.kind.as_str()))
        .filter(|it| !it.r#abstract.unwrap_or(false) && it.variant_of.is_none())
        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
        .collect();
    addable.sort_by(|a, b| a.1.cmp(&b.1));
    let addable = StoredValue::new(addable);

    let groups = rules::CARGO_CONTAINERS
        .iter()
        .map(|container| {
            let container: &'static str = container;
            let garment_rn = rules::cargo_garment(&picks_now, container).map(|(_, rn)| rn);
            let rows: Vec<(usize, rules::CargoRow)> = rows_now
                .iter()
                .enumerate()
                .filter(|(_, r)| r.container == container)
                .map(|(i, r)| (i, r.clone()))
                .collect();
            if garment_rn.is_none() && rows.is_empty() {
                return ().into_any();
            }
            let garment_item = garment_rn.and_then(|rn| idx.get(rn).copied());
            let garment_label = garment_rn
                .map(|rn| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.to_string())))
                .unwrap_or_else(|| "no garment worn".to_string());
            let only_rows: Vec<rules::CargoRow> = rows.iter().map(|(_, r)| r.clone()).collect();
            let budget = rules::cargo_budget(&idx, garment_item, &only_rows);
            let budget_line = match (budget.max_weight, budget.max_volume) {
                (None, None) if only_rows.is_empty() => None,
                _ => {
                    let kg = match budget.max_weight {
                        Some(m) => format!("{:.1} / {m} kg", budget.weight),
                        None => format!("{:.1} kg", budget.weight),
                    };
                    let vol = match budget.max_volume {
                        Some(m) => format!("{:.0} / {m} cm³", budget.volume),
                        None => format!("{:.0} cm³", budget.volume),
                    };
                    Some((format!("{kg} · {vol}"), budget.over()))
                }
            };
            view! {
                <div class="mb-2 last:mb-0" data-cargo-container=container>
                    <div class="flex items-center justify-between px-1">
                        <span class="text-label-sm font-semibold uppercase tracking-wider text-on-surface">
                            {container} " — " <span class="normal-case font-normal text-on-surface-variant">{garment_label}</span>
                        </span>
                        {budget_line.map(|(text, over)| {
                            let cls = if over {
                                "font-mono text-label-sm tabular-nums normal-case text-error-alert"
                            } else {
                                "font-mono text-label-sm tabular-nums normal-case text-outline"
                            };
                            view! { <span class=cls data-cargo-budget=container>{text}</span> }
                        })}
                    </div>
                    {rows.into_iter().map(|(i, r)| {
                        let label = names.with_value(|n| n.get(&r.item).cloned().unwrap_or_else(|| r.item.clone()));
                        let qty = r.qty;
                        view! {
                            <div class="flex items-center justify-between gap-2 rounded px-2 py-0.5 hover:bg-white/5">
                                <span class="truncate text-label-sm normal-case text-on-surface-variant">{label}</span>
                                <span class="flex shrink-0 items-center gap-1">
                                    <button type="button" aria-label="Fewer" class="rounded px-1 font-mono text-label-sm text-outline hover:bg-white/10 hover:text-on-surface"
                                        on:click=move |_| {
                                            cargo.update(|c| { if let Some(r) = c.get_mut(i) { r.qty = (r.qty - 1).max(1); } });
                                            on_change(&items.get_value());
                                        }
                                    >"−"</button>
                                    <span class="min-w-[2ch] text-center font-mono text-label-sm tabular-nums text-on-surface">{qty}</span>
                                    <button type="button" aria-label="More" class="rounded px-1 font-mono text-label-sm text-outline hover:bg-white/10 hover:text-on-surface"
                                        on:click=move |_| {
                                            cargo.update(|c| { if let Some(r) = c.get_mut(i) { r.qty += 1; } });
                                            on_change(&items.get_value());
                                        }
                                    >"+"</button>
                                    <button type="button" aria-label="Remove" class="rounded px-1 font-mono text-label-sm text-outline hover:bg-white/10 hover:text-error"
                                        on:click=move |_| {
                                            cargo.update(|c| { c.remove(i); });
                                            on_change(&items.get_value());
                                        }
                                    >"✕"</button>
                                </span>
                            </div>
                        }
                    }).collect_view()}
                    <select
                        class="mt-0.5 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface-variant outline-none focus:border-primary/60"
                        aria-label=format!("Add cargo to {container}")
                        prop:value=""
                        on:change=move |ev| {
                            let rn = event_target_value(&ev);
                            if rn.is_empty() { return; }
                            cargo.update(|c| {
                                if let Some(row) = c.iter_mut().find(|r| r.container == container && r.item == rn) {
                                    row.qty += 1;
                                } else {
                                    c.push(rules::CargoRow { container: container.to_string(), item: rn.clone(), qty: 1 });
                                }
                            });
                            on_change(&items.get_value());
                        }
                    >
                        <option value="" selected>"+ Add item…"</option>
                        {addable.with_value(|a| a.iter().map(|(rn, label)| {
                            view! { <option value=rn.clone()>{label.clone()}</option> }
                        }).collect_view())}
                    </select>
                </div>
            }
            .into_any()
        })
        .collect_view();

    view! {
        <p class="px-1 pb-1 font-mono text-[10px] tracking-widest text-outline uppercase">"Cargo"</p>
        {groups}
    }
    .into_any()
}

/// The center doll: `ArsenalDoll` (wgpu) with the SVG `paper_doll` as the create-error fallback
/// (T-154 contract). Native shell: always the SVG (no GPU).
fn doll_view(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
    names: StoredValue<HashMap<String, String>>,
    unavailable: RwSignal<bool>,
) -> AnyView {
    #[cfg(target_arch = "wasm32")]
    {
        if !unavailable.get() {
            return view! {
                <crate::arsenal_doll::ArsenalDoll
                    picks
                    active_key
                    names
                    unavailable
                    on_select=Callback::new(move |key: String| active_key.set(key))
                />
            }
            .into_any();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (names, unavailable);
    paper_doll(picks, active_key).into_any()
}

/// Small check glyph for the current pick row.
#[component]
fn MaterialCheck() -> impl IntoView {
    view! { <span class="material-symbols-outlined shrink-0 text-[16px]">"check"</span> }
}

/// Rail tooltip title per region.
fn region_title(key: &str) -> &'static str {
    rules::LOADOUT_ROWS
        .iter()
        .find(|r| r.key == key)
        .map_or("", |r| r.label)
}

/// Rail icon per region (Material Symbols approximations of the screen-04 glyphs).
fn region_icon(key: &str) -> &'static str {
    match key {
        "primary" => "swords",
        "optic" => "filter_center_focus",
        "magazine" => "dataset",
        "launcher" => "rocket_launch",
        "handgun" => "front_hand",
        "throwable" => "bomb",
        "headCover" => "sports_motorsports",
        "jacket" => "apparel",
        "vest" => "shield",
        "armoredVest" => "security",
        "backpack" => "backpack",
        "handwear" => "waving_hand",
        "pants" => "accessibility",
        _ => "footprint", // boots
    }
}

/// T-197 — the **ATTACHMENTS** block of the compat panel: the `attachment_on_weapon` set the active
/// weapon accepts, rendered as toggles. This is the Arsenal's one multi-select surface, because it
/// is the one slot a weapon holds several of at once.
///
/// Returns `None` when the active region is not a weapon, when no weapon is picked, or when the
/// graph offers nothing **and** nothing is picked — so a family with no edges (vanilla
/// launcher/handgun/throwable all have zero) adds no empty section to the panel.
fn attachments_panel(
    active: &str,
    map: &HashMap<String, String>,
    feed: &CompatFeed,
    names: StoredValue<HashMap<String, String>>,
    items: StoredValue<Vec<RegistryItem>>,
    pick_item: impl Fn(String, String) + Copy + 'static,
) -> Option<AnyView> {
    let &(weapon_key, _, _) = rules::WEAPON_SLOTS.iter().find(|(k, _, _)| *k == active)?;
    let host = map.get(weapon_key).filter(|s| !s.is_empty())?.clone();
    let its = items.get_value();
    let idx = index_by_name(&its);
    // Synthesised here rather than added as a 15th `LOADOUT_ROWS` entry: the set must stay out of
    // the single-value row machinery (weight, validation, the doll rail all key off that table),
    // while still reusing the row RULES verbatim — graph-fed, abstract/variant filtered,
    // display-name sorted. `depends_on` is the weapon key, so the graph lookup is host-agnostic.
    let row = rules::LoadoutRow {
        key: "attachments",
        label: "Attachments",
        source: rules::RowSource::Edge {
            edge: ATTACHMENT_EDGE,
            depends_on: weapon_key,
        },
    };
    let mut opts = row_options(&row, "", map, &its, &idx, feed.ready_graph());
    let picked = attachments_of(map, weapon_key);
    let display =
        |rn: &str| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.to_string()));
    // A pick the option list dropped stays VISIBLE — deselecting it is the only way to remove it.
    // It is flagged only when the graph actually REJECTS it: an `abstract`/variant prefab the
    // filter hid is still a compatible pick, and an outage is not evidence of anything at all.
    for rn in &picked {
        if opts.iter().any(|o| &o.value == rn) {
            continue;
        }
        let ok = feed
            .ready_graph()
            .is_none_or(|g| g.accepts(&host, rn, ATTACHMENT_EDGE));
        opts.push(rules::RowOption {
            value: rn.clone(),
            label: if ok {
                display(rn)
            } else {
                format!("{} — incompatible", display(rn))
            },
            incompatible: !ok,
        });
    }
    if opts.is_empty() {
        return None;
    }
    let rows = opts
        .into_iter()
        .map(|o| {
            let selected = picked.contains(&o.value);
            let cls = match (selected, o.incompatible) {
                (true, true) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm bg-error/10 text-error",
                (true, false) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm bg-primary/15 text-primary",
                (false, true) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm text-error transition-colors hover:bg-white/10",
                (false, false) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface",
            };
            // The toggled set is computed HERE, not in the handler: `pick_item` is the one
            // persist path (`insert`-or-`remove` + one undo step), so a toggle is just a normal
            // pick whose value happens to be the packed set.
            let mut next = picked.clone();
            match next.iter().position(|p| *p == o.value) {
                Some(at) => {
                    next.remove(at);
                }
                None => next.push(o.value.clone()),
            }
            let packed = pack_attachments(&next);
            let akey = attachments_key(weapon_key);
            // `data-value` keeps the panel's uniform click contract (the smoke harness in
            // `tbd-tools` sweeps `[data-value]`); `data-attachment` additionally marks this as a
            // TOGGLE, since a second click removes rather than replaces. `resource_name` is unique
            // per registry row, so the extra nodes cannot shadow a weapon/optic lookup.
            let data_value = o.value.clone();
            let data_attachment = o.value.clone();
            view! {
                <button
                    type="button"
                    data-value=data_value
                    data-attachment=data_attachment
                    aria-pressed=selected.to_string()
                    class=cls
                    on:click=move |_| pick_item(akey.clone(), packed.clone())
                >
                    <span class="truncate normal-case">{o.label}</span>
                    {selected.then(|| view! { <MaterialCheck /> })}
                </button>
            }
        })
        .collect_view();
    Some(
        view! {
            <p class="mt-3 font-mono text-[10px] tracking-widest text-outline uppercase">
                "Attachments"
            </p>
            {rows}
        }
        .into_any(),
    )
}

/// The right compat panel: the active pick's display name, each edge slot that depends on the
/// active region (screen 04: OPTIC "Nothing compatible." / MAGAZINE list), and — for a weapon
/// region — the T-197 multi-select attachment set. Rows click-pick.
fn compat_panel(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
    compat: RwSignal<CompatFeed>,
    names: StoredValue<HashMap<String, String>>,
    items: StoredValue<Vec<RegistryItem>>,
    pick_item: impl Fn(String, String) + Copy + 'static,
) -> AnyView {
    let key = active_key.get();
    let map = picks.get();
    let host = map.get(key.as_str()).cloned().unwrap_or_default();
    let head = if host.is_empty() {
        format!("{} — empty", region_title(&key))
    } else {
        names.with_value(|n| n.get(&host).cloned().unwrap_or_else(|| host.clone()))
    };
    let dependents: Vec<&'static rules::LoadoutRow> = rules::LOADOUT_ROWS
        .iter()
        .filter(
            |r| matches!(r.source, rules::RowSource::Edge { depends_on, .. } if depends_on == key),
        )
        .collect();
    let feed = compat.get();
    let attachments = attachments_panel(&key, &map, &feed, names, items, pick_item);
    let body = if dependents.is_empty() {
        // "No dependent slots." is a claim about the whole panel, so it must not survive an
        // attachment set — a modded launcher has no edge ROWS but can still have attachments.
        if attachments.is_none() {
            view! {
                <p class="mt-2 text-label-sm normal-case text-outline">"No dependent slots."</p>
            }
            .into_any()
        } else {
            ().into_any()
        }
    } else {
        dependents
            .into_iter()
            .map(|row| {
                let rules::RowSource::Edge { edge, .. } = row.source else {
                    unreachable!()
                };
                let section = view! {
                    <p class="mt-3 font-mono text-[10px] tracking-widest text-outline uppercase">
                        {row.label}
                    </p>
                };
                let content = if host.is_empty() {
                    view! {
                        <p class="text-label-sm normal-case text-outline">
                            {format!("Pick a {} first.", region_title(&key).to_lowercase())}
                        </p>
                    }
                    .into_any()
                } else if let Some(g) = feed.ready_graph() {
                    let options = g.items_for(&host, edge);
                    if options.is_empty() {
                        view! {
                            <p class="text-label-sm normal-case text-outline">"Nothing compatible."</p>
                        }
                        .into_any()
                    } else {
                        let current = map.get(row.key).cloned().unwrap_or_default();
                        let row_key = row.key;
                        options
                            .into_iter()
                            .map(|rn| {
                                let label = names
                                    .with_value(|n| n.get(&rn).cloned().unwrap_or_else(|| rn.clone()));
                                let is_current = rn == current;
                                let cls = if is_current {
                                    "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm bg-primary/15 text-primary"
                                } else {
                                    "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                };
                                let data_value = rn.clone();
                                view! {
                                    <button
                                        type="button"
                                        data-value=data_value
                                        class=cls
                                        on:click=move |_| pick_item(row_key.to_string(), rn.clone())
                                    >
                                        <span class="truncate normal-case">{label}</span>
                                        {is_current.then(|| view! { <MaterialCheck /> })}
                                    </button>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }
                } else {
                    view! {
                        <p class="text-label-sm normal-case text-outline">"Compat unavailable."</p>
                    }
                    .into_any()
                };
                view! {
                    {section}
                    {content}
                }
                .into_any()
            })
            .collect::<Vec<_>>()
            .collect_view()
            .into_any()
    };
    view! {
        <p class="text-label-md font-semibold normal-case text-on-surface">{head}</p>
        {body}
        {attachments}
    }
    .into_any()
}

/// The Mode-D 2D **SVG paper-doll** (SoldierSilhouette.tsx port). Keyboard-accessible
/// `<g role="button">` hotspots per `DOLL_REGIONS` (optic/magazine nest on the rifle group); three
/// visual states — empty (dashed), equipped (`primary/15`), active (`primary/25`). A hotspot click
/// sets `active_key` (two-way synced with the row list); it never mutates the loadout itself.
fn paper_doll(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
) -> impl IntoView {
    // (key, label, svg path/rect element) — geometry adapted from the React ref (viewBox 360×640).
    // Each region is one `<g>` hotspot; `shape` is its clickable silhouette.
    struct Region {
        key: &'static str,
        shape: &'static str, // an SVG element string (rect/path) sans fill/stroke.
    }
    // Ordered back-to-front (paint order): backpack, body, wear, then the rifle group last.
    const REGIONS: &[Region] = &[
        Region {
            key: "backpack",
            shape: r#"<rect x="84" y="165" width="44" height="120" rx="12"/>"#,
        },
        Region {
            key: "launcher",
            shape: r#"<rect x="246" y="72" width="18" height="120" rx="6" transform="rotate(28 255 132)"/>"#,
        },
        Region {
            key: "jacket",
            shape: r#"<rect x="140" y="132" width="80" height="150" rx="10"/>"#,
        },
        Region {
            key: "pants",
            shape: r#"<rect x="146" y="282" width="68" height="196" rx="8"/>"#,
        },
        Region {
            key: "boots",
            shape: r#"<rect x="146" y="484" width="68" height="40" rx="6"/>"#,
        },
        Region {
            key: "handwear",
            shape: r#"<path d="M108 288 h22 v22 h-22 z M230 288 h22 v22 h-22 z"/>"#,
        },
        Region {
            key: "vest",
            shape: r#"<rect x="150" y="150" width="60" height="64" rx="6"/>"#,
        },
        Region {
            key: "armoredVest",
            shape: r#"<rect x="142" y="142" width="76" height="110" rx="8"/>"#,
        },
        Region {
            key: "headCover",
            shape: r#"<circle cx="180" cy="92" r="26"/>"#,
        },
        Region {
            key: "throwable",
            shape: r#"<rect x="112" y="326" width="26" height="30" rx="4"/>"#,
        },
        Region {
            key: "handgun",
            shape: r#"<rect x="222" y="312" width="26" height="34" rx="4"/>"#,
        },
    ];
    // The rifle group (primary + nested optic/magazine), drawn front-most.
    const RIFLE: &[Region] = &[
        Region {
            key: "primary",
            shape: r#"<rect x="96" y="322" width="150" height="14" rx="3"/>"#,
        },
        Region {
            key: "optic",
            shape: r#"<rect x="150" y="306" width="26" height="12" rx="3"/>"#,
        },
        Region {
            key: "magazine",
            shape: r#"<path d="M168 336 q6 26 18 30 l6 -4 q-10 -6 -12 -28 z"/>"#,
        },
    ];

    let hotspot = move |r: &'static Region| {
        let key = r.key;
        let cls = move || {
            let equipped = picks.with(|m| m.get(key).map(|v| !v.is_empty()).unwrap_or(false));
            let active = active_key.get() == key;
            let base = "cursor-pointer transition-colors";
            if active {
                format!("{base} fill-primary/25 stroke-primary [stroke-width:2.5]")
            } else if equipped {
                format!("{base} fill-primary/15 stroke-primary/60 [stroke-width:1.5]")
            } else {
                format!("{base} fill-on-surface/5 stroke-outline/50 [stroke-width:1.2] [stroke-dasharray:4_3]")
            }
        };
        let label = rules::row(key).map(|r| r.label).unwrap_or(key);
        // inject the shape verbatim; add the reactive class on the group.
        view! {
            <g
                role="button"
                tabindex="0"
                aria-label=label
                aria-pressed=move || (active_key.get() == key).to_string()
                class=cls
                on:click=move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); active_key.set(key.to_string()); }
                inner_html=r.shape
            ></g>
        }
    };

    view! {
        <svg viewBox="0 0 360 640" class="mx-auto h-[52vh] w-full" role="group" aria-label="Loadout paper-doll">
            // decorative head/neck (non-clickable)
            <circle cx="180" cy="92" r="22" class="fill-on-surface/10"></circle>
            <rect x="170" y="112" width="20" height="18" class="fill-on-surface/10"></rect>
            {REGIONS.iter().map(hotspot).collect_view()}
            {RIFLE.iter().map(hotspot).collect_view()}
        </svg>
    }
}

/// ═══════════ T-503 / T-601 — the shared Class-R scrubber (**cure 2**) ═══════════
///
/// A Class-R "pin" that does `include_str!("x.rs")` then `.contains("needle")` is the repo's
/// signature defect wearing a costume: it reports success over source it never proved was live.
/// The needle can sit in a comment, in a string literal, in a `#[cfg(any())]` item the build never
/// compiles, in an `if false { … }` block, or after a `return;`.
///
/// Five waves of pins tried to fix that by **blocklisting wrapper shapes** (`if false`,
/// `if true == false`, `loop { break; … }`, `#[cfg(any())]`, `while false`, `if !true`) and each
/// generation was walked around by the next spelling. Deciding reachability from source text is the
/// halting problem in a costume, so a blocklist can only ever be one round behind.
///
/// This module is the **cheap** answer: rather than enumerate wrappers, lex the file once and then
/// decide each construct *structurally* — a `cfg` predicate is evaluated as a predicate, an `if`
/// condition is constant-folded as an expression. Whitespace, spelling and nesting stop mattering
/// because nothing is matched literally. The expensive-but-sound answer is **cure 1**
/// (`mission_title_prefer::t570_tests`): lift the item out, compile it, *run* it, and assert on
/// behaviour. Dead code produces no behaviour, so cure 1 is closed by construction. Use cure 1 for
/// any invariant with a runtime signature; use this for pure source-shape invariants (a banned
/// literal, a wiring seam that has no callable surface).
///
/// # What this is honest about — the residual, restated at T-622
///
/// This is still a grep, so it still cannot decide reachability in general. What it *can* do is
/// remove the constructs it can prove dead **and treat the ones it cannot read as dead too**, so
/// that the direction of every mistake is a false RED rather than a false GREEN.
///
/// T-601 claimed to be fail-closed and was not. It removed a block only on a provable
/// `Some(false)`; every condition its evaluator could not parse fell through to "keep", which is
/// "report as live". Six wrappers walked past it on the real production files — measured, not
/// theorised — and three of them were named in T-601's own brief. The rule that replaced it is in
/// [`class_r_scrub::Scrub::kill_const_false_blocks`]: an `if`/`while` condition made **only** of
/// compile-time material ([`class_r_scrub::constant_shaped`]) that does not fold to `true` is
/// scrubbed, whatever shape it is. That is closed under wrappers nobody has invented yet, because a
/// wrapper built out of literals and `const`s cannot smuggle in a runtime name and still be a
/// wrapper.
///
/// **What genuinely remains, after the change and measured against the real sources:**
///
/// * **Build-conditional compilation.** `kill_dead_cfg_items` removes an item only when
///   [`cfg_eval`] proves the predicate false for *every* build. `#[cfg(feature = "nobody-enables-
///   this")]` and `#[cfg(target_arch = "…")]` are undecidable from source text alone and are
///   **kept**. This one is fail-open on purpose and it is the only one: scrubbing them would delete
///   the shipped wasm32 SPA, which is the branch these pins exist to examine. A needle parked under
///   an unenabled `feature` gate will still green a pin.
/// * **Runtime conditions that are never true in practice.** `if let` / `while let` patterns that
///   never match, `if flag_that_is_always_false()`, an opaque `const fn` predicate called on a
///   runtime path. These mention names the program computes, so they are not constant-shaped and
///   are kept — correctly, since a text pass cannot know the call always returns `false`.
/// * **Scope.** Binding collection is not scope-aware: a `const C: bool = false;` in one function
///   silences `if C` in another. The failure direction is a false strip → RED.
/// * **`unsafe`, panics, unreachable-by-typestate, and everything else the halting problem owns.**
///
/// Note what is **not** on that list any more: an expression the evaluator cannot fold. That used
/// to be the residual and it was the bug.
///
/// **What the calibration tests certify, and what they do not.** The
/// `the_*_rejects_every_dead_code_wrapper` batteries in `sse.rs`, `client.rs`, `content.rs`,
/// `event_hub.rs` and `mission_commands.rs` each run **twelve** enumerated shapes. Twelve shapes is
/// evidence about twelve shapes and nothing else — five previous waves were beaten by shape
/// thirteen. The property that covers the unnamed thirteenth is
/// [`class_r_scrub::constant_shaped`], and the test that states it as a *property* rather than a
/// list is `the_unknown_condition_fails_closed` in this file's own test module. A green battery
/// without that test would mean only that nobody had tried a new spelling yet.
///
/// Pins that cannot tolerate the residual above go to cure 1.
///
/// # The W77-F3 holes this closes
///
/// * `strip_cfg_any_items` matched the **literal** `"#[cfg(any())]"`, so `#[cfg( any() )]`,
///   `#[ cfg(any()) ]` and `#[cfg(all(any(), unix))]` all sailed through. [`cfg_eval`] now parses
///   the predicate.
/// * `strip_const_false_blocks` whitelisted **seven** condition spellings, so `if 1 > 2`,
///   `if std::hint::black_box(false)`, `while false` and `const C: bool = false; if C` all sailed
///   through. [`eval_bool`] now constant-folds the condition.
/// * `fn_body` took the **first** match of a marker, so a pristine shadow definition parked in a
///   never-called `mod` fed the pin a decoy. [`only_body`] refuses ambiguity.
///
/// # The T-622 holes this closes
///
/// * [`eval_bool`] folded each `const` initialiser against an **empty** const map, so
///   `const A: bool = false; const B: bool = A;` left `B` unknown and `if B { … }` was kept.
///   [`class_r_scrub::constants`] now iterates to a fixpoint.
/// * `{ false }`, `::std::hint::black_box(false)` — a block-expression initialiser and a leading
///   path `::` both lexed to unknown bytes. `lex` reads both now.
/// * `(true, false).1`, `1 + 1 > 3`, `false | false`, `[false, true][0]`, `(|| false)()` — the
///   evaluator still cannot read any of these, and no longer needs to: they name nothing the
///   program computes, so they fail closed.
#[cfg(test)]
pub(crate) mod class_r_scrub {
    use std::collections::{HashMap, HashSet};

    pub(crate) fn is_ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    /// `kw` occurs at `i` as a whole word.
    fn kw_at(c: &[char], i: usize, kw: &str) -> bool {
        let k: Vec<char> = kw.chars().collect();
        if i + k.len() > c.len() || c[i..i + k.len()] != k[..] {
            return false;
        }
        (i == 0 || !is_ident_char(c[i - 1]))
            && (i + k.len() >= c.len() || !is_ident_char(c[i + k.len()]))
    }

    fn blank(c: char) -> char {
        if c == '\n' {
            '\n'
        } else {
            ' '
        }
    }

    /// Index of the delimiter matching the one at `at`.
    fn balanced(c: &[char], at: usize, open: char, close: char) -> Option<usize> {
        debug_assert_eq!(c[at], open);
        let mut depth = 0usize;
        for (i, ch) in c.iter().enumerate().skip(at) {
            if *ch == open {
                depth += 1;
            } else if *ch == close {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Same-length copy of `chars` with comments blanked to spaces (newlines kept, so line numbers
    /// survive) and string/char literals blanked when `blank_literals`.
    ///
    /// Length preservation is the whole point: every structural decision below is taken on the
    /// literal-blanked copy, so a `{` inside a string or a `fn foo(` inside a doc comment can never
    /// steer brace balancing — while the indices still address the original text.
    fn mask(chars: &[char], blank_literals: bool) -> Vec<char> {
        let mut out: Vec<char> = Vec::with_capacity(chars.len());
        let mut i = 0usize;
        while i < chars.len() {
            // `// …`
            if chars[i] == '/' && chars.get(i + 1) == Some(&'/') {
                while i < chars.len() && chars[i] != '\n' {
                    out.push(blank(chars[i]));
                    i += 1;
                }
                continue;
            }
            // `/* … */`, nesting as rustc allows
            if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                let mut depth = 0usize;
                while i < chars.len() {
                    if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                        depth += 1;
                        out.push(' ');
                        out.push(' ');
                        i += 2;
                        continue;
                    }
                    if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                        depth -= 1;
                        out.push(' ');
                        out.push(' ');
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                        continue;
                    }
                    out.push(blank(chars[i]));
                    i += 1;
                }
                continue;
            }
            // literal spans: `r#"…"#`, `"…"`, `'c'`
            let span = literal_span(chars, i);
            if let Some(end) = span {
                for k in i..end {
                    out.push(if blank_literals {
                        blank(chars[k])
                    } else {
                        chars[k]
                    });
                }
                i = end;
                continue;
            }
            out.push(chars[i]);
            i += 1;
        }
        assert_eq!(
            out.len(),
            chars.len(),
            "T-601: scrubber mask lost alignment with the source — nothing built on it can be \
             trusted, so this is a hard failure rather than a silent skip"
        );
        out
    }

    /// End index (exclusive) of the string/char literal starting at `i`, if one does.
    /// A lifetime (`'a`) is deliberately not a literal.
    fn literal_span(chars: &[char], i: usize) -> Option<usize> {
        // r"…" / r#"…"# / r##"…"##
        if chars[i] == 'r' && (i == 0 || !is_ident_char(chars[i - 1])) {
            let mut j = i + 1;
            let mut hashes = 0usize;
            while j < chars.len() && chars[j] == '#' {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                let mut k = j + 1;
                while k < chars.len() {
                    if chars[k] == '"' && (1..=hashes).all(|h| chars.get(k + h) == Some(&'#')) {
                        return Some((k + hashes + 1).min(chars.len()));
                    }
                    k += 1;
                }
                return Some(chars.len());
            }
        }
        if chars[i] == '"' {
            let mut k = i + 1;
            while k < chars.len() {
                if chars[k] == '\\' {
                    k += 2;
                    continue;
                }
                if chars[k] == '"' {
                    return Some((k + 1).min(chars.len()));
                }
                k += 1;
            }
            return Some(chars.len());
        }
        if chars[i] == '\'' {
            let escaped = chars.get(i + 1) == Some(&'\\');
            let single = chars.get(i + 2) == Some(&'\'');
            if escaped || single {
                let mut k = i + 1;
                while k < chars.len() {
                    if chars[k] == '\\' {
                        k += 2;
                        continue;
                    }
                    if chars[k] == '\'' {
                        return Some((k + 1).min(chars.len()));
                    }
                    k += 1;
                }
                return Some(chars.len());
            }
        }
        None
    }

    /* ───────────────────────── `cfg` predicates, evaluated ───────────────────────── */

    /// `s` is exactly `name( … )` → the argument text.
    ///
    /// Word-bounded by construction: `cfg_attr(…)` does not strip as `cfg` because what follows the
    /// prefix is `_attr(`, not `(`.
    fn call_args(s: &str, name: &str) -> Option<String> {
        let t = s.trim();
        let rest = t.strip_prefix(name)?.trim_start();
        let inner = rest.strip_prefix('(')?.strip_suffix(')')?;
        let mut d = 0i32;
        for ch in inner.chars() {
            match ch {
                '(' => d += 1,
                ')' => {
                    d -= 1;
                    if d < 0 {
                        return None; // the ')' we stripped was not the matching one
                    }
                }
                _ => {}
            }
        }
        (d == 0).then(|| inner.to_string())
    }

    /// Split on commas that are not inside a nested group. Empty input → no arms (not one empty).
    fn split_top_commas(s: &str) -> Vec<String> {
        if s.trim().is_empty() {
            return Vec::new();
        }
        let mut parts = Vec::new();
        let mut d = 0i32;
        let mut cur = String::new();
        for ch in s.chars() {
            match ch {
                '(' | '[' | '{' => d += 1,
                ')' | ']' | '}' => d -= 1,
                ',' if d == 0 => {
                    parts.push(std::mem::take(&mut cur));
                    continue;
                }
                _ => {}
            }
            cur.push(ch);
        }
        if !cur.trim().is_empty() {
            parts.push(cur);
        }
        parts
    }

    /// Statically-decidable truth of a `cfg` predicate, with `leaf` deciding the atoms
    /// (`target_arch = "wasm32"`, `feature = "x"`, a bare ident).
    ///
    /// Follows rustc's own empty-list rule: `any()` is false, `all()` is true. That is what makes
    /// `#[cfg(any())]` the canonical never-compiled attribute — and what makes this a *parse*
    /// rather than the literal `"#[cfg(any())]"` match that `#[cfg( any() )]` walked straight past.
    fn cfg_eval_with(pred: &str, leaf: &dyn Fn(&str) -> Option<bool>) -> Option<bool> {
        let p = pred.trim();
        if p.is_empty() {
            return None;
        }
        for name in ["any", "all"] {
            if let Some(args) = call_args(p, name) {
                let vals: Vec<Option<bool>> = split_top_commas(&args)
                    .iter()
                    .map(|s| cfg_eval_with(s, leaf))
                    .collect();
                return if name == "any" {
                    if vals.iter().any(|v| *v == Some(true)) {
                        Some(true)
                    } else if vals.iter().all(|v| *v == Some(false)) {
                        Some(false) // includes `any()` — no arm is true
                    } else {
                        None
                    }
                } else if vals.iter().any(|v| *v == Some(false)) {
                    Some(false)
                } else if vals.iter().all(|v| *v == Some(true)) {
                    Some(true) // includes `all()` — no arm is false
                } else {
                    None
                };
            }
        }
        if let Some(args) = call_args(p, "not") {
            return cfg_eval_with(&args, leaf).map(|b| !b);
        }
        leaf(p)
    }

    /// Truth of a `cfg` predicate for **any** build. `None` = build-dependent, so **leave it
    /// alone** (`target_arch = "wasm32"` and `feature = "x"` are real production code).
    pub(crate) fn cfg_eval(pred: &str) -> Option<bool> {
        cfg_eval_with(pred, &|_| None)
    }

    /// Truth of a `cfg` predicate **for the wasm32 SPA build** — the build that actually ships.
    /// Only `target_arch` is decided; everything else stays unknown, which callers must treat as
    /// a refusal rather than a default.
    pub(crate) fn cfg_eval_wasm(pred: &str) -> Option<bool> {
        cfg_eval_with(pred, &|atom| {
            let (k, v) = atom.split_once('=')?;
            (k.trim() == "target_arch").then(|| v.trim().trim_matches('"') == "wasm32")
        })
    }

    /// Any whole-word identifier in the `cfg` family — `cfg`, `cfg_attr`, `cfg_match`, whatever
    /// the next one is called. The prefix rule is deliberate: a defence that knows only the exact
    /// spelling `cfg` is the same class of miss as the literal `"#[cfg(any())]"` match it replaced.
    pub(crate) fn mentions_cfg_family(src: &str) -> bool {
        let c: Vec<char> = src.chars().collect();
        let mut i = 0;
        while i < c.len() {
            if is_ident_char(c[i]) && (i == 0 || !is_ident_char(c[i - 1])) {
                let s = i;
                while i < c.len() && is_ident_char(c[i]) {
                    i += 1;
                }
                let w: String = c[s..i].iter().collect();
                if w == "cfg" || w.starts_with("cfg_") {
                    return true;
                }
                continue;
            }
            i += 1;
        }
        false
    }

    /// Resolve every `#[cfg(…)]` inside `item` **as the wasm32 SPA build sees it**: keep what wasm
    /// compiles, delete what it does not, and refuse anything undecidable.
    ///
    /// This is the seam that lets **cure 1** (compile-and-run) reach code that only exists on
    /// wasm32. `mission_title_prefer`'s harness refuses `cfg` inside a pinned item outright,
    /// because there the wire is unconditional and a `cfg` could only be a decoy. On this page the
    /// live branch *is* the `#[cfg(target_arch = "wasm32")]` one, so refusing would mean never
    /// pinning it at all.
    ///
    /// The transformation is narrow on purpose and stated in full:
    ///
    /// * a `cfg` that is **true** on wasm32 → the attribute is removed, the item is kept verbatim;
    /// * a `cfg` that is **false** on wasm32 → the attribute *and its item* are removed, exactly as
    ///   the shipped build removes them;
    /// * anything else (`feature = …`, a bare ident, `cfg_attr`) → **panic**. An undecidable gate
    ///   means the harness and the shipped build could disagree, and a pin that runs a different
    ///   program from the one that ships is the defect, not the fix.
    ///
    /// The final assertion is the belt: no `cfg` of any spelling survives into the code that gets
    /// compiled and run.
    pub(crate) fn resolve_wasm_cfg(item: &str) -> String {
        let chars: Vec<char> = item.chars().collect();
        let scan = mask(&chars, true);
        let mut out = chars.clone();
        let mut i = 0usize;
        while i < scan.len() {
            if scan[i] != '#' {
                i += 1;
                continue;
            }
            let mut j = i + 1;
            if scan.get(j) == Some(&'!') {
                j += 1;
            }
            if scan.get(j) != Some(&'[') {
                i += 1;
                continue;
            }
            let Some(close) = balanced(&scan, j, '[', ']') else {
                i += 1;
                continue;
            };
            // Literals intact here: the predicate is `target_arch = "wasm32"`.
            let inner: String = chars[j + 1..close].iter().collect();
            if let Some(pred) = call_args(&inner, "cfg") {
                match cfg_eval_wasm(&pred) {
                    Some(true) => {
                        for k in i..=close {
                            out[k] = blank(out[k]);
                        }
                    }
                    Some(false) => {
                        let end = item_end_after(&scan, close + 1);
                        for k in i..end {
                            out[k] = blank(out[k]);
                        }
                    }
                    None => panic!(
                        "T-601: `#[cfg({pred})]` inside a cure-1 pinned item cannot be resolved \
                         for the wasm32 build. This pin compiles and runs the item to prove the \
                         path is live, so a gate the harness cannot decide would let it run a \
                         different program from the one that ships. Move the conditional out of \
                         the pinned item, or teach `cfg_eval_wasm` the atom."
                    ),
                }
            }
            i = close + 1;
        }
        let resolved: String = out.into_iter().collect();
        assert!(
            !mentions_cfg_family(&resolved),
            "T-601: conditional compilation survived resolution:\n{resolved}"
        );
        resolved
    }

    /* ─────────────────── boolean conditions, constant-folded ─────────────────── */

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum Val {
        B(bool),
        N(f64),
        /// This pass could not decide the expression. **Not** a value — a refusal. Whether a `U`
        /// keeps a block or removes it is decided by [`constant_shaped`], never by defaulting.
        U,
    }

    #[derive(Debug, Clone, PartialEq)]
    enum Tok {
        Ident(String),
        Num(f64),
        Bool(bool),
        Op(&'static str),
        /// A byte the grammar does not model — `+`, `|`, `.`, `^`. Its presence is precisely the
        /// evaluator admitting it cannot read the expression, so it must never be shrugged off.
        Other,
    }

    /// Cast targets, so `x as u8` does not read as a runtime identifier.
    const PRIMITIVE_TYPES: &[&str] = &[
        "bool", "char", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
        "u128", "usize", "f32", "f64",
    ];

    /// What the *compiler* decides, as opposed to what the program computes.
    ///
    /// The split is the whole fail-closed mechanism. `known` is what this pass folded. `opaque` is
    /// the set of names that are compile-time constant **by Rust's own rules** — every `const` and
    /// `static`, plus a `let` whose initialiser is itself made only of compile-time material —
    /// which this pass could **not** fold. A condition gated on an `opaque` name is a condition
    /// whose truth was fixed at compile time and which this evaluator failed to read: exactly the
    /// case that must not be reported as live.
    #[derive(Debug, Clone, Default, PartialEq)]
    pub(crate) struct Consts {
        known: HashMap<String, Val>,
        opaque: HashSet<String>,
    }

    impl Consts {
        /// An identifier the compiler resolves: a folded constant, an unfolded-but-constant name,
        /// a primitive cast target, or a call this pass folds through ([`transparent_call`]).
        fn is_compile_time(&self, name: &str) -> bool {
            let last = name.rsplit("::").next().unwrap_or(name);
            PRIMITIVE_TYPES.contains(&name)
                || matches!(last, "black_box" | "identity")
                || self.known.contains_key(name)
                || self.opaque.contains(name)
        }
    }

    /// `expr` is built **only** out of material the compiler decides: literals, operators, and
    /// identifiers that [`Consts::is_compile_time`] recognises.
    ///
    /// This is the predicate that lets the scrubber fail closed without deleting the program. A
    /// condition containing a runtime name (`resp`, `loading`, a method, an `if let` pattern) is
    /// genuinely conditional and must be left alone. A condition containing *no* runtime name is a
    /// compile-time constant whatever else is in it — `(true, false).1`, `1 + 1 > 3`,
    /// `false | false`, `[false, true][0]`, `(|| false)()` — so if [`eval_bool`] could not fold it,
    /// the failure is the evaluator's, not the code's, and the block is treated as possibly dead.
    ///
    /// Note what is **not** enumerated here: the operators. `Tok::Other` — the evaluator's own
    /// admission that it met a byte it does not model — does not disqualify an expression from
    /// being constant-shaped. That is the inversion. Every previous round of this defect lost by
    /// growing a list of shapes; this predicate is closed under shapes nobody has thought of yet,
    /// because a wrapper made of literals cannot smuggle in a runtime name and stay a wrapper.
    fn constant_shaped(expr: &str, consts: &Consts) -> bool {
        let toks = lex(&fold_cfg_macros(expr));
        !toks.is_empty()
            && toks.iter().all(|t| match t {
                Tok::Ident(name) => consts.is_compile_time(name),
                _ => true,
            })
    }

    fn lex(expr: &str) -> Vec<Tok> {
        const SUFFIXES: &[&str] = &[
            "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
            "f32", "f64",
        ];
        let c: Vec<char> = expr.chars().collect();
        let mut t = Vec::new();
        let mut i = 0usize;
        while i < c.len() {
            if c[i].is_whitespace() {
                i += 1;
                continue;
            }
            if c[i].is_ascii_digit() {
                let s = i;
                while i < c.len() && (c[i].is_ascii_digit() || c[i] == '_' || c[i] == '.') {
                    i += 1;
                }
                let ns = i;
                while i < c.len() && is_ident_char(c[i]) {
                    i += 1;
                }
                let lit: String = c[s..ns].iter().filter(|x| **x != '_').collect();
                let suffix: String = c[ns..i].iter().collect();
                if !suffix.is_empty() && !SUFFIXES.contains(&suffix.as_str()) {
                    t.push(Tok::Other);
                    continue;
                }
                t.push(lit.parse::<f64>().map(Tok::Num).unwrap_or(Tok::Other));
                continue;
            }
            // A **leading** `::` is part of the path, not punctuation. `::std::hint::black_box`
            // names the same function as `std::hint::black_box`; lexing the two colons as unknown
            // bytes was enough to make the whole expression undecidable, which used to mean
            // "keep the block". Skipped, not emitted, so the path text still matches a const name.
            let leading_path = c[i] == ':'
                && c.get(i + 1) == Some(&':')
                && c.get(i + 2).is_some_and(|x| is_ident_char(*x));
            if leading_path {
                i += 2;
            }
            if leading_path || is_ident_char(c[i]) {
                let s = i;
                while i < c.len() {
                    if is_ident_char(c[i]) {
                        i += 1;
                    } else if c[i] == ':' && c.get(i + 1) == Some(&':') {
                        i += 2;
                    } else {
                        break;
                    }
                }
                let w: String = c[s..i].iter().collect();
                t.push(match w.as_str() {
                    "true" => Tok::Bool(true),
                    "false" => Tok::Bool(false),
                    "as" => Tok::Op("as"),
                    _ => Tok::Ident(w),
                });
                continue;
            }
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            let two_op = match two.as_str() {
                "&&" => Some("&&"),
                "||" => Some("||"),
                "==" => Some("=="),
                "!=" => Some("!="),
                "<=" => Some("<="),
                ">=" => Some(">="),
                _ => None,
            };
            if let Some(op) = two_op {
                t.push(Tok::Op(op));
                i += 2;
                continue;
            }
            t.push(match c[i] {
                '!' => Tok::Op("!"),
                '<' => Tok::Op("<"),
                '>' => Tok::Op(">"),
                '(' => Tok::Op("("),
                ')' => Tok::Op(")"),
                ',' => Tok::Op(","),
                // `const NEVER: bool = { false };` — a block whose only expression is its tail.
                '{' => Tok::Op("{"),
                '}' => Tok::Op("}"),
                _ => Tok::Other,
            });
            i += 1;
        }
        t
    }

    struct Parser<'a> {
        t: Vec<Tok>,
        i: usize,
        consts: &'a Consts,
    }

    impl Parser<'_> {
        fn peek(&self) -> Option<&Tok> {
            self.t.get(self.i)
        }
        fn eat(&mut self, op: &str) -> bool {
            if matches!(self.peek(), Some(Tok::Op(x)) if *x == op) {
                self.i += 1;
                true
            } else {
                false
            }
        }
        fn or(&mut self) -> Val {
            let mut l = self.and();
            while self.eat("||") {
                let r = self.and();
                l = match (l, r) {
                    (Val::B(true), _) | (_, Val::B(true)) => Val::B(true),
                    (Val::B(a), Val::B(b)) => Val::B(a || b),
                    _ => Val::U,
                };
            }
            l
        }
        fn and(&mut self) -> Val {
            let mut l = self.cmp();
            while self.eat("&&") {
                let r = self.cmp();
                l = match (l, r) {
                    (Val::B(false), _) | (_, Val::B(false)) => Val::B(false),
                    (Val::B(a), Val::B(b)) => Val::B(a && b),
                    _ => Val::U,
                };
            }
            l
        }
        fn cmp(&mut self) -> Val {
            let l = self.unary();
            for op in ["==", "!=", "<=", ">=", "<", ">"] {
                if self.eat(op) {
                    let r = self.unary();
                    return compare(op, l, r);
                }
            }
            l
        }
        fn unary(&mut self) -> Val {
            if self.eat("!") {
                return match self.unary() {
                    Val::B(b) => Val::B(!b),
                    _ => Val::U,
                };
            }
            let v = self.primary();
            // `<expr> as bool` / `as u8` — the cast target is an identifier; a non-identifier
            // target is something this pass does not model, so the whole expression is unknown.
            let mut v = v;
            while self.eat("as") {
                match self.peek() {
                    Some(Tok::Ident(ty)) => {
                        if ty != "bool" {
                            v = Val::U;
                        }
                        self.i += 1;
                    }
                    _ => return Val::U,
                }
            }
            v
        }
        fn primary(&mut self) -> Val {
            match self.t.get(self.i).cloned() {
                Some(Tok::Bool(b)) => {
                    self.i += 1;
                    Val::B(b)
                }
                Some(Tok::Num(n)) => {
                    self.i += 1;
                    Val::N(n)
                }
                Some(Tok::Op("(")) => {
                    self.i += 1;
                    let v = self.or();
                    if !self.eat(")") {
                        return Val::U;
                    }
                    v
                }
                // `{ <expr> }` — a block whose value is its tail expression. `const NEVER: bool =
                // { false };` was a survivor purely because `{` lexed as an unknown byte.
                // A block with statements in it stops here and the trailing-token check refuses.
                Some(Tok::Op("{")) => {
                    self.i += 1;
                    let v = self.or();
                    if !self.eat("}") {
                        return Val::U;
                    }
                    v
                }
                Some(Tok::Ident(name)) => {
                    self.i += 1;
                    let macro_bang = self.eat("!");
                    if self.eat("(") {
                        let mut args = Vec::new();
                        if !self.eat(")") {
                            loop {
                                args.push(self.or());
                                if self.eat(")") {
                                    break;
                                }
                                if !self.eat(",") {
                                    return Val::U;
                                }
                                if self.eat(")") {
                                    break;
                                }
                            }
                        }
                        return if macro_bang {
                            Val::U // `cfg!(…)` is folded before lexing; every other macro is opaque
                        } else {
                            transparent_call(&name, &args)
                        };
                    }
                    if macro_bang {
                        return Val::U;
                    }
                    self.consts.known.get(&name).copied().unwrap_or(Val::U)
                }
                _ => {
                    self.i = self.t.len();
                    Val::U
                }
            }
        }
    }

    /// Calls that are the identity on their argument, so the argument's constness passes through.
    ///
    /// `std::hint::black_box` is the interesting one: it exists precisely to hide a value from the
    /// optimiser, which is what made `if std::hint::black_box(false)` a working decoy against a
    /// condition **whitelist**. It does not hide anything from a reader, and it does not change the
    /// value — so folding through it is the correct reading, not a special case bolted on.
    fn transparent_call(path: &str, args: &[Val]) -> Val {
        let last = path.rsplit("::").next().unwrap_or(path);
        if args.len() == 1 && matches!(last, "black_box" | "identity") {
            return args[0];
        }
        Val::U
    }

    fn compare(op: &str, l: Val, r: Val) -> Val {
        match (l, r) {
            (Val::N(a), Val::N(b)) => Val::B(match op {
                "==" => a == b,
                "!=" => a != b,
                "<=" => a <= b,
                ">=" => a >= b,
                "<" => a < b,
                _ => a > b,
            }),
            (Val::B(a), Val::B(b)) => match op {
                "==" => Val::B(a == b),
                "!=" => Val::B(a != b),
                _ => Val::U,
            },
            _ => Val::U,
        }
    }

    /// Replace every `cfg!(…)` with the literal its predicate evaluates to, so the expression
    /// parser never has to model `any()`/`all()` twice.
    fn fold_cfg_macros(expr: &str) -> String {
        let c: Vec<char> = expr.chars().collect();
        let mut out = String::with_capacity(expr.len());
        let mut i = 0usize;
        while i < c.len() {
            if kw_at(&c, i, "cfg") && c.get(i + 3) == Some(&'!') {
                let mut j = i + 4;
                while j < c.len() && c[j].is_whitespace() {
                    j += 1;
                }
                if c.get(j) == Some(&'(') {
                    if let Some(close) = balanced(&c, j, '(', ')') {
                        let pred: String = c[j + 1..close].iter().collect();
                        match cfg_eval(&pred) {
                            Some(true) => out.push_str("true"),
                            Some(false) => out.push_str("false"),
                            None => out.push_str("__unknown_cfg__"),
                        }
                        i = close + 1;
                        continue;
                    }
                }
            }
            out.push(c[i]);
            i += 1;
        }
        out
    }

    /// Constant-fold an expression to a bool **or a number** — numbers so that
    /// `const LIMIT: usize = 5; if LIMIT > 3` folds instead of being scrubbed as an undecidable
    /// constant. `None` is a refusal, never a value.
    fn eval_value(expr: &str, consts: &Consts) -> Option<Val> {
        let folded = fold_cfg_macros(expr);
        let mut p = Parser {
            t: lex(&folded),
            i: 0,
            consts,
        };
        let v = p.or();
        // Trailing tokens mean the grammar did not describe this expression; refuse rather than
        // act on a partial read — a partial read is exactly the defect this file exists to remove.
        if p.i != p.t.len() {
            return None;
        }
        match v {
            Val::U => None,
            v => Some(v),
        }
    }

    /// Constant-fold a boolean condition.
    ///
    /// `None` means **this evaluator could not read the expression** — it does not mean "live".
    /// Callers must decide the unknown case explicitly; [`Scrub::kill_const_false_blocks`] does it
    /// with [`constant_shaped`].
    pub(crate) fn eval_bool(expr: &str, consts: &Consts) -> Option<bool> {
        match eval_value(expr, consts)? {
            Val::B(b) => Some(b),
            _ => None,
        }
    }

    /// One `const` / `static` / `let` binding site, harvested textually.
    struct Binding {
        name: String,
        expr: String,
        /// `const` or `static`: the compiler fixes its value, so the *name* is compile-time
        /// material whether or not this pass can fold the initialiser.
        compile_time: bool,
        /// The value may be trusted: `: bool`-annotated, or a `const`/`static` of any type, or a
        /// bare `true`/`false`. An un-annotated `let x = some_call();` is not a constant just
        /// because the call is opaque.
        trusted: bool,
    }

    /// Every `const NAME[: T] = …;` / `static …` / `let …` binding in `scan`, in source order.
    ///
    /// Deliberately conservative: `mut` bindings are skipped (they can be reassigned out of sight).
    /// This pass is not scope-aware, so the failure direction is a *false* strip — which turns a
    /// pin RED, loudly, rather than green.
    ///
    /// # The bug this scan had, found by running the battery against real files
    ///
    /// The cursor used to resume at the **end of the initializer** after recording a binding, which
    /// is correct for finding the next *sibling* binding and catastrophic for anything nested: a
    /// `let run = async { … };` or a `let send = move |t| { … };` swallowed its entire body, so no
    /// binding inside it was ever seen. `sse.rs`, `client.rs` and `arsenal.rs` all wrap their live
    /// path in exactly that shape, and a `const C: bool = false; if C { … }` planted inside one of
    /// them survived scrubbing and greened the pin — measured, not theorised. The cursor now
    /// advances one keyword at a time, so a nested binding is just another binding.
    fn binding_sites(scan: &[char]) -> Vec<Binding> {
        let mut sites: Vec<Binding> = Vec::new();
        let n = scan.len();
        let mut i = 0usize;
        while i < n {
            let Some(kw) = ["const", "static", "let"]
                .iter()
                .find(|k| kw_at(scan, i, k))
                .copied()
            else {
                i += 1;
                continue;
            };
            let mut j = i + kw.len();
            while j < n && scan[j].is_whitespace() {
                j += 1;
            }
            if kw_at(scan, j, "mut") {
                i += kw.len();
                continue; // reassignable — out of scope for a text pass
            }
            let s = j;
            while j < n && is_ident_char(scan[j]) {
                j += 1;
            }
            if j == s {
                i += kw.len();
                continue;
            }
            let name: String = scan[s..j].iter().collect();
            while j < n && scan[j].is_whitespace() {
                j += 1;
            }
            let compile_time = kw != "let";
            let mut annotated = false;
            if scan.get(j) == Some(&':') {
                j += 1;
                while j < n && scan[j].is_whitespace() {
                    j += 1;
                }
                let ts = j;
                while j < n && is_ident_char(scan[j]) {
                    j += 1;
                }
                let ty: String = scan[ts..j].iter().collect();
                // A non-`bool` `let` annotation is a runtime binding this pass has no business
                // folding. A non-`bool` `const`/`static` is still compile-time, and folding its
                // number is what keeps `const LIMIT: usize = 5; if LIMIT > 3` out of the
                // fail-closed path.
                if ty != "bool" && !compile_time {
                    i += kw.len();
                    continue;
                }
                annotated = ty == "bool";
                while j < n && scan[j].is_whitespace() {
                    j += 1;
                }
            }
            if scan.get(j) != Some(&'=') || scan.get(j + 1) == Some(&'=') {
                i += kw.len();
                continue;
            }
            j += 1;
            let es = j;
            let mut d = 0i32;
            while j < n {
                match scan[j] {
                    '(' | '[' | '{' => d += 1,
                    ')' | ']' | '}' => d -= 1,
                    ';' if d <= 0 => break,
                    _ => {}
                }
                j += 1;
            }
            let expr: String = scan[es..j.min(n)].iter().collect();
            let trusted = compile_time || annotated || matches!(expr.trim(), "true" | "false");
            sites.push(Binding {
                name,
                expr,
                compile_time,
                trusted,
            });
            // One keyword forward, NOT to the end of the initializer — see the note above.
            i += kw.len();
        }
        sites
    }

    /// How many rounds of const-to-const substitution to run. A `const B = A; const A = false;`
    /// chain needs one round per link, and the links can appear in any order — but a real chain is
    /// two or three long, and an unbounded loop inside a test harness is its own defect.
    const CONST_FOLD_ROUNDS: usize = 8;

    /// The compile-time constants of `scan`: what folded, and what provably did not.
    ///
    /// # Why this is a fixpoint and not one pass
    ///
    /// T-601 evaluated every initialiser against an **empty** const map
    /// (`eval_bool(&expr, &HashMap::new())`), so `const A: bool = false; const B: bool = A;` left
    /// `B` unknown — and unknown meant the `if B { … }` block was kept, which greened the SSE
    /// abort pin over a dead signal wire on the real `sse.rs`. Measured, not theorised. One extra
    /// hop was all the indirection it took. Iterating to a fixpoint costs nothing and removes the
    /// whole family rather than the one spelling that was reported.
    ///
    /// # Why the unfolded names are kept rather than dropped
    ///
    /// `opaque` is the fail-closed half. A `const`/`static` is compile-time **by definition**, so a
    /// `const` this pass cannot fold is a constant it failed to read, not a runtime value — and
    /// [`constant_shaped`] uses that to scrub the block instead of trusting it. A `let` earns the
    /// same treatment only when its initialiser is itself made of compile-time material, because a
    /// `let ok = resp.ok();` genuinely is runtime and scrubbing `if ok { … }` would delete the
    /// program.
    fn constants(scan: &[char]) -> Consts {
        let sites = binding_sites(scan);
        let mut consts = Consts {
            known: HashMap::new(),
            opaque: sites
                .iter()
                .filter(|b| b.compile_time)
                .map(|b| b.name.clone())
                .collect(),
        };
        for _ in 0..CONST_FOLD_ROUNDS {
            let mut round: HashMap<String, Option<Val>> = HashMap::new();
            for b in &sites {
                let v = b.trusted.then(|| eval_value(&b.expr, &consts)).flatten();
                // A name bound twice to different values tells this pass nothing it can use.
                round
                    .entry(b.name.clone())
                    .and_modify(|e| {
                        if *e != v {
                            *e = None;
                        }
                    })
                    .or_insert(v);
            }
            let known: HashMap<String, Val> = round
                .into_iter()
                .filter_map(|(k, v)| v.map(|x| (k, x)))
                .collect();
            if known == consts.known {
                break;
            }
            consts.known = known;
        }
        // A `let` whose initialiser mentions nothing the program computes is a constant wearing a
        // `let`: `let w: bool = (true, false).1;` must not launder a dead block into a live one.
        for b in &sites {
            if !consts.known.contains_key(&b.name) && constant_shaped(&b.expr, &consts) {
                consts.opaque.insert(b.name.clone());
            }
        }
        let known = std::mem::take(&mut consts.known);
        consts.opaque.retain(|n| !known.contains_key(n));
        consts.known = known;
        consts
    }

    /* ─────────────────────────── the scrubber itself ─────────────────────────── */

    struct Scrub {
        /// Structure is read from here only: comments and literals blanked, length preserved.
        scan: Vec<char>,
        /// What the pin ends up greping.
        out: Vec<char>,
    }

    impl Scrub {
        /// Blank a range in both buffers, so later passes cannot see what an earlier pass removed
        /// and brace balance is preserved (a balanced region blanked stays balanced).
        fn kill(&mut self, range: std::ops::Range<usize>) {
            for k in range {
                if k < self.scan.len() {
                    self.scan[k] = blank(self.scan[k]);
                    self.out[k] = blank(self.out[k]);
                }
            }
        }

        /// Everything from the crate's `#[cfg(test)]` boundary onward, so a pin can never read its
        /// own assertion strings back as evidence.
        fn cut_test_module(&mut self) {
            let needle: Vec<char> = "#[cfg(test)]".chars().collect();
            if let Some(at) = find_from(&self.scan, &needle, 0) {
                self.kill(at..self.scan.len());
            }
        }

        /// Remove every item whose `cfg` predicate is provably false, attribute and body together.
        fn kill_dead_cfg_items(&mut self) {
            let n = self.scan.len();
            let mut i = 0usize;
            while i < n {
                if self.scan[i] != '#' {
                    i += 1;
                    continue;
                }
                let mut j = i + 1;
                if self.scan.get(j) == Some(&'!') {
                    j += 1;
                }
                if self.scan.get(j) != Some(&'[') {
                    i += 1;
                    continue;
                }
                let Some(close) = balanced(&self.scan, j, '[', ']') else {
                    i += 1;
                    continue;
                };
                let inner: String = self.scan[j + 1..close].iter().collect();
                if let Some(pred) = call_args(&inner, "cfg") {
                    if cfg_eval(&pred) == Some(false) {
                        let end = item_end_after(&self.scan, close + 1);
                        self.kill(i..end);
                        i = end;
                        continue;
                    }
                }
                i = close + 1;
            }
        }

        /// Remove `if { … }` / `while { … }` blocks — and the `match` arm form `_ if … => …` —
        /// whose condition this pass cannot prove will run.
        ///
        /// # This is the fail-closed seam (T-622)
        ///
        /// T-601 removed a block only on `eval_bool(…) == Some(false)`. Everything else — including
        /// every condition the evaluator simply could not read — was **kept**, i.e. reported as
        /// live. That is the file's own signature defect wearing the fix's costume: a tool
        /// reporting success over an input it never examined. Six wrappers walked straight through
        /// it (`const B = A`, `{ false }`, `(true, false).1`, `1 + 1 > 3`, `false | false`,
        /// `::std::hint::black_box(false)`), three of them named in T-601's own brief.
        ///
        /// The rule now has three arms and no default:
        ///
        /// * `Some(false)` — provably dead. Removed, as before.
        /// * `Some(true)` — provably live. Kept.
        /// * `None` — **undecided**, and the direction is chosen by [`constant_shaped`] rather than
        ///   by assumption. A condition made only of compile-time material is a constant this
        ///   evaluator failed to read, so the block is treated as possibly dead and removed; a
        ///   condition mentioning anything the program computes is genuinely conditional and kept.
        ///
        /// Removing a block that was in fact live costs a **false RED**: the pin loses its needle
        /// and says so, loudly, on the next test run. Keeping a block that was in fact dead costs a
        /// **false GREEN**: silence, forever, over code the build never runs. The whole point of
        /// this ticket is that those two are not symmetric, and the evaluator must lean the first
        /// way. An attack shape nobody has thought of yet is now a bug report, not a bypass.
        fn kill_const_false_blocks(&mut self) {
            let consts = constants(&self.scan);
            let n = self.scan.len();
            let mut i = 0usize;
            while i < n {
                let klen = if kw_at(&self.scan, i, "if") {
                    2
                } else if kw_at(&self.scan, i, "while") {
                    5
                } else {
                    i += 1;
                    continue;
                };
                let mut j = i + klen;
                let mut d = 0i32;
                let mut stop = None;
                while j < n {
                    match self.scan[j] {
                        '(' | '[' => d += 1,
                        ')' | ']' => d -= 1,
                        '{' if d <= 0 => {
                            stop = Some((j, false));
                            break;
                        }
                        '=' if d <= 0 && self.scan.get(j + 1) == Some(&'>') => {
                            stop = Some((j, true));
                            break;
                        }
                        ';' if d <= 0 => break,
                        _ => {}
                    }
                    j += 1;
                }
                let Some((at, arrow)) = stop else {
                    i += klen;
                    continue;
                };
                let cond: String = self.scan[i + klen..at].iter().collect();
                let dead = match eval_bool(&cond, &consts) {
                    Some(b) => !b,
                    // Unknown never means "live" — see the doc comment on this function.
                    None => constant_shaped(&cond, &consts),
                };
                if dead {
                    let end = if arrow {
                        arm_end(&self.scan, at + 2)
                    } else {
                        balanced(&self.scan, at, '{', '}')
                            .map(|e| e + 1)
                            .unwrap_or(n)
                    };
                    self.kill(i..end);
                    i = end;
                } else {
                    i += klen;
                }
            }
        }

        /// Remove everything between a bare `break;` / `continue;` / `return;` and the `}` that
        /// closes the block it sits in.
        fn kill_after_unconditional_jump(&mut self) {
            let n = self.scan.len();
            let mut i = 0usize;
            while i < n {
                let Some(kw) = ["break", "continue", "return"]
                    .iter()
                    .find(|k| kw_at(&self.scan, i, k))
                    .copied()
                else {
                    i += 1;
                    continue;
                };
                let mut j = i + kw.len();
                while j < n && self.scan[j].is_whitespace() {
                    j += 1;
                }
                if self.scan.get(j) != Some(&';') {
                    i += kw.len();
                    continue;
                }
                j += 1;
                let from = j;
                let mut depth = 0i32;
                while j < n {
                    match self.scan[j] {
                        '{' => depth += 1,
                        '}' if depth == 0 => break,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    j += 1;
                }
                self.kill(from..j);
                i = j;
            }
        }
    }

    fn find_from(hay: &[char], needle: &[char], from: usize) -> Option<usize> {
        if needle.is_empty() || hay.len() < needle.len() {
            return None;
        }
        (from..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
    }

    /// End (exclusive) of the item an attribute annotates: its balanced `{…}` body, or its `;`.
    /// Depth-tracked, so the `;` inside `[u8; 3]` is not mistaken for the item terminator.
    fn item_end_after(scan: &[char], from: usize) -> usize {
        let n = scan.len();
        let mut i = from;
        let mut d = 0i32;
        while i < n {
            match scan[i] {
                '(' | '[' => d += 1,
                ')' | ']' => d -= 1,
                ';' if d <= 0 => return i + 1,
                '{' if d <= 0 => {
                    return balanced(scan, i, '{', '}').map(|e| e + 1).unwrap_or(n);
                }
                _ => {}
            }
            i += 1;
        }
        n
    }

    /// End (exclusive) of a `match` arm body starting at `from` (just past the `=>`).
    fn arm_end(scan: &[char], from: usize) -> usize {
        let n = scan.len();
        let mut i = from;
        while i < n && scan[i].is_whitespace() {
            i += 1;
        }
        if scan.get(i) == Some(&'{') {
            let end = balanced(scan, i, '{', '}').map(|e| e + 1).unwrap_or(n);
            // an optional trailing comma belongs to the arm
            let mut k = end;
            while k < n && scan[k].is_whitespace() {
                k += 1;
            }
            return if scan.get(k) == Some(&',') {
                k + 1
            } else {
                end
            };
        }
        let mut d = 0i32;
        while i < n {
            match scan[i] {
                '(' | '[' | '{' => d += 1,
                ')' | ']' => d -= 1,
                '}' if d == 0 => return i,
                '}' => d -= 1,
                ',' if d <= 0 => return i + 1,
                _ => {}
            }
            i += 1;
        }
        n
    }

    fn scrub(src: &str, keep_literals: bool) -> String {
        let chars: Vec<char> = src.chars().collect();
        let mut s = Scrub {
            scan: mask(&chars, true),
            out: mask(&chars, !keep_literals),
        };
        s.cut_test_module();
        s.kill_dead_cfg_items();
        s.kill_const_false_blocks();
        s.kill_after_unconditional_jump();
        s.out.into_iter().collect()
    }

    /// The production half of `src` with comments and unreachable constructs removed. **String
    /// literals are kept** — a route path, a `data-testid` or user-visible copy is code that ships,
    /// and pinning it is not the same defect as pinning a comment.
    pub(crate) fn live_source(src: &str) -> String {
        scrub(src, true)
    }

    /// Same, with string/char literals blanked as well — for pins that mean "this is a **call**,
    /// not a mention", where a needle sitting inside a literal is precisely the decoy.
    pub(crate) fn live_code(src: &str) -> String {
        scrub(src, false)
    }

    /// `(signature_tail, body)` of the **only** item matching `marker`.
    ///
    /// Panics on zero (a rename must be new information, not "no match") **and on two or more**:
    /// a second definition of the same name is how a pin is fed a pristine decoy while the real
    /// item is cut, and a grep cannot tell which one ships. Ambiguity is RED, not a coin flip.
    ///
    /// This is the check the old `fn_body` did not have. "Two definitions would not compile" is
    /// not a defence — a copy inside a `mod`, an `impl`, or a `#[cfg(any())]` block compiles
    /// perfectly well beside the real one, and that is the whole shadow-copy attack.
    fn split_only<'a>(src: &'a str, marker: &str) -> (usize, usize, usize) {
        let hits = src.matches(marker).count();
        assert_eq!(
            hits, 1,
            "T-601: expected exactly one `{marker}` in the live source, found {hits}. \
             0 means it was renamed or deleted; 2+ means a shadow definition — either way this pin \
             cannot examine code it cannot unambiguously find, so it fails rather than guesses."
        );
        let at = src.find(marker).expect("counted above");
        let tail = &src[at + marker.len()..];
        let open = tail
            .find('{')
            .unwrap_or_else(|| panic!("`{marker}` has no body"));
        let bytes = tail.as_bytes();
        let mut depth = 1usize;
        let mut i = open + 1;
        while i < tail.len() && depth > 0 {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        assert_eq!(depth, 0, "`{marker}` body is unbalanced");
        (at + marker.len(), open, i)
    }

    /// The whole of the **only** item matching `marker`: signature and balanced body.
    ///
    /// Use this when the assertion is about the item's *shape* — a parameter type, a return type —
    /// and not only about what it calls.
    pub(crate) fn only_item<'a>(src: &'a str, marker: &str) -> &'a str {
        let (base, _open, end) = split_only(src, marker);
        &src[base - marker.len()..base + end]
    }

    /// The balanced `{…}` body of the **only** item matching `marker`.
    pub(crate) fn only_body<'a>(src: &'a str, marker: &str) -> &'a str {
        let (base, open, end) = split_only(src, marker);
        &src[base + open + 1..base + end - 1]
    }
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
        let rows: Vec<crate::dto::RegistryCompatEdge> = edges
            .iter()
            .enumerate()
            .map(|(i, (from, to))| crate::dto::RegistryCompatEdge {
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
        assert_eq!(errs[0].key, "primary"); // keyed on the row the author must change
        assert!(errs[0].message.contains("not compatible"));
        // No weapon at all → the wording `validate_loadout` gives a hostless optic.
        p.remove("primary");
        assert!(attachment_errors(&p, &feed)[0]
            .message
            .contains("requires a Primary"));
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
        let it = crate::dto::RegistryItem {
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
        let edges: Vec<crate::dto::RegistryCompatEdge> = ["res://mag_stanag", "res://bandage"]
            .iter()
            .enumerate()
            .map(|(i, item)| crate::dto::RegistryCompatEdge {
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

    /* ═══════════ T-503 — the Arsenal commits on the spot, and now says so ═══════════ */

    /// arsenal.rs with everything **unreachable** removed, so a source pin cannot be greened by a
    /// needle that no running build can reach.
    ///
    /// T-601 moved the machinery to [`super::class_r_scrub`], which every Class-R pin in this crate
    /// now shares. The behaviour it replaced was literal matching: `#[cfg(any())]` was a **string**
    /// compare and the constant-false conditions were a **seven-entry whitelist**, so
    /// `#[cfg( any() )]`, `if 1 > 2`, `if std::hint::black_box(false)` and `while false` all walked
    /// straight past it (measured, wave 77 F3). The replacement parses the `cfg` predicate and
    /// constant-folds the condition, so spelling and whitespace stop being the defence.
    fn live_production_src() -> String {
        super::class_r_scrub::live_code(include_str!("arsenal.rs"))
    }

    use super::class_r_scrub::only_body as fn_body;

    /// T-503 Class-R: every cargo mutation in the panel must commit through `on_change`, and the
    /// commit must reach `editor_ops::set_loadout`. Staging — a mutation that updates the local
    /// signal and waits for a Save button — goes red here.
    ///
    /// RED (staging): delete the `on_change(&items.get_value());` after the qty `+` handler in
    /// `cargo_panel` → "every cargo mutation must commit: 4 `cargo.update(` vs 3 `on_change(`".
    /// RED (decoy, `if true == false`): move `crate::editor_ops::set_loadout(…)` inside
    /// `if true == false { … }` → "ArsenalTab must reach editor_ops::set_loadout".
    /// RED (decoy, `#[cfg(any())]`): park the call in an `#[cfg(any())] fn dead_persist() { … }`
    /// → same failure.
    /// RED (decoy, `loop { break; … }`): park the call after a bare `break;` → same failure.
    #[test]
    fn cargo_mutations_commit_without_a_staging_gate() {
        let live = live_production_src();
        let panel = fn_body(&live, "fn cargo_panel(");
        let mutations = panel.matches("cargo.update(").count();
        let commits = panel.matches("on_change(").count();
        assert!(
            mutations >= 4,
            "cargo_panel should still own the qty -/+, remove and add mutations; found {mutations}"
        );
        assert!(
            commits >= mutations,
            "every cargo mutation must commit: {mutations} `cargo.update(` vs {commits} `on_change(`"
        );

        let tab = fn_body(&live, "pub fn ArsenalTab(");
        assert!(
            tab.contains("crate::editor_ops::set_loadout("),
            "ArsenalTab must reach editor_ops::set_loadout on a live path"
        );
        assert!(
            tab.contains("persist(&picks.get_untracked(), items)"),
            "persist_cargo must forward to the same commit the pick path uses"
        );
    }

    /// T-503 Class-R: the panel must state the persistence contract, because the platform's only
    /// unsaved indicator (the top-strip `•`) sits behind this modal's blur scrim.
    ///
    /// RED (removed): delete the `data-arsenal-persist` block from the view → "the Arsenal must
    /// carry a data-arsenal-persist line".
    /// RED (decoy): re-add it inside `if true == false { … }` → same failure.
    #[test]
    fn the_panel_states_the_persistence_contract() {
        let live = live_production_src();
        let tab = fn_body(&live, "pub fn ArsenalTab(");
        assert!(
            tab.contains("data-arsenal-persist"),
            "the Arsenal must carry a data-arsenal-persist line the author can read"
        );
        for needle in [
            "PERSIST_ALWAYS",
            "PERSIST_CLEAN",
            "PERSIST_UNSAVED",
            "mission_has_unsaved_work()",
        ] {
            assert!(
                tab.contains(needle),
                "the persistence line must render {needle} on a live path"
            );
        }
        // The verdict badge and the per-row line both read `loadout_faults`, which is where the
        // T-504 warning lands — if either stops, the warning stops being visible.
        assert!(
            tab.matches("loadout_faults(").count() >= 2,
            "both the per-row line and the verdict badge must read loadout_faults"
        );

        // The shipped copy has to answer the question the author actually has ("did that stick?")
        // without claiming the mission is on the server, which is a different promise.
        assert!(
            PERSIST_ALWAYS.contains("no Save button"),
            "{PERSIST_ALWAYS}"
        );
        assert!(PERSIST_ALWAYS.contains("Ctrl+Z"), "{PERSIST_ALWAYS}");
        assert!(
            PERSIST_UNSAVED.contains("Save Version"),
            "{PERSIST_UNSAVED}"
        );
        assert!(
            PERSIST_CLEAN.contains("no unsaved changes"),
            "{PERSIST_CLEAN}"
        );
        assert!(!mission_has_unsaved_work(), "native shell hosts no editor");
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

        /// T-686 Class-R: the import must reach the live document through EXACTLY ONE commit, so
        /// Ctrl+Z restores the whole pre-import loadout rather than the last field of it.
        ///
        /// RED (N steps): replace the three `set`s with a `for (k, v) in doc.picks` loop that
        /// calls `persist` per pick → "an import is ONE undo step, so apply_import applies the
        /// whole document at once and has nothing to iterate — found a `for `". A textual
        /// `persist(` count alone does NOT catch that shape (the loop has one call site), which is
        /// why the loop itself is what this asserts on.
        /// RED (ungated): call `apply_import` outside the `Ok(doc)` arm → the `try_import` pin.
        /// RED (decoy, `#[cfg(any())]`): park the picker in a dead item → same failure.
        #[test]
        fn the_import_applies_in_one_commit() {
            let live = live_production_src();
            let tab = fn_body(&live, "pub fn ArsenalTab(");
            assert!(
                tab.contains("try_import("),
                "the import must be gated on a live path"
            );
            assert!(
                tab.contains("apply_import(doc, &its)"),
                "only an accepted document may be applied"
            );
            assert!(
                tab.contains("data-loadout-import"),
                "the panel must carry an import control the author can reach"
            );

            let apply = fn_body(&live, "let apply_import =");
            let commits = apply.matches("persist(").count();
            assert_eq!(
                commits, 1,
                "an import is ONE undo step: apply_import must commit exactly once, found {commits}"
            );
            // The call-site count is necessary and NOT sufficient: one `persist(` inside a loop is
            // still N undo steps. `apply_import` replaces the whole document with three signal
            // writes, so it has nothing to iterate — and any iteration in it is the N-step shape.
            for loopy in ["for ", "while ", "for_each", ".iter()"] {
                assert!(
                    !apply.contains(loopy),
                    "an import is ONE undo step, so apply_import applies the whole document at \
                     once and has nothing to iterate — found a `{loopy}`"
                );
            }
            for needle in ["picks.set(", "cargo.set(", "cargo_present.set("] {
                assert!(
                    apply.contains(needle),
                    "the apply must replace the whole loadout — missing {needle}"
                );
            }
            assert!(
                !apply.contains("set_loadout"),
                "the apply must go through the same `persist` every other pick uses"
            );
        }
    }

    /// **The scrubber's own pin.** Every shape the Class-R pins in this crate claim to defeat is
    /// fed through and must come out empty — because a scrubber that quietly stopped scrubbing
    /// would leave every pin built on it hollow while all of them stayed green. That is this
    /// repo's signature defect (a tool reporting success over an input it never examined) applied
    /// to the tool itself, so it gets a test rather than a comment.
    ///
    /// The list is the full attack battery, in three tiers:
    ///
    /// 1. **Comment / literal decoys** — T-554…T-561.
    /// 2. **Dead-code wrappers** — the shapes that beat T-564…T-570 and wave 77 (`if false`,
    ///    `if true == false`, `loop { break; … }`, `#[cfg(any())]`, `while false`, `if !true`,
    ///    `if 1 > 2`, the `match` guard, `const C: bool = false; if C`, `black_box(false)`,
    ///    a `return;` above, and the `#[cfg(any())] mod` shadow copy).
    /// 3. **The measured wave-77-F3 survivors** — the spelling variations that walked past the
    ///    literal `"#[cfg(any())]"` match and the seven-condition whitelist. These are the reason
    ///    T-601 replaced both with a parser.
    ///
    /// Plus two attacks the handed-down list does **not** contain, because a list is exactly what a
    /// fixer special-cases; see [`two_attacks_the_known_list_does_not_contain`].
    #[test]
    fn the_scrubber_actually_removes_every_decoy_shape() {
        use super::class_r_scrub::live_code;
        let cases = [
            // ── tier 1: the needle is text, not code
            ("line comment", "// set_loadout(x)\nlet a = 1;"),
            ("block comment", "/* set_loadout(x) */ let a = 1;"),
            ("nested block comment", "/* a /* set_loadout(x) */ b */ x"),
            ("string literal", "let s = \"set_loadout(x)\";"),
            ("raw string", "let s = r#\"set_loadout(x)\"#;"),
            // ── tier 2: the known dead-code wrappers
            ("if false", "if false { set_loadout(x); }"),
            ("if true == false", "if true == false { set_loadout(x); }"),
            ("if false == true", "if false == true { set_loadout(x); }"),
            ("if !true", "if !true { set_loadout(x); }"),
            ("if 1 > 2", "if 1 > 2 { set_loadout(x); }"),
            ("while false", "while false { set_loadout(x); }"),
            ("cfg(any())", "#[cfg(any())] fn d() { set_loadout(x); }"),
            (
                "cfg(any()) mod shadow copy",
                "#[cfg(any())] mod shadow { fn cargo_panel() { set_loadout(x); } }",
            ),
            ("after break", "loop { break; set_loadout(x); }"),
            ("after continue", "loop { continue; set_loadout(x); }"),
            ("after return", "fn f() { return; set_loadout(x); }"),
            (
                "match guard",
                "match () { _ if false => { set_loadout(x); } _ => {} }",
            ),
            (
                "const false binding",
                "const C: bool = false; fn f() { if C { set_loadout(x); } }",
            ),
            (
                "black_box(false)",
                "if std::hint::black_box(false) { set_loadout(x); }",
            ),
            ("cfg!(any())", "if cfg!(any()) { set_loadout(x); }"),
            // ── tier 3: wave 77 F3's measured survivors — spelling, not structure
            ("cfg(any()) spaced", "#[cfg( any() )] fn d() { set_loadout(x); }"),
            (
                "cfg(any()) spaced brackets",
                "#[ cfg(any()) ] fn d() { set_loadout(x); }",
            ),
            (
                "cfg(any()) inner spaces",
                "#[cfg(any( ))]\nfn d() { set_loadout(x); }",
            ),
            (
                "if condition with odd spacing",
                "if  true  ==  false  { set_loadout(x); }",
            ),
            (
                "black_box, core path",
                "if core::hint::black_box(1) > core::hint::black_box(2) { set_loadout(x); }",
            ),
            // ── measured against the real files by the T-601 battery, not imagined. The first two
            // shipped GREEN in the first cut of this scrubber: the binding scanner walked the
            // source one keyword at a time and, once any earlier `const`/`let` in the file failed
            // its checks, resumed *inside* that binding's own text — from where it could never see
            // a later one. Every pin whose file had such a binding above the decoy was hollow.
            (
                "const declared on the same line as the if",
                "fn f() {\nconst T601C: bool = false; if T601C {\n    set_loadout(x);\n}\n}",
            ),
            (
                "const folded through a comparison, same line",
                "fn f() {\nconst T601N: bool = 1 > 2; if T601N {\n    set_loadout(x);\n}\n}",
            ),
            (
                "const behind an unrelated non-bool const",
                "const OTHER: &str = \"x\";\nconst T601C: bool = false;\nfn f() { if T601C { set_loadout(x); } }",
            ),
            (
                "const behind a let-else",
                "fn g() { let Ok(v) = h() else { return; }; }\nconst T601C: bool = false;\nfn f() { if T601C { set_loadout(x); } }",
            ),
            // ── THE ONE THAT SHIPPED GREEN. `sse.rs`, `client.rs` and `arsenal.rs` all park their
            // live path inside a binding whose initializer is a block (`let run = async { … };`,
            // `let send = move |t| { … };`), and the binding scanner used to resume after the
            // initializer — so nothing inside one was ever seen. Measured against the real files.
            (
                "const nested inside a block-initialised binding",
                "fn f() { let run = async {\nconst T601C: bool = false; if T601C { set_loadout(x); }\n}; }",
            ),
            (
                "const nested inside a closure-initialised binding",
                "fn f() { let send = move |t| {\nconst T601N: bool = 1 > 2; if T601N { set_loadout(x); }\n}; }",
            ),
            (
                "const inside an async block",
                "fn f() { spawn(async move {\nconst T601C: bool = false; if T601C {\n    set_loadout(x);\n}\n}); }",
            ),
            // ── tier 4: the six wave-79 survivors of T-601's own fix, measured against the real
            // production files (`sse.rs`, `event_hub.rs` ×2, `client.rs`, `mission_commands.rs`,
            // `content.rs`) before they were fixed. Three of them were named in T-601's brief.
            // They are listed for regression value only — the thing that actually stops the
            // seventh is `the_unknown_condition_fails_closed`.
            (
                "T-622 S1: const referencing const",
                "const W_A: bool = false; const W_B: bool = W_A;\nfn f() { if W_B { set_loadout(x); } }",
            ),
            (
                "T-622 S1': the same chain, declared out of order",
                "const W_B: bool = W_A; const W_A: bool = false;\nfn f() { if W_B { set_loadout(x); } }",
            ),
            (
                "T-622 S2: block-expression initialiser",
                "const W_NEVER: bool = { false };\nfn f() { if W_NEVER { set_loadout(x); } }",
            ),
            (
                "T-622 S3: tuple index",
                "fn f() { if (true, false).1 { set_loadout(x); } }",
            ),
            (
                "T-622 S4: arithmetic inside a comparison",
                "fn f() { if 1 + 1 > 3 { set_loadout(x); } }",
            ),
            (
                "T-622 S5: bitwise rather than logical",
                "fn f() { if false | false { set_loadout(x); } }",
            ),
            (
                "T-622 S6: leading :: on a transparent call",
                "fn f() { if ::std::hint::black_box(false) { set_loadout(x); } }",
            ),
            // ── tier 5: shapes invented against the T-622 fix, not handed down by any verifier.
            // With the unknown case failing closed these cost nothing to defeat, which is the
            // point: none of them required the fixer to have thought of them first.
            (
                "T-622 I1: array index",
                "fn f() { if [false, true][0] { set_loadout(x); } }",
            ),
            (
                "T-622 I2: if-expression const initialiser",
                "const W_C: bool = if true { false } else { true };\nfn f() { if W_C { set_loadout(x); } }",
            ),
            (
                "T-622 I3: immediately-invoked closure",
                "fn f() { if (|| false)() { set_loadout(x); } }",
            ),
            (
                "T-622 I4: xor",
                "fn f() { if false ^ false { set_loadout(x); } }",
            ),
            (
                "T-622 I5: shift compared to a literal",
                "fn f() { if 1 << 2 == 7 { set_loadout(x); } }",
            ),
            (
                "T-622 I6: constant laundered through a let",
                "fn f() { let w: bool = (true, false).1; if w { set_loadout(x); } }",
            ),
        ];
        for (label, src) in cases {
            let scrubbed = live_code(src);
            assert!(
                !scrubbed.contains("set_loadout"),
                "{label}: decoy survived scrubbing — every pin built on this scrubber is hollow \
                 while staying green, which is the exact defect T-601 exists to remove.\n{scrubbed}"
            );
        }

        // …and it must not eat live code while it is at it. A scrubber that removed everything
        // would pass every case above and pin nothing.
        let live = "if x { set_loadout(a); } else { set_loadout(b); }";
        assert_eq!(live_code(live).matches("set_loadout(").count(), 2);
        for kept in [
            "if 2 > 1 { set_loadout(a); }",
            "while running { set_loadout(a); }",
            "#[cfg(target_arch = \"wasm32\")] fn d() { set_loadout(a); }",
            "#[cfg(feature = \"never-enabled\")] fn d() { set_loadout(a); }",
            "const C: bool = true; fn f() { if C { set_loadout(a); } }",
            "match () { _ if x => { set_loadout(a); } _ => {} }",
            "fn f() { if a { return; } set_loadout(a); }",
            // T-622 — the shapes a fail-closed evaluator could plausibly eat. Every one of these
            // names something the program computes, so none of them is constant-shaped and none
            // may be scrubbed. Without this half, "scrub whatever you cannot read" would pass the
            // whole battery above by deleting the crate.
            "fn f() { if let Some(v) = opt { set_loadout(v); } }",
            "fn f() { while let Some(v) = it.next() { set_loadout(v); } }",
            "fn f() { if resp.ok() { set_loadout(a); } }",
            "fn f() { let ok = resp.ok(); if ok { set_loadout(a); } }",
            "fn f() { let ok: bool = resp.ok(); if ok { set_loadout(a); } }",
            "fn f() { if !items.is_empty() { set_loadout(a); } }",
            "fn f() { if i < n { set_loadout(a); } }",
            "fn f() { if cfg!(feature = \"x\") { set_loadout(a); } }",
            "fn f() { if cfg!(target_arch = \"wasm32\") { set_loadout(a); } }",
            // A numeric `const` is compile-time material, so it MUST fold rather than fail closed —
            // otherwise every `const LIMIT: usize = …; if LIMIT > n` in the crate turns RED.
            "const LIMIT: usize = 5; fn f() { if LIMIT > 3 { set_loadout(a); } }",
            "const LIMIT: usize = 5; fn f() { if LIMIT > 3 && x { set_loadout(a); } }",
            "const NAME: &str = \"x\"; fn f() { if p == NAME { set_loadout(a); } }",
        ] {
            assert!(
                live_code(kept).contains("set_loadout"),
                "the scrubber ate live code: {kept}"
            );
        }
        // A lifetime is not a char literal; a `;` inside a type is not an item terminator.
        assert!(live_code("fn f<'a>(x: &'a str) { set_loadout(x); }").contains("'a"));
        assert!(
            live_code("#[cfg(any())] const D: [u8; 3] = [1, 2, 3];\nfn f() { set_loadout(x); }")
                .contains("set_loadout"),
            "the `;` inside `[u8; 3]` must not end the cfg'd item early"
        );
        // `live_source` keeps literals — a route path or a `data-testid` is shipped code.
        assert!(super::class_r_scrub::live_source("let p = \"/servers\";").contains("/servers"));
        assert!(!live_code("let p = \"/servers\";").contains("/servers"));
    }

    /// **T-622 — the property, not the list.**
    ///
    /// Five rounds of this defect (T-517 → T-567 → T-570 → W77-F2/F3 → W79) were each closed by
    /// enumerating the wrapper shapes that had been reported, and each was walked around by the
    /// next spelling. T-601's own fix lost the same way: it replaced two blocklists with a real
    /// evaluator, and then let every expression the evaluator could not read fall through to
    /// "keep" — which is "report as live". Six wrappers survived it on real production source.
    ///
    /// The list above is regression value. **This** is the thing that stops the seventh: it asserts
    /// the invariant directly, over conditions chosen so that no fixer could have special-cased
    /// them, using operators the evaluator provably does not model.
    ///
    /// The invariant has two halves and both are load-bearing:
    ///
    /// 1. A condition naming nothing the program computes is a compile-time constant. If it does
    ///    not fold to `true`, the block goes — **whatever** shape it is.
    /// 2. A condition naming anything the program computes is genuinely conditional and stays. A
    ///    "fail-closed" scrubber without this half would pass every attack test by deleting the
    ///    crate, and would turn all five cure-2 pins permanently RED.
    #[test]
    fn the_unknown_condition_fails_closed() {
        use super::class_r_scrub::live_code;

        // Half 1 — pure compile-time material, spelled with operators `lex` emits `Tok::Other`
        // for. None of these is parsed; all of them must still be removed.
        for cond in [
            "(true, false).1",
            "1 + 1 > 3",
            "false | false",
            "false ^ false",
            "[false, true][0]",
            "(|| false)()",
            "1 << 2 == 7",
            "10 % 3 == 2",
            "-1 > 0",
            "*&false",
            "(true && false) & true",
            "({ false })",
            "::std::hint::black_box(false)",
            "0xff_u8 as bool",
        ] {
            let src = format!("fn f() {{ if {cond} {{ set_loadout(x); }} }}");
            assert!(
                !live_code(&src).contains("set_loadout"),
                "`if {cond}` mentions nothing this program computes, so its truth was fixed at \
                 compile time. The evaluator could not read it — and an evaluator that cannot \
                 prove code is live must not report it as live. This is a false GREEN, the exact \
                 defect five waves have now failed to close by enumeration."
            );
        }

        // The same shapes behind one level of `const` indirection, which is how the wave-79
        // reproduction on the real `sse.rs` was built.
        for init in ["(true, false).1", "{ false }", "1 + 1 > 3", "false | false"] {
            let src = format!(
                "const W_A: bool = {init}; const W_B: bool = W_A;\n\
                 fn f() {{ if W_B {{ set_loadout(x); }} }}"
            );
            assert!(
                !live_code(&src).contains("set_loadout"),
                "`const W_A: bool = {init}; const W_B: bool = W_A` — a `const` is compile-time by \
                 Rust's own rules, so a `const` this pass cannot fold is a constant it failed to \
                 read, never a runtime value"
            );
        }

        // Half 2 — one runtime name is enough to make the condition genuinely conditional. These
        // are the same operators; the only difference is that something in them is computed.
        for cond in [
            "(true, flag).1",
            "n + 1 > 3",
            "flag | false",
            "[flag, true][0]",
            "(|| flag)()",
            "resp.ok()",
            "!items.is_empty()",
            "cfg!(feature = \"x\")",
            "let Some(v) = opt",
        ] {
            let src = format!("fn f() {{ if {cond} {{ set_loadout(x); }} }}");
            assert!(
                live_code(&src).contains("set_loadout"),
                "`if {cond}` names something the program computes, so it is live code the \
                 scrubber must leave alone. Eating it would turn every cure-2 pin permanently RED \
                 — a fail-closed evaluator that scrubs the program is not a fix, it is an outage."
            );
        }

        // ── the residual, pinned so it cannot grow in silence ────────────────────────────────
        //
        // These DO survive, and the module doc says so. A call is the boundary: to this pass
        // `Option::<bool>::None.unwrap_or(false)` and `resp.ok()` are the same three tokens in the
        // same order, and there is no reading of the text that separates them. Folding calls by
        // name would be the blocklist again, one level down — and folding them *all* would delete
        // every `if resp.ok()` in the crate. So an opaque call stays live, loudly documented,
        // rather than quietly half-handled.
        //
        // Asserted rather than omitted: if a later change closes one of these, this test fails and
        // whoever closed it gets to move the line in the module doc too. That is the opposite of
        // how the last five rounds of this defect were "fixed".
        for cond in [
            "Option::<bool>::None.unwrap_or(false)",
            "bool::default()",
            "\"\".is_empty() && false == true",
        ] {
            let src = format!("fn f() {{ if {cond} {{ set_loadout(x); }} }}");
            assert!(
                live_code(&src).contains("set_loadout"),
                "`if {cond}` is a KNOWN residual (an opaque call). If it now scrubs, that is an \
                 improvement — say so in the residual list at the top of this file instead of \
                 leaving this assertion lying about what the scrubber does."
            );
        }
    }

    /// **Two attacks the handed-down list does not contain.**
    ///
    /// The listed shapes are the ones a fixer naturally special-cases, so passing them proves
    /// little on its own. These two were invented against the *fix*:
    ///
    /// * **A1 — the shadow copy with no `cfg` at all.** The known variant parks the decoy under
    ///   `#[cfg(any())]`, so every cfg-based defence catches it. Move the real item into a plain
    ///   `mod` nobody calls and leave the pristine copy at column 0 and there is no cfg to find,
    ///   no dead-code wrapper to strip, and both copies compile. Only refusing **ambiguity**
    ///   catches this, which is why [`class_r_scrub::only_body`] counts before it reads.
    /// * **A2 — the constant folded through a comparison.** The known variant is
    ///   `const C: bool = false; if C`, which a fixer answers by looking for `= false`.
    ///   `const NEVER: bool = 1 > 2;` has no `false` anywhere in it. Only actually evaluating the
    ///   initialiser catches it.
    ///
    /// Bonus third, same family as A2 but on the `cfg` side: `#[cfg(all(any(), unix))]` contains
    /// `any()` but is not the literal `#[cfg(any())]`, and `#[cfg(not(all()))]` contains neither.
    #[test]
    fn two_attacks_the_known_list_does_not_contain() {
        use super::class_r_scrub::{live_code, only_body};

        // A1 — pristine decoy at column 0, real (cut) code in a live module. No cfg, no wrapper.
        let a1 = "\
fn cargo_panel() { on_change(&items); }
mod real {
    pub fn cargo_panel() { /* wire cut */ }
}
";
        let scrubbed = live_code(a1);
        let hits = scrubbed.matches("fn cargo_panel(").count();
        assert_eq!(
            hits, 2,
            "both definitions must survive scrubbing: {scrubbed}"
        );
        let caught = std::panic::catch_unwind(|| only_body(&scrubbed, "fn cargo_panel(")).is_err();
        assert!(
            caught,
            "A1: a shadow definition with no cfg and no dead-code wrapper fed the pin a decoy — \
             only an ambiguity refusal catches this shape"
        );

        // A2 — the constant never spells `false`.
        let a2 = "const NEVER: bool = 1 > 2;\nfn f() { if NEVER { on_change(&items); } }";
        assert!(
            !live_code(a2).contains("on_change"),
            "A2: `const NEVER: bool = 1 > 2` must fold — a fixer that grepped for `= false` \
             would have shipped this hole"
        );

        // Bonus — composite never-true cfg predicates.
        for src in [
            "#[cfg(all(any(), unix))] fn d() { on_change(&items); }",
            "#[cfg(not(all()))] fn d() { on_change(&items); }",
            "#[cfg(any(any(), any()))] fn d() { on_change(&items); }",
        ] {
            assert!(
                !live_code(src).contains("on_change"),
                "composite false cfg survived: {src}"
            );
        }
    }
}
