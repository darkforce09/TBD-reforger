//! T-661 — the zone draw tool (T-582), split from `eden_chrome.rs`.
//!
//! The PURE half — the rules/type vocabularies read from the embedded `mission.schema.json`, the
//! 0.1 m grid, and the two shape predicates — is deliberately NOT cfg-gated so it TESTS on the
//! native `cargo test` shell. The panel/attributes/rule-control views are wasm-only (they drive
//! `editor_ops`, a wasm32-only module). The doc-mutating half lives in `editor_ops`.
#![allow(dead_code)]
// Ungated: the native `zones_panel` stub returns `AnyView` and calls `.into_any()`, so it needs
// leptos in scope too (the wasm views additionally use the tree row recipes + MaterialIcon).

use leptos::prelude::*;
#[cfg(any(test, target_arch = "wasm32"))]
pub use website_map_engine::data::store::operations::zones::DrawTarget;

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::ui::outliner::tree::{ROW, ROW_ACTIVE};
#[cfg(target_arch = "wasm32")]
use crate::v2::core::ui::MaterialIcon;

/// T-582 — the Zones panel: draw controls, the authored-zone list, and the schema-driven
/// Attributes panel.
///
/// The whole panel is one function rather than a component so the native shell can stub it with the
/// same signature, exactly as [`placed_vehicles_panel`] does.
#[cfg(target_arch = "wasm32")]
pub(crate) fn zones_panel(doc_tick: RwSignal<u64>, selected: RwSignal<Option<String>>) -> AnyView {
    use website_map_engine::editing::hosted_commands as engine_ops;

    // The type the next draw will carry. Seeded from the schema, not typed here; `boundary` is the
    // play area, which is the zone a mission is most likely to want first.
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
        // T-079 — the zone panel arms the SAME draw tool as the trigger panel, targeting the ZONE
        // collection. `begin_zone_draw` takes the target so the trigger panel is a second consumer of
        // the identical call (see [`DrawTarget`]).
        armed_placement::begin_zone_draw(&kind, shape, DrawTarget::Zone);
        doc_tick.update(|n| *n = n.wrapping_add(1));
    };

    // T-946.86 (.84) — the tactical-graphic kind the next draw will carry. Seeded from the core's
    // own `KINDS` vocabulary rather than a list typed here, for the reason `zone_types()` reads the
    // schema: a second copy of the vocabulary drifts the moment a kind is added, and `min_points`
    // (which `begin_tactical_draw` refuses an unknown kind by) is the same core module's function.
    let tactical_kind = RwSignal::new(
        website_map_engine::data::scenario::tactical_graphics::KINDS
            .first()
            .map_or_else(String::new, |k| (*k).to_string()),
    );

    // T-946.86 (.84) — THE ARM. Peer of `arm` above, and deliberately the same three lines:
    // read the kind, press the ops call, bump the tick so the draft block below re-reads.
    // `begin_tactical_draw` returns false for a kind `tactical_min_points` does not know; the
    // select can only offer `KINDS`, so a false here means the vocabulary moved underneath us and
    // the absent draft block is the honest result rather than a silently armed tool.
    let arm_tactical = move || {
        let kind = tactical_kind.get_untracked();
        tactical_graphics_authoring::begin_tactical_draw(&kind);
        doc_tick.update(|n| *n = n.wrapping_add(1));
    };

    view! {
        <div class="mt-2 flex items-center gap-2">
            <h3 class="text-label-md font-semibold text-on-surface">"Zones"</h3>
            // T-211's cheap count getter — "does this mission declare a play area?" without
            // materialising every row.
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

        // ── Whole-terrain zone (T-702 / 3DEN-MISC-001 E11) ─────────────────────────────────
        // One press authors the play area every mission wants first, sized to the map from the
        // same `terrain_bounds` the compile reads — instead of the author walking a 12.8 km ring
        // vertex by vertex. It is a CREATE, not a gesture: no draw is armed and no click follows.
        // The returned id goes straight into `selected`, which is what makes the Attributes panel
        // below open on the new zone; the tick bump re-reads the count and the list. A `None`
        // (no document, or a rect the world guard refused) leaves the selection untouched rather
        // than pointing Attributes at an id that was never minted.
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

        // ── Draw controls ──────────────────────────────────────────────────────────────────
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

        // ── Live draw state ────────────────────────────────────────────────────────────────
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

        // ── Authored zones ─────────────────────────────────────────────────────────────────
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

        // ── Attributes for the selected zone ───────────────────────────────────────────────
        {move || {
            let _ = doc_tick.get();
            let Some(id) = selected.get() else {
                return ().into_any();
            };
            let Some(z) = engine_ops::zone_rows().into_iter().find(|r| r.id == id) else {
                // Deleted underneath us (undo, or a reload that dropped it).
                return ().into_any();
            };
            zone_attributes(z, doc_tick, selected).into_any()
        }}

        // ══════ T-946.86 (.84) — TACTICAL GRAPHICS: the arm the draw tool never had ══════
        //
        // `begin_tactical_draw` — the engine's `data/store/operations/tactical_graphics.rs`,
        // reached from here through `bridge/tactical_graphics_authoring.rs` — needs exactly one
        // call site for the whole path to be live, and this is it. Everything downstream is
        // already wired: `gestures.rs` appends a vertex per canvas click, its `oncontextmenu`
        // finishes the draw, `commands.rs` Esc abandons it.
        //
        // A BUTTON, NOT A KEYBINDING, and that is a constraint rather than a preference: a chord
        // in `input/window_keydown.rs` compiles but reddens `help_modal.rs`'s
        // `every_binding_has_a_help_entry` (no matching `Shortcut` row) and risks
        // `no_two_listeners_claim_the_same_chord`. `help_modal.rs` is another slice's this wave.
        //
        // It rides in the Zones panel because a control measure is the same KIND of authoring act
        // as a zone — arm a multi-click draw, click vertices on the map, close it — so the draw
        // affordances live together and the `arm` closure above is the precedent this copies.
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

            // Live draft state — the same shape the zone draw block above uses, so an armed
            // tactical draw is as visible as an armed zone draw and cannot be silently in flight.
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

/// T-582 — the Attributes panel for one zone. Identity comes from the six declared `zone` keys;
/// the rules half is GENERATED from `$defs/zoneRules` by [`zone_rule_fields`].
#[cfg(target_arch = "wasm32")]
fn zone_attributes(
    z: website_map_engine::editing::hosted_commands::ZoneRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use website_map_engine::editing::hosted_commands as engine_ops;

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
                    engine_ops::set_zone_kind(&id_type, &event_target_value(&ev), |k| {
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

            // `label` is optional AND allows the empty string, which the mod reads as "use the
            // PrettyZoneTitle fallback". Both states are reachable on purpose: typing nothing into
            // the box writes `""`, and Clear removes the key.
            <label class=field_label>"Label"</label>
            <div class="flex items-center gap-1.5">
                <input
                    type="text"
                    aria-label="Zone label"
                    class=input_class
                    prop:value=z.label.clone().unwrap_or_default()
                    on:change=move |ev| {
                        engine_ops::set_zone_label(&id_label, Some(event_target_value(&ev)));
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
                                engine_ops::set_zone_label(&id_clear, None);
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
                    engine_ops::set_zone_faction(&id_faction, next);
                    bump();
                }
            />

            // Reshape — `set_zone_circle` / `set_zone_polygon` replace the whole `shape`, so the
            // label, faction and rules above survive. Delete-and-redraw would lose all three.
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
                                armed_placement::begin_zone_reshape(&a, ZoneShape::Circle, DrawTarget::Zone);
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
                                armed_placement::begin_zone_reshape(&b, ZoneShape::Polygon, DrawTarget::Zone);
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
                .map(|f| zone_rule_control(zid.clone(), f, rules.clone(), doc_tick))
                .collect_view()}

            <button
                type="button"
                class="mt-3 w-full rounded-md border border-error/40 px-2 py-1.5 text-label-sm text-error transition-colors hover:bg-error/15"
                on:click=move |_| {
                    engine_ops::delete_zone(&id_delete);
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

/// T-582 — ONE `$defs/zoneRules` property as a control. The `key` is the schema's, verbatim; this
/// function never spells a rule name. Clearing a control removes the key (the mod's default returns)
/// rather than writing a zero, because "authored 0" and "not authored" are different documents for
/// every numeric key — the schema carries an ABSENT sentinel precisely to keep them apart.
#[cfg(target_arch = "wasm32")]
fn zone_rule_control(
    zone_id: String,
    f: ZoneRuleField,
    rules: serde_json::Value,
    doc_tick: RwSignal<u64>,
) -> AnyView {
    use website_map_engine::editing::hosted_commands as engine_ops;

    let current = rules.get(&f.key).cloned();
    let bump = move || doc_tick.update(|n| *n = n.wrapping_add(1));
    let label = humanize_key(&f.key);
    let doc = f.doc.clone();
    let key = f.key.clone();
    let row = "mt-2";
    let ctl = "mt-1 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface outline-none focus:border-primary/60";

    let body = match f.kind {
        ZoneRuleKind::Bool { default } => {
            let checked = current.as_ref().and_then(serde_json::Value::as_bool);
            let k = key.clone();
            view! {
                <label class="mt-2 flex items-center gap-2 text-label-sm text-on-surface">
                    <input
                        type="checkbox"
                        aria-label=label.clone()
                        prop:checked=checked.unwrap_or(default)
                        prop:indeterminate=checked.is_none()
                        on:change=move |ev| {
                            let on = event_target_checked(&ev);
                            engine_ops::set_zone_rule(&zone_id, &k, Some(serde_json::Value::Bool(on)));
                            bump();
                        }
                    />
                    <span>{label.clone()}</span>
                    <span class="ml-auto font-mono text-code-md text-outline">
                        {format!("default {default}")}
                    </span>
                </label>
            }
            .into_any()
        }
        ZoneRuleKind::Choice { options, default } => {
            let cur = current
                .as_ref()
                .and_then(|v| v.as_str())
                .map(ToString::to_string);
            let k = key.clone();
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <select
                        aria-label=label.clone()
                        class=ctl
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let next = (!v.is_empty()).then(|| serde_json::Value::String(v));
                            engine_ops::set_zone_rule(&zone_id, &k, next);
                            bump();
                        }
                    >
                        <option value="" selected=cur.is_none()>
                            {default
                                .as_ref()
                                .map_or_else(
                                    || "(not authored)".to_string(),
                                    |d| format!("(not authored — default {d})"),
                                )}
                        </option>
                        {options
                            .into_iter()
                            .map(|o| {
                                let is = cur.as_deref() == Some(o.as_str());
                                let l = humanize_token(&o);
                                view! { <option value=o selected=is>{l}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
            }
            .into_any()
        }
        ZoneRuleKind::Number {
            default,
            minimum,
            exclusive_minimum,
            maximum,
            integer,
        } => {
            let cur = current.as_ref().and_then(serde_json::Value::as_f64);
            let k = key.clone();
            // `min` on the control is the schema's, so the browser refuses out-of-range before the
            // save does. `exclusiveMinimum` has no HTML equivalent, so it becomes the smallest
            // representable step above the bound rather than being silently dropped.
            let step = if integer { 1.0 } else { 0.1 };
            let min_attr = minimum.or_else(|| exclusive_minimum.map(|m| m + step));
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <input
                        type="number"
                        aria-label=label.clone()
                        class=ctl
                        step=step
                        min=min_attr.map(|m| m.to_string())
                        max=maximum.map(|m| m.to_string())
                        placeholder=default
                            .map_or_else(
                                || "(not authored)".to_string(),
                                |d| format!("(not authored — default {d})"),
                            )
                        prop:value=cur.map(|v| v.to_string()).unwrap_or_default()
                        on:change=move |ev| {
                            let raw = event_target_value(&ev);
                            let next = if raw.trim().is_empty() {
                                None
                            } else {
                                raw.trim()
                                    .parse::<f64>()
                                    .ok()
                                    .and_then(serde_json::Number::from_f64)
                                    .map(serde_json::Value::Number)
                            };
                            // A blank box removes the key; an unparseable one changes nothing.
                            if next.is_some() || raw.trim().is_empty() {
                                engine_ops::set_zone_rule(&zone_id, &k, next);
                                bump();
                            }
                        }
                    />
                </div>
            }
            .into_any()
        }
        ZoneRuleKind::Text { default, pattern } => {
            let cur = current
                .as_ref()
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            let k = key.clone();
            view! {
                <div class=row>
                    <label class="block text-label-sm text-on-surface">{label.clone()}</label>
                    <input
                        type="text"
                        aria-label=label.clone()
                        class=ctl
                        pattern=pattern
                        placeholder=default.unwrap_or_else(|| "(not authored)".to_string())
                        prop:value=cur
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            let next = (!v.trim().is_empty())
                                .then(|| serde_json::Value::String(v.trim().to_string()));
                            engine_ops::set_zone_rule(&zone_id, &k, next);
                            bump();
                        }
                    />
                </div>
            }
            .into_any()
        }
    };
    view! {
        <div title=doc>{body}</div>
    }
    .into_any()
}

/// Native shell: no document, so no zones. See the wasm sibling.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn zones_panel(doc_tick: RwSignal<u64>, selected: RwSignal<Option<String>>) -> AnyView {
    let _ = (doc_tick, selected);
    ().into_any()
}

/* ══════════════════════ T-582 — the zone draw tool (document half is T-211) ══════════════════════ */

// T-211 shipped `zones` + eleven mutators on `MissionDocCore` and proved authored zones reach the
// mod through flatten. It shipped NO product surface: before this slice a zone was authorable only
// from native test code. T-581 then made authoring SAFE by refusing at save what `/compiled` would
// refuse at serve — without it the first thing this tool would do is let an author permanently 500
// their own mission, which is why T-582 was blocked on it.
//
// This block is the tool's PURE half: the rules vocabulary, the type vocabulary, the 0.1 m grid and
// the two shape predicates. It is deliberately NOT `#[cfg(target_arch = "wasm32")]` — everything
// here is plain arithmetic and JSON, so it compiles and TESTS on the native target, where
// `cargo test -p website-frontend` can actually run it. The doc-mutating half lives in `editor_ops`
// (wasm-only, because `MissionDocCore` is a wasm32-only dependency of this crate — see Cargo.toml).

/// `mission.schema.json`, embedded so the rules panel is GENERATED from the vocabulary rather than
/// from a list typed here.
///
/// ═══ WHY THIS IS AN `include_str!` AND NOT SIXTEEN `const`s ═══
///
/// `$defs/zoneRules` is `additionalProperties: false` over exactly sixteen keys, and T-241 closed it
/// that way *specifically* so its four consumer tickets would not each invent their own copy. The
/// schema's own prose says why: both mod readers are TYPED, so a key they do not declare is
/// INVISIBLE to them — not rejected, not logged — which makes the schema "the ONLY place a
/// misspelled rule key can be caught". `doc/store.rs` `set_zone_rules` stores the object OPAQUE for
/// the same reason, and T-581 validates saves against these same bytes rather than restating them.
///
/// A hand-typed list of the sixteen keys in this file would be the second vocabulary all three of
/// those went out of their way to avoid: it would drift the moment a key is added, it would silently
/// omit a rule the schema declares, and — per T-216 — emitting a key the schema does NOT declare
/// 500s `/compiled` for every mission. So the panel reads the vocabulary at runtime and renders
/// whatever it finds. Add a key to `$defs/zoneRules` and the control appears with no edit here;
/// remove one and it disappears. `zone_rule_fields_cover_the_whole_vocabulary` pins that property.
///
/// Embedded once for the crate via this `pub(crate)` const (T-757); other modules read it
/// rather than a second `include_str!`. Bundle size follows the schema file — do not restate it.
pub(crate) const MISSION_SCHEMA: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../packages/tbd-schema/schema/mission.schema.json"
));

/// One authored `rules` control, derived from one `$defs/zoneRules` property.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneRuleField {
    /// The schema property name — this IS the wire key; never re-spelled.
    pub key: String,
    pub kind: ZoneRuleKind,
    /// The schema `description`, shown as the control's help text (the mod call sites are in there).
    pub doc: String,
}

