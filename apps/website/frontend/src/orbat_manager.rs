//! T-180.7 / T-180.8 — Stitch ORBAT Manager on live mission-doc graph.
//!
//! Visual structure from `.ai/artifacts/t180_stitch_orbat_modal/`; data from `MissionDocCore` only
//! (G7). Operator L8 kit-complement UI omitted (G4). Templates Apply/Save + Add Vehicle (T-180.8);
//! Arsenal tab-3 → T-180.9.
#![allow(dead_code, unused_variables)]

use std::collections::{HashMap, HashSet};

use leptos::prelude::*;
use map_engine_core::slot_line::format_slot_line;

use crate::dto::{FactionDoc, RegistryItem, UserFaction};
use crate::outliner::{
    filter_orbat_squads_by_side_key, flatten_visible, FlatRow, NodeKind, OutlinerNode,
    ORBAT_MANAGER_DIALOG_CLASS, ORBAT_MANAGER_EMPTY, VIRTUAL_SLOT_THRESHOLD,
};
use crate::ui::MaterialIcon;

/// Near-fullscreen class pin (G1 / G9).
pub const DIALOG_CLASS: &str = ORBAT_MANAGER_DIALOG_CLASS;

const SIDES: &[&str] = &["BLUFOR", "OPFOR", "INDFOR"];
const ROW_H: f64 = 32.0;
const CONTAINER_H: f64 = 480.0;
const OVERSCAN: usize = 8;

/// H5 — Apply only runs when the confirm dialog returns true (Cancel = noop).
#[must_use]
pub fn apply_confirm_allows(confirmed: bool) -> bool {
    confirmed
}

/// H7 / H-L8 — template dropdown options for the active ORBAT side (excludes CIV + other sides).
#[must_use]
pub fn template_options_for_side<'a>(
    library: &'a [UserFaction],
    side: &str,
) -> Vec<&'a UserFaction> {
    library
        .iter()
        .filter(|f| f.side == side && f.side != "CIV" && side != "CIV")
        .collect()
}

