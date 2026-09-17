//! Right dock markers behavior.

use super::*;

/// Render marker icon selection, authored markers, and selected marker attributes.
#[cfg(target_arch = "wasm32")]
pub(crate) fn markers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::humanize_token;
    use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};

    let icon_search = RwSignal::new(String::new());

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Markers"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    engine_ops::marker_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Map markers for the active side's briefing. Pick an icon, then click the map to drop it. \
             Select a marker to caption it or nudge its position."
        </p>

        <input
            type="search"
            aria-label="Search marker icons"
            placeholder="Search icons"
            class="mt-3 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
            prop:value=move || icon_search.get()
            on:input=move |ev| icon_search.set(event_target_value(&ev))
        />
        {move || {
            let rows = canonical_marker_rows(&icon_search.get());
            if rows.is_empty() {
                return view! {
                    <p class="mt-2 text-label-sm normal-case text-outline">"No icon matches."</p>
                }
                    .into_any();
            }
            view! {
                <ul
                    class="mt-1.5 flex max-h-48 flex-col gap-0.5 overflow-y-auto"
                    role="list"
                    aria-label="Marker icons"
                >
                    {rows
                        .into_iter()
                        .map(|row| {
                            let armed = row.slug.to_string();
                            let glyph = row.glyph;
                            let label = row.label;
                            let aria = format!("{label} marker");
                            let tip = if row.aliases.len() > 1 {
                                format!(
                                    "{} — arm, then click the map to place. Slug: {}. Also: {}",
                                    label,
                                    row.slug,
                                    row.aliases.join(", "),
                                )
                            } else {
                                format!(
                                    "{} — arm, then click the map to place. Slug: {}",
                                    label, row.slug,
                                )
                            };
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        class=PALETTE_LEAF
                                        title=tip
                                        aria-label=aria
                                        on:pointerdown=move |_| {
                                            armed_placement::begin_place_marker(armed.clone());
                                            doc_tick.update(|n| *n = n.wrapping_add(1));
                                        }
                                    >
                                        {marker_glyph_svg(glyph)}
                                        <span class="truncate">{label}</span>
                                    </button>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            }
                .into_any()
        }}

        {move || {
            let _ = doc_tick.get();
            let Some(icon) = armed_placement::armed_marker_icon() else {
                return ().into_any();
            };
            view! {
                <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                    <p class="text-label-sm normal-case text-on-surface">
                        {format!("Placing a {} marker", humanize_token(&icon))}
                    </p>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">
                        "Click the map to drop it on the active side's briefing."
                    </p>
                </div>
            }
                .into_any()
        }}

        {move || {
            let _ = doc_tick.get();
            let rows = engine_ops::marker_rows();
            if rows.is_empty() {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">"No markers yet."</p>
                }
                    .into_any();
            }
            view! {
                <ul class="mt-3 flex flex-col gap-0.5" role="list" aria-label="Authored markers">
                    {rows
                        .into_iter()
                        .map(|m| {
                            let addr = (m.faction_id.clone(), m.id.clone());
                            let sel_addr = addr.clone();
                            let sel_addr2 = addr.clone();
                            let title = if m.label.is_empty() {
                                format!("{} ({})", humanize_token(&m.icon), m.side())
                            } else {
                                format!("{} ({})", m.label, m.side())
                            };
                            let pos = m.position_summary();
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        aria-pressed=move || selected.get().as_ref() == Some(&sel_addr)
                                        class=move || {
                                            if selected.get().as_ref() == Some(&sel_addr2) {
                                                ROW_ACTIVE
                                            } else {
                                                ROW
                                            }
                                        }
                                        on:click=move |_| selected.set(Some(addr.clone()))
                                    >
                                        <MaterialIcon name="place" class="block text-sm" />
                                        <span class="truncate">{title}</span>
                                        <span class="ml-auto shrink-0 font-mono text-code-md text-outline">
                                            {pos}
                                        </span>
                                    </button>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            }
                .into_any()
        }}

        {move || {
            let _ = doc_tick.get();
            let Some((faction_id, id)) = selected.get() else {
                return ().into_any();
            };
            let Some(m) = engine_ops::marker_rows()
                .into_iter()
                .find(|r| r.faction_id == faction_id && r.id == id)
            else {
                return ().into_any();
            };
            marker_attributes(m, doc_tick, selected).into_any()
        }}
    }
    .into_any()
}

/// Render the selected marker's icon, label, position, and delete controls.
#[cfg(target_arch = "wasm32")]
pub(in crate::v2::apps::editor::ui::docks::dock_right) fn marker_attributes(
    m: engine_ops::MarkerRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::humanize_token;

    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let faction = m.faction_id.clone();
    let mid = m.id.clone();

    let (f_icon, i_icon) = (faction.clone(), mid.clone());
    let (f_label, i_label) = (faction.clone(), mid.clone());
    let (f_x, i_x) = (faction.clone(), mid.clone());
    let (f_z, i_z) = (faction.clone(), mid.clone());
    let (f_del, i_del) = (faction.clone(), mid.clone());

    let current_icon = m.icon.clone();
    let label_value = m.label.clone();
    let (x_value, z_value) = (m.x, m.z);

    view! {
        <div class="mt-3 rounded-md border border-outline-variant/40 p-2">
            <h4 class="text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                {format!("Marker {} — {}", m.id, m.side())}
            </h4>

            <label class="mt-2 block text-label-sm text-on-surface-variant">"Type"</label>
            <select
                aria-label="Marker type"
                class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                on:change=move |ev| {
                    let next = event_target_value(&ev);
                    if engine_ops::set_marker_icon(&f_icon, &i_icon, &next, marker_icon_is_authorable) {
                        bump();
                    }
                }
            >
                {marker_icons()
                    .iter()
                    .map(|alias| {
                        let a = alias.clone();
                        let is_current = *alias == current_icon;
                        let label = humanize_token(alias);
                        view! { <option value=a selected=is_current>{label}</option> }
                    })
                    .collect_view()}
            </select>

            <label class="mt-2 block text-label-sm text-on-surface-variant">"Text"</label>
            <input
                type="text"
                aria-label="Marker text"
                placeholder="Caption shown on the map"
                class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                prop:value=label_value
                on:change=move |ev| {
                    let next = event_target_value(&ev);
                    if engine_ops::set_marker_label(&f_label, &i_label, &next) {
                        bump();
                    }
                }
            />

            <label class="mt-2 block text-label-sm text-on-surface-variant">
                "Position (x, z metres)"
            </label>
            <div class="mt-1 flex gap-1.5">
                <input
                    type="number"
                    step="0.1"
                    aria-label="Marker position x"
                    class="w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 font-mono text-code-md text-on-surface outline-none focus:border-primary/60"
                    prop:value=x_value
                    on:change=move |ev| {
                        let Ok(next) = event_target_value(&ev).trim().parse::<f64>() else {
                            return;
                        };
                        if engine_ops::set_marker_position(&f_x, &i_x, next, z_value) {
                            bump();
                        }
                    }
                />
                <input
                    type="number"
                    step="0.1"
                    aria-label="Marker position z"
                    class="w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 font-mono text-code-md text-on-surface outline-none focus:border-primary/60"
                    prop:value=z_value
                    on:change=move |ev| {
                        let Ok(next) = event_target_value(&ev).trim().parse::<f64>() else {
                            return;
                        };
                        if engine_ops::set_marker_position(&f_z, &i_z, x_value, next) {
                            bump();
                        }
                    }
                />
            </div>

            <button
                type="button"
                class="mt-2 rounded-md px-2 py-1 text-label-sm text-error transition-colors hover:bg-error/10"
                on:click=move |_| {
                    if engine_ops::remove_marker(&f_del, &i_del) {
                        selected.set(None);
                        bump();
                    }
                }
            >
                "Delete marker"
            </button>
        </div>
    }
    .into_any()
}

/// Native shell: no document, so no markers. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn markers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<(String, String)>>,
) -> AnyView {
    let _ = (doc_tick, selected);
    ().into_any()
}
