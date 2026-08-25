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

#[cfg(target_arch = "wasm32")]
use crate::core::ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use crate::editor::panels::outliner_tree::{ROW, ROW_ACTIVE};

/// T-582 — the Zones panel: draw controls, the authored-zone list, and the schema-driven
/// Attributes panel.
///
/// The whole panel is one function rather than a component so the native shell can stub it with the
/// same signature, exactly as [`placed_vehicles_panel`] does.
#[cfg(target_arch = "wasm32")]
pub(crate) fn zones_panel(doc_tick: RwSignal<u64>, selected: RwSignal<Option<String>>) -> AnyView {
    use crate::editor::state::operations as ops;

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
        ops::begin_zone_draw(&kind, shape, DrawTarget::Zone);
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
                    ops::zone_count()
                }}
            </span>
        </div>
        <p class="mt-0.5 text-label-sm normal-case text-outline">
            "Play areas and objectives. Circle: click the centre, then click the rim. Polygon: click each vertex, then Close."
        </p>

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
            let Some(d) = ops::zone_draft() else {
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
                                            ops::close_zone_polygon();
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
                                            ops::zone_draw_pop_vertex();
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
                                ops::cancel_zone_draw();
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
            let rows = ops::zone_rows();
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
            let Some(z) = ops::zone_rows().into_iter().find(|r| r.id == id) else {
                // Deleted underneath us (undo, or a reload that dropped it).
                return ().into_any();
            };
            zone_attributes(z, doc_tick, selected).into_any()
        }}
    }
    .into_any()
}

/// T-582 — the Attributes panel for one zone. Identity comes from the six declared `zone` keys;
/// the rules half is GENERATED from `$defs/zoneRules` by [`zone_rule_fields`].
#[cfg(target_arch = "wasm32")]
fn zone_attributes(
    z: crate::editor::state::operations::ZoneRow,
    doc_tick: RwSignal<u64>,
    selected: RwSignal<Option<String>>,
) -> AnyView {
    use crate::editor::state::operations as ops;

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
                    ops::set_zone_kind(&id_type, &event_target_value(&ev));
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
                        ops::set_zone_label(&id_label, Some(event_target_value(&ev)));
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
                                ops::set_zone_label(&id_clear, None);
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
                    ops::set_zone_faction(&id_faction, next);
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
                                ops::begin_zone_reshape(&a, ZoneShape::Circle, DrawTarget::Zone);
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
                                ops::begin_zone_reshape(&b, ZoneShape::Polygon, DrawTarget::Zone);
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
                    ops::delete_zone(&id_delete);
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
    use crate::editor::state::operations as ops;

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
                            ops::set_zone_rule(&zone_id, &k, Some(serde_json::Value::Bool(on)));
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
                            ops::set_zone_rule(&zone_id, &k, next);
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
                                ops::set_zone_rule(&zone_id, &k, next);
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
                            ops::set_zone_rule(&zone_id, &k, next);
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
pub(crate) const MISSION_SCHEMA: &str =
    include_str!("../../../../../../packages/tbd-schema/schema/mission.schema.json");

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

/// Which shape a zone draw is building.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneShape {
    Circle,
    Polygon,
}

/// T-079 (CONN-TRG-OWNER-001) — the owner-link line's SCREEN geometry: the projected endpoints
/// (trigger centre → owner) ready for one `<line>`. Pure so a native `cargo test` proves the
/// projection with no engine/`window`, the [`crate::editor::tools::ruler_tool::ProjectedLeg`] idiom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedOwnerLine {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

/// Project the owner-link line's two WORLD endpoints (trigger centre `a`, owner position `b`) to
/// screen space through a world→pixel projector (the live `OrthoCamera::project` on wasm; injected
/// here so this stays pure + native-testable, exactly like [`crate::editor::tools::ruler_tool::project_legs`]).
#[must_use]
pub fn project_owner_line<F>(a: (f64, f64), b: (f64, f64), project: F) -> ProjectedOwnerLine
where
    F: Fn(f64, f64) -> (f64, f64),
{
    let (x1, y1) = project(a.0, a.1);
    let (x2, y2) = project(b.0, b.1);
    ProjectedOwnerLine { x1, y1, x2, y2 }
}