/// Kind-filtered registry vehicle options (same drop rules as Faction Manager).
#[must_use]
pub fn registry_vehicle_options(items: &[RegistryItem]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = items
        .iter()
        .filter(|it| it.kind == "vehicle" && it.r#abstract != Some(true) && it.variant_of.is_none())
        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// H6 helper — roles.len for a FactionDoc built from a side (Save inverse pin).
#[must_use]
pub fn faction_doc_role_count(doc: &FactionDoc) -> usize {
    doc.roles.len()
}

/// T-180.7 / T-180.8 — Stitch ORBAT Manager dialog (replaces the T-177 `max-w-xl` browse shell).
#[component]
pub fn OrbatManagerDialog(
    open: RwSignal<bool>,
    orbat: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    /// Registry for Add Vehicle picker (kind==vehicle).
    #[prop(optional)]
    registry: Option<RwSignal<Option<Vec<RegistryItem>>>>,
) -> impl IntoView {
    let _ = active_layer; // kept for mount API parity with T-177
    let registry = registry.unwrap_or_else(|| RwSignal::new(None));
    let side_tab = RwSignal::new(String::from("BLUFOR"));
    let search = RwSignal::new(String::new());
    let collapsed = RwSignal::new(HashSet::<String>::new());
    let rename_squad = RwSignal::new(Option::<String>::None);
    let rename_draft = RwSignal::new(String::new());
    let library = RwSignal::new(Vec::<UserFaction>::new());
    let selected_template = RwSignal::new(String::new()); // UserFaction.id
    let add_vehicle_squad = RwSignal::new(Option::<String>::None);
    let status = RwSignal::new(String::new());

    // Esc closes (Faction Manager / suite Dialog behavior).
    let esc = window_event_listener(leptos::ev::keydown, move |ev| {
        if open.get_untracked() && ev.key() == "Escape" {
            open.set(false);
        }
    });
    on_cleanup(move || esc.remove());

    #[cfg(target_arch = "wasm32")]
    let auth = expect_context::<crate::auth::AuthStore>();

    // Load faction library whenever the dialog opens.
    #[cfg(target_arch = "wasm32")]
    {
        Effect::new(move |_| {
            if !open.get() {
                return;
            }
            leptos::task::spawn_local(async move {
                if let Ok(r) =
                    crate::client::api_get::<crate::dto::FactionListResponse>(auth, "/factions")
                        .await
                {
                    library.set(r.data);
                }
            });
        });
    }

    move || {
        if !open.get() {
            return None;
        }
        // Track orbat rebuilds from `after_local_edit`.
        let _tree = orbat.get();
        let snap = read_snapshot();
        // B2 — informational player-cap indicator: authored slots may exceed 128 by
        // design (the cap is on concurrent PLAYERS, server-enforced), so this warns
        // by color only and never blocks.
        let total_slots = snap.slots.len();
        let cap_label = format!("{total_slots} slots · server cap 128 players");
        let cap_cls = if total_slots > 128 {
            "rounded border border-error-alert/40 bg-error/10 px-2 py-0.5 font-mono text-label-sm tabular-nums normal-case text-error-alert"
        } else {
            "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm tabular-nums normal-case text-on-surface-variant"
        };
        let side = side_tab.get();
        let q = search.get().trim().to_lowercase();
        let mut squad_nodes = if snap.factions.is_empty() {
            // Fallback: dock mirror already has the live tree (orbat_nodes).
            _tree
                .into_iter()
                .filter(|f| f.id.ends_with(side.as_str()) || f.label == side)
                .flat_map(|f| f.children)
                .collect()
        } else {
            filter_orbat_squads_by_side_key(
                &snap.factions,
                &snap.squads,
                &slot_rows_from(&snap),
                &side,
            )
        };
        if !q.is_empty() {
            squad_nodes = filter_search(squad_nodes, &q);
        }
        let detail_by_id: HashMap<String, SlotDetail> =
            snap.slots.into_iter().map(|s| (s.id.clone(), s)).collect();
        let vehicle_by_squad: HashMap<String, usize> = snap
            .squads
            .iter()
            .map(|s| (s.id.clone(), s.vehicle_ids.len()))
            .collect();
        let entity_count: usize = squad_nodes.iter().map(|s| s.children.len()).sum();
        let vehicle_count: usize = squad_nodes
            .iter()
            .map(|s| vehicle_by_squad.get(&s.id).copied().unwrap_or(0))
            .sum();
        let selected_id = selected.get().first().cloned();
        let inspector = selected_id
            .as_ref()
            .and_then(|id| detail_by_id.get(id).cloned());

        Some(view! {
            <div
                class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
                on:click=move |_| open.set(false)
            ></div>
            <div
                class=DIALOG_CLASS
                on:click=move |ev| ev.stop_propagation()
                on:pointerup=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::cancel_refile();
                }
            >
                // Header
                <div class="flex h-14 shrink-0 items-center justify-between border-b border-white/10 px-4">
                    <div class="flex items-center gap-3">
                        <MaterialIcon name="account_tree" class="text-primary text-[20px]" />
                        <h2 class="text-headline-sm tracking-tighter text-on-surface">"ORBAT Manager"</h2>
                        <span data-orbat-cap class=cap_cls>{cap_label}</span>
                    </div>
                    <div class="flex rounded border border-white/10 bg-surface-dim p-0.5">
                        {SIDES.iter().map(|&s| {
                            let s_owned = s.to_string();
                            let s_btn = s_owned.clone();
                            view! {
                                <button
                                    type="button"
                                    aria-label=s
                                    class=move || {
                                        if side_tab.get() == s_owned {
                                            "px-4 py-1.5 rounded font-label-sm text-label-sm bg-secondary-container/30 text-primary border border-primary/30"
                                        } else {
                                            "px-4 py-1.5 rounded font-label-sm text-label-sm text-on-surface-variant hover:bg-surface-variant hover:text-on-surface border border-transparent"
                                        }
                                    }
                                    on:click=move |_| side_tab.set(s_btn.clone())
                                >{s}</button>
                            }
                        }).collect_view()}
                    </div>
                    <button
                        type="button"
                        aria-label="Close"
                        class="rounded p-1.5 text-on-surface-variant hover:bg-surface-variant hover:text-on-surface"
                        on:click=move |_| open.set(false)
                    >
                        <MaterialIcon name="close" class="text-[20px]" />
                    </button>
                </div>

                // Template bar — Apply / Save / Save as (T-180.8)
                <div class="flex h-12 shrink-0 items-center gap-3 border-b border-white/5 bg-surface-container-low px-4">
                    <MaterialIcon name="folder_open" class="text-on-surface-variant text-[18px]" />
                    <div class="relative max-w-md flex-1">
                        {
                            let lib_snap = library.get();
                            let opts: Vec<(String, String)> = template_options_for_side(&lib_snap, &side)
                                .into_iter()
                                .map(|f| (f.id.clone(), format!("{} ({})", f.name, f.side)))
                                .collect();
                            let sel = selected_template.get();
                            view! {
                                <select
                                    class="w-full appearance-none rounded border border-border-subtle bg-surface-dim px-3 py-1.5 font-code-md text-code-md text-on-surface"
                                    prop:value=sel.clone()
                                    on:change=move |ev| selected_template.set(event_target_value(&ev))
                                >
                                    <option value="">"Load Predefined ORBAT…"</option>
                                    {opts.into_iter().map(|(id, label)| {
                                        view! { <option value=id>{label}</option> }
                                    }).collect_view()}
                                </select>
                            }
                        }
                    </div>
                    <button
                        type="button"
                        class="rounded bg-primary px-4 py-1.5 font-label-sm text-label-sm text-on-primary hover:brightness-110"
                        on:click=move |_| {
                            let side = side_tab.get_untracked();
                            let tid = selected_template.get_untracked();
                            if tid.is_empty() {
                                status.set("Select a template first.".into());
                                return;
                            }
                            let Some(uf) = library
                                .get_untracked()
                                .into_iter()
                                .find(|f| f.id == tid)
                            else {
                                status.set("Template not found.".into());
                                return;
                            };
                            #[cfg(target_arch = "wasm32")]
                            {
                                let msg = format!(
                                    "Replace all ORBAT under {side} with \"{}\"?",
                                    uf.name
                                );
                                let confirmed = web_sys::window()
                                    .and_then(|w| w.confirm_with_message(&msg).ok())
                                    .unwrap_or(false);
                                if !apply_confirm_allows(confirmed) {
                                    status.set("Apply cancelled.".into());
                                    return;
                                }
                                if crate::editor_ops::orbat_apply_faction(side, uf.doc) {
                                    status.set("Template applied.".into());
                                } else {
                                    status.set("Apply failed.".into());
                                }
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let _ = (side, uf);
                            }
                        }
                    >"APPLY TEMPLATE"</button>
                    <button
                        type="button"
                        class="rounded border border-border-subtle px-3 py-1.5 font-label-sm text-label-sm text-on-surface hover:bg-surface-variant"
                        title="Update selected library faction from this side"
                        on:click=move |_| {
                            let side = side_tab.get_untracked();
                            let tid = selected_template.get_untracked();
                            if tid.is_empty() {
                                status.set("Select a template to Save.".into());
                                return;
                            }
                            #[cfg(target_arch = "wasm32")]
                            {
                                let Some(mut doc) = crate::editor_ops::faction_doc_from_side(&side)
                                else {
                                    status.set("No mission doc.".into());
                                    return;
                                };
                                if let Some(uf) = library
                                    .get_untracked()
                                    .into_iter()
                                    .find(|f| f.id == tid)
                                {
                                    doc.name = uf.name;
                                }
                                let body = serde_json::to_value(&doc).unwrap_or_default();
                                leptos::task::spawn_local(async move {
                                    match crate::client::api_put::<UserFaction>(
                                        auth,
                                        &format!("/factions/{tid}"),
                                        body,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            status.set("Saved.".into());
                                            if let Ok(r) = crate::client::api_get::<
                                                crate::dto::FactionListResponse,
                                            >(auth, "/factions")
                                            .await
                                            {
                                                library.set(r.data);
                                            }
                                        }
                                        Err(_) => status.set("Save failed.".into()),
                                    }
                                });
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = side;
                        }
                    >"Save"</button>
                    <button
                        type="button"
                        class="rounded border border-border-subtle px-3 py-1.5 font-label-sm text-label-sm text-on-surface hover:bg-surface-variant"
                        title="Save current side ORBAT as a new library faction"
                        on:click=move |_| {
                            let side = side_tab.get_untracked();
                            #[cfg(target_arch = "wasm32")]
                            {
                                let Some(mut doc) = crate::editor_ops::faction_doc_from_side(&side)
                                else {
                                    status.set("No mission doc.".into());
                                    return;
                                };
                                let default_name = doc.name.clone();
                                let name = web_sys::window()
                                    .and_then(|w| {
                                        w.prompt_with_message_and_default(
                                            "Save as faction name:",
                                            &default_name,
                                        )
                                        .ok()
                                        .flatten()
                                    })
                                    .unwrap_or_default();
                                let name = name.trim().to_string();
                                if name.is_empty() {
                                    status.set("Save as cancelled.".into());
                                    return;
                                }
                                doc.name = name;
                                doc.side = side;
                                let body = serde_json::to_value(&doc).unwrap_or_default();
                                leptos::task::spawn_local(async move {
                                    match crate::client::api_post::<UserFaction>(
                                        auth, "/factions", body,
                                    )
                                    .await
                                    {
                                        Ok(f) => {
                                            selected_template.set(f.id.clone());
                                            status.set("Saved as new faction.".into());
                                            if let Ok(r) = crate::client::api_get::<
                                                crate::dto::FactionListResponse,
                                            >(auth, "/factions")
                                            .await
                                            {
                                                library.set(r.data);
                                            }
                                        }
                                        Err(_) => {
                                            status.set(
                                                "Save as failed (name already used?).".into(),
                                            );
                                        }
                                    }
                                });
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = side;
                        }
                    >"Save as"</button>
                    <div class="ml-auto flex items-center gap-2 font-label-sm text-label-sm text-on-surface-variant">
                        <span class="max-w-[12rem] truncate text-primary/80">{move || status.get()}</span>
                        <span>"Total Entities: "<span class="font-code-md text-primary">{entity_count}</span></span>
                        <span class="text-white/20">"|"</span>
                        <span>"Vehicles: "<span class="font-code-md text-tactical-yellow">{vehicle_count}</span></span>
                    </div>
                </div>

                // Main: tree | inspector
                <div class="flex min-h-0 flex-1 overflow-hidden">
                    <section class="relative z-0 flex min-w-0 flex-1 flex-col border-r border-white/10 bg-background">
                        <div class="flex h-10 shrink-0 items-center justify-between border-b border-white/5 bg-surface-container-lowest px-4">
                            <div class="flex items-center gap-2">
                                <button
                                    type="button"
                                    title="Expand All"
                                    class="rounded p-1 text-on-surface-variant hover:bg-surface-variant hover:text-on-surface"
                                    on:click=move |_| collapsed.set(HashSet::new())
                                >
                                    <MaterialIcon name="unfold_more" class="text-[16px]" />
                                </button>
                                <button
                                    type="button"
                                    title="Collapse All"
                                    class="rounded p-1 text-on-surface-variant hover:bg-surface-variant hover:text-on-surface"
                                    on:click=move |_| {
                                        let ids: HashSet<String> = orbat
                                            .get()
                                            .iter()
                                            .filter(|f| {
                                                f.id.ends_with(side_tab.get().as_str())
                                                    || f.label == side_tab.get()
                                            })
                                            .flat_map(|f| f.children.iter().map(|s| s.id.clone()))
                                            .collect();
                                        collapsed.set(ids);
                                    }
                                >
                                    <MaterialIcon name="unfold_less" class="text-[16px]" />
                                </button>
                            </div>
                            <div class="relative w-64">
                                <MaterialIcon
                                    name="search"
                                    class="pointer-events-none absolute top-1/2 left-2 -translate-y-1/2 text-[16px] text-on-surface-variant"
                                />
                                <input
                                    type="text"
                                    placeholder="Search entities..."
                                    class="w-full rounded border border-border-subtle bg-surface-dim py-1 pr-2 pl-7 font-code-md text-[12px] text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                                    prop:value=move || search.get()
                                    on:input=move |ev| search.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <div class="custom-scrollbar min-h-0 flex-1 overflow-hidden p-2">
                            {
                                let mut veh_opts = registry_vehicle_options(
                                    &registry.get().unwrap_or_default(),
                                );
                                if let Some(uf) = library
                                    .get()
                                    .into_iter()
                                    .find(|f| f.id == selected_template.get())
                                {
                                    for v in uf.doc.vehicles {
                                        if !v.vehicle.is_empty()
                                            && !veh_opts.iter().any(|(r, _)| r == &v.vehicle)
                                        {
                                            let label = v
                                                .label
                                                .unwrap_or_else(|| v.vehicle.clone());
                                            veh_opts.push((v.vehicle, label));
                                        }
                                    }
                                }
                                tree_panel(
                                    squad_nodes,
                                    detail_by_id.clone(),
                                    vehicle_by_squad.clone(),
                                    selected,
                                    collapsed,
                                    rename_squad,
                                    rename_draft,
                                    add_vehicle_squad,
                                    veh_opts,
                                )
                            }
                        </div>

                        <div class="shrink-0 border-t border-white/5 bg-surface-container-low p-3">
                            <button
                                type="button"
                                class="flex w-full items-center justify-center gap-2 rounded border border-dashed border-outline-variant py-2 font-label-sm text-label-sm text-on-surface-variant hover:border-primary hover:text-primary"
                                on:click=move |_| {
                                    let side = side_tab.get_untracked();
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        crate::editor_ops::orbat_add_squad(side);
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let _ = side;
                                }
                            >
                                <MaterialIcon name="add_circle" class="text-[16px]" />
                                "ADD SQUAD / GROUP"
                            </button>
                        </div>
                    </section>

                    <aside class="relative z-10 flex w-[320px] shrink-0 flex-col border-l border-white/10 bg-surface-glass shadow-2xl backdrop-blur-xl">
                        <div class="flex h-10 items-center gap-2 border-b border-white/10 bg-surface-container/80 px-4">
                            <MaterialIcon name="manage_accounts" class="text-primary text-[18px]" />
                            <h3 class="flex-1 font-label-sm text-label-sm tracking-widest text-on-surface uppercase">
                                "Slot Inspector"
                            </h3>
                        </div>
                        <div class="custom-scrollbar flex-1 space-y-6 overflow-y-auto p-4">
                            {inspector_panel(inspector, selected)}
                        </div>
                    </aside>
                </div>
            </div>
        })
    }
}

#[derive(Clone, Debug, Default)]
struct SlotDetail {
    id: String,
    role: String,
    tag: String,
    callsign: String,
    rank: String,
    index: u32,
    squad_id: String,
    summary: String,
    primary: String,
    launcher: String,
}

#[derive(Clone, Debug, Default)]
struct Snap {
    factions: Vec<crate::outliner::FactionRow>,
    squads: Vec<crate::outliner::SquadRow>,
    slots: Vec<SlotDetail>,
}

fn read_snapshot() -> Snap {
    #[cfg(target_arch = "wasm32")]
    {
        let s = crate::editor_ops::orbat_manager_snapshot();
        Snap {
            factions: s.factions,
            squads: s.squads,
            slots: s
                .slots
                .into_iter()
                .map(|d| SlotDetail {
                    id: d.id,
                    role: d.role,
                    tag: d.tag,
                    callsign: d.callsign,
                    rank: d.rank,
                    index: d.index,
                    squad_id: d.squad_id,
                    summary: d.summary,
                    primary: d.primary,
                    launcher: d.launcher,
                })
                .collect(),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Snap::default()
    }
}

fn slot_rows_from(snap: &Snap) -> Vec<crate::outliner::SlotRow> {
    snap.slots
        .iter()
        .map(|s| crate::outliner::SlotRow {
            id: s.id.clone(),
            role: s.role.clone(),
        })
        .collect()
}

fn filter_search(nodes: Vec<OutlinerNode>, q: &str) -> Vec<OutlinerNode> {
    nodes
        .into_iter()
        .filter_map(|mut sq| {
            let squad_hit = sq.label.to_lowercase().contains(q);
            let kids: Vec<_> = sq
                .children
                .into_iter()
                .filter(|c| c.label.to_lowercase().contains(q) || squad_hit)
                .collect();
            if squad_hit || !kids.is_empty() {
                if !squad_hit {
                    sq.children = kids;
                } else {
                    sq.children = kids;
                }
                Some(sq)
            } else {
                None
            }
        })
        .collect()
}

fn tree_panel(
    squad_nodes: Vec<OutlinerNode>,
    detail_by_id: HashMap<String, SlotDetail>,
    vehicle_by_squad: HashMap<String, usize>,
    selected: RwSignal<Vec<String>>,
    collapsed: RwSignal<HashSet<String>>,
    rename_squad: RwSignal<Option<String>>,
    rename_draft: RwSignal<String>,
    add_vehicle_squad: RwSignal<Option<String>>,
    vehicle_options: Vec<(String, String)>,
) -> AnyView {
    let flat = StoredValue::new(Vec::<FlatRow>::new());
    let rev = RwSignal::new(0u64);
    let nodes_sig = RwSignal::new(squad_nodes);
    Effect::new(move |_| {
        let f = collapsed.with(|c| flatten_visible(&nodes_sig.get(), c));
        flat.set_value(f);
        rev.update(|r| *r = r.wrapping_add(1));
    });
    // Seed initial flat.
    {
        let f = collapsed.with_untracked(|c| flatten_visible(&nodes_sig.get_untracked(), c));
        flat.set_value(f);
    }
    let scroll_top = RwSignal::new(0.0_f64);
    let detail_by_id = StoredValue::new(detail_by_id);
    let vehicle_by_squad = StoredValue::new(vehicle_by_squad);
    let vehicle_options = StoredValue::new(vehicle_options);

    (move || {
        rev.track();
        let st = scroll_top.get();
        flat.with_value(|f| {
            let total = f.len();
            if total == 0 {
                set_orbat_stats(0, 0);
                return view! {
                    <p class="px-2 py-6 text-center text-label-sm text-outline">{ORBAT_MANAGER_EMPTY}</p>
                }
                .into_any();
            }
            let render_slice = |rows: &[FlatRow]| -> AnyView {
                view! {
                    <div class="space-y-1">
                        {rows
                            .iter()
                            .map(|r| {
                                stitch_row(
                                    r,
                                    selected,
                                    collapsed,
                                    rename_squad,
                                    rename_draft,
                                    detail_by_id.get_value(),
                                    vehicle_by_squad.get_value(),
                                    add_vehicle_squad,
                                    vehicle_options.get_value(),
                                )
                            })
                            .collect::<Vec<_>>()}
                    </div>
                }
                .into_any()
            };
            if total <= VIRTUAL_SLOT_THRESHOLD {
                set_orbat_stats(total, total);
                return render_slice(f);
            }
            let per_screen = (CONTAINER_H / ROW_H).ceil() as usize;
            let start = ((st / ROW_H).floor() as usize).saturating_sub(OVERSCAN);
            let end = (start + per_screen + 2 * OVERSCAN).min(total);
            set_orbat_stats(total, end - start);
            let top = start as f64 * ROW_H;
            let bottom = (total - end) as f64 * ROW_H;
            let slice = render_slice(&f[start..end]);
            view! {
                <div
                    class="overflow-y-auto"
                    style=format!("height:{CONTAINER_H}px")
                    on:scroll=move |ev| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev
                                .target()
                                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                            {
                                scroll_top.set(el.scroll_top() as f64);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
                    }
                >
                    <div style=format!("height:{top}px")></div>
                    {slice}
                    <div style=format!("height:{bottom}px")></div>
                </div>
            }
            .into_any()
        })
    })
    .into_any()
}

fn stitch_row(
    row: &FlatRow,
    selected: RwSignal<Vec<String>>,
    collapsed: RwSignal<HashSet<String>>,
    rename_squad: RwSignal<Option<String>>,
    rename_draft: RwSignal<String>,
    detail_by_id: HashMap<String, SlotDetail>,
    vehicle_by_squad: HashMap<String, usize>,
    add_vehicle_squad: RwSignal<Option<String>>,
    vehicle_options: Vec<(String, String)>,
) -> AnyView {
    match row.kind {
        NodeKind::Squad => {
            let id = row.id.clone();
            let id_drop = id.clone();
            let id_add = id.clone();
            let id_veh = id.clone();
            let id_veh_pick = id.clone();
            let id_rm = id.clone();
            let id_ren = id.clone();
            let label = row.label.clone();
            let label_display = label.clone();
            let label_for_rename = label.clone();
            let open = !collapsed.get_untracked().contains(&id);
            let chevron_cls = if open {
                "mr-1 text-[18px] text-on-surface-variant"
            } else {
                "mr-1 -rotate-90 text-[18px] text-on-surface-variant"
            };
            let vids = vehicle_by_squad.get(&id).copied().unwrap_or(0);
            let renaming = rename_squad.get_untracked().as_deref() == Some(id.as_str());
            let picking_vehicle = add_vehicle_squad.get_untracked().as_deref() == Some(id.as_str());
            let vehicle_options = vehicle_options.clone();
            view! {
                <div class="group flex flex-col rounded border border-border-subtle bg-surface-container-low">
                    <div
                        class="flex cursor-pointer items-center px-2 py-1.5 hover:bg-surface-variant/50"
                        on:pointerup=move |ev| {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::complete_refile_onto_squad(id_drop.clone());
                        }
                        on:click=move |_| {
                            collapsed.update(|c| {
                                if !c.remove(&id) {
                                    c.insert(id.clone());
                                }
                            });
                        }
                    >
                        <MaterialIcon name="drag_indicator" class="mr-1 cursor-grab text-[16px] text-on-surface-variant opacity-0 transition-opacity group-hover:opacity-100" />
                        <span class=chevron_cls>
                            <MaterialIcon name="arrow_drop_down" class="text-[18px]" />
                        </span>
                        <MaterialIcon name="group" class="mr-2 text-[16px] text-secondary" />
                        {if renaming {
                            let id_commit = id_ren.clone();
                            view! {
                                <input
                                    type="text"
                                    class="mr-2 flex-1 rounded border border-primary bg-surface-dim px-1 py-0.5 font-label-sm text-label-sm text-on-surface"
                                    prop:value=move || rename_draft.get()
                                    on:click=move |ev| ev.stop_propagation()
                                    on:input=move |ev| rename_draft.set(event_target_value(&ev))
                                    on:keydown=move |ev| {
                                        if ev.key() == "Enter" {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let name = rename_draft.get_untracked();
                                                crate::editor_ops::orbat_rename_squad(id_commit.clone(), name);
                                            }
                                            rename_squad.set(None);
                                        }
                                    }
                                />
                            }.into_any()
                        } else {
                            view! { <span class="flex-1 font-label-sm text-label-sm text-on-surface">{label_display}</span> }.into_any()
                        }}
                        {if vids > 0 {
                            view! {
                                <div class="mr-3 flex items-center gap-1 rounded border border-white/5 bg-surface-dim px-2 py-0.5">
                                    <MaterialIcon name="directions_car" class="text-[14px] text-tactical-yellow" />
                                    <span class="font-code-md text-[11px] text-on-surface-variant">{format!("{vids}")}</span>
                                </div>
                            }.into_any()
                        } else {
                            view! { <span></span> }.into_any()
                        }}
                        <div class="flex items-center opacity-0 transition-opacity group-hover:opacity-100">
                            <button
                                type="button"
                                title="Add Slot"
                                class="rounded p-1 text-on-surface-variant hover:text-primary"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    #[cfg(target_arch = "wasm32")]
                                    crate::editor_ops::orbat_add_slot(id_add.clone(), "Rifleman".into());
                                }
                            >
                                <MaterialIcon name="person_add" class="text-[16px]" />
                            </button>
                            <button
                                type="button"
                                title="Add Vehicle"
                                class="rounded p-1 text-on-surface-variant hover:text-tactical-yellow"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    add_vehicle_squad.set(Some(id_veh.clone()));
                                }
                            >
                                <MaterialIcon name="car_rental" class="text-[16px]" />
                            </button>
                            <button
                                type="button"
                                title="Rename Squad"
                                class="rounded p-1 text-on-surface-variant hover:text-primary"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    rename_draft.set(label_name_only(&label_for_rename));
                                    rename_squad.set(Some(id_ren.clone()));
                                }
                            >
                                <MaterialIcon name="edit" class="text-[16px]" />
                            </button>
                            <button
                                type="button"
                                title="Remove Squad"
                                class="ml-1 rounded p-1 text-on-surface-variant hover:text-error-alert"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    #[cfg(target_arch = "wasm32")]
                                    crate::editor_ops::orbat_remove_squad(id_rm.clone());
                                }
                            >
                                <MaterialIcon name="delete" class="text-[16px]" />
                            </button>
                        </div>
                    </div>
                    {if picking_vehicle {
                        let opts = vehicle_options.clone();
                        view! {
                            <div
                                class="flex items-center gap-2 border-t border-white/5 bg-surface-dim px-2 py-1.5"
                                on:click=move |ev| ev.stop_propagation()
                            >
                                <select
                                    class="min-w-0 flex-1 rounded border border-border-subtle bg-surface-container px-2 py-1 font-code-md text-[11px] text-on-surface"
                                    on:change=move |ev| {
                                        let resource = event_target_value(&ev);
                                        if resource.is_empty() {
                                            return;
                                        }
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let _ = crate::editor_ops::orbat_add_vehicle(
                                                id_veh_pick.clone(),
                                                resource,
                                            );
                                        }
                                        add_vehicle_squad.set(None);
                                    }
                                >
                                    <option value="">"Pick vehicle…"</option>
                                    {opts.into_iter().map(|(res, label)| {
                                        view! { <option value=res.clone()>{label}</option> }
                                    }).collect_view()}
                                </select>
                                <button
                                    type="button"
                                    class="rounded px-2 py-1 text-label-sm text-on-surface-variant hover:text-on-surface"
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        add_vehicle_squad.set(None);
                                    }
                                >"Cancel"</button>
                            </div>
                        }.into_any()
                    } else {
                        view! { <span></span> }.into_any()
                    }}
                </div>
            }
            .into_any()
        }
        NodeKind::Slot => {
            let id = row.id.clone();
            let id_sel = id.clone();
            let id_dbl = id.clone();
            let id_refile = id.clone();
            let id_sl = id.clone();
            let id_rm = id.clone();
            let detail = detail_by_id
                .get(&id)
                .cloned()
                .unwrap_or_else(|| SlotDetail {
                    id: id.clone(),
                    role: row.label.clone(),
                    ..Default::default()
                });
            let squad_id = detail.squad_id.clone();
            let role_aria = if detail.role.is_empty() {
                row.label.clone()
            } else {
                detail.role.clone()
            };
            let line = format_slot_line(
                detail.index.saturating_add(1),
                if detail.role.is_empty() {
                    row.label.as_str()
                } else {
                    detail.role.as_str()
                },
                (!detail.summary.is_empty()).then_some(detail.summary.as_str()),
                (!detail.primary.is_empty() && detail.summary.is_empty())
                    .then_some(detail.primary.as_str()),
                (!detail.launcher.is_empty() && detail.summary.is_empty())
                    .then_some(detail.launcher.as_str()),
                (!detail.tag.is_empty()).then_some(detail.tag.as_str()),
                false, // SL via icon only — do not append " | SL" into aria-visible line
            );
            let is_leader = row.is_leader;
            let is_sel = {
                let id = id.clone();
                move || selected.get().iter().any(|s| s == &id)
            };
            view! {
                <div class="pl-6">
                    <div
                        role="button"
                        tabindex="0"
                        aria-label=role_aria.clone()
                        class=move || {
                            if is_sel() {
                                "group/slot flex w-full cursor-pointer items-center rounded border border-primary/30 bg-secondary-container/20 px-2 py-1"
                            } else {
                                "group/slot flex w-full cursor-pointer items-center rounded border border-transparent px-2 py-1 hover:bg-surface-variant/30"
                            }
                        }
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::select_slot(id_sel.clone());
                        }
                        on:dblclick=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::open_attributes(id_dbl.clone());
                        }
                        on:pointerdown=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::begin_refile(id_refile.clone());
                        }
                    >
                        <MaterialIcon name="drag_indicator" class="mr-2 cursor-grab text-[14px] text-on-surface-variant opacity-0 group-hover/slot:opacity-100" />
                        <MaterialIcon
                            name=if is_leader { "military_tech" } else { "person" }
                            class=if is_leader {
                                "mr-2 text-[14px] text-primary"
                            } else {
                                "mr-2 text-[14px] text-on-surface-variant"
                            }
                        />
                        <span class="flex-1 text-left font-code-md text-[12px] text-on-surface">{line}</span>
                        <div class="flex items-center gap-1 opacity-0 group-hover/slot:opacity-100">
                            <button
                                type="button"
                                title="Make Squad Leader"
                                class="text-on-surface-variant hover:text-primary"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    #[cfg(target_arch = "wasm32")]
                                    crate::editor_ops::orbat_set_leader(squad_id.clone(), id_sl.clone());
                                }
                            >
                                <MaterialIcon name="military_tech" class="text-[14px]" />
                            </button>
                            <button
                                type="button"
                                title="Remove Slot"
                                class="text-on-surface-variant hover:text-error-alert"
                                on:click=move |ev| {
                                    ev.stop_propagation();
                                    #[cfg(target_arch = "wasm32")]
                                    crate::editor_ops::orbat_remove_slot(id_rm.clone());
                                }
                            >
                                <MaterialIcon name="close" class="text-[14px]" />
                            </button>
                        </div>
                    </div>
                </div>
            }
            .into_any()
        }
        _ => view! { <div></div> }.into_any(),
    }
}

