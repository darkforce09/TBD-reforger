//! T-661 — the placed-vehicles panel under the DockRight Vehicles tab, split from `eden_chrome.rs`.
//!
//! Every `vehiclesById` row with its map position, a heading field, a delete, and an expandable
//! `{item, qty}` cargo editor (`VEHICLE_CARGO_KINDS` is the cargo picker's allow-list). wasm-only:
//! `editor_ops` is a wasm32-only module, so the native shell renders nothing (same signature stub).
#![allow(dead_code)]
use leptos::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use crate::eden_layout::HOVER_FILL;
#[cfg(target_arch = "wasm32")]
use crate::ui::{cn, MaterialIcon};

/// T-215 — registry kinds the vehicle cargo picker offers.
///
/// A superset of `arsenal::CARGO_ADD_KINDS` (magazine / ammo / gear_item / throwable / explosive) on
/// purpose. That list answers "what goes inside a **worn garment**", so it excludes rifles, vests
/// and backpacks — things a person carries *on* rather than *in* themselves. A truck bed has no such
/// distinction: spare weapons, spare armour and spare packs are exactly what a resupply vehicle is
/// loaded with, and excluding them would make the feature useless for its main use.
///
/// Still excluded: `character` (a person is not cargo — crews are ORBAT slots), `vehicle` and
/// `vehicle_weapon` (nesting a vehicle inside a vehicle is not a thing the engine's storage does),
/// and `other` (the export's escape hatch, whose contents are by definition unclassified).
/// T-076 — the **generic** seat model shipped ahead of a per-class seat schema.
///
/// Vehicle data has no per-class seat layout yet (that is T-205, out of scope), so every placed
/// vehicle offers the same fixed crew stations — a driver, a gunner and a commander — plus a run of
/// cargo seats. This is deliberately a lowest-common-denominator model: it over-offers a seat a real
/// prefab may lack (a jeep has no commander) rather than under-offering, because an empty seat is
/// harmless (it authors nothing) while a missing seat would make a soldier unassignable. When T-205
/// lands a real schema, this constant is what it replaces.
///
/// `(seat_id, label)`. The `seat_id` is the stable doc key written into `vehicle.crew`; the label is
/// display-only. Cargo seats are appended by [`seat_model`] as `cargoN` (`N` from the vehicle's cargo
/// capacity when the registry ever exposes one, else [`DEFAULT_CARGO_SEATS`]).
const FIXED_SEATS: &[(&str, &str)] = &[
    ("driver", "Driver"),
    ("gunner", "Gunner"),
    ("commander", "Commander"),
];

/// T-076 — cargo seats offered when the vehicle has no declared cargo capacity. The registry row has
/// no per-vehicle seat count today (T-205), so this is the count every vehicle gets.
const DEFAULT_CARGO_SEATS: usize = 4;

/// T-076 — the ordered `(seat_id, label)` list a placed vehicle offers: the three fixed stations
/// then `n_cargo` cargo seats (`cargo1`…`cargoN`). Pure (no doc, no runtime) so the generic seat
/// model is unit-testable in this file's const-assertion idiom, even though the panel that consumes
/// it is wasm-only. `seat_id`s are the keys written into `vehicle.crew`.
fn seat_model(n_cargo: usize) -> Vec<(String, String)> {
    FIXED_SEATS
        .iter()
        .map(|(id, label)| ((*id).to_string(), (*label).to_string()))
        .chain((1..=n_cargo).map(|n| (format!("cargo{n}"), format!("Cargo {n}"))))
        .collect()
}

