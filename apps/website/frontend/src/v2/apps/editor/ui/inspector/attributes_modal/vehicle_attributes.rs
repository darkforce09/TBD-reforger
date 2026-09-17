//! Attributes modal vehicle attributes behavior.

use super::*;

/// Fixed crew stations before vehicle cargo seats.
pub(super) const FIXED_SEATS: &[(&str, &str)] = &[
    ("driver", "Driver"),
    ("gunner", "Gunner"),
    ("commander", "Commander"),
];

/// Cargo seat count used without a declared vehicle capacity.
pub(super) const DEFAULT_CARGO_SEATS: usize = 4;

/// Builds ordered fixed and cargo seat labels.
pub(super) fn seat_model(n_cargo: usize) -> Vec<(String, String)> {
    FIXED_SEATS
        .iter()
        .map(|(id, label)| ((*id).to_string(), (*label).to_string()))
        .chain((1..=n_cargo).map(|n| (format!("cargo{n}"), format!("Cargo {n}"))))
        .collect()
}

/// Registry kinds available for vehicle cargo selection.
pub(super) const VEHICLE_CARGO_KINDS: &[&str] = &[
    "magazine",
    "ammo",
    "gear_item",
    "gear_throwable",
    "gear_explosive",
    "gear_primary",
    "gear_handgun",
    "gear_launcher",
    "gear_binoculars",
    "gear_vest",
    "gear_armored_vest",
    "gear_backpack",
    "gear_helmet",
    "gear_jacket",
    "gear_pants",
    "gear_boots",
    "gear_gloves",
    "gear_glasses",
    "optic",
    "attachment",
    "crate",
];

