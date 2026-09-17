//! Attributes modal for editing slot and vehicle properties.

#![allow(dead_code)]

use leptos::prelude::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub use website_map_engine::data::store::operations::reassign::faction_label;
#[cfg(target_arch = "wasm32")]
use website_map_engine::editing::hosted_commands as engine_ops;

mod asset_type_picker;
mod attribute_commits_and_revert;
mod faction_and_squad_reassignment;
mod field_gates_and_labels;
mod field_inputs;
mod identity_tab;
mod spatial_transform_tab;
mod vehicle_attributes;

#[cfg(target_arch = "wasm32")]
use asset_type_picker::type_picker;
#[cfg(target_arch = "wasm32")]
use attribute_commits_and_revert::{commit_position, commit_slot, revert_to_snapshot};
#[cfg(target_arch = "wasm32")]
use faction_and_squad_reassignment::reassign_picker;
pub(crate) use field_gates_and_labels::attrs_multi_subtitle;
use field_gates_and_labels::MultiOpts;
#[cfg(target_arch = "wasm32")]
use field_gates_and_labels::{field_label, Gate, CONTROL, CONTROL_LOCKED, TABS};
#[cfg(target_arch = "wasm32")]
use field_inputs::{number_field, text_field};
#[cfg(target_arch = "wasm32")]
use identity_tab::identity_tab;
#[cfg(target_arch = "wasm32")]
use spatial_transform_tab::transform_tab;
#[cfg(target_arch = "wasm32")]
use vehicle_attributes::vehicle_attrs_view;