/// How to render a `$defs/zoneRules` property, read from its declared type/enum/bounds.
#[derive(Clone, Debug, PartialEq)]
pub enum ZoneRuleKind {
    /// `type: boolean`. The schema's own note: absent and `false` are indistinguishable to the mod,
    /// so the default IS what the author gets by writing nothing.
    Bool { default: bool },
    /// `type: string` + `enum` → a fixed option list (`penalty`, `onEmpty`).
    Choice {
        options: Vec<String>,
        default: Option<String>,
    },
    /// `type: string` with no enum, possibly behind a `$ref` (`targetAlias` → `$defs/alias`).
    Text {
        default: Option<String>,
        pattern: Option<String>,
    },
    /// `type: number` / `integer`, carrying whichever bounds the schema declares.
    Number {
        default: Option<f64>,
        /// `minimum` (inclusive) — the reader's own `< 0` error branch.
        minimum: Option<f64>,
        /// `exclusiveMinimum` — e.g. `warnEverySeconds`, where 0 would mean "warn every frame".
        exclusive_minimum: Option<f64>,
        /// `maximum` — T-275 pinned these to the mod's sanity ceilings.
        maximum: Option<f64>,
        integer: bool,
    },
}

/// Resolve a one-hop `$ref` into `#/$defs/*`. `targetAlias` is declared as a `$ref` to `$defs/alias`
/// rather than inline, so a resolver that ignored `$ref` would render it as an untyped control and
/// drop the alias `pattern` — the exact silent-omission this whole approach exists to prevent.
fn resolve_ref<'a>(
    schema: &'a serde_json::Value,
    node: &'a serde_json::Value,
) -> &'a serde_json::Value {
    let Some(r) = node.get("$ref").and_then(serde_json::Value::as_str) else {
        return node;
    };
    r.strip_prefix("#/$defs/")
        .and_then(|name| schema.get("$defs").and_then(|d| d.get(name)))
        .unwrap_or(node)
}

