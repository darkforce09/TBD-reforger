//! Tree rows for the ORBAT manager.

use super::*;

/// Renders one squad or slot row with editing actions.
pub(super) fn stitch_row(
    row: &FlatRow,
    nodes: RwSignal<Vec<OutlinerNode>>,
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
            let renaming = rename_squad.get().as_deref() == Some(id.as_str());
            let picking_vehicle = add_vehicle_squad.get().as_deref() == Some(id.as_str());
            let vehicle_options = vehicle_options.clone();
            view! {
                <div class="group flex flex-col rounded border border-border-subtle bg-surface-container-low">
                    <div
                        class="flex cursor-pointer items-center px-2 py-1.5 hover:bg-surface-variant/50"
                        on:pointerup=move |ev| {
                            ev.stop_propagation();
                            #[cfg(target_arch = "wasm32")]
                            {
                                if !crate::v2::apps::editor::ui::outliner::drag::complete_multi_refile_onto_squad(&id_drop) {
                                    engine_ops::complete_refile_onto_squad(id_drop.clone());
                                }
                            }
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
                            let rename_ref = NodeRef::<leptos::html::Input>::new();
                            rename_ref.on_load(|el: web_sys::HtmlInputElement| {
                                let _ = el.focus();
                                el.select();
                                let el2 = el.clone();
                                if let Some(win) = web_sys::window() {
                                    use wasm_bindgen::JsCast;
                                    let cb = wasm_bindgen::closure::Closure::once(move || {
                                        let _ = el2.focus();
                                        el2.select();
                                    });
                                    let _ = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                                        cb.as_ref().unchecked_ref(),
                                        0,
                                    );
                                    cb.forget();
                                }
                            });
                            view! {
                                <input
                                    type="text"
                                    node_ref=rename_ref
                                    data-testid="orbat-squad-rename"
                                    aria-label="Rename squad"
                                    autofocus
                                    class="mr-2 flex-1 rounded border border-primary bg-surface-dim px-1 py-0.5 font-label-sm text-label-sm text-on-surface"
                                    value=rename_draft.get_untracked()
                                    on:click=move |ev| ev.stop_propagation()
                                    on:input=move |ev| rename_draft.set(event_target_value(&ev))
                                    on:keydown=move |ev| {
                                        match ev.key().as_str() {
                                            "Enter" => {
                                                ev.prevent_default();
                                                ev.stop_propagation();
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let name = rename_draft.get_untracked();
                                                    engine_ops::orbat_rename_squad(id_commit.clone(), name);
                                                }
                                                rename_squad.set(None);
                                            }
                                            "Escape" => {
                                                ev.prevent_default();
                                                ev.stop_propagation();
                                                rename_squad.set(None);
                                            }
                                            _ => {}
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
                                    engine_ops::orbat_add_slot(
                                        id_add.clone(),
                                        "Rifleman".into(),
                                        outliner::ensure_active_layer,
                                    );
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
                                    engine_ops::orbat_remove_squad(id_rm.clone());
                                }
                            >
                                <MaterialIcon name="delete" class="text-[16px]" />
                            </button>
                        </div>
                    </div>
                    {if picking_vehicle && vehicle_options.is_empty() {
                        view! {
                            <div
                                class="flex items-center gap-2 border-t border-white/5 bg-surface-dim px-2 py-1.5"
                                data-testid="add-vehicle-empty-explainer"
                                on:click=move |ev| ev.stop_propagation()
                            >
                                <span class="min-w-0 flex-1 text-label-sm text-on-surface-variant">
                                    "No placeable vehicles in the active modpack — nothing to add. Seed or select a modpack that ships vehicles."
                                </span>
                                <button
                                    type="button"
                                    class="rounded px-2 py-1 text-label-sm text-on-surface-variant hover:text-on-surface"
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        add_vehicle_squad.set(None);
                                    }
                                >"Dismiss"</button>
                            </div>
                        }.into_any()
                    } else if picking_vehicle {
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
                                            if engine_ops::orbat_add_vehicle(
                                                id_veh_pick.clone(),
                                                &resource,
                                            )
                                            .is_some()
                                            {
                                                crate::v2::apps::editor::ui::docks::dock_right::record_placed(
                                                    resource.clone(),
                                                    resource,
                                                );
                                            }
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
                            entity_selection::select_slot(id_sel.clone());
                        }
                        on:dblclick=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::v2::apps::editor::bridge::host_state::editor_context::open_attributes(id_dbl.clone());
                        }
                        on:pointerdown=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let drag = crate::v2::apps::editor::ui::outliner::tree::drag_set_for(
                                    &id_refile,
                                    &selected.get_untracked(),
                                    &nodes.get_untracked(),
                                );
                                crate::v2::apps::editor::ui::outliner::drag::begin_refile(drag);
                                engine_ops::begin_refile(id_refile.clone());
                            }
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
                                    engine_ops::orbat_set_leader(squad_id.clone(), id_sl.clone());
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
                                    engine_ops::orbat_remove_slot(id_rm.clone());
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

/// Removes the count suffix from a squad label.
pub(super) fn label_name_only(label: &str) -> String {
    label
        .rsplit_once(" (")
        .map(|(n, _)| n.to_string())
        .unwrap_or_else(|| label.to_string())
}
