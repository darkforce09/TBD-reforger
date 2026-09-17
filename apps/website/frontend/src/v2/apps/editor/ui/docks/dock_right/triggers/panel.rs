//! Right dock triggers behavior.

use super::*;

/// Render trigger drawing, selection, attributes, and owner links.
#[cfg(target_arch = "wasm32")]
pub(crate) fn triggers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::v2::apps::editor::ui::inspector::zones_panel::{
        humanize_token, DrawTarget, ZoneShape,
    };
    use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};

    let draw_activation = RwSignal::new(
        engine_ops::TRIGGER_ACTIVATIONS
            .first()
            .copied()
            .unwrap_or("presence")
            .to_string(),
    );

    let arm = move |shape: ZoneShape| {
        let activation = draw_activation.get_untracked();
        armed_placement::begin_zone_draw(&activation, shape, DrawTarget::Trigger);
        doc_tick.update(|n| *n = n.wrapping_add(1));
    };

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Triggers"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    engine_ops::trigger_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Trigger areas. Pick an activation, then draw the area exactly like a zone — Circle: click centre then rim. Polygon: click each vertex, then Close. Select a trigger to set its owner."
        </p>

        <label class="mt-3 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
            "Activation"
        </label>
        <select
            aria-label="Trigger activation to draw"
            class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
            on:change=move |ev| draw_activation.set(event_target_value(&ev))
        >
            {engine_ops::TRIGGER_ACTIVATIONS
                .iter()
                .map(|a| {
                    let a = (*a).to_string();
                    let label = humanize_token(&a);
                    view! {
                        <option value=a.clone() selected=move || draw_activation.get() == a>
                            {label}
                        </option>
                    }
                })
                .collect_view()}
        </select>
        <div class="mt-2 flex gap-1.5">
            <button
                type="button"
                class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                on:click=move |_| arm(ZoneShape::Circle)
            >
                "Circle"
            </button>
            <button
                type="button"
                class="flex-1 rounded-md border border-outline-variant/40 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-white/10"
                on:click=move |_| arm(ZoneShape::Polygon)
            >
                "Polygon"
            </button>
        </div>

        {move || {
            let _ = doc_tick.get();
            let Some(d) = armed_placement::zone_draft() else {
                return ().into_any();
            };
            if d.collection != DrawTarget::Trigger {
                return ().into_any();
            }
            let is_poly = d.shape == ZoneShape::Polygon;
            let n = d.verts.len();
            let hint = if is_poly {
                match n {
                    0 => "Click the first vertex.".to_string(),
                    1 | 2 => format!("{n} of 3 vertices — a ring needs at least three."),
                    _ => format!("{n} vertices. Close to commit."),
                }
            } else if d.centre.is_some() {
                "Centre set. Click the rim.".to_string()
            } else {
                "Click the centre.".to_string()
            };
            let can_close = is_poly && crate::v2::apps::editor::ui::inspector::zones_panel::polygon_is_committable(&d.verts);
            view! {
                <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                    <p class="text-label-sm normal-case text-on-surface">
                        {
                            let shape = if is_poly { "polygon" } else { "circle" };
                            d.target.as_ref().map_or_else(
                                || format!("Drawing a {} trigger {shape}", humanize_token(&d.kind)),
                                |id| format!("Reshaping {id} as a {shape} — name, owner and rules are kept"),
                            )
                        }
                    </p>
                    <p class="mt-0.5 text-label-sm normal-case text-outline">{hint}</p>
                    <div class="mt-1.5 flex gap-1.5">
                        {is_poly
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        disabled=!can_close
                                        class="rounded-md bg-primary/25 px-2 py-1 text-label-sm text-on-surface transition-colors hover:bg-primary/40 disabled:opacity-30 disabled:hover:bg-primary/25"
                                        on:click=move |_| {
                                            armed_placement::close_zone_polygon();
                                            doc_tick.update(|n| *n = n.wrapping_add(1));
                                        }
                                    >
                                        "Close ring"
                                    </button>
                                    <button
                                        type="button"
                                        disabled=n == 0
                                        class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 disabled:opacity-30"
                                        on:click=move |_| {
                                            armed_placement::zone_draw_pop_vertex();
                                            doc_tick.update(|n| *n = n.wrapping_add(1));
                                        }
                                    >
                                        "Undo vertex"
                                    </button>
                                }
                            })}
                        <button
                            type="button"
                            class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                armed_placement::cancel_zone_draw();
                                doc_tick.update(|n| *n = n.wrapping_add(1));
                            }
                        >
                            "Cancel"
                        </button>
                    </div>
                </div>
            }
                .into_any()
        }}

        {move || {
            let _ = doc_tick.get();
            let rows = engine_ops::trigger_rows();
            if rows.is_empty() {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "No triggers yet."
                    </p>
                }
                    .into_any();
            }
            view! {
                <ul class="mt-3 flex flex-col gap-0.5" role="list" aria-label="Authored triggers">
                    {rows
                        .into_iter()
                        .map(|t| {
                            let id = t.id.clone();
                            let sel_id = t.id.clone();
                            let sel_id2 = t.id.clone();
                            let title = t
                                .name
                                .clone()
                                .filter(|l| !l.is_empty())
                                .unwrap_or_else(|| format!("Trigger {}", t.id));
                            let summary = t.shape_summary();
                            view! {
                                <li>
                                    <button
                                        type="button"
                                        aria-pressed=move || selected.get().as_deref() == Some(sel_id.as_str())
                                        class=move || {
                                            if selected.get().as_deref() == Some(sel_id2.as_str()) {
                                                ROW_ACTIVE
                                            } else {
                                                ROW
                                            }
                                        }
                                        on:click=move |_| selected.set(Some(id.clone()))
                                    >
                                        <MaterialIcon
                                            name=if t.circle.is_some() {
                                                "radio_button_unchecked"
                                            } else {
                                                "pentagon"
                                            }
                                            class="block text-sm"
                                        />
                                        <span class="truncate">{title}</span>
                                        <span class="ml-auto shrink-0 font-mono text-code-md text-outline">
                                            {summary}
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
            let Some(id) = selected.get() else {
                return ().into_any();
            };
            let Some(t) = engine_ops::trigger_rows().into_iter().find(|r| r.id == id) else {
                return ().into_any();
            };
            trigger_attributes(t, doc_tick, selected).into_any()
        }}

        <TriggerOwnerLine selected doc_tick />
    }
    .into_any()
}

/// Native shell for the trigger panel.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn triggers_panel(
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    let _ = (doc_tick, selected);
    ().into_any()
}
