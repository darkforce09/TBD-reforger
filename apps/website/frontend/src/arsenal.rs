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
//! * **Every other mission-document editor commits on the spot.** `editor_ops.rs` funnels 28 call
//!   sites into `mission_history::after_local_edit`, and the Arsenal's `set_loadout`
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
    /// needle that no running build can reach. Strips, in order: comments, string/char literals,
    /// `#[cfg(any())]` items, constant-false `if` blocks, and everything after an unconditional
    /// `break;` / `continue;` / `return;`. The test module is cut off first, so a pin can never
    /// read its own assertion.
    ///
    /// This exists because a pin that greps raw text is worthless — the needle sits in a comment or
    /// a dead block and the pin greens over a broken code path. Each pin below records the decoys
    /// that were actually run against it.
    fn live_production_src() -> String {
        const SRC: &str = include_str!("arsenal.rs");
        let production = SRC.split("#[cfg(test)]").next().unwrap_or(SRC);
        strip_after_unconditional_jump(&strip_const_false_blocks(&strip_cfg_any_items(
            &strip_comments_and_literals(production),
        )))
    }

    fn ident_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    /// Word-boundary check for a keyword starting at `i`.
    fn kw_at(chars: &[char], i: usize, kw: &str) -> bool {
        let k: Vec<char> = kw.chars().collect();
        if i + k.len() > chars.len() || chars[i..i + k.len()] != k[..] {
            return false;
        }
        (i == 0 || !ident_char(chars[i - 1]))
            && (i + k.len() >= chars.len() || !ident_char(chars[i + k.len()]))
    }

    /// Drop `//` / `/* */` (nesting) comments and `"…"` / `r#"…"#` / `'c'` literals, replacing each
    /// with a space so tokens cannot fuse. Lifetimes (`'static`) survive — only real char literals
    /// are eaten.
    fn strip_comments_and_literals(src: &str) -> String {
        let c: Vec<char> = src.chars().collect();
        let mut out = String::with_capacity(src.len());
        let mut i = 0;
        while i < c.len() {
            // line comment
            if c[i] == '/' && i + 1 < c.len() && c[i + 1] == '/' {
                while i < c.len() && c[i] != '\n' {
                    i += 1;
                }
                out.push(' ');
                continue;
            }
            // block comment (nesting, as rustc allows)
            if c[i] == '/' && i + 1 < c.len() && c[i + 1] == '*' {
                let mut depth = 1usize;
                i += 2;
                while i < c.len() && depth > 0 {
                    if c[i] == '/' && i + 1 < c.len() && c[i + 1] == '*' {
                        depth += 1;
                        i += 2;
                    } else if c[i] == '*' && i + 1 < c.len() && c[i + 1] == '/' {
                        depth -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                out.push(' ');
                continue;
            }
            // raw string: r"…" / r#"…"# / r##"…"##
            if c[i] == 'r' && (i == 0 || !ident_char(c[i - 1])) {
                let mut j = i + 1;
                let mut hashes = 0usize;
                while j < c.len() && c[j] == '#' {
                    hashes += 1;
                    j += 1;
                }
                if j < c.len() && c[j] == '"' {
                    j += 1;
                    while j < c.len() {
                        if c[j] == '"' {
                            let closed = (1..=hashes).all(|k| j + k < c.len() && c[j + k] == '#');
                            if closed {
                                j += hashes + 1;
                                break;
                            }
                        }
                        j += 1;
                    }
                    out.push(' ');
                    i = j;
                    continue;
                }
            }
            // normal string
            if c[i] == '"' {
                i += 1;
                while i < c.len() {
                    if c[i] == '\\' {
                        i += 2;
                        continue;
                    }
                    if c[i] == '"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                out.push(' ');
                continue;
            }
            // char literal vs lifetime
            if c[i] == '\'' {
                let escaped = i + 1 < c.len() && c[i + 1] == '\\';
                let single = i + 2 < c.len() && c[i + 2] == '\'';
                if escaped || single {
                    i += 1;
                    while i < c.len() {
                        if c[i] == '\\' {
                            i += 2;
                            continue;
                        }
                        if c[i] == '\'' {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                    out.push(' ');
                    continue;
                }
            }
            out.push(c[i]);
            i += 1;
        }
        out
    }

    /// Skip past the item a `#[cfg(any())]` attribute annotates: to the end of its balanced `{…}`
    /// body, or to its terminating `;`, whichever comes first.
    fn strip_cfg_any_items(src: &str) -> String {
        const ATTR: &str = "#[cfg(any())]";
        let mut out = String::with_capacity(src.len());
        let mut rest = src;
        while let Some(at) = rest.find(ATTR) {
            out.push_str(&rest[..at]);
            out.push(' ');
            let after: Vec<char> = rest[at + ATTR.len()..].chars().collect();
            let mut i = 0;
            while i < after.len() && after[i] != '{' && after[i] != ';' {
                i += 1;
            }
            if i < after.len() && after[i] == '{' {
                let mut depth = 1usize;
                i += 1;
                while i < after.len() && depth > 0 {
                    match after[i] {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
            } else if i < after.len() {
                i += 1; // the `;`
            }
            rest =
                &rest[at + ATTR.len() + after[..i].iter().map(|c| c.len_utf8()).sum::<usize>()..];
        }
        out.push_str(rest);
        out
    }

    /// Drop `if <constant false> { … }` blocks (brace-balanced), leaving any `else` arm — so a
    /// decoy parked in `if false { … }` or `if true == false { … }` cannot green a pin.
    fn strip_const_false_blocks(src: &str) -> String {
        const DEAD: &[&str] = &[
            "false",
            "true==false",
            "false==true",
            "1==0",
            "0==1",
            "!true",
            "cfg!(any())",
        ];
        let c: Vec<char> = src.chars().collect();
        let mut out = String::with_capacity(src.len());
        let mut i = 0;
        while i < c.len() {
            if kw_at(&c, i, "if") {
                // Read the condition up to the block's opening brace.
                let mut j = i + 2;
                let mut cond = String::new();
                while j < c.len() && c[j] != '{' {
                    if !c[j].is_ascii_whitespace() {
                        cond.push(c[j]);
                    }
                    j += 1;
                }
                if j < c.len() && DEAD.contains(&cond.as_str()) {
                    let mut depth = 1usize;
                    j += 1;
                    while j < c.len() && depth > 0 {
                        match c[j] {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        j += 1;
                    }
                    out.push(' ');
                    i = j;
                    continue;
                }
            }
            out.push(c[i]);
            i += 1;
        }
        out
    }

    /// Drop everything between a bare `break;` / `continue;` / `return;` and the `}` that closes
    /// the block it sits in — so a decoy parked after `loop { break; … }` cannot green a pin.
    fn strip_after_unconditional_jump(src: &str) -> String {
        let c: Vec<char> = src.chars().collect();
        let mut out = String::with_capacity(src.len());
        let mut i = 0;
        while i < c.len() {
            let jump = ["break", "continue", "return"]
                .iter()
                .find(|k| kw_at(&c, i, k))
                .copied();
            if let Some(kw) = jump {
                let mut j = i + kw.len();
                while j < c.len() && c[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j < c.len() && c[j] == ';' {
                    out.push_str(kw);
                    out.push(';');
                    j += 1;
                    // Skip to the first unmatched `}` — the end of the enclosing block.
                    let mut depth = 0i32;
                    while j < c.len() {
                        match c[j] {
                            '{' => depth += 1,
                            '}' if depth == 0 => break,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        j += 1;
                    }
                    out.push(' ');
                    i = j;
                    continue;
                }
            }
            out.push(c[i]);
            i += 1;
        }
        out
    }

    /// The balanced `{…}` body of the first item matching `marker`. Panics rather than returning
    /// an empty string, so a renamed function fails loudly instead of vacuously passing.
    fn fn_body<'a>(src: &'a str, marker: &str) -> &'a str {
        let at = src
            .find(marker)
            .unwrap_or_else(|| panic!("live source has no `{marker}` — did it get renamed?"));
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
        &tail[open + 1..i - 1]
    }

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

    /// The scrubber is load-bearing for both pins above, so it gets its own test: each decoy shape
    /// the pins claim to defeat is fed through and must come out empty.
    #[test]
    fn the_scrubber_actually_removes_every_decoy_shape() {
        let cases = [
            ("line comment", "// set_loadout(x)\nlet a = 1;"),
            ("block comment", "/* set_loadout(x) */ let a = 1;"),
            ("nested block comment", "/* a /* set_loadout(x) */ b */ x"),
            ("string literal", "let s = \"set_loadout(x)\";"),
            ("raw string", "let s = r#\"set_loadout(x)\"#;"),
            ("if false", "if false { set_loadout(x); }"),
            ("if true == false", "if true == false { set_loadout(x); }"),
            ("cfg(any())", "#[cfg(any())] fn d() { set_loadout(x); }"),
            ("after break", "loop { break; set_loadout(x); }"),
            ("after return", "fn f() { return; set_loadout(x); }"),
        ];
        for (label, src) in cases {
            let scrubbed = strip_after_unconditional_jump(&strip_const_false_blocks(
                &strip_cfg_any_items(&strip_comments_and_literals(src)),
            ));
            assert!(
                !scrubbed.contains("set_loadout"),
                "{label}: decoy survived scrubbing — pins built on this are hollow: {scrubbed}"
            );
        }
        // …and it must not eat live code while it is at it.
        let live = "if x { set_loadout(a); } else { set_loadout(b); }";
        let kept = strip_after_unconditional_jump(&strip_const_false_blocks(&strip_cfg_any_items(
            &strip_comments_and_literals(live),
        )));
        assert_eq!(kept.matches("set_loadout(").count(), 2, "{kept}");
        // A lifetime is not a char literal.
        assert!(strip_comments_and_literals("fn f<'a>(x: &'a str) {}").contains("'a"));
    }
}