/// The `rules` vocabulary as controls, in schema declaration order (which groups play-area keys
/// before objective keys — the order the schema author chose, not one re-imposed here).
///
/// Returns empty only if the embedded schema stops having `$defs/zoneRules/properties`, which
/// `zone_rule_fields_cover_the_whole_vocabulary` fails loudly on rather than rendering a blank panel.
#[must_use]
pub fn zone_rule_fields() -> Vec<ZoneRuleField> {
    let Ok(schema) = serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA) else {
        return Vec::new();
    };
    let Some(props) = schema
        .get("$defs")
        .and_then(|d| d.get("zoneRules"))
        .and_then(|z| z.get("properties"))
        .and_then(serde_json::Value::as_object)
    else {
        return Vec::new();
    };
    props
        .iter()
        .map(|(key, raw)| {
            let node = resolve_ref(&schema, raw);
            // `description` is read from the AUTHORED property, not the resolved `$ref` target:
            // `targetAlias` documents its own role ("objective_destroy, and EFFECTIVELY REQUIRED
            // there"), which the shared `$defs/alias` blurb does not.
            let doc = raw
                .get("description")
                .or_else(|| node.get("description"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string();
            let default = raw.get("default").or_else(|| node.get("default"));
            let ty = node.get("type").and_then(serde_json::Value::as_str);
            let enum_opts = node.get("enum").and_then(serde_json::Value::as_array);
            let kind = match (ty, enum_opts) {
                (Some("boolean"), _) => ZoneRuleKind::Bool {
                    default: default
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                },
                (_, Some(opts)) => ZoneRuleKind::Choice {
                    options: opts
                        .iter()
                        .filter_map(|o| o.as_str().map(ToString::to_string))
                        .collect(),
                    default: default
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                },
                (Some("number" | "integer"), _) => ZoneRuleKind::Number {
                    default: default.and_then(serde_json::Value::as_f64),
                    minimum: node.get("minimum").and_then(serde_json::Value::as_f64),
                    exclusive_minimum: node
                        .get("exclusiveMinimum")
                        .and_then(serde_json::Value::as_f64),
                    maximum: node.get("maximum").and_then(serde_json::Value::as_f64),
                    integer: ty == Some("integer"),
                },
                _ => ZoneRuleKind::Text {
                    default: default
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                    pattern: node
                        .get("pattern")
                        .and_then(serde_json::Value::as_str)
                        .map(ToString::to_string),
                },
            };
            ZoneRuleField {
                key: key.clone(),
                kind,
                doc,
            }
        })
        .collect()
}

/// The six `zone.type` values, read from `$defs/zone/properties/type/enum` for the same reason the
/// rules are: `set_zone_type` writes whatever it is handed, and a seventh value typed here would
/// save 201 and then 500 `/compiled` forever (T-581's measured failure, from the other side).
#[must_use]
pub fn zone_types() -> Vec<String> {
    let Ok(schema) = serde_json::from_str::<serde_json::Value>(MISSION_SCHEMA) else {
        return Vec::new();
    };
    schema
        .get("$defs")
        .and_then(|d| d.get("zone"))
        .and_then(|z| z.get("properties"))
        .and_then(|p| p.get("type"))
        .and_then(|t| t.get("enum"))
        .and_then(serde_json::Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// A human label for a schema key/value token (`objective_hold_until` → "Objective hold until").
/// Presentation only — the token itself is what reaches the document, never this string.
#[must_use]
pub fn humanize_token(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    for (i, part) in token.split('_').enumerate() {
        if i > 0 {
            out.push(' ');
        }
        let mut cs = part.chars();
        if let Some(f) = cs.next() {
            if i == 0 {
                out.extend(f.to_uppercase());
            } else {
                out.push(f);
            }
            out.push_str(cs.as_str());
        }
    }
    out
}

/// A camelCase schema key as a label (`warnEverySeconds` → "Warn every seconds").
#[must_use]
pub fn humanize_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    for (i, c) in key.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push(' ');
            }
            out.extend(c.to_lowercase());
        } else if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// One-decimal metre quantisation. **Mirrors `mission::flatten::round_coord`**, which is private
/// there, pinned against its source by `zone_quantisation_mirrors_flatten` — the same guarded-mirror
/// idiom `api/src/contract/validate.rs` uses for this exact line, and for the same reason: the bug
/// class is DISAGREEMENT between two sites, so the copy is made to go red on drift rather than
/// avoided by discipline.
fn round_coord(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

/// Does a circle of radius `r` still have area once the compile quantises it?
///
/// ═══ WHY THE TOOL ASKS THIS AT ALL ═══
///
/// `flatten.rs` `round_coord` hits a circle's x/z/**r**, so the authored radius is not the compiled
/// one. `round_coord(0.04) == 0.0`, and `0.0` violates `$defs/circle.r`'s `exclusiveMinimum: 0` —
/// a radius that is schema-VALID going in is schema-INVALID coming out. T-581 now catches that at
/// save with a message telling the author to drag out a radius, so it can no longer 500 a mission.
/// But a tool that lets an author build a zone the save will reject is still a tool that wastes
/// their time, so this predicate refuses the shape at CREATION and the save-time check becomes the
/// backstop it should be rather than the first line of defence.
///
/// Expressed as `round_coord(r) > 0.0` and not as a literal threshold on purpose: the threshold is a
/// CONSEQUENCE of the grid (it works out to [`MIN_AUTHORABLE_RADIUS_M`]), and writing the
/// consequence down instead of the cause is how the two drift apart when the grid changes.
#[must_use]
pub fn radius_survives_compile(r: f64) -> bool {
    r.is_finite() && r > 0.0 && round_coord(r) > 0.0
}

/// The smallest radius that survives [`round_coord`] — documentation for the UI hint, ASSERTED
/// against the predicate by `min_radius_is_the_grid_consequence` rather than trusted.
pub const MIN_AUTHORABLE_RADIUS_M: f64 = 0.05;

/// The mod's coordinate grid, in metres. Every polygon vertex and a circle's x/z/r land on it, so a
/// precision affordance finer than this would be quietly wrong — the tool rounds its readouts here
/// and offers no sub-decimetre control.
pub const ZONE_GRID_M: f64 = 0.1;

/// A circle authored as centre-click → rim-click, as the document will store it: `(x, z, r)`.
///
/// Returns `None` for a rim that coincides with the centre (the degenerate click-without-travel that
/// produces the `r → 0.0` zone), and for any non-finite input (an unproject against a singular
/// camera matrix reads as NaN, and NaN must not reach the document).
///
/// Note the argument names: the document's second axis is `z`, not `y`. `flatten.rs` writes
/// `circle {x, z, r}` and the map's world `y` IS that `z` — naming it `z` here keeps the tool
/// speaking the document's vocabulary rather than the viewport's.
#[must_use]
pub fn circle_from_clicks(cx: f64, cz: f64, rim_x: f64, rim_z: f64) -> Option<(f64, f64, f64)> {
    if ![cx, cz, rim_x, rim_z].iter().all(|v| v.is_finite()) {
        return None;
    }
    let r = (rim_x - cx).hypot(rim_z - cz);
    radius_survives_compile(r).then_some((cx, cz, r))
}

/// May an in-progress polygon be COMMITTED?
///
/// `$defs/polygon` is `minItems: 3`, and `doc/store.rs` deliberately does not guard it: its own note
/// says "the guard that an in-progress polygon needs (do not COMMIT a zone until the ring closes
/// with ≥3 points) is the draw tool's". This is that guard. A two-vertex ring would be a document
/// the schema refuses, so the Close control stays disabled until the third vertex lands.
#[must_use]
pub fn polygon_is_committable(verts: &[(f64, f64)]) -> bool {
    verts.len() >= 3 && verts.iter().all(|(x, z)| x.is_finite() && z.is_finite())
}

/// The flat `[x0,z0,x1,z1,…]` ring the doc layer's `add_polygon_zone` / `set_zone_polygon` take.
/// They cross as one `&[f64]` because that is the shape a wasm boundary carries cheaply — kept here
/// even though this build has no such boundary, since the doc-layer signature is the contract.
#[must_use]
pub fn polygon_flat(verts: &[(f64, f64)]) -> Vec<f64> {
    let mut out = Vec::with_capacity(verts.len() * 2);
    for (x, z) in verts {
        out.push(*x);
        out.push(*z);
    }
    out
}

/* ════════════ T-702 (3DEN-MISC-001 E11) — the whole-terrain zone, the pure half ════════════ */

/// The label a whole-terrain zone is authored with. Presentation seed, not a wire key: it lands in
/// `zone.label` (which the schema leaves free — no `minLength`, empty allowed) and the author can
/// rename or clear it in Attributes exactly like a hand-drawn zone.
pub const WHOLE_TERRAIN_ZONE_LABEL: &str = "Play Area";

/// The schema `zone.type` a whole-terrain zone carries — `boundary`, the play area, resolved from
/// [`zone_types`] rather than spelled.
///
/// Same reason the draw picker seeds itself from the schema and `zone_rule_fields` generates its
/// controls: `add_polygon_zone` writes whatever `type` it is handed, and a value this file invented
/// would save 201 and then 500 `/compiled` forever (T-581, measured from the other side). `None`
/// means the schema no longer declares `boundary`, and the affordance must then offer nothing
/// rather than guess — a missing type is new information, not a default.
#[must_use]
pub fn whole_terrain_zone_type() -> Option<String> {
    zone_types().into_iter().find(|t| t == "boundary")
}

/// May `bounds` be authored as a whole-terrain rect at all? The rect twin of
/// [`radius_survives_compile`].
///
/// A rect whose extent quantises to zero on either axis collapses two of its four corners onto each
/// other under `flatten`'s [`round_coord`], leaving a ring the `$defs/polygon` `minItems: 3` intent
/// refuses — the same `r → 0.0` failure a degenerate circle has, in two dimensions. Expressed over
/// the grid and over [`polygon_is_committable`] (not as a bespoke threshold) so it cannot disagree
/// with the rule the rest of the tool already applies.
///
/// **Honest scope:** both shipped terrains (Everon `12800²`, Arland `4096²`) pass this today, and
/// `terrain_bounds` resolves every unknown key to Everon — so no *current* terrain can trip it. It
/// is a precondition on the BOUNDS, not on the terrain table, and it is the guard that keeps this
/// affordance safe the moment a terrain's extent stops being a compile-time literal (a `custom`
/// terrain sized from a manifest is already a shape `MissionMeta::custom_terrain_name` anticipates).
#[must_use]
pub fn terrain_rect_is_authorable(bounds: [f64; 4]) -> bool {
    let [min_x, min_z, max_x, max_z] = bounds;
    // Positive area AFTER quantisation: `round_coord(0.04) == 0.0`, so a rect thinner than the
    // 0.1 m grid on either axis is refused here rather than committed and rejected at save.
    let has_area = round_coord(max_x - min_x) > 0.0 && round_coord(max_z - min_z) > 0.0;
    has_area && polygon_is_committable(&terrain_rect_corners(bounds))
}

/// The four corners of the terrain rect, SW → SE → NE → NW.
///
/// The winding is not a taste: it is byte-for-byte the order `flatten::synthesize_terrain_boundary`
/// emits for its `z_bounds` fallback AO, so an authored whole-terrain zone and the boundary the
/// compile would have synthesised for the same map are the SAME ring rather than two rings that
/// merely cover the same ground. `whole_terrain_ring_is_the_rect_the_compile_reads` proves that
/// against the real `flatten_to_mod_document`, for both terrains.
///
/// The document's second axis is `z`, not `y` (see [`circle_from_clicks`]) — `terrain_bounds`'
/// `minY`/`maxY` are this ring's z coordinates.
fn terrain_rect_corners(bounds: [f64; 4]) -> [(f64, f64); 4] {
    let [min_x, min_z, max_x, max_z] = bounds;
    [
        (min_x, min_z),
        (max_x, min_z),
        (max_x, max_z),
        (min_x, max_z),
    ]
}

/// T-702 — the whole-terrain ring for `terrain`, as the flat `[x0,z0,…]` `add_polygon_zone` takes.
///
/// ═══ THE ANSWER CARRIES THE WORLD IT WAS BUILT FOR ═══
///
/// This is a spatial answer — "the map, exactly" — so it names the map it answered for and
/// **refuses a caller whose bounds disagree with it**. `bounds` is not trusted: it must equal
/// `compile::terrain_bounds(terrain)`, the one helper `flatten` (`synthesize_terrain_boundary`),
/// `validate` (`V3-SLOT-IN-BOUNDS`) and the editor's own `terrain_bounds_of` all read. A caller
/// that resolved the extent some other way — a manifest field, a remembered `12800`, a stale
/// terrain key read separately from the bounds — gets `None` and authors nothing, instead of
/// silently drawing a "whole-terrain" zone that is not the terrain. That failure is invisible in
/// the editor (a big rectangle looks like a big rectangle) and only shows up in-game as a play
/// area with the wrong edge, which is precisely why the disagreement is refused here.
///
/// `None` also for a rect [`terrain_rect_is_authorable`] rejects.
///
/// Returns a POLYGON, never a circle: the terrain is a square, and a disc inscribed in it leaves
/// the corners outside the play area while one circumscribing it runs far past the map edge.
#[must_use]
pub fn terrain_rect_ring(terrain: &str, bounds: [f64; 4]) -> Option<Vec<f64>> {
    if bounds != website_map_engine::data::scenario::compile::terrain_bounds(terrain) {
        return None;
    }
    if !terrain_rect_is_authorable(bounds) {
        return None;
    }
    Some(polygon_flat(&terrain_rect_corners(bounds)))
}

#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::host_state::armed_placement;
#[cfg(target_arch = "wasm32")]
use crate::v2::apps::editor::bridge::tactical_graphics_authoring;
/// Which shape a zone draw is building. The vocabulary is the document's, so the panel and the
/// authored row can never disagree about what a draw is producing.
pub use website_map_engine::data::store::operations::zones::ZoneShape;

/// T-079 (CONN-TRG-OWNER-001) — the owner-link line's SCREEN geometry: the projected endpoints
/// (trigger centre → owner) ready for one `<line>`. Pure so a native `cargo test` proves the
/// projection with no engine/`window`, the [`website_map_engine::editing::tools::ruler::ProjectedLeg`] idiom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedOwnerLine {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// Project the owner-link line's two WORLD endpoints (trigger centre `a`, owner position `b`) to
/// screen space through a world→pixel projector (the live `OrthoCamera::project` on wasm; injected
/// here so this stays pure + native-testable, exactly like [`website_map_engine::editing::tools::ruler::project_legs`]).
#[must_use]
pub fn project_owner_line<F>(a: (f64, f64), b: (f64, f64), project: F) -> ProjectedOwnerLine
where
    F: Fn(f64, f64) -> (f64, f64),
{
    let (x1, y1) = project(a.0, a.1);
    let (x2, y2) = project(b.0, b.1);
    ProjectedOwnerLine { x1, y1, x2, y2 }
}

/// Author one boundary zone covering the whole terrain, sized from the mission's own map.
///
/// Every mission wants a play area, and the only other way to get one is to walk a 12.8 km ring
/// vertex by vertex through the draw tool, on a map where a pixel is metres. The ring, the zone
/// type and the label are this panel's vocabulary — the schema enum and the panel's own terrain
/// rectangle — so they are resolved here and handed to the document already decided.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn add_whole_terrain_zone() -> Option<String> {
    use website_map_engine::data::store::operations::entity::{terrain_bounds_of, terrain_key_of};
    use website_map_engine::editing::hosted_commands as engine_ops;

    let (terrain, bounds) = website_map_engine::editing::host::with_doc(|core| {
        (terrain_key_of(core), terrain_bounds_of(core))
    })?;
    let ring = terrain_rect_ring(&terrain, bounds)?;

    let kind = whole_terrain_zone_type()?;
    engine_ops::add_authored_row(DrawTarget::Zone, |core, id| {
        core.add_polygon_zone_labelled(id, &kind, &ring, Some(WHOLE_TERRAIN_ZONE_LABEL));
    })
}

#[cfg(test)]
#[path = "tests/zones_panel/tactical_draw_trigger.rs"]
mod tactical_draw_trigger_tests;
#[cfg(test)]
#[path = "tests/zones_panel/zone_geometry_and_schema.rs"]
mod zone_geometry_and_schema_tests;
