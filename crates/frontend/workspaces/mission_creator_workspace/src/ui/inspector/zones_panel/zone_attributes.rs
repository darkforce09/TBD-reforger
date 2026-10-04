//! Zones panel zone attributes.

#[cfg(target_arch = "wasm32")]
use super::*;

/// Renders identity and rule controls for one authored zone.
#[cfg(target_arch = "wasm32")]
pub(super) fn zone_attributes(
    z: mission_operations::entity::ZoneRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use mission_editing_commands::hosted_commands as engine_ops;

    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let zid = z.id.clone();
    let input_class = "mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60";
    let field_label =
        "mt-2 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant";

    let (id_type, id_label, id_faction, id_delete) =
        (zid.clone(), zid.clone(), zid.clone(), zid.clone());
    let rules = z.rules.clone();

    view! {
        <div class="mt-3 border-t border-white/10 pt-2">
            <h4 class="text-label-md font-semibold text-on-surface">
                {format!("Attributes — {}", z.id)}
            </h4>

            <label class=field_label>"Type"</label>
            <select
                aria-label="Zone type"
                class=input_class
                on:change=move |ev| {
                    engine_ops::set_zone_kind(id_type.as_str(), &event_target_value(&ev), |k| {
                        zone_types().iter().any(|t| t == k)
                    });
                    bump();
                }
            >
                {
                    let current = z.kind.clone();
                    zone_types()
                        .into_iter()
                        .map(|t| {
                            let is = t == current;
                            let label = humanize_token(&t);
                            view! { <option value=t selected=is>{label}</option> }
                        })
                        .collect_view()
                }
            </select>

            <label class=field_label>"Label"</label>
            <div class="flex items-center gap-1.5">
                <input
                    type="text"
                    aria-label="Zone label"
                    class=input_class
                    prop:value=z.label.clone().unwrap_or_default()
                    on:change=move |ev| {
                        engine_ops::set_zone_label(id_label.as_str(), Some(event_target_value(&ev)));
                        bump();
                    }
                />
                {
                    let id_clear = zid.clone();
                    view! {
                        <button
                            type="button"
                            title="Remove the label key (not the same as an empty label)"
                            class="mt-1 shrink-0 rounded-md px-1.5 py-1.5 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                engine_ops::set_zone_label(id_clear.as_str(), None);
                                bump();
                            }
                        >
                            "Clear"
                        </button>
                    }
                }
            </div>

            <label class=field_label>"Faction"</label>
            <input
                type="text"
                aria-label="Zone faction"
                placeholder="blufor (empty = neutral)"
                class=input_class
                prop:value=z.faction.clone().unwrap_or_default()
                on:change=move |ev| {
                    let v = event_target_value(&ev);
                    let next = (!v.trim().is_empty()).then_some(v);
                    engine_ops::set_zone_faction(id_faction.as_str(), next);
                    bump();
                }
            />

            <label class=field_label>"Shape"</label>
            <div class="flex gap-1.5">
                {
                    let (a, b) = (zid.clone(), zid.clone());
                    view! {
                        <button
                            type="button"
                            title="Redraw this zone as a circle — click the centre, then the rim"
                            class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                armed_placement::begin_zone_reshape(armed_placement::ZoneReshapeTarget::Zone(a.clone()), ZoneShape::Circle);
                                bump();
                            }
                        >
                            "Redraw circle"
                        </button>
                        <button
                            type="button"
                            title="Redraw this zone as a polygon — click each vertex, then Close"
                            class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                armed_placement::begin_zone_reshape(armed_placement::ZoneReshapeTarget::Zone(b.clone()), ZoneShape::Polygon);
                                bump();
                            }
                        >
                            "Redraw polygon"
                        </button>
                    }
                }
            </div>

            <h4 class="mt-3 text-label-md font-semibold text-on-surface">"Rules"</h4>
            <p class="mt-0.5 text-label-sm normal-case text-outline">
                "Every control below is generated from the mission schema's zoneRules vocabulary. Blank means the key is not authored and the mod's default applies."
            </p>
            {zone_rule_fields()
                .into_iter()
                .map(|f| zone_rule_control(zid.to_string(), f, rules.clone(), doc_tick))
                .collect_view()}

            <button
                type="button"
                class="mt-3 w-full rounded-md border border-error/40 px-2 py-1.5 text-label-sm text-error transition-colors hover:bg-error/15"
                on:click=move |_| {
                    engine_ops::delete_zone(id_delete.as_str());
                    selected.set(None);
                    bump();
                }
            >
                "Delete zone"
            </button>
        </div>
    }
    .into_any()
}