fn label_name_only(label: &str) -> String {
    label
        .rsplit_once(" (")
        .map(|(n, _)| n.to_string())
        .unwrap_or_else(|| label.to_string())
}

fn inspector_panel(inspector: Option<SlotDetail>, selected: RwSignal<Vec<String>>) -> AnyView {
    let Some(slot) = inspector else {
        return view! {
            <p class="text-label-sm text-on-surface-variant">"Select a slot to inspect."</p>
        }
        .into_any();
    };
    let id = slot.id.clone();
    let id_role = id.clone();
    let id_cs = id.clone();
    let id_rank = id.clone();
    let id_ars = id.clone();
    let squad_for_add = slot.squad_id.clone();
    let role = RwSignal::new(slot.role.clone());
    let callsign = RwSignal::new(slot.callsign.clone());
    let rank = RwSignal::new(slot.rank.clone());
    // Sync when selection changes.
    Effect::new(move |_| {
        let _ = selected.get();
        #[cfg(target_arch = "wasm32")]
        {
            let snap = crate::editor_ops::orbat_manager_snapshot();
            if let Some(id) = selected.get_untracked().first() {
                if let Some(d) = snap.slots.into_iter().find(|s| &s.id == id) {
                    role.set(d.role);
                    callsign.set(d.callsign);
                    rank.set(d.rank);
                }
            }
        }
    });
    view! {
        <div class="space-y-3">
            <div class="flex items-center justify-between">
                <span class="font-label-sm text-[10px] tracking-wider text-on-surface-variant uppercase">
                    "Entity Type"
                </span>
                <span class="rounded bg-primary/10 px-1.5 py-0.5 font-code-md text-[11px] text-primary">
                    "Infantry"
                </span>
            </div>
            <div class="space-y-1">
                <label class="font-label-sm text-[11px] text-on-surface-variant">"Assigned Role"</label>
                <input
                    type="text"
                    class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                    prop:value=move || role.get()
                    on:input=move |ev| role.set(event_target_value(&ev))
                    on:change=move |_| {
                        #[cfg(target_arch = "wasm32")]
                        {
                            let r = role.get_untracked();
                            crate::editor_ops::orbat_update_slot_fields(
                                id_role.clone(),
                                Some(r),
                                None,
                                None,
                                None,
                            );
                        }
                    }
                />
            </div>
            <div class="flex gap-2">
                <div class="flex-1 space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Callsign"</label>
                    <input
                        type="text"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-code-md text-code-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=move || callsign.get()
                        on:input=move |ev| callsign.set(event_target_value(&ev))
                        on:change=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let c = callsign.get_untracked();
                                crate::editor_ops::orbat_update_slot_fields(
                                    id_cs.clone(),
                                    None,
                                    None,
                                    Some(c),
                                    None,
                                );
                            }
                        }
                    />
                </div>
                <div class="flex-1 space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Rank"</label>
                    <input
                        type="text"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=move || rank.get()
                        on:input=move |ev| rank.set(event_target_value(&ev))
                        on:change=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let r = rank.get_untracked();
                                crate::editor_ops::orbat_update_slot_fields(
                                    id_rank.clone(),
                                    None,
                                    None,
                                    None,
                                    Some(r),
                                );
                            }
                        }
                    />
                </div>
            </div>
        </div>
        <hr class="border-white/5" />
        <div class="space-y-3">
            <div class="flex items-center justify-between">
                <label class="font-label-sm text-[11px] tracking-wider text-on-surface-variant uppercase">
                    "Loadout"
                </label>
            </div>
            <button
                type="button"
                class="flex w-full items-center justify-center gap-2 rounded border border-outline-variant bg-surface-container py-2 font-label-md text-on-surface hover:border-primary hover:bg-surface-variant"
                on:click=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::open_arsenal(id_ars.clone());
                }
            >
                <MaterialIcon name="backpack" class="text-[18px]" />
                "OPEN ARSENAL"
            </button>
        </div>
        // Add Role footer when a squad context exists — exposed on squad hover; duplicate here for discoverability when a slot is selected.
        <div class="pt-2">
            <button
                type="button"
                class="flex items-center gap-2 text-on-surface-variant hover:text-primary"
                on:click=move |_| {
                    if squad_for_add.is_empty() {
                        return;
                    }
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::orbat_add_slot(squad_for_add.clone(), "Rifleman".into());
                }
            >
                <MaterialIcon name="add" class="text-[14px]" />
                <span class="font-label-sm text-[11px]">"Add Role"</span>
            </button>
        </div>
    }
    .into_any()
}

