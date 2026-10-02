//! Dialog for the ORBAT manager.

use super::*;

/// Renders the ORBAT tree, faction library actions, and slot inspector.
#[component]
/// Renders the ORBAT tree, faction library actions, and slot inspector.
pub fn OrbatManagerDialog(
    open: RwSignal<bool>,
    orbat: RwSignal<Vec<OutlinerNode>>,
    selected: RwSignal<Vec<String>>,
    active_layer: RwSignal<Option<String>>,
    #[prop(optional)] registry: Option<RwSignal<Option<Vec<RegistryItem>>>>,
) -> impl IntoView {
    let _ = active_layer; // The component keeps the layer signal in its mount interface.
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

    #[cfg(target_arch = "wasm32")]
    let auth = expect_context::<crate::v2::core::auth::AuthStore>();
    let modal_id = install_orbat_dialog_lifecycle(
        open,
        library,
        #[cfg(target_arch = "wasm32")]
        auth,
    );

    move || {
        if !open.get() {
            return None;
        }
        let _tree = orbat.get();
        let snap = read_snapshot();
        let total_slots = snap.slots.len();
        let cap_label = format!(
            "{total_slots} slot{} · server cap 128 players",
            if total_slots == 1 { "" } else { "s" }
        );
        let cap_cls = if total_slots > 128 {
            "rounded border border-error-alert/40 bg-error/10 px-2 py-0.5 font-mono text-label-sm tabular-nums normal-case text-error-alert"
        } else {
            "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm tabular-nums normal-case text-on-surface-variant"
        };
        let side = side_tab.get();
        let q = search.get().trim().to_lowercase();
        let mut squad_nodes = if snap.factions.is_empty() {
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

        let z = crate::v2::core::ui::modal_stack::z_class(modal_id);
        let scrim_class =
            format!("animate-overlay-fade fixed inset-0 {z} bg-black/50 backdrop-blur-sm");
        let dialog_class = DIALOG_CLASS.replace("z-50", z);
        Some(view! {
            <div
                class=scrim_class
                on:click=move |_| open.set(false)
            ></div>
            <div
                class=dialog_class
                on:click=move |ev| ev.stop_propagation()
                on:pointerup=move |_| {
                    #[cfg(target_arch = "wasm32")]
                    crate::v2::apps::editor::ui::outliner::drag::cancel_layer_drag();
                }
            >
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
                                match engine_ops::orbat_apply_faction(
                                    side,
                                    uf.doc,
                                    outliner::ensure_active_layer,
                                ) {
                                    Ok(()) => status.set("Template applied.".into()),
                                    Err(msg) => {
                                        leptos::logging::warn!("Apply Template refused: {msg}");
                                        status.set(msg);
                                    }
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
                                let Some(derived) = engine_ops::faction_doc_from_side(&side)
                                else {
                                    status.set("Could not read this side's ORBAT.".into());
                                    return;
                                };
                                leptos::task::spawn_local(async move {
                                    let Ok(stored) = crate::v2::core::api::client::api_get::<UserFaction>(
                                        auth,
                                        &format!("/factions/{tid}"),
                                    )
                                    .await
                                    else {
                                        status.set(
                                            "Could not re-read the stored faction — nothing saved."
                                                .into(),
                                        );
                                        return;
                                    };
                                    let doc = match merge_faction_doc_from_side(
                                        &stored.doc,
                                        derived,
                                    ) {
                                        Ok(doc) => doc,
                                        Err(refusal) => {
                                            let msg = refusal.message(&side, &stored.doc.name);
                                            leptos::logging::warn!("Save refused: {msg}");
                                            status.set(msg);
                                            return;
                                        }
                                    };
                                    if let Some(warning) = save_from_side_shrink_warning(
                                        &stored.doc,
                                        &doc,
                                        &side,
                                    ) {
                                        let confirmed = web_sys::window()
                                            .and_then(|w| w.confirm_with_message(&warning).ok())
                                            .unwrap_or(false);
                                        if !apply_confirm_allows(confirmed) {
                                            status.set("Save cancelled.".into());
                                            return;
                                        }
                                    }
                                    let body = serde_json::to_value(&doc).unwrap_or_default();
                                    match crate::v2::core::api::client::api_put::<UserFaction>(
                                        auth,
                                        &format!("/factions/{tid}"),
                                        body,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            status.set("Saved.".into());
                                            if let Ok(r) = crate::v2::core::api::client::api_get::<
                                                crate::v2::core::api::dto::FactionListResponse,
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
                                let Some(mut doc) = engine_ops::faction_doc_from_side(&side)
                                else {
                                    status.set("Could not read this side's ORBAT.".into());
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
                                    match crate::v2::core::api::client::api_post::<UserFaction>(
                                        auth, "/factions", body,
                                    )
                                    .await
                                    {
                                        Ok(f) => {
                                            selected_template.set(f.id.clone());
                                            status.set("Saved as new faction.".into());
                                            if let Ok(r) = crate::v2::core::api::client::api_get::<
                                                crate::v2::core::api::dto::FactionListResponse,
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
                        <span>"Total Entities: "<span class="font-code-md text-primary">{entity_count}</span></span>
                        <span class="text-white/20">"|"</span>
                        <span>"Vehicles: "<span class="font-code-md text-tactical-yellow">{vehicle_count}</span></span>
                    </div>
                </div>

                <Show when=move || !status.get().is_empty()>
                    <div
                        role="status"
                        aria-live="polite"
                        class="shrink-0 whitespace-pre-wrap border-b border-white/5 bg-surface-container-low px-4 py-2 font-label-sm text-label-sm leading-relaxed text-primary/90"
                    >
                        {move || status.get()}
                    </div>
                </Show>

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
                                    orbat,
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
                                        engine_ops::orbat_add_squad(side);
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