/// Renders the editor for selected slot or vehicle attributes.
#[component]
pub fn AttributesModal(
    attrs_open: RwSignal<Option<String>>,
    attrs_tab: RwSignal<usize>,
    doc_tick: RwSignal<u64>,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    compat: RwSignal<crate::v2::apps::editor::arsenal::rules::CompatFeed>,
) -> impl IntoView {
    #[cfg(target_arch = "wasm32")]
    {
        let modal_id = crate::v2::core::ui::modal_stack::register(move || {
            attrs_open.try_get_untracked().flatten().is_some()
        });
        let esc = window_event_listener(leptos::ev::keydown, move |ev| {
            if attrs_open.get_untracked().is_some()
                && ev.key() == "Escape"
                && crate::v2::core::ui::modal_stack::is_topmost_open(modal_id)
            {
                crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes();
            }
        });
        on_cleanup(move || {
            esc.remove();
            crate::v2::core::ui::modal_stack::unregister(modal_id);
        });
    }
    let opts = MultiOpts::new();
    #[cfg(target_arch = "wasm32")]
    let snapshot: StoredValue<Vec<engine_ops::SlotAttrs>> = StoredValue::new(Vec::new());
    Effect::new(move |_| {
        let open = attrs_open.get();
        opts.reset();
        #[cfg(target_arch = "wasm32")]
        {
            let snap = open
                .as_deref()
                .map(|id| {
                    let mut ids = engine_ops::attrs_multi_ids(id);
                    if ids.is_empty() {
                        ids = vec![id.to_string()];
                    }
                    ids.iter()
                        .filter_map(|i| engine_ops::read_attrs(i))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            snapshot.set_value(snap);
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = open;
    });
    move || {
        let id = attrs_open.get()?;
        let _ = doc_tick.get(); // re-read fields on every doc change (undo/redo/drag)
        #[cfg(not(target_arch = "wasm32"))]
        let _ = (&id, registry_items, compat, attrs_tab, opts);
        #[cfg(target_arch = "wasm32")]
        {
            match engine_ops::read_attrs(&id) {
                Some(attrs) => {
                    let multi = engine_ops::attrs_multi_ids(&id);
                    let selection_n = website_map_engine::editing::host::selection_len();
                    let diff = engine_ops::read_attrs_diff(&multi);
                    Some(modal_view(
                        attrs,
                        multi,
                        selection_n,
                        diff,
                        opts,
                        snapshot,
                        registry_items,
                        compat,
                        attrs_tab,
                    ))
                }
                None => {
                    if engine_ops::is_vehicle_id(&id) {
                        Some(vehicle_attrs_view(id, registry_items))
                    } else {
                        crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes();
                        None
                    }
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            None::<AnyView>
        }
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
fn modal_view(
    attrs: engine_ops::SlotAttrs,
    multi: Vec<String>,
    selection_n: usize,
    diff: engine_ops::AttrDiff,
    opts: MultiOpts,
    snapshot: StoredValue<Vec<engine_ops::SlotAttrs>>,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
    compat: RwSignal<crate::v2::apps::editor::arsenal::rules::CompatFeed>,
    tab: RwSignal<usize>,
) -> AnyView {
    let slot_id = StoredValue::new(attrs.id.clone());
    let is_multi = multi.len() > 1;
    let multi_n = multi.len();
    let targets = StoredValue::new(if is_multi {
        multi
    } else {
        vec![attrs.id.clone()]
    });
    let locked_n = engine_ops::attrs_locked_count(&targets.get_value());
    let attrs = StoredValue::new(attrs);
    let subtitle = {
        let a = attrs.get_value();
        if is_multi {
            attrs_multi_subtitle(multi_n, selection_n)
        } else {
            let role = if a.role.is_empty() {
                "Slot".to_string()
            } else {
                a.role.clone()
            };
            format!("{role} · {}", a.id)
        }
    };
    view! {
        <div
            class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
            on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
        ></div>
        <div class=move || {
            let width = if tab.get() == 3 { "max-w-6xl" } else { "max-w-lg" };
            format!("glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] {width} -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200")
        }>
            <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                <div class="min-w-0">
                    <h2 class="text-headline-sm text-on-surface">"Attributes"</h2>
                    <p class="mt-1 text-label-md text-on-surface-variant">{subtitle}</p>
                    <p class="mt-1 text-label-sm normal-case text-outline">
                        "Edits apply live. Revert restores the values from when this panel opened."
                    </p>
                </div>
                <div class="flex shrink-0 items-center gap-1">
                    <button
                        type="button"
                        data-testid="attrs-revert"
                        on:click=move |_| revert_to_snapshot(snapshot)
                        class="rounded-md border border-outline-variant/40 px-2.5 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        "Revert"
                    </button>
                    <button
                        type="button"
                        aria-label="Close"
                        on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
                        class="rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                    >
                        <crate::v2::core::ui::MaterialIcon name="close" />
                    </button>
                </div>
            </div>
            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                <div class="flex flex-col gap-4">
                    {(is_multi && diff.any())
                        .then(|| {
                            view! {
                                <p class="rounded-md border border-primary/30 bg-primary/10 px-3 py-2 text-label-sm normal-case text-on-surface-variant">
                                    "Fields that differ across the selection are blank and locked. Tick "
                                    <span class="text-primary">"Apply to all"</span>
                                    " to overwrite that field on every selected slot."
                                </p>
                            }
                        })}
                    <div class="flex gap-1 rounded-lg bg-surface-container-lowest/50 p-1">
                        {TABS
                            .iter()
                            .enumerate()
                            .map(|(i, label)| {
                                view! {
                                    <button
                                        type="button"
                                        aria-label=*label
                                        on:click=move |_| tab.set(i)
                                        class=move || {
                                            if tab.get() == i {
                                                "flex-1 rounded-md px-2 py-1.5 text-label-md transition-colors bg-primary/20 text-primary"
                                            } else {
                                                "flex-1 rounded-md px-2 py-1.5 text-label-md transition-colors text-on-surface-variant hover:bg-white/5"
                                            }
                                        }
                                    >
                                        {*label}
                                    </button>
                                }
                            })
                            .collect_view()}
                    </div>
                    {move || match tab.get() {
                        0 => transform_tab(targets, attrs, is_multi, diff, opts, locked_n)
                            .into_any(),
                        1 => identity_tab(targets, attrs, is_multi, diff, opts, registry_items)
                            .into_any(),
                        2 => states_tab().into_any(),
                        _ => {
                            let loadout = engine_ops::read_loadout(&slot_id.get_value());
                            view! {
                                {is_multi
                                    .then(|| {
                                        view! {
                                            <p class="mb-3 rounded-md border border-outline-variant/40 bg-surface-container-lowest/50 px-3 py-2 text-label-sm normal-case text-on-surface-variant">
                                                "Pick and cargo edits apply to this one entity ("
                                                {slot_id.get_value()}
                                                "); Copy, Apply, and Remove Everything act on the whole selection."
                                            </p>
                                        }
                                    })}
                                <crate::v2::apps::editor::arsenal::ArsenalTab
                                    slot_id=slot_id.get_value()
                                    loadout_json=loadout
                                    registry=registry_items
                                    compat=compat
                                />
                            }
                            .into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
    .into_any()
}
#[cfg(target_arch = "wasm32")]
fn states_tab() -> impl IntoView {
    view! {
        <div class="flex flex-col gap-3">
            <p class="text-label-sm normal-case text-outline">
                "Unit traits — wired to the compiler in a later phase."
            </p>
            <div class="flex items-center justify-between py-0.5">
                <span class="text-label-md text-on-surface-variant">"Medic (soon)"</span>
                <span class="text-label-sm text-outline">"—"</span>
            </div>
            <div class="flex items-center justify-between py-0.5">
                <span class="text-label-md text-on-surface-variant">"Engineer (soon)"</span>
                <span class="text-label-sm text-outline">"—"</span>
            </div>
        </div>
    }
}

#[cfg(test)]
const ATTRIBUTES_MODAL_SOURCE: &str = concat!(
    include_str!("attributes_modal/asset_type_picker.rs"),
    include_str!("attributes_modal/attribute_commits_and_revert.rs"),
    include_str!("attributes_modal/faction_and_squad_reassignment.rs"),
    include_str!("attributes_modal/field_gates_and_labels.rs"),
    include_str!("attributes_modal/field_inputs.rs"),
    include_str!("attributes_modal/identity_tab.rs"),
    include_str!("attributes_modal/spatial_transform_tab.rs"),
    include_str!("attributes_modal/vehicle_attributes.rs"),
    include_str!("attributes_modal.rs"),
);

#[cfg(test)]
#[path = "tests/attributes_modal/batch_faction_and_squad_reassignment.rs"]
mod batch_faction_and_squad_reassignment_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/identity_and_raw_attributes.rs"]
mod identity_and_raw_attributes_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/modal_escape_stack.rs"]
mod modal_escape_stack_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/numeric_field_input.rs"]
mod numeric_field_input_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/position_and_selection.rs"]
mod position_and_selection_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/transform_field_copy.rs"]
mod transform_field_copy_tests;
#[cfg(test)]
#[path = "tests/attributes_modal/type_picker_revert_and_vehicle.rs"]
mod type_picker_revert_and_vehicle_tests;
