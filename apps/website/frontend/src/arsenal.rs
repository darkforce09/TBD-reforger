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

/// `picksToLoadout` — build the canonical `SlotLoadoutV2` from the picks. All-empty (picks AND
/// cargo) → `None` (clear the doc field). Wear map + weapons array; primary re-emits its sticky
/// `optic`/`magazine` (String or null) and the always-empty `attachments` (React hardcodes `[]`
/// until the attachments slice). `cargo`: `Some(rows)` re-emits the key verbatim on every persist
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
    if let Some(rows) = cargo {
        loadout["cargo"] = rules::cargo_rows_json(rows);
    }
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
    // T-068.15.2 — open-time cargo seed for pre-existing slots (place/apply already
    // seed at their own hooks): only fires when the loadout has no `cargo` key and
    // the character has `character_default_cargo` defaults; returns the seeded JSON
    // so this render uses it without a re-read.
    #[cfg(target_arch = "wasm32")]
    let loadout_json = crate::editor_ops::seed_slot_cargo(&slot_id).or(loadout_json);
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

    // Persist the current picks + cargo as the canonical V2 loadout (one undo step). wasm-only.
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
                                    let w = format_loadout_weight(&loadout_weight(&picks.get(), &idx));
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
                                    let err = validate_loadout(&map, feed.ready_graph(), feed.status)
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
                        <div class="flex items-center justify-between">
                            {move || {
                                let feed = compat.get();
                                let errs = validate_loadout(&picks.get(), feed.ready_graph(), feed.status);
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
                            <button
                                type="button"
                                class="flex items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10"
                                on:click=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let map = picks.get_untracked();
                                        let names: HashMap<String, String> = items
                                            .get_value()
                                            .iter()
                                            .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                                            .collect();
                                        let rows = cargo.get_untracked();
                                        let rows = cargo_present.get_untracked().then_some(rows.as_slice());
                                        let json = picks_to_loadout(&map, &names, rows)
                                            .unwrap_or_else(|| "{\"version\":2,\"wear\":{},\"weapons\":[]}".to_string());
                                        let _ = crate::mission_commands::download_json("loadout-export.json", &json);
                                    }
                                }
                            >
                                <span class="material-symbols-outlined text-[16px]">"download"</span>
                                "Download loadout JSON"
                            </button>
                        </div>
                        <p class="text-label-sm normal-case text-outline">
                            "Container cargo (mags, medical, throwables) lives in the Cargo panel above — seeded from the character's engine defaults. Dedicated equipment wear rows (binoculars, radios, glasses) come with the equipment slice."
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

/// The worn garment backing a cargo container key (`vest` accepts the armoredVest row).
fn cargo_garment(picks: &HashMap<String, String>, container: &str) -> Option<String> {
    let direct = picks.get(container).filter(|v| !v.is_empty()).cloned();
    if container == "vest" {
        return direct.or_else(|| picks.get("armoredVest").filter(|v| !v.is_empty()).cloned());
    }
    direct
}

/// T-068.15.2 — the per-container cargo editor: rows (name × qty, stepper, remove),
/// an add picker, and the warn-only budget vs the garment's registry capacity.
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
            let garment_rn = cargo_garment(&picks_now, container);
            let rows: Vec<(usize, rules::CargoRow)> = rows_now
                .iter()
                .enumerate()
                .filter(|(_, r)| r.container == container)
                .map(|(i, r)| (i, r.clone()))
                .collect();
            if garment_rn.is_none() && rows.is_empty() {
                return ().into_any();
            }
            let garment_item = garment_rn.as_deref().and_then(|rn| idx.get(rn).copied());
            let garment_label = garment_rn
                .as_ref()
                .map(|rn| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.clone())))
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

/// The right compat panel: the active pick's display name + each edge slot that depends on the
/// active region (screen 04: OPTIC "Nothing compatible." / MAGAZINE list). Rows click-pick.
fn compat_panel(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
    compat: RwSignal<CompatFeed>,
    names: StoredValue<HashMap<String, String>>,
    _items: StoredValue<Vec<RegistryItem>>,
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
    let body = if dependents.is_empty() {
        view! {
            <p class="mt-2 text-label-sm normal-case text-outline">"No dependent slots."</p>
        }
        .into_any()
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
}
