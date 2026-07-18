//! Arsenal tab — the **Smart Forge** (ArsenalTab.tsx + arsenalRules.ts + SoldierSilhouette.tsx
//! port, T-159.27 → T-167). A doc-backed loadout editor: the 14 loadout rows (incl. the compat
//! `edge` rows optic/magazine keyed off the picked weapon), a clickable **SVG paper-doll**, an
//! honest **weight** readout, and per-row **compat validation** — persisted on the slot via
//! `editor_ops::set_loadout` (one undo step per pick) as the canonical `SlotLoadoutV2` shape (the
//! same `picksToLoadout` output the mod equip reads), so a pick round-trips through Save/Export.
//!
//! The domain decisions (rows, compat graph, option building, validation, doll regions, weight)
//! live in [`crate::arsenal_rules`] (pure, native-tested). This module is the UI + the persisted
//! serialization ([`picks_to_loadout`] / [`loadout_to_picks`], unchanged since the dumb Forge:
//! optic/magazine ride `weapons[0]` as sticky sub-fields).
#![allow(dead_code)]
use std::collections::HashMap;

use leptos::prelude::*;

use crate::arsenal_rules::{
    self as rules, format_loadout_weight, index_by_name, loadout_weight, row_options,
    validate_loadout, CompatFeed,
};
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
pub fn picks_to_loadout(
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
    let id = StoredValue::new(slot_id);
    // Reactive picks so the doll, weight, validation, and dependent edge rows all re-render live.
    let picks = RwSignal::new(loadout_to_picks(loadout_json.as_deref()));
    // The rail/doll active region (highlighted row + hotspot). Default to the primary weapon.
    let active_key = RwSignal::new("primary".to_string());
    #[cfg(not(target_arch = "wasm32"))]
    let _ = id;

    // Persist the current picks as the canonical V2 loadout (one undo step). wasm-only.
    let persist = move |map: &HashMap<String, String>, items: &[RegistryItem]| {
        #[cfg(target_arch = "wasm32")]
        {
            let names: HashMap<String, String> = items
                .iter()
                .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                .collect();
            crate::editor_ops::set_loadout(&id.get_value(), picks_to_loadout(map, &names));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (map, items);
    };

    view! {
        <div class="flex flex-col gap-2">
            {move || match registry.get() {
                None => view! {
                    <p class="text-label-sm normal-case text-outline">"Loading catalog…"</p>
                }.into_any(),
                Some(items) => {
                    let items = StoredValue::new(items);
                    view! {
                        <div class="grid grid-cols-[1fr_300px] gap-4">
                            // LEFT: the 14 compat-aware rows + weight + validation.
                            <div class="flex max-h-[64vh] flex-col gap-2.5 overflow-y-auto pr-1">
                                {move || {
                                    let feed = compat.get();
                                    let map = picks.get();
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let errs = validate_loadout(&map, feed.ready_graph(), feed.status);
                                    rules::LOADOUT_ROWS.iter().map(|row| {
                                        let current = map.get(row.key).cloned().unwrap_or_default();
                                        let opts = row_options(
                                            row, &current, &map, &its, &idx, feed.ready_graph(),
                                        );
                                        let err = errs.iter().find(|e| e.key == row.key).map(|e| e.message.clone());
                                        let key = row.key;
                                        let is_active = active_key.get() == row.key;
                                        let ring = if is_active { "ring-1 ring-primary/60" } else { "" };
                                        let on_change = move |ev: leptos::ev::Event| {
                                            let v = event_target_value(&ev);
                                            picks.update(|m| {
                                                if v.is_empty() { m.remove(key); } else { m.insert(key.to_string(), v.clone()); }
                                            });
                                            active_key.set(key.to_string());
                                            persist(&picks.get_untracked(), &items.get_value());
                                        };
                                        view! {
                                            <label
                                                class=format!("flex flex-col gap-1 rounded-md p-1 {ring}")
                                                on:click=move |_| active_key.set(key.to_string())
                                            >
                                                <span class="text-label-sm uppercase tracking-wider text-outline">{row.label}</span>
                                                <select prop:value=current.clone() on:change=on_change class=CONTROL>
                                                    <option value="">"— None —"</option>
                                                    {opts.into_iter().map(|o| {
                                                        view! { <option value=o.value.clone()>{o.label}</option> }
                                                    }).collect_view()}
                                                </select>
                                                {err.map(|m| view! {
                                                    <span class="text-label-sm normal-case text-error">{m}</span>
                                                })}
                                            </label>
                                        }
                                    }).collect_view()
                                }}
                            </div>
                            // RIGHT: the SVG paper-doll + weight readout.
                            <div class="flex flex-col gap-2">
                                {paper_doll(picks, active_key)}
                                {move || {
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let w = format_loadout_weight(&loadout_weight(&picks.get(), &idx));
                                    view! {
                                        <p class="text-center font-mono text-label-md tabular-nums text-on-surface-variant">{w}</p>
                                    }
                                }}
                            </div>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
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