/// Renders vehicle heading, cargo, and crew controls.
#[cfg(target_arch = "wasm32")]
pub(super) fn vehicle_attrs_view(
    id: String,
    registry_items: RwSignal<Option<Vec<crate::v2::core::api::dto::RegistryItem>>>,
) -> AnyView {
    use std::collections::HashMap;
    use website_map_engine::editing::hosted_commands::VehicleCargoRow;

    let Some(v) = engine_ops::vehicle_rows().into_iter().find(|r| r.id == id) else {
        crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes();
        return ().into_any();
    };

    let items = registry_items.get().unwrap_or_default();
    let names: HashMap<String, String> = items
        .iter()
        .map(|i| (i.resource_name.clone(), i.display_name.clone()))
        .collect();
    let title = names
        .get(&v.resource_name)
        .cloned()
        .unwrap_or_else(|| v.resource_name.clone());
    let mut addable: Vec<(String, String)> = items
        .iter()
        .filter(|i| VEHICLE_CARGO_KINDS.contains(&i.kind.as_str()))
        .filter(|i| !i.r#abstract.unwrap_or(false))
        .map(|i| (i.resource_name.clone(), i.display_name.clone()))
        .collect();
    addable.sort_by(|a, b| a.1.cmp(&b.1));
    let addable = StoredValue::new(addable);
    let names = StoredValue::new(names);
    let label_of =
        move |rn: &str| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.to_string()));

    let vid = v.id.clone();
    let subtitle = format!("{title} · {vid}");
    let heading = v.rotation;
    let cargo = v.cargo.clone();
    let crew = v.crew.clone();

    let heading_row = if let Some(h) = heading {
        let id_h = StoredValue::new(vid.clone());
        number_field("Heading", h, Some("°"), Gate::open(), move |raw| {
            let deg = ((raw % 360.0) + 360.0) % 360.0;
            engine_ops::set_vehicle_heading(id_h.get_value(), deg);
        })
        .into_any()
    } else {
        ().into_any()
    };

    let rows_for_edit = cargo.clone();
    let id_add = vid.clone();
    let cargo_rows = cargo
        .into_iter()
        .enumerate()
        .map(|(i, row)| {
            let label = label_of(&row.item);
            let (base_q, base_r) = (rows_for_edit.clone(), rows_for_edit.clone());
            let (id_q, id_r) = (vid.clone(), vid.clone());
            view! {
                <div class="flex items-center gap-1.5 py-0.5">
                    <span class="min-w-0 flex-1 truncate text-label-sm text-on-surface-variant">
                        {label}
                    </span>
                    <input
                        type="number"
                        min="1"
                        aria-label="Quantity"
                        class="w-14 shrink-0 rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1 py-0.5 text-right font-mono text-label-sm tabular-nums text-on-surface outline-none focus:border-primary/60"
                        prop:value=row.qty.to_string()
                        on:change=move |ev| {
                            let Ok(q) = event_target_value(&ev).trim().parse::<i64>() else {
                                return;
                            };
                            let mut next = base_q.clone();
                            if let Some(r) = next.get_mut(i) {
                                r.qty = q;
                            }
                            engine_ops::set_vehicle_cargo(id_q.clone(), next);
                        }
                    />
                    <button
                        type="button"
                        aria-label="Remove cargo row"
                        class="shrink-0 rounded p-0.5 text-on-surface-variant hover:text-error-alert"
                        on:click=move |_| {
                            let mut next = base_r.clone();
                            if i < next.len() {
                                next.remove(i);
                            }
                            engine_ops::set_vehicle_cargo(id_r.clone(), next);
                        }
                    >
                        <crate::v2::core::ui::MaterialIcon name="close" class="block text-sm" />
                    </button>
                </div>
            }
        })
        .collect_view();

    let seat_choices = StoredValue::new(engine_ops::placed_slot_choices());
    let n_cargo_seats = DEFAULT_CARGO_SEATS;
    let seat_list = seat_model(n_cargo_seats)
        .into_iter()
        .map(|(seat_id, seat_label)| {
            let occupant = crew.get(&seat_id).cloned().unwrap_or_default();
            let id_seat = vid.clone();
            let sid = seat_id.clone();
            view! {
                <div class="flex items-center gap-1.5 py-0.5">
                    <span class="w-16 shrink-0 text-label-sm text-on-surface-variant">
                        {seat_label}
                    </span>
                    <select
                        aria-label=format!("Assign {seat_id}")
                        class="min-w-0 flex-1 rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1.5 py-0.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                        prop:value=occupant.clone()
                        on:change=move |ev| {
                            let slot = event_target_value(&ev);
                            if slot.is_empty() {
                                engine_ops::clear_crew_seat(id_seat.clone(), sid.clone());
                            } else {
                                engine_ops::assign_crew_seat(
                                    id_seat.clone(),
                                    sid.clone(),
                                    slot,
                                );
                            }
                        }
                    >
                        <option value="" selected=occupant.is_empty()>
                            "— empty —"
                        </option>
                        {seat_choices
                            .get_value()
                            .into_iter()
                            .map(|choice| {
                                let is_sel = choice.id == occupant;
                                view! {
                                    <option value=choice.id.clone() selected=is_sel>
                                        {choice.label}
                                    </option>
                                }
                            })
                            .collect_view()}
                    </select>
                </div>
            }
        })
        .collect_view();

    let base_add = rows_for_edit;
    view! {
        <div
            class="animate-overlay-fade fixed inset-0 z-50 bg-black/50 backdrop-blur-sm transition-opacity duration-200"
            on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
        ></div>
        <div class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[85vh] w-[92vw] max-w-lg -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none transition-all duration-200">
            <div class="flex items-start justify-between gap-4 border-b border-outline-variant/30 px-6 py-4">
                <div class="min-w-0">
                    <h2 class="text-headline-sm text-on-surface">"Attributes"</h2>
                    <p class="mt-1 text-label-md text-on-surface-variant">{subtitle}</p>
                    <p class="mt-1 text-label-sm normal-case text-outline">
                        "Edits apply live."
                    </p>
                </div>
                <button
                    type="button"
                    aria-label="Close"
                    on:click=move |_| crate::v2::apps::editor::bridge::host_state::editor_context::close_attributes()
                    class="rounded-md p-1 text-outline transition-colors hover:bg-surface-variant/50 hover:text-on-surface"
                >
                    <crate::v2::core::ui::MaterialIcon name="close" />
                </button>
            </div>
            <div class="custom-scrollbar flex-1 overflow-y-auto px-6 py-5">
                <div class="flex flex-col gap-4">
                    {heading_row}
                    <div class="flex flex-col gap-1">
                        <div class="flex items-center gap-1.5">
                            <crate::v2::core::ui::MaterialIcon
                                name="inventory_2"
                                class="block shrink-0 text-sm text-outline"
                            />
                            <span class="text-label-sm font-semibold text-on-surface-variant">
                                "Cargo"
                            </span>
                        </div>
                        {cargo_rows}
                        <select
                            aria-label="Add cargo"
                            class="w-full rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1.5 py-0.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                            on:change=move |ev| {
                                let item = event_target_value(&ev);
                                if item.is_empty() {
                                    return;
                                }
                                let mut next = base_add.clone();
                                if let Some(r) = next.iter_mut().find(|r| r.item == item) {
                                    r.qty = r.qty.saturating_add(1);
                                } else {
                                    next.push(VehicleCargoRow { item, qty: 1 });
                                }
                                engine_ops::set_vehicle_cargo(id_add.clone(), next);
                            }
                        >
                            <option value="">"Add cargo…"</option>
                            {addable
                                .get_value()
                                .into_iter()
                                .map(|(rn, label)| view! { <option value=rn>{label}</option> })
                                .collect_view()}
                        </select>
                    </div>
                    <div class="flex flex-col gap-1">
                        <div class="flex items-center gap-1.5">
                            <crate::v2::core::ui::MaterialIcon
                                name="group"
                                class="block shrink-0 text-sm text-outline"
                            />
                            <span class="text-label-sm font-semibold text-on-surface-variant">
                                "Crew"
                            </span>
                        </div>
                        {seat_list}
                    </div>
                </div>
            </div>
        </div>
    }
    .into_any()
}