/// T-079 — WHICH COLLECTION a draw commits into. The trigger AREA is a SECOND CONSUMER of the
/// shipped zone draw tool (the ticket's explicit constraint: "parameterize the draw flow by
/// target-kind, do not fork it"). Every stage of the draw — the arm, the multi-click accumulation,
/// the reshape, the commit — is identical for a zone and a trigger; the ONLY difference is which pair
/// of core mutators the final commit calls (`add_*_zone` / `set_zone_*` vs `add_*_trigger` /
/// `set_trigger_*`). So the difference is carried as this one-bit target on the in-flight draft
/// rather than as a forked `begin_trigger_draw` / `advance_trigger_draw` / … set that would duplicate
/// the whole geometry state machine and be free to drift from it.
///
/// It lives here beside [`ZoneShape`] — the pure, native-tested home — for the same reason `ZoneShape`
/// does: `editor_ops` (wasm-only) branches on it, and keeping it here is what lets a native
/// `cargo test -p website-frontend` prove any pure logic that reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawTarget {
    /// The play-area / objective zones the T-582 panel authors (`zonesById`).
    Zone,
    /// T-079 — the trigger areas the Triggers palette authors (`triggersById`).
    Trigger,
}

impl DrawTarget {
    /// A human word for the target, for the live draw hint ("Drawing a boundary circle" vs
    /// "Drawing a presence trigger circle"). Presentation only.
    #[must_use]
    pub fn noun(self) -> &'static str {
        match self {
            Self::Zone => "zone",
            Self::Trigger => "trigger",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        circle_from_clicks, humanize_key, humanize_token, polygon_flat, polygon_is_committable,
        radius_survives_compile, round_coord, zone_rule_fields, zone_types, ZoneRuleKind,
        MIN_AUTHORABLE_RADIUS_M, MISSION_SCHEMA, ZONE_GRID_M,
    };

    /// The quantisation this file mirrors is `flatten::round_coord`, which is private there. Pin it
    /// against that source so the mirror cannot drift silently — RED if `flatten.rs` changes its
    /// grid without this file following. Same guard `api/src/contract/validate.rs` puts on its own
    /// copy of the same line.
    #[test]
    fn zone_quantisation_mirrors_flatten() {
        let flatten =
            include_str!("../../../../../../crates/map-engine-core/src/mission/flatten.rs");
        let body = flatten
            .split("fn round_coord(v: f64) -> f64 {")
            .nth(1)
            .expect("flatten::round_coord must exist");
        let expr = body.split('}').next().expect("body").trim();
        assert_eq!(
            expr, "(v * 10.0).round() / 10.0",
            "flatten::round_coord changed — update eden_chrome::round_coord to match"
        );
        // And the mirror agrees on the value that produced the defect T-581 documented.
        assert_eq!(round_coord(0.04), 0.0);
        assert_eq!(round_coord(0.05), 0.1);
    }

    /// The published minimum is a CONSEQUENCE of the grid, not an independent constant. If the grid
    /// ever changes, this fails rather than letting the UI advertise a stale threshold.
    #[test]
    fn min_radius_is_the_grid_consequence() {
        assert!(
            radius_survives_compile(MIN_AUTHORABLE_RADIUS_M),
            "the advertised minimum must itself survive the compile"
        );
        assert!(
            !radius_survives_compile(MIN_AUTHORABLE_RADIUS_M - ZONE_GRID_M / 100.0),
            "anything below the advertised minimum must be refused"
        );
        // The exact radius a click-without-travel produced before this tool existed (T-581).
        assert!(!radius_survives_compile(0.04));
        assert!(!radius_survives_compile(0.0));
        assert!(!radius_survives_compile(-5.0));
        assert!(!radius_survives_compile(f64::NAN));
        assert!(radius_survives_compile(250.0));
    }

    /// A click without travel is the r=0.04 shape T-581 has to reject at save. The tool refuses to
    /// CREATE it, so the save-time check is a backstop rather than the first line of defence.
    #[test]
    fn circle_refuses_the_click_without_drag() {
        assert_eq!(circle_from_clicks(100.0, 200.0, 100.0, 200.0), None);
        // 0.04 m of travel — schema-valid authored, schema-INVALID once quantised.
        assert_eq!(circle_from_clicks(0.0, 0.0, 0.04, 0.0), None);
        // A real drag survives, and carries the document's (x, z, r) — not the viewport's y.
        let (x, z, r) = circle_from_clicks(10.0, 20.0, 13.0, 24.0).expect("a real drag commits");
        assert!((x - 10.0).abs() < f64::EPSILON && (z - 20.0).abs() < f64::EPSILON);
        assert!((r - 5.0).abs() < 1e-12, "3-4-5 triangle: r = 5, got {r}");
        // NaN from a singular-matrix unproject must never reach the document.
        assert_eq!(circle_from_clicks(f64::NAN, 0.0, 1.0, 1.0), None);
    }

