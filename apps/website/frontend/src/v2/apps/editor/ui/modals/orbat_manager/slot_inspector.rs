//! Slot inspector for the ORBAT manager.

use super::*;

/// Renders editable details for the selected slot.
pub(super) fn inspector_panel(
    inspector: Option<SlotDetail>,
    selected: RwSignal<Vec<String>>,
) -> AnyView {
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
    Effect::new(move |_| {
        let _ = selected.get();
        #[cfg(target_arch = "wasm32")]
        {
            let snap = engine_ops::orbat_manager_snapshot();
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
                            engine_ops::orbat_update_slot_fields(
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
                                engine_ops::orbat_update_slot_fields(
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
                                engine_ops::orbat_update_slot_fields(
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
                    crate::v2::apps::editor::bridge::host_state::editor_context::open_arsenal(id_ars.clone());
                }
            >
                <MaterialIcon name="backpack" class="text-[18px]" />
                "OPEN ARSENAL"
            </button>
        </div>
        <div class="pt-2">
            <button
                type="button"
                class="flex items-center gap-2 text-on-surface-variant hover:text-primary"
                on:click=move |_| {
                    if squad_for_add.is_empty() {
                        return;
                    }
                    #[cfg(target_arch = "wasm32")]
                    engine_ops::orbat_add_slot(
                        squad_for_add.clone(),
                        "Rifleman".into(),
                        outliner::ensure_active_layer,
                    );
                }
            >
                <MaterialIcon name="add" class="text-[14px]" />
                <span class="font-label-sm text-[11px]">"Add Role"</span>
            </button>
        </div>
    }
    .into_any()
}
