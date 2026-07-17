//! Arsenal tab — the loadout Forge (the ArsenalTab.tsx + arsenalRules.ts port, T-159.27). Replaces
//! the T-159.26 disabled stub with a real, doc-backed loadout editor: a dropdown per gear row,
//! sourced from the flat `/registry` filtered by item kind, persisted on the slot via
//! `editor_ops::set_loadout` (one undo step per pick) as the canonical `SlotLoadoutV2` shape (the
//! same `picksToLoadout` output the mod equip reads), so a pick round-trips through Save/Export.
//!
//! **Scope (the "dumb Forge", T-068.4 essence):** the `kind`-sourced rows only — clothing +
//! weapons picked from the flat catalog. The compat-worker `edge` rows (optic / magazine feeds
//! keyed off the weapon family) + the clickable paper-doll + weight/validation + the Faction
//! Manager fold forward to a Smart-Forge follow-on; the doc contract (`SlotLoadoutV2`) is the same,
//! so those add rows/panels without changing what's persisted here.
#![allow(dead_code)]
use leptos::prelude::*;

use crate::dto::RegistryItem;

const CONTROL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";

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

/// `loadoutToPicks` — read the slot's `SlotLoadoutV2` JSON into a per-key `resource_name` map. An
/// absent loadout → all-empty picks. Weapons resolve by `slotIndex`; wear by key.
fn loadout_to_picks(loadout_json: Option<&str>) -> std::collections::HashMap<String, String> {
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
                }
            }
        }
    }
    picks
}

/// `picksToLoadout` — build the canonical `SlotLoadoutV2` from the picks. All-empty → `None` (clear
/// the doc field). Wear map + weapons array; primary re-emits its sticky `optic`/`magazine` (String
/// or null) and the always-empty `attachments` (React hardcodes `[]` until the attachments slice).
/// `names` resolves `resource_name` → `display_name` for the `summary` (falls back to the raw name).
fn picks_to_loadout(
    picks: &std::collections::HashMap<String, String>,
    names: &std::collections::HashMap<String, String>,
) -> Option<String> {
    if ROWS
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
        if row.key == "primary" {
            obj["optic"] = sticky("optic");
            obj["magazine"] = sticky("magazine");
            obj["attachments"] = serde_json::json!([]);
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
    if !summary.is_empty() {
        loadout["summary"] = serde_json::Value::String(summary);
    }
    Some(loadout.to_string())
}

/// The Arsenal tab — mounted in the Attributes modal (T-159.26 seam). `registry` is the flat
/// catalog (fetched once by the editor); `slot_id` + `loadout_json` come from the modal's re-read.
#[component]
pub fn ArsenalTab(
    slot_id: String,
    /// The slot's current `loadout` JSON (from `editor_ops::read_loadout`), for the dropdowns.
    loadout_json: Option<String>,
    /// The flat registry gear rows (kind-filtered client-side), `None` while loading.
    registry: RwSignal<Option<Vec<RegistryItem>>>,
) -> impl IntoView {
    let id = StoredValue::new(slot_id);
    let picks = StoredValue::new(loadout_to_picks(loadout_json.as_deref()));
    // `resource_name` → `display_name` for the loadout `summary` (rebuilt every commit from the
    // live catalog; empty until the registry resolves, matching the "Loading catalog…" gate).
    let names = StoredValue::new(std::collections::HashMap::<String, String>::new());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (id, picks, names);

    // Commit a single row change: update the picks map, rebuild the loadout, persist.
    let commit = move |key: &'static str, value: String| {
        #[cfg(target_arch = "wasm32")]
        {
            let mut map = picks.get_value();
            if value.is_empty() {
                map.remove(key);
            } else {
                map.insert(key.to_string(), value);
            }
            crate::editor_ops::set_loadout(
                &id.get_value(),
                picks_to_loadout(&map, &names.get_value()),
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (key, value);
    };

    view! {
        <div class="flex flex-col gap-3">
            {move || match registry.get() {
                None => {
                    view! {
                        <p class="text-label-sm normal-case text-outline">"Loading catalog…"</p>
                    }
                        .into_any()
                }
                Some(items) => {
                    // Feed the summary resolver from the resolved catalog (one pass, ignored on
                    // native where `commit` is a no-op).
                    #[cfg(target_arch = "wasm32")]
                    names.set_value(
                        items
                            .iter()
                            .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                            .collect(),
                    );
                    let items = StoredValue::new(items);
                    ROWS.iter()
                        .map(|row| {
                            let current = picks
                                .get_value()
                                .get(row.key)
                                .cloned()
                                .unwrap_or_default();
                            // Options for this row: registry items of the row's kind, non-abstract,
                            // sorted by display_name; a live pick never blanks (kept even if abstract).
                            let mut opts: Vec<(String, String)> = items
                                .get_value()
                                .iter()
                                .filter(|it| {
                                    it.kind == row.kind
                                        && (it.r#abstract != Some(true)
                                            || it.resource_name == current)
                                })
                                .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                                .collect();
                            opts.sort_by(|a, b| a.1.cmp(&b.1));
                            let key = row.key;
                            view! {
                                <label class="flex flex-col gap-1">
                                    <span class="text-label-sm uppercase tracking-wider text-outline">
                                        {row.label}
                                    </span>
                                    <select
                                        prop:value=current.clone()
                                        on:change=move |ev| commit(key, event_target_value(&ev))
                                        class=CONTROL
                                    >
                                        <option value="">"— None —"</option>
                                        {opts
                                            .into_iter()
                                            .map(|(value, label)| {
                                                view! { <option value=value>{label}</option> }
                                            })
                                            .collect_view()}
                                    </select>
                                </label>
                            }
                        })
                        .collect_view()
                        .into_any()
                }
            }}
            <p class="mt-1 text-label-sm normal-case text-outline">
                "Compat-validated optics/magazines, the paper-doll, and weight land with the Smart Forge follow-on."
            </p>
        </div>
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

    #[test]
    fn all_empty_picks_clear_the_field() {
        assert!(picks_to_loadout(&HashMap::new(), &names()).is_none());
        // An unknown (non-row) key alone still counts as empty — no row is set.
        assert!(picks_to_loadout(&picks(&[("optic", "res://acog")]), &names()).is_none());
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
        let lo = picks_to_loadout(&p, &names()).unwrap();
        let back = loadout_to_picks(Some(&lo));
        for k in ["primary", "launcher", "headCover", "vest"] {
            assert_eq!(back.get(k), p.get(k), "key {k} lost on round-trip");
        }
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
        let resaved = picks_to_loadout(&back, &names()).unwrap();
        let v: serde_json::Value = serde_json::from_str(&resaved).unwrap();
        assert_eq!(v["weapons"][0]["optic"], "res://acog");
        assert_eq!(v["weapons"][0]["magazine"], "res://mag_stanag");
        // summary resolves display names of primary · optic · magazine (launcher absent).
        assert_eq!(v["summary"], "M16A2 · ACOG · STANAG 30rd");
    }
}
