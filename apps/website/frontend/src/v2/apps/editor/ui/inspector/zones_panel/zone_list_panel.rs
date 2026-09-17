//! Zones panel zone list panel.

use super::*;

/// Renders authored zones and draw controls, or an empty native stub.
#[cfg(target_arch = "wasm32")]
pub(crate) fn zones_panel(doc_tick: RwSignal<u64>, selected: RwSignal<Option<String>>) -> AnyView {
    use website_map_engine::editing::hosted_commands as engine_ops;

    let types = zone_types();
    let initial = types
        .iter()
        .find(|t| *t == "boundary")
        .or_else(|| types.first())
        .cloned()
        .unwrap_or_default();
    let draw_kind = RwSignal::new(initial);

    let arm = move |shape: ZoneShape| {
        let kind = draw_kind.get_untracked();
        armed_placement::begin_zone_draw(&kind, shape, DrawTarget::Zone);
        doc_tick.update(|n| *n = n.wrapping_add(1));
    };

    let tactical_kind = RwSignal::new(
        website_map_engine::data::scenario::tactical_graphics::KINDS
            .first()
            .map_or_else(String::new, |k| (*k).to_string()),
    );

    let arm_tactical = move || {
        let kind = tactical_kind.get_untracked();
        tactical_graphics_authoring::begin_tactical_draw(&kind);
        doc_tick.update(|n| *n = n.wrapping_add(1));
    };

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Zones"</h3>
            <span class="font-mono text-code-md text-outline">
                {move || {
                    let _ = doc_tick.get();
                    engine_ops::zone_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Play areas and objectives. Circle: click the centre, then click the rim. Polygon: click each vertex, then Close."
        </p>

        <button
            type="button"
            title="Author one boundary zone covering the whole terrain, sized from the mission's map"
            class="mt-2 w-full rounded-md border border-primary/40 bg-primary/10 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-primary/20"
            on:click=move |_| {
                if let Some(id) = add_whole_terrain_zone() {
                    selected.set(Some(id));
                }
                doc_tick.update(|n| *n = n.wrapping_add(1));
            }
        >
            "Whole-terrain zone"
        </button>

        <label class="mt-3 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
            "Type"
        </label>
        <select
            aria-label="Zone type to draw"
            class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
            on:change=move |ev| draw_kind.set(event_target_value(&ev))
        >
            {zone_types()
                .into_iter()
                .map(|t| {
                    let label = humanize_token(&t);
                    view! {
                        <option value=t.clone() selected=move || draw_kind.get() == t>
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
            let is_poly = d.shape == ZoneShape::Polygon;
            let n = d.verts.len();
            let hint = if is_poly {
                match n {
                    0 => "Click the first vertex.".to_string(),
                    1 | 2 => {
                        format!("{n} of 3 vertices — a ring needs at least three.")
                    }
                    _ => format!("{n} vertices. Close to commit."),
                }
            } else if d.centre.is_some() {
                format!(
                    "Centre set. Click the rim — a radius under {MIN_AUTHORABLE_RADIUS_M} m rounds to zero on the {ZONE_GRID_M} m grid and is refused."
                )
            } else {
                "Click the centre.".to_string()
            };
            let can_close = is_poly && polygon_is_committable(&d.verts);
            view! {
                <div class="mt-3 rounded-md border border-primary/40 bg-primary/10 p-2">
                    <p class="text-label-sm normal-case text-on-surface">
                        {
                            let shape = if is_poly { "polygon" } else { "circle" };
                            d.target.as_ref().map_or_else(
                                || format!("Drawing {} {shape}", humanize_token(&d.kind)),
                                |id| format!("Reshaping {id} as a {shape} — label, faction and rules are kept"),
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
            let rows = engine_ops::zone_rows();
            if rows.is_empty() {
                return view! {
                    <p class="mt-3 text-label-sm normal-case text-outline">
                        "No zones yet. This mission declares no play area."
                    </p>
                }
                    .into_any();
            }
            view! {
                <ul class="mt-3 flex flex-col gap-0.5" role="list" aria-label="Authored zones">
                    {rows
                        .into_iter()
                        .map(|z| {
                            let id = z.id.clone();
                            let sel_id = z.id.clone();
                            let sel_id2 = z.id.clone();
                            let title = z
                                .label
                                .clone()
                                .filter(|l| !l.is_empty())
                                .unwrap_or_else(|| humanize_token(&z.kind));
                            let summary = z.shape_summary();
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
                                            name=if z.circle.is_some() {
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
            let Some(z) = engine_ops::zone_rows().into_iter().find(|r| r.id == id) else {
                return ().into_any();
            };
            zone_attributes(z, doc_tick, selected).into_any()
        }}

        <div class="mt-4 border-t border-outline-variant/30 pt-3">
            <div class="flex items-center gap-2">
                <h3 class="text-label-md font-semibold text-on-surface">"Tactical graphics"</h3>
                <span class="font-mono text-code-md text-outline">
                    {move || {
                        let _ = doc_tick.get();
                        tactical_graphics_authoring::tactical_graphic_count()
                    }}
                </span>
            </div>
            <p class="mt-0.5 text-label-sm normal-case text-outline">
                "Control measures. Pick a kind, press Draw, then click each vertex on the map. Right-click finishes; Esc abandons."
            </p>
            <label class="mt-2 block text-label-sm font-semibold uppercase tracking-wide text-on-surface-variant">
                "Kind"
            </label>
            <select
                aria-label="Tactical graphic kind to draw"
                data-testid="tactical-draw-kind"
                class="mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1.5 text-label-sm text-on-surface outline-none focus:border-primary/60"
                on:change=move |ev| tactical_kind.set(event_target_value(&ev))
            >
                {website_map_engine::data::scenario::tactical_graphics::KINDS
                    .iter()
                    .map(|k| {
                        let k = (*k).to_string();
                        let label = humanize_token(&k);
                        let is_sel = k.clone();
                        view! {
                            <option value=k.clone() selected=move || tactical_kind.get() == is_sel>
                                {label}
                            </option>
                        }
                    })
                    .collect_view()}
            </select>
            <button
                type="button"
                data-testid="tactical-draw-arm"
                title="Arm a tactical graphic draw — then click each vertex on the map"
                class="mt-2 w-full rounded-md border border-primary/40 bg-primary/10 px-2 py-1.5 text-label-sm text-on-surface transition-colors hover:bg-primary/20"
                on:click=move |_| arm_tactical()
            >
                "Draw"
            </button>

            {move || {
                let _ = doc_tick.get();
                let Some(d) = tactical_graphics_authoring::tactical_draft() else {
                    return ().into_any();
                };
                let n = d.verts.len();
                let floor = tactical_graphics_authoring::tactical_min_points(&d.kind).unwrap_or(2);
                let hint = if n < floor {
                    format!("{n} of {floor} vertices — {} needs at least {floor}.", humanize_token(&d.kind))
                } else {
                    format!("{n} vertices. Right-click on the map to finish.")
                };
                view! {
                    <div
                        data-testid="tactical-draw-draft"
                        class="mt-2 rounded-md border border-primary/40 bg-primary/10 p-2"
                    >
                        <p class="text-label-sm normal-case text-on-surface">
                            {format!("Drawing {}", humanize_token(&d.kind))}
                        </p>
                        <p class="mt-0.5 text-label-sm normal-case text-outline">{hint}</p>
                        <div class="mt-1.5 flex gap-1.5">
                            <button
                                type="button"
                                disabled=n < floor
                                class="rounded-md bg-primary/25 px-2 py-1 text-label-sm text-on-surface transition-colors hover:bg-primary/40 disabled:opacity-30 disabled:hover:bg-primary/25"
                                on:click=move |_| {
                                    tactical_graphics_authoring::complete_tactical_draw();
                                    doc_tick.update(|n| *n = n.wrapping_add(1));
                                }
                            >
                                "Finish"
                            </button>
                            <button
                                type="button"
                                disabled=n == 0
                                class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 disabled:opacity-30"
                                on:click=move |_| {
                                    tactical_graphics_authoring::tactical_draw_pop_vertex();
                                    doc_tick.update(|n| *n = n.wrapping_add(1));
                                }
                            >
                                "Undo vertex"
                            </button>
                            <button
                                type="button"
                                class="rounded-md px-2 py-1 text-label-sm text-on-surface-variant transition-colors hover:bg-white/10"
                                on:click=move |_| {
                                    tactical_graphics_authoring::cancel_tactical_draw();
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
        </div>
    }
    .into_any()
}