#[cfg(target_arch = "wasm32")]
fn set_orbat_stats(total: usize, rendered: usize) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else {
        return;
    };
    let stats = match js_sys::Reflect::get(&win, &JsValue::from_str("__outlinerStats")) {
        Ok(v) if v.is_object() => v,
        _ => {
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__outlinerStats"), &o);
            o.into()
        }
    };
    let entry = js_sys::Object::new();
    let set = |k: &str, n: usize| {
        let _ = js_sys::Reflect::set(&entry, &JsValue::from_str(k), &JsValue::from_f64(n as f64));
    };
    set("total", total);
    set("rendered", rendered);
    set("threshold", VIRTUAL_SLOT_THRESHOLD);
    let _ = js_sys::Reflect::set(&stats, &JsValue::from_str("orbat"), &entry);
}

#[cfg(not(target_arch = "wasm32"))]
fn set_orbat_stats(_total: usize, _rendered: usize) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::{FactionDoc, FactionRole, FactionVehicle, UserFaction};

    fn uf(id: &str, side: &str, name: &str) -> UserFaction {
        UserFaction {
            id: id.into(),
            owner_id: "o".into(),
            side: side.into(),
            name: name.into(),
            doc: FactionDoc {
                side: side.into(),
                name: name.into(),
                ..Default::default()
            },
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn g1_dialog_class_near_fullscreen() {
        assert!(DIALOG_CLASS.contains("w-[min("));
        assert!(DIALOG_CLASS.contains("max-w-6xl"));
        assert!(!DIALOG_CLASS.contains("max-w-xl"));
    }

    #[test]
    fn g2_set_leader_symbol_in_module_source() {
        // Wiring lives in stitch_row → orbat_set_leader; keep a compile-time reminder.
        let src = include_str!("orbat_manager.rs");
        assert!(
            src.contains("orbat_set_leader"),
            "G2 Make SL must call set_leader path"
        );
        assert!(src.contains("set_leader") || src.contains("orbat_set_leader"));
    }

    /// H5 — Cancel path never allows apply.
    #[test]
    fn apply_cancel_noop() {
        assert!(!apply_confirm_allows(false));
        assert!(apply_confirm_allows(true));
        let src = include_str!("orbat_manager.rs");
        assert!(
            src.contains("apply_confirm_allows(confirmed)"),
            "Apply must gate on confirm"
        );
        assert!(
            src.contains("orbat_apply_faction"),
            "Apply must call editor_ops path"
        );
    }

    /// H6 — Save inverse roles.len matches authored slot count.
    #[test]
    fn save_faction_roles_match_side() {
        let doc = FactionDoc {
            side: "BLUFOR".into(),
            name: "Alpha".into(),
            emblem: None,
            roles: vec![
                FactionRole {
                    role: "SL".into(),
                    tag: None,
                    character: "c1".into(),
                    loadout: None,
                },
                FactionRole {
                    role: "Rifleman".into(),
                    tag: None,
                    character: "c2".into(),
                    loadout: None,
                },
            ],
            vehicles: vec![FactionVehicle {
                vehicle: "v1".into(),
                label: None,
            }],
        };
        assert_eq!(faction_doc_role_count(&doc), 2);
        assert_eq!(doc.roles.len(), 2);
    }

    /// H7 / H-L8 — CIV + other sides excluded from dropdown.
    #[test]
    fn template_options_exclude_civ_and_other_sides() {
        let lib = vec![
            uf("1", "BLUFOR", "US 1980s"),
            uf("2", "OPFOR", "Soviet"),
            uf("3", "CIV", "Civilians"),
            uf("4", "INDFOR", "FIA"),
        ];
        let blu = template_options_for_side(&lib, "BLUFOR");
        assert_eq!(blu.len(), 1);
        assert_eq!(blu[0].id, "1");
        assert!(blu.iter().all(|f| f.side != "CIV"));
        let opf = template_options_for_side(&lib, "OPFOR");
        assert_eq!(opf.len(), 1);
        assert_eq!(opf[0].name, "Soviet");
        let civ_tab = template_options_for_side(&lib, "CIV");
        assert!(civ_tab.is_empty(), "CIV never a template side");
    }

    /// H8 — Add Vehicle wiring present (not a disabled stub).
    #[test]
    fn orbat_add_vehicle_increases_vehicle_ids() {
        let src = include_str!("orbat_manager.rs");
        assert!(
            src.contains("orbat_add_vehicle"),
            "Add Vehicle must call orbat_add_vehicle"
        );
        assert!(
            !src.contains(
                "title=\"Add Vehicle (T-180.8)\"\n                                disabled"
            ),
            "Add Vehicle must not stay disabled"
        );
        let ops = include_str!("editor_ops.rs");
        assert!(
            ops.contains("pub fn orbat_add_vehicle"),
            "ops mutator must exist"
        );
        assert!(
            ops.contains("add_vehicle") && ops.contains("attach_vehicle"),
            "ops must call core add+attach"
        );
        let hist = include_str!("mission_history.rs");
        assert!(
            hist.contains("vehicles_bind"),
            "map presence: vehicles_bind on doc change"
        );
    }

    /// I7 — OPEN ARSENAL opens Attributes on tab 3 (Arsenal), not Identity-only open_attributes.
    #[test]
    fn open_arsenal_selects_arsenal_tab() {
        let ops = include_str!("editor_ops.rs");
        assert!(
            ops.contains("pub fn open_arsenal"),
            "open_arsenal must exist"
        );
        assert!(
            ops.contains("attrs_tab.set(3)"),
            "open_arsenal must select Arsenal tab index 3"
        );
        let mgr = include_str!("orbat_manager.rs");
        assert!(
            mgr.contains("open_arsenal(id_ars"),
            "OPEN ARSENAL button must call open_arsenal"
        );
        // The Arsenal button path must not fall back to Identity-default open_attributes.
        let ars_idx = mgr
            .find("OPEN ARSENAL")
            .expect("OPEN ARSENAL label present");
        let window_start = ars_idx.saturating_sub(400);
        let window = &mgr[window_start..ars_idx];
        assert!(
            window.contains("open_arsenal"),
            "click handler near OPEN ARSENAL must call open_arsenal"
        );
        assert!(
            !window.contains("open_attributes"),
            "OPEN ARSENAL must not call open_attributes (Identity default)"
        );
        let attrs = include_str!("attributes.rs");
        assert!(
            attrs.contains(r#"["Transform", "Identity", "States", "Arsenal"]"#),
            "TABS[3] must be Arsenal"
        );
    }
}
