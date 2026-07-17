//! Faction Manager dialog (T-167 / T-153 `FactionManagerDialog.tsx` port). Operator-authored
//! reusable factions: side → name → role templates (character + tag + optional kind-only loadout)
//! + a vehicle pool, wired to the live `/api/v1/factions` CRUD (owner-scoped, contract-validated).
//!
//! The character/vehicle pickers reuse the flat `/registry` (kind-filtered, abstract/variant
//! dropped); per-role loadout reuses the Arsenal serialization ([`crate::arsenal::picks_to_loadout`])
//! in **kind-only, no-compat** mode — the same `SlotLoadoutV2` shape a slot writes.
#![allow(dead_code)]
use leptos::prelude::*;

use crate::dto::{FactionDoc, FactionRole, FactionVehicle, RegistryItem, FACTION_SIDES};

const CTRL: &str = "w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2.5 py-1.5 text-label-md text-on-surface outline-none transition-colors focus:border-primary/60";
const BTN: &str =
    "rounded-md px-2 py-1 text-label-sm font-semibold uppercase tracking-wide transition-colors";

/// Kind-filtered, abstract/variant-dropped `(resource_name, display_name)` options, sorted.
fn kind_options(items: &[RegistryItem], kind: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = items
        .iter()
        .filter(|it| it.kind == kind && it.r#abstract != Some(true) && it.variant_of.is_none())
        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

/// The Faction Manager dialog. `open` toggles it; `registry` supplies the character/vehicle pickers.
#[component]
pub fn FactionManagerDialog(
    open: RwSignal<bool>,
    registry: RwSignal<Option<Vec<RegistryItem>>>,
) -> impl IntoView {
    // Library + editor state.
    let library = RwSignal::new(Vec::<crate::dto::UserFaction>::new());
    let editing = RwSignal::new(FactionDoc {
        side: "BLUFOR".into(),
        ..Default::default()
    });
    let editing_id = RwSignal::new(None::<String>); // None = new (POST); Some = existing (PUT)
    let status = RwSignal::new(String::new()); // inline error/notice

    #[cfg(target_arch = "wasm32")]
    let auth = expect_context::<crate::auth::AuthStore>();

    // Load the library whenever the dialog opens.
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

    // Esc closes.
    #[cfg(target_arch = "wasm32")]
    {
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                open.set(false);
            }
        });
        on_cleanup(move || esc.remove());
    }

    let new_faction = move |_| {
        editing.set(FactionDoc {
            side: "BLUFOR".into(),
            ..Default::default()
        });
        editing_id.set(None);
        status.set(String::new());
    };

    let select_faction = move |f: crate::dto::UserFaction| {
        editing.set(f.doc.clone());
        editing_id.set(Some(f.id.clone()));
        status.set(String::new());
    };

    let save = move |_| {
        let doc = editing.get_untracked();
        if doc.name.trim().is_empty() {
            status.set("A faction name is required.".into());
            return;
        }
        if doc
            .roles
            .iter()
            .any(|r| r.role.trim().is_empty() || r.character.trim().is_empty())
        {
            status.set("Every role needs a name and a character.".into());
            return;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let body = serde_json::to_value(&doc).unwrap_or(serde_json::Value::Null);
            let id = editing_id.get_untracked();
            leptos::task::spawn_local(async move {
                let res = match id {
                    Some(id) => {
                        crate::client::api_put::<crate::dto::UserFaction>(
                            auth,
                            &format!("/factions/{id}"),
                            body,
                        )
                        .await
                    }
                    None => {
                        crate::client::api_post::<crate::dto::UserFaction>(auth, "/factions", body)
                            .await
                    }
                };
                match res {
                    Ok(f) => {
                        editing_id.set(Some(f.id.clone()));
                        status.set("Saved.".into());
                        if let Ok(r) = crate::client::api_get::<crate::dto::FactionListResponse>(
                            auth,
                            "/factions",
                        )
                        .await
                        {
                            library.set(r.data);
                        }
                    }
                    Err(_) => status.set("Could not save the faction (name already used?).".into()),
                }
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = doc;
    };

    let delete = move |_| {
        #[cfg(target_arch = "wasm32")]
        if let Some(id) = editing_id.get_untracked() {
            leptos::task::spawn_local(async move {
                let _ = crate::client::api_delete(auth, &format!("/factions/{id}")).await;
                editing.set(FactionDoc {
                    side: "BLUFOR".into(),
                    ..Default::default()
                });
                editing_id.set(None);
                if let Ok(r) =
                    crate::client::api_get::<crate::dto::FactionListResponse>(auth, "/factions")
                        .await
                {
                    library.set(r.data);
                }
            });
        }
    };

    move || {
        if !open.get() {
            return None;
        }
        let items = registry.get().unwrap_or_default();
        let items = StoredValue::new(items);
        Some(view! {
            <div
                class="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm"
                on:click=move |_| open.set(false)
            ></div>
            <div class="glass fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-4xl -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                <div class="flex items-center justify-between border-b border-outline-variant/30 px-6 py-4">
                    <h2 class="text-headline-sm text-on-surface">"Faction Manager"</h2>
                    <button type="button" aria-label="Close" on:click=move |_| open.set(false)
                        class="rounded-md p-1 text-outline hover:bg-surface-variant/50 hover:text-on-surface">"✕"</button>
                </div>
                <div class="grid flex-1 grid-cols-[220px_1fr] gap-0 overflow-hidden">
                    // LEFT: library list + New.
                    <div class="flex flex-col gap-1 overflow-y-auto border-r border-outline-variant/30 p-3">
                        <button type="button" aria-label="New faction" on:click=new_faction
                            class=format!("{BTN} bg-primary/15 text-primary hover:bg-primary/25")>"+ New faction"</button>
                        {move || library.get().into_iter().map(|f| {
                            let f2 = f.clone();
                            let label = format!("{} · {}", f.side, f.name);
                            let aria = label.clone();
                            view! {
                                <button type="button" aria-label=aria
                                    on:click=move |_| select_faction(f2.clone())
                                    class="rounded-md px-2 py-1 text-left text-label-md text-on-surface-variant hover:bg-surface-variant/40">
                                    {label}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                    // RIGHT: editor.
                    <div class="custom-scrollbar flex flex-col gap-3 overflow-y-auto p-4">
                        <div class="grid grid-cols-[140px_1fr] gap-3">
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">"Side"</span>
                                <select class=CTRL prop:value=move || editing.get().side
                                    on:change=move |ev| { let v = event_target_value(&ev); editing.update(|d| d.side = v); }>
                                    {FACTION_SIDES.iter().map(|s| view! { <option value=*s>{*s}</option> }).collect_view()}
                                </select>
                            </label>
                            <label class="flex flex-col gap-1">
                                <span class="text-label-sm uppercase tracking-wider text-outline">"Name"</span>
                                <input class=CTRL prop:value=move || editing.get().name placeholder="e.g. 3rd Ranger Bn"
                                    on:input=move |ev| { let v = event_target_value(&ev); editing.update(|d| d.name = v); } />
                            </label>
                        </div>

                        // Roles.
                        <div class="flex items-center justify-between">
                            <span class="text-label-sm uppercase tracking-wider text-outline">"Roles"</span>
                            <button type="button" aria-label="Add role"
                                on:click=move |_| editing.update(|d| d.roles.push(FactionRole::default()))
                                class=format!("{BTN} bg-surface-variant/40 text-on-surface-variant hover:bg-surface-variant/60")>"+ Role"</button>
                        </div>
                        {move || {
                            let chars = kind_options(&items.get_value(), "character");
                            let chars = StoredValue::new(chars);
                            editing.get().roles.into_iter().enumerate().map(|(i, r)| {
                                let chars = chars.get_value();
                                view! {
                                    <div class="grid grid-cols-[1fr_90px_1fr_28px] items-center gap-2">
                                        <input class=CTRL placeholder="Role" prop:value=r.role.clone()
                                            on:input=move |ev| { let v = event_target_value(&ev); editing.update(|d| { if let Some(x) = d.roles.get_mut(i) { x.role = v; } }); } />
                                        <input class=CTRL placeholder="Tag" prop:value=r.tag.clone().unwrap_or_default()
                                            on:input=move |ev| { let v = event_target_value(&ev); editing.update(|d| { if let Some(x) = d.roles.get_mut(i) { x.tag = if v.is_empty() { None } else { Some(v) }; } }); } />
                                        <select class=CTRL prop:value=r.character.clone()
                                            on:change=move |ev| { let v = event_target_value(&ev); editing.update(|d| { if let Some(x) = d.roles.get_mut(i) { x.character = v; } }); }>
                                            <option value="">"— Character —"</option>
                                            {chars.into_iter().map(|(v, l)| view! { <option value=v>{l}</option> }).collect_view()}
                                        </select>
                                        <button type="button" aria-label="Remove role"
                                            on:click=move |_| editing.update(|d| { if i < d.roles.len() { d.roles.remove(i); } })
                                            class="rounded-md px-1 text-error hover:bg-error/10">"✕"</button>
                                    </div>
                                }
                            }).collect_view()
                        }}

                        // Vehicles.
                        <div class="flex items-center justify-between">
                            <span class="text-label-sm uppercase tracking-wider text-outline">"Vehicles"</span>
                            <button type="button" aria-label="Add vehicle"
                                on:click=move |_| editing.update(|d| d.vehicles.push(FactionVehicle::default()))
                                class=format!("{BTN} bg-surface-variant/40 text-on-surface-variant hover:bg-surface-variant/60")>"+ Vehicle"</button>
                        </div>
                        {move || {
                            let vehs = kind_options(&items.get_value(), "vehicle");
                            let vehs = StoredValue::new(vehs);
                            editing.get().vehicles.into_iter().enumerate().map(|(i, v)| {
                                let vehs = vehs.get_value();
                                view! {
                                    <div class="grid grid-cols-[1fr_1fr_28px] items-center gap-2">
                                        <select class=CTRL prop:value=v.vehicle.clone()
                                            on:change=move |ev| { let val = event_target_value(&ev); editing.update(|d| { if let Some(x) = d.vehicles.get_mut(i) { x.vehicle = val; } }); }>
                                            <option value="">"— Vehicle —"</option>
                                            {vehs.into_iter().map(|(val, l)| view! { <option value=val>{l}</option> }).collect_view()}
                                        </select>
                                        <input class=CTRL placeholder="Label (optional)" prop:value=v.label.clone().unwrap_or_default()
                                            on:input=move |ev| { let val = event_target_value(&ev); editing.update(|d| { if let Some(x) = d.vehicles.get_mut(i) { x.label = if val.is_empty() { None } else { Some(val) }; } }); } />
                                        <button type="button" aria-label="Remove vehicle"
                                            on:click=move |_| editing.update(|d| { if i < d.vehicles.len() { d.vehicles.remove(i); } })
                                            class="rounded-md px-1 text-error hover:bg-error/10">"✕"</button>
                                    </div>
                                }
                            }).collect_view()
                        }}

                        <div class="mt-2 flex items-center gap-2">
                            <button type="button" aria-label="Save faction" on:click=save
                                class=format!("{BTN} bg-primary text-on-primary hover:bg-primary/90")>"Save"</button>
                            {move || editing_id.get().map(|_| view! {
                                <button type="button" aria-label="Delete faction" on:click=delete
                                    class=format!("{BTN} bg-error/15 text-error hover:bg-error/25")>"Delete"</button>
                            })}
                            <span class="text-label-sm normal-case text-on-surface-variant">{move || status.get()}</span>
                        </div>
                    </div>
                </div>
            </div>
        })
    }
}