const VEHICLE_CARGO_KINDS: &[&str] = &[
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

/// T-215 — the **Placed** section under the Vehicles palette: every `vehiclesById` row with its map
/// position, a delete, and an expandable `{item, qty}` cargo editor.
///
/// This is where authored vehicle cargo is entered. It lives in the Vehicles tab rather than in the
/// Attributes modal because Attributes is keyed on a **slot** id and reads the slot SoA — a vehicle
/// is deliberately off that SoA, so opening it there would need a second, parallel modal for a
/// two-field entity. Under the palette that produced them, the placed list is also the only surface
/// that shows an author what they have already put down.
///
/// Native builds render nothing: `editor_ops` is `#![cfg(target_arch = "wasm32")]`, so there is no
/// document to read (the same reason `CatalogState` never leaves `Loading` on the native shell).
#[cfg(target_arch = "wasm32")]
pub(crate) fn placed_vehicles_panel(
    doc_tick: RwSignal<u64>,
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    expanded: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    use crate::editor_ops::{VehicleCargoRow, VehicleRow};

    // Re-read the doc on every mutation — `MissionDocCore` has no change subscription, so this is
    // the same pull-mirror tick the Attributes modal uses.
    doc_tick.track();
    let rows: Vec<VehicleRow> = crate::editor_ops::vehicle_rows();
    if rows.is_empty() {
        return ().into_any();
    }

    let items = registry_items.get().unwrap_or_default();
    let names: HashMap<String, String> = items
        .iter()
        .map(|i| (i.resource_name.clone(), i.display_name.clone()))
        .collect();
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

    let body = rows
        .into_iter()
        .map(|v| {
            let vid = v.id.clone();
            let open = expanded.with(|e| e.contains(&vid));
            let title = label_of(&v.resource_name);
            let pos = v.xy.map_or_else(
                // A vehicle added from the ORBAT Manager before it was ever dropped has no
                // position. Saying so is the point — it is the state this ticket exists to end.
                || "not placed".to_string(),
                |(x, y)| format!("{x:.1}, {y:.1}"),
            );
            let heading = v.rotation;
            let cargo = v.cargo.clone();
            let n_cargo = cargo.len();
            let crew = v.crew.clone();

            let (id_toggle, id_del) = (vid.clone(), vid.clone());
            let head = view! {
                // T-668 — the placed-vehicle header row wears HOVER_FILL, the one hover fill the
                // chrome uses (was a weaker ad-hoc `hover:bg-white/5`).
                <div class=cn(&["flex items-center gap-1.5 rounded px-1.5 py-1", HOVER_FILL])>
                    <span
                        role="button"
                        tabindex="-1"
                        aria-label=format!("Toggle cargo for {title}")
                        aria-expanded=if open { "true" } else { "false" }
                        class="flex size-4 shrink-0 cursor-pointer items-center justify-center rounded text-outline hover:text-on-surface"
                        on:click=move |_| {
                            expanded
                                .update(|e| {
                                    if !e.remove(&id_toggle) {
                                        e.insert(id_toggle.clone());
                                    }
                                });
                        }
                    >
                        <MaterialIcon
                            name=if open { "expand_more" } else { "chevron_right" }
                            class="block text-sm"
                        />
                    </span>
                    <MaterialIcon name="directions_car" class="block shrink-0 text-sm" />
                    <span class="min-w-0 flex-1 truncate text-label-sm text-on-surface">{title}</span>
                    <span class="shrink-0 font-mono text-label-sm tabular-nums text-outline">
                        {pos}
                    </span>
                    <span class="shrink-0 font-mono text-label-sm tabular-nums text-outline">
                        {format!("{n_cargo}\u{a0}items")}
                    </span>
                    <button
                        type="button"
                        aria-label="Remove vehicle"
                        class="shrink-0 rounded p-0.5 text-on-surface-variant hover:text-error-alert"
                        on:click=move |_| {
                            crate::editor_ops::remove_vehicle(id_del.clone());
                        }
                    >
                        <MaterialIcon name="delete" class="block text-sm" />
                    </button>
                </div>
            };

            if !open {
                return head.into_any();
            }

            // T-425 — heading authoring. Placed vehicles defaulted to rotation 0.0 at drop; this
            // field is how the operator sets a real heading without delete-and-replace.
            let heading_row = if let Some(h) = heading {
                let id_h = vid.clone();
                view! {
                    <div class="flex items-center gap-1.5 py-0.5 pl-7 pr-1.5">
                        <span class="shrink-0 text-label-sm text-on-surface-variant">"Heading°"</span>
                        <input
                            type="number"
                            min="0"
                            max="360"
                            step="1"
                            aria-label="Vehicle heading degrees"
                            class="w-16 shrink-0 rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1 py-0.5 text-right font-mono text-label-sm tabular-nums text-on-surface outline-none focus:border-primary/60"
                            prop:value=format!("{h:.0}")
                            on:change=move |ev| {
                                let Ok(raw) = event_target_value(&ev).trim().parse::<f64>() else {
                                    return;
                                };
                                let deg = ((raw % 360.0) + 360.0) % 360.0;
                                crate::editor_ops::set_vehicle_heading(id_h.clone(), deg);
                            }
                        />
                    </div>
                }
                .into_any()
            } else {
                ().into_any()
            };

            let rows_for_edit = cargo.clone();
            let id_add = vid.clone();
            let editor = cargo
                .into_iter()
                .enumerate()
                .map(|(i, row)| {
                    let label = label_of(&row.item);
                    let (base_q, base_r) = (rows_for_edit.clone(), rows_for_edit.clone());
                    let (id_q, id_r) = (vid.clone(), vid.clone());
                    view! {
                        <div class="flex items-center gap-1.5 py-0.5 pl-7 pr-1.5">
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
                                    crate::editor_ops::set_vehicle_cargo(id_q.clone(), next);
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
                                    crate::editor_ops::set_vehicle_cargo(id_r.clone(), next);
                                }
                            >
                                <MaterialIcon name="close" class="block text-sm" />
                            </button>
                        </div>
                    }
                })
                .collect_view();

            // T-076 — the CREW seat list. This panel is the SHIPPED crew-authoring path: the
            // context-menu entry point (CREW-SEAT-001) stays the DISABLED row T-664 shipped in
            // `context_menu.rs`, so an author boards from here, not from the map right-click.
            //
            // Every placed character is a boarding candidate; the picker options are
            // `placed_slot_choices()` (read once per render). Each seat is a `<select>` whose current
            // value is the slot the crew map assigns to it — choosing a slot boards (assign), the
            // empty option unboards (clear). The one-seat-per-slot rule lives in the op, so a slot
            // already crewing another seat is simply MOVED here; no client-side guard is needed.
            let seat_choices = StoredValue::new(crate::editor_ops::placed_slot_choices());
            // Cargo-seat count: from the vehicle's declared capacity when one exists, else the
            // generic default. The registry exposes no per-vehicle seat count today (T-205), so this
            // is `DEFAULT_CARGO_SEATS` for every vehicle — the branch is here for when it does.
            let n_cargo_seats = DEFAULT_CARGO_SEATS;
            let seat_list = seat_model(n_cargo_seats)
                .into_iter()
                .map(|(seat_id, seat_label)| {
                    let occupant = crew.get(&seat_id).cloned().unwrap_or_default();
                    let id_seat = vid.clone();
                    let sid = seat_id.clone();
                    view! {
                        <div class="flex items-center gap-1.5 py-0.5 pl-7 pr-1.5">
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
                                        crate::editor_ops::clear_crew_seat(
                                            id_seat.clone(),
                                            sid.clone(),
                                        );
                                    } else {
                                        crate::editor_ops::assign_crew_seat(
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
                {head}
                {heading_row}
                {editor}
                <div class="py-0.5 pl-7 pr-1.5">
                    <select
                        aria-label="Add cargo"
                        class="w-full rounded border border-outline-variant/40 bg-surface-container-lowest/60 px-1.5 py-0.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                        on:change=move |ev| {
                            let item = event_target_value(&ev);
                            if item.is_empty() {
                                return;
                            }
                            let mut next = base_add.clone();
                            // Adding an item already present bumps it rather than writing a second
                            // row for the same prefab: `qty` is a unit count, so two rows of 3 and
                            // one row of 6 are the same load, and the collapsed form is the one an
                            // author can read.
                            if let Some(r) = next.iter_mut().find(|r| r.item == item) {
                                r.qty = r.qty.saturating_add(1);
                            } else {
                                next.push(VehicleCargoRow { item, qty: 1 });
                            }
                            crate::editor_ops::set_vehicle_cargo(id_add.clone(), next);
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
                <div class="mt-1 flex items-center gap-1.5 pl-7 pr-1.5">
                    <MaterialIcon name="group" class="block shrink-0 text-sm text-outline" />
                    <span class="text-label-sm font-semibold text-on-surface-variant">"Crew"</span>
                </div>
                {seat_list}
            }
            .into_any()
        })
        .collect_view();

    view! {
        <div class="mt-3 border-t border-white/5 pt-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Placed"</h3>
            <div class="mt-1">{body}</div>
        </div>
    }
    .into_any()
}

/// Native shell: no document, nothing to list. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn placed_vehicles_panel(
    doc_tick: RwSignal<u64>,
    registry_items: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    expanded: RwSignal<std::collections::HashSet<String>>,
) -> AnyView {
    let _ = (doc_tick, registry_items, expanded);
    ().into_any()
}

#[cfg(test)]
mod tests {
    use super::{seat_model, DEFAULT_CARGO_SEATS, VEHICLE_CARGO_KINDS};

    /// T-076 — the generic seat model the crew list draws: three fixed stations
    /// (driver/gunner/commander) then N cargo seats, `cargo1`…`cargoN`. Pins the seat_ids (the doc
    /// keys written into `vehicle.crew`) and the default cargo count, so a rename or a reorder that
    /// would silently orphan an already-authored `vehicle.crew` entry fails here first.
    #[test]
    fn seat_model_is_driver_gunner_commander_then_cargo() {
        let seats = seat_model(DEFAULT_CARGO_SEATS);
        let ids: Vec<&str> = seats.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "driver",
                "gunner",
                "commander",
                "cargo1",
                "cargo2",
                "cargo3",
                "cargo4"
            ],
            "generic seat ids + default {DEFAULT_CARGO_SEATS} cargo seats, in order"
        );
        // The labels are display copy, but the fixed three must read as their station names.
        assert_eq!(seats[0].1, "Driver");
        assert_eq!(seats[1].1, "Gunner");
        assert_eq!(seats[2].1, "Commander");
        assert_eq!(
            seats[3].1, "Cargo 1",
            "cargo seats are 1-indexed for the operator"
        );

        // Capacity drives the cargo-seat count (the branch the registry will feed once T-205 lands):
        // zero cargo seats leaves exactly the three fixed stations, no `cargoN`.
        let none = seat_model(0);
        assert_eq!(none.len(), 3, "no cargo capacity ⇒ only the fixed stations");
        assert!(
            none.iter().all(|(id, _)| !id.starts_with("cargo")),
            "no cargo seats emitted at capacity 0"
        );
    }

    /// A vehicle's cargo picker must never offer a person or another vehicle. `character` rows are
    /// crews (ORBAT slots, not freight) and nesting a vehicle inside a vehicle is not something the
    /// engine's storage does — either would author a document whose only failure mode is silence.
    #[test]
    fn vehicle_cargo_picker_excludes_people_and_vehicles() {
        for banned in ["character", "vehicle", "vehicle_weapon", "other"] {
            assert!(
                !VEHICLE_CARGO_KINDS.contains(&banned),
                "{banned} must not be offered as vehicle cargo"
            );
        }
        // …and it is a genuine superset of the worn-garment list, which is the whole reason it is a
        // separate constant rather than a reuse of `arsenal::CARGO_ADD_KINDS`.
        for expected in ["magazine", "ammo", "gear_primary", "gear_backpack"] {
            assert!(
                VEHICLE_CARGO_KINDS.contains(&expected),
                "{expected} is exactly what a resupply vehicle carries"
            );
        }
    }

    /// T-668 — the placed-vehicle header row wears HOVER_FILL (the one chrome hover fill), not the
    /// weaker ad-hoc `hover:bg-white/5` it used to. The panel is wasm-only so a native test cannot
    /// render it; this reads the source. The needle is assembled so this test's own prose can't
    /// satisfy the absence check.
    #[test]
    fn header_row_uses_hover_fill_not_the_weak_ad_hoc_fill() {
        use crate::arsenal::class_r_scrub::live_code;
        let code = live_code(include_str!("eden_vehicles_panel.rs"));
        assert!(
            code.contains("HOVER_FILL"),
            "the header row must consume HOVER_FILL"
        );
        let weak = ["hover:bg-", "white/5"].concat();
        assert!(
            !code.contains(&weak),
            "T-668: the ad-hoc `hover:bg-white/5` header fill must be gone (use HOVER_FILL)"
        );
    }
}