    /// `$defs/polygon` is `minItems: 3` and the doc layer deliberately does not guard it — its own
    /// comment assigns the guard to this tool. Two vertices must not be committable.
    #[test]
    fn polygon_commits_only_at_three_vertices() {
        assert!(!polygon_is_committable(&[]));
        assert!(!polygon_is_committable(&[(0.0, 0.0)]));
        assert!(!polygon_is_committable(&[(0.0, 0.0), (10.0, 0.0)]));
        assert!(polygon_is_committable(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0)
        ]));
        assert!(!polygon_is_committable(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (f64::NAN, 10.0)
        ]));
        assert_eq!(
            polygon_flat(&[(1.0, 2.0), (3.0, 4.0), (5.0, 6.0)]),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            "the doc layer takes a FLAT ring"
        );
    }

    /// ═══ THE ANTI-SECOND-VOCABULARY TEST ═══
    ///
    /// The panel must render whatever `$defs/zoneRules` declares. This reads the vocabulary a SECOND
    /// way — straight out of the embedded JSON — and demands the two agree key-for-key. A panel
    /// built from a hand-typed list would pass every "the panel renders" assertion while silently
    /// omitting a key; this is the assertion that cannot.
    #[test]
    fn zone_rule_fields_cover_the_whole_vocabulary() {
        let schema: serde_json::Value =
            serde_json::from_str(MISSION_SCHEMA).expect("mission.schema.json parses");
        let props = schema["$defs"]["zoneRules"]["properties"]
            .as_object()
            .expect("$defs/zoneRules/properties");
        assert!(
            !props.is_empty(),
            "an empty vocabulary would make the panel vacuously correct"
        );
        assert_eq!(
            schema["$defs"]["zoneRules"]["additionalProperties"],
            serde_json::Value::Bool(false),
            "the vocabulary must stay CLOSED — an open one would make this whole approach unsound"
        );

        let fields = zone_rule_fields();
        let mut from_fields: Vec<&str> = fields.iter().map(|f| f.key.as_str()).collect();
        let mut from_schema: Vec<&str> = props.keys().map(String::as_str).collect();
        from_fields.sort_unstable();
        from_schema.sort_unstable();
        assert_eq!(
            from_fields, from_schema,
            "every declared rule key must reach the panel, and the panel must invent none"
        );

        // A control per declared kind, so a new key of an existing shape needs no code here.
        let kind = |k: &str| {
            fields
                .iter()
                .find(|f| f.key == k)
                .unwrap_or_else(|| panic!("{k} missing"))
                .kind
                .clone()
        };
        assert!(matches!(
            kind("contestable"),
            ZoneRuleKind::Bool { default: true }
        ));
        match kind("penalty") {
            ZoneRuleKind::Choice { options, default } => {
                assert_eq!(options, vec!["none", "warn", "kill"]);
                assert_eq!(default.as_deref(), Some("warn"));
            }
            other => panic!("penalty must be a Choice, got {other:?}"),
        }
        match kind("graceSeconds") {
            ZoneRuleKind::Number {
                default,
                minimum,
                maximum,
                integer,
                ..
            } => {
                assert_eq!(default, Some(30.0));
                assert_eq!(minimum, Some(0.0));
                // T-275 pinned this to TBD_ZoneRegistry.MAX_GRACE_SECONDS.
                assert_eq!(maximum, Some(3600.0));
                assert!(!integer);
            }
            other => panic!("graceSeconds must be a Number, got {other:?}"),
        }
        match kind("warnEverySeconds") {
            ZoneRuleKind::Number {
                exclusive_minimum, ..
            } => assert_eq!(
                exclusive_minimum,
                Some(0.0),
                "0 would mean 'warn every frame' — the reader requires > 0"
            ),
            other => panic!("warnEverySeconds must be a Number, got {other:?}"),
        }
        assert!(
            matches!(
                kind("targetCount"),
                ZoneRuleKind::Number { integer: true, .. }
            ),
            "targetCount is the one integer"
        );
        // `targetAlias` is declared as a `$ref` — a resolver that ignored it would drop the pattern.
        match kind("targetAlias") {
            ZoneRuleKind::Text { pattern, .. } => assert_eq!(
                pattern.as_deref(),
                Some("^(kit|comp|veh|preset|layer|prop|item):[a-z0-9_]+$"),
                "the $ref into $defs/alias must be resolved"
            ),
            other => panic!("targetAlias must resolve to Text, got {other:?}"),
        }
        // Every field carries the schema's prose, which names the mod call site.
        assert!(
            fields.iter().all(|f| !f.doc.is_empty()),
            "each control shows the schema's own description"
        );
    }

    /// The type picker is schema-driven for the same reason the rules are: `set_zone_type` writes
    /// whatever it is handed, and an invented seventh value saves 201 then 500s `/compiled`.
    #[test]
    fn zone_types_come_from_the_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(MISSION_SCHEMA).expect("mission.schema.json parses");
        let declared: Vec<String> = schema["$defs"]["zone"]["properties"]["type"]["enum"]
            .as_array()
            .expect("$defs/zone/properties/type/enum")
            .iter()
            .map(|v| v.as_str().expect("string").to_string())
            .collect();
        assert_eq!(zone_types(), declared);
        assert!(
            zone_types().contains(&"boundary".to_string()),
            "the play-area type must be offerable"
        );
    }

    /// ═══ THE TICKET, AS AN ASSERTION ═══
    ///
    /// T-582's measurement was "zero references to any zone mutator in the frontend — zones are
    /// authorable only from native test code". This is that measurement, inverted and kept: every
    /// one of T-211's eleven mutators must have a caller.
    ///
    /// Source inspection, following `vehicles_tab_places_instead_of_promising`'s precedent, because
    /// the thing under test is a wasm-only module (`editor_ops` is `#![cfg(target_arch = "wasm32")]`)
    /// that no native test can link. A behavioural test is impossible here; a source assertion is
    /// not, and the alternative is no assertion at all.
    #[test]
    fn every_t211_mutator_has_a_caller() {
        const OPS: &str = include_str!("../state/operations.rs");
        // The eleven, verbatim from `doc/store.rs`'s T-211 block.
        for m in [
            "add_circle_zone",
            "add_polygon_zone",
            "set_zone_circle",
            "set_zone_polygon",
            "set_zone_type",
            "set_zone_label",
            "set_zone_faction",
            "set_zone_rules",
            "remove_zone",
            "zones_json",
            "zone_count",
        ] {
            assert!(
                OPS.contains(&format!("core.{m}("))
                    || OPS.contains(&format!("MissionDocCore::{m}")),
                "T-211 mutator `{m}` still has no caller — that was the whole T-582 defect"
            );
        }
        // And the reshape pair specifically: they are the two that are easy to leave unwired,
        // because create-only looks finished.
        assert!(
            OPS.contains("begin_zone_reshape"),
            "reshape must be reachable, not just create"
        );
    }

    /// Labels are presentation only — the token itself is what reaches the document.
    #[test]
    fn labels_never_replace_tokens() {
        assert_eq!(
            humanize_token("objective_hold_until"),
            "Objective hold until"
        );
        assert_eq!(humanize_token("spawn"), "Spawn");
        assert_eq!(humanize_key("warnEverySeconds"), "Warn every seconds");
        assert_eq!(humanize_key("penalty"), "Penalty");
        // Round-trip safety: a label is never fed back as a key.
        for f in zone_rule_fields() {
            assert_ne!(
                humanize_key(&f.key),
                f.key,
                "label must differ from wire key"
            );
        }
    }

    // ── T-792 — Esc cancels an in-progress zone (and trigger) draw ──────────────────────────────
    // These are source pins because `editor_ops` / the `mission_editor` keydown closure are
    // wasm-only (`#![cfg(target_arch = "wasm32")]`), the same reason `every_t211_mutator_has_a_caller`
    // above is a source assertion — a behavioural test cannot link the module. All pins run on
    // `class_r_scrub::live_code`, which DELETES comments and BLANKS string literals, so a needle can
    // only be satisfied by real shipping code, never by the very comments that describe the fix (this
    // is the T-759-class discipline: never grep the raw file for the token you just added).

    /// The keydown Escape arm lives inside `MissionEditorPage`. Slice from there before scrubbing —
    /// the same anchor `t642_ruler_wiring::editor_live` uses to keep the keydown closure intact (a
    /// whole-file `live_code` prunes reachable-only-after-a-jump statements too aggressively for a
    /// deep-nested match arm). `live_code` then deletes comments + blanks string literals.
    fn editor_live_from_page() -> String {
        use crate::editor::arsenal::class_r_scrub::live_code;
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("../mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// The keyboard Esc arm must ROUTE an in-progress draw to `cancel_zone_draw`. Before T-792 the arm
    /// only called `cancel_pending`, which a zone draw deliberately survives — so F-31 was: arm Circle,
    /// click the centre, press Esc, and the draft stayed armed (the panel kept prompting for the rim,
    /// and the next click completed the circle). The pin proves the real call is present in the live
    /// keydown closure, not in a comment.
    #[test]
    fn t792_escape_arm_cancels_the_zone_draw() {
        let ed = editor_live_from_page();
        assert!(
            ed.contains("editor_ops::cancel_zone_draw()"),
            "T-792: the editor keydown must call editor_ops::cancel_zone_draw() (the ONE cancel a \
             multi-click draw honours) — cancel_pending alone leaves the draft armed"
        );
        // It rides the SAME shared keydown Escape seam as the place/connect/measure cancels — not a
        // new window listener (the T-726 pile-up must not grow). Proven by co-location with the
        // keydown dispatch and the sibling place cancel it sits beside.
        assert!(
            ed.contains("code().as_str()")
                && ed.contains("editor_ops::has_pending()")
                && ed.contains("editor_ops::cancel_zone_draw()"),
            "T-792: the zone-draw cancel must live in the ONE shared keydown Escape arm, beside the \
             armed-place (has_pending) cancel — no second window keydown listener"
        );
    }

    /// One-Esc-one-layer (T-813/T-814): when the draw cancel ACTS it must feed the arm's "handled"
    /// result, so the press is consumed (prevent_default) and no lower Esc layer — a dialog, a menu,
    /// the tab — also closes on the SAME keypress. The binding `zone_draw_acted` must therefore flow
    /// into the trailing `||` chain that the arm returns.
    #[test]
    fn t792_zone_cancel_consumes_the_press() {
        let ed = editor_live_from_page();
        assert!(
            ed.contains("let zone_draw_acted =") && ed.contains("|| zone_draw_acted"),
            "T-792: the draw-cancel result must join the Escape arm's handled `||` chain, so a real \
             cancel consumes the press (one Esc, one layer)"
        );
    }

    /// The hint-clear half. `cancel_zone_draw` must, on a real clear, bump the dock tick — the Zones
    /// AND Triggers panels re-read `zone_draft()` under `doc_tick` and their "click the rim"/vertex
    /// hint (plus the Cancel/Close controls) vanish once the draft is `None`. Mirrors T-791's
    /// `cancel_pending` bump. Pinned on the function's OWN body via `only_body` (unique name), so a
    /// bump elsewhere in the file cannot satisfy it.
    #[test]
    fn t792_cancel_zone_draw_bumps_the_dock_tick() {
        use crate::editor::arsenal::class_r_scrub::{live_code, only_body};
        let ops = live_code(include_str!("../state/operations.rs"));
        let body = only_body(&ops, "pub fn cancel_zone_draw() -> bool");
        assert!(
            body.contains("bump_doc_tick()"),
            "T-792: cancel_zone_draw must bump the dock tick on a real clear, so the rim/vertex hint \
             (gated on zone_draft() under doc_tick) disappears — the panel-Cancel effect, on Esc"
        );
        // Collection-agnostic: it clears on `Pending::Zone(_)`, so the SHARED draft cancels a TRIGGER
        // draw exactly as it cancels a zone draw (trigger arms via begin_zone_draw(.., Trigger) into
        // the identical Pending::Zone). This is what lets ONE Esc call cover both consumers.
        assert!(
            body.contains("Some(Pending::Zone(_))"),
            "T-792: cancel_zone_draw must clear on Pending::Zone(_) regardless of collection, so a \
             trigger draw (the second consumer) is cancelled by the same call"
        );
    }
}
