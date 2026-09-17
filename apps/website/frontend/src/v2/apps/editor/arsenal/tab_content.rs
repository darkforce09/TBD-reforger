//! Catalog actions and loaded Arsenal view.

use super::*;

mod catalog_header;
mod selection_grid;
mod status_and_persistence;

/// Renders the loaded catalog with its action handlers.
pub(super) fn loaded_catalog(items: Vec<RegistryItem>, state: ArsenalTabState) -> impl IntoView {
    let ArsenalTabState {
        id,
        asset_id,
        picks,
        cargo,
        cargo_present,
        active_key,
        commits,
        persist_refused,
        import_status,
        import_refusals,
        buffer_status,
        buffer_refusals,
        buffer_epoch,
        doll_unavailable,
        filter,
        compat,
    } = state;

    // Every pick and cargo edit commits one canonical loadout write.
    let persist = move |map: &HashMap<String, String>, items: &[RegistryItem]| {
        #[cfg(target_arch = "wasm32")]
        {
            let names: HashMap<String, String> = items
                .iter()
                .map(|it| (it.resource_name.clone(), it.display_name.clone()))
                .collect();
            let rows = cargo.get_untracked();
            let rows = cargo_present.get_untracked().then_some(rows.as_slice());
            let took =
                loadout_commands::set_loadout(&id.get_value(), picks_to_loadout(map, &names, rows));
            // Set on every persist, not only on a refusal: a later pick that DOES land must clear
            // the warning, or the panel starts lying in the other direction.
            persist_refused.set(!took);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (map, items);
        // A re-render tick, not a success claim — the verdict itself is read from the document and
        // from `persist_refused` below, never from this counter.
        commits.update(|n| *n = n.wrapping_add(1));
    };
    // Cargo edits mark the key present, then persist through the same path.
    let persist_cargo = move |items: &[RegistryItem]| {
        cargo_present.set(true);
        persist(&picks.get_untracked(), items);
    };

    let names: HashMap<String, String> = items
        .iter()
        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
        .collect();
    let items = StoredValue::new(items);
    let names = StoredValue::new(names);
    let pick_item = move |key: String, value: String| {
        picks.update(|m| {
            if value.is_empty() {
                m.remove(key.as_str());
            } else {
                m.insert(key.clone(), value.clone());
            }
        });
        persist(&picks.get_untracked(), &items.get_value());
    };
    // Replace all local signals, then make one document write for an accepted import.
    let apply_import = move |doc: ImportedLoadout, items: &[RegistryItem]| {
        picks.set(doc.picks);
        cargo.set(doc.cargo);
        cargo_present.set(doc.cargo_present);
        persist(&picks.get_untracked(), items);
    };
    // Read and validate the selected file before changing any loadout signal.
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
                import_refusals.set(vec!["Could not open the file picker.".to_string()]);
                return;
            };
            input.set_type("file");
            input.set_accept("application/json,.json");

            let input_for_cb = input.clone();
            let on_change = Closure::once(move |_ev: web_sys::Event| {
                let Some(file) = input_for_cb.files().and_then(|list| list.item(0)) else {
                    return;
                };
                let name = file.name();
                import_refusals.set(Vec::new());
                import_status.set(format!("Reading {name}…"));
                leptos::task::spawn_local(async move {
                    // `Blob::text()` is a Promise — the browser reads off disk on
                    // its own thread and the tab stays interactive through it.
                    let text = match wasm_bindgen_futures::JsFuture::from(file.text()).await {
                        Ok(v) => v.as_string().unwrap_or_default(),
                        Err(_) => {
                            import_status.set(String::new());
                            import_refusals.set(vec![format!("Could not read {name}.")]);
                            return;
                        }
                    };
                    let its = items.get_value();
                    match try_import(&text, &its, &compat.get_untracked()) {
                        Ok(doc) => {
                            let line = import_summary(&name, &doc, &export_modpack_id(&its));
                            apply_import(doc, &its);
                            import_refusals.set(Vec::new());
                            import_status.set(line);
                        }
                        Err(refusals) => {
                            // Refusals identify each affected row and leave the loadout unchanged.
                            import_status.set(String::new());
                            import_refusals.set(
                                std::iter::once(format!(
                                    "{name} was not applied — this loadout is unchanged.",
                                ))
                                .chain(refusals.iter().map(refusal_line))
                                .collect(),
                            );
                        }
                    }
                });
            });
            let _ = input
                .add_event_listener_with_callback("change", on_change.as_ref().unchecked_ref());
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
    // Whole-selection writes refresh this open slot from the document without recommitting.
    let resync_open_slot = move || {
        #[cfg(target_arch = "wasm32")]
        {
            let lo = engine_ops::read_loadout(&id.get_value());
            picks.set(loadout_to_picks(lo.as_deref()));
            let (rows, present) = rules::cargo_from_loadout(lo.as_deref());
            cargo.set(rows);
            cargo_present.set(present);
        }
    };
    // Copy each selected entity into the shared loadout buffer.
    let copy_loadouts = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let n = engine_ops::copy_loadouts_from_selection();
            buffer_refusals.set(Vec::new());
            buffer_status.set(if n == 0 {
                "Nothing to copy — select the soldiers to copy from first. The buffer is unchanged."
                    .to_string()
            } else {
                copy_receipt(&engine_ops::loadout_buffer())
            });
            buffer_epoch.update(|e| *e = e.wrapping_add(1));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (buffer_status, buffer_refusals, buffer_epoch);
    };
    // Apply one validated buffered loadout per selected entity.
    let apply_loadouts = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let its = items.get_value();
            let buffered = engine_ops::loadout_buffer_len();
            match loadout_commands::apply_loadout_buffer_to_selection(&its, &compat.get_untracked())
            {
                Ok((0, _)) => {
                    buffer_refusals.set(Vec::new());
                    buffer_status.set(
                                        "Nothing was applied — copy at least one loadout, then select the entities to write it to.".to_string(),
                                    );
                }
                Ok((planned, commits)) => {
                    resync_open_slot();
                    buffer_refusals.set(Vec::new());
                    buffer_status.set(apply_receipt(planned, buffered, commits));
                }
                Err(refusals) => {
                    // A refused buffer leaves every target unchanged and names each bad row.
                    buffer_status.set(String::new());
                    buffer_refusals.set(
                        std::iter::once(
                            "Nothing was applied — every selected loadout is unchanged."
                                .to_string(),
                        )
                        .chain(refusals.iter().map(refusal_line))
                        .collect(),
                    );
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (
            buffer_status,
            buffer_refusals,
            resync_open_slot,
            items,
            compat,
        );
    };
    // Strip every selected entity and refresh this open slot.
    let strip_loadouts = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            let (planned, commits) = loadout_commands::remove_all_loadouts_from_selection();
            resync_open_slot();
            buffer_refusals.set(Vec::new());
            buffer_status.set(if planned == 0 {
                "Nothing to strip — select one or more soldiers first.".to_string()
            } else {
                remove_receipt(planned, commits)
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (buffer_status, buffer_refusals, resync_open_slot);
    };
    view! {
        {catalog_header::catalog_header(state, items)}
        {selection_grid::selection_grid(state, items, names, pick_item)}
                        // Cargo uses the slot document and a catalogued capacity estimate.
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
                                // The verdict includes compat, attachment, capacity, and cargo delivery findings.
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
                            // the export refusal, said out loud next to the button that
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
                                // the other half of the round-trip. Never disabled: the
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
                                        // Download only bytes accepted by the export gate.
                                        #[cfg(target_arch = "wasm32")]
                                        if let Ok(json) = try_export(
                                            &picks.get_untracked(),
                                            &cargo.get_untracked(),
                                            &items.get_value(),
                                            &export_modpack_id(&items.get_value()),
                                        ) {
                                            let _ = crate::v2::apps::editor::shell::document_commands::download_json("loadout-export.json", &json);
                                        }
                                    }
                                >
                                    <span class="material-symbols-outlined text-[16px]">"download"</span>
                                    "Download loadout JSON"
                                </button>
                            </div>
                        </div>
                        // Buffer verbs act on the whole selection.
                        <div class="flex min-w-0 flex-wrap items-center gap-2 rounded-lg border border-outline-variant/20 bg-surface-container-lowest/40 p-2">
                            <span class="shrink-0 font-mono text-label-sm uppercase tracking-wider text-on-surface-variant">
                                "Loadout buffer"
                            </span>
                            <button
                                type="button"
                                data-loadout-copy
                                title="Buffer the loadout of every selected entity."
                                class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10"
                                on:click=copy_loadouts
                            >
                                <span class="material-symbols-outlined text-[16px]">"content_copy"</span>
                                "Copy"
                            </button>
                            <button
                                type="button"
                                data-loadout-apply
                                title="Write one buffered loadout to each selected entity, picked at random when several are buffered."
                                prop:disabled=move || {
                                    buffer_epoch.track();
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        engine_ops::loadout_buffer_len() == 0
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    {
                                        true
                                    }
                                }
                                class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10 disabled:cursor-not-allowed disabled:border-outline-variant/20 disabled:text-outline disabled:hover:bg-transparent"
                                on:click=apply_loadouts
                            >
                                <span class="material-symbols-outlined text-[16px]">"casino"</span>
                                "Apply"
                            </button>
                            <button
                                type="button"
                                data-loadout-strip
                                title="Clear every wear row, weapon and cargo row on the selection. Cargo stays cleared."
                                class="flex shrink-0 items-center gap-1.5 rounded-lg border border-outline-variant/40 px-3 py-1.5 text-label-sm font-medium text-on-surface transition-colors hover:bg-white/10"
                                on:click=strip_loadouts
                            >
                                <span class="material-symbols-outlined text-[16px]">"delete_sweep"</span>
                                "Remove Everything"
                            </button>
                            <span
                                data-loadout-buffered
                                class="truncate font-mono text-label-sm tabular-nums normal-case text-outline"
                            >
                                {move || {
                                    buffer_epoch.track();
                                    #[cfg(target_arch = "wasm32")]
                                    let n = engine_ops::loadout_buffer_len();
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let n = 0usize;
                                    format!("{n} buffered")
                                }}
                            </span>
                            // Attributes banner already owns both scopes (one-entity
                            // picks/cargo vs whole-selection Copy/Apply/Remove Everything). Local
                            // reminder only — kept short so the two disclosures do not compete.
                            <span class="basis-full text-label-sm normal-case text-outline">
                                "Buffer verbs: whole selection."
                            </span>
                        </div>
        {status_and_persistence::status_and_persistence(state)}
                    }.into_any()
}
