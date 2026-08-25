//! The Arsenal view panels — the cargo editor, the doll host (3D with SVG fallback), the
//! attachments / compat panels and the SVG paper-doll. Split out of `arsenal/mod.rs` at
//! T-934.8 with bodies unchanged; [`super::ArsenalTab`] is the only caller.

use std::collections::HashMap;

use leptos::prelude::*;

use crate::core::dto::RegistryItem;
use crate::editor::arsenal::arsenal_rules::{
    self as rules, index_by_name, row_options, CompatFeed,
};

use super::loadout::{attachments_key, attachments_of, pack_attachments, ATTACHMENT_EDGE};
use super::{region_title, MaterialCheck};

/// Registry kinds offered by the cargo "add" picker (worn/held gear stays on the wear
/// and weapon rows — cargo is what goes *inside* containers).
const CARGO_ADD_KINDS: &[&str] = &[
    "magazine",
    "ammo",
    "gear_item",
    "gear_throwable",
    "gear_explosive",
];

/// T-068.15.2 — the per-container cargo editor: rows (name × qty, stepper, remove),
/// an add picker, and the budget vs the garment's registry capacity.
///
/// T-240 — the container→worn-garment alias used to live here as a second copy of the rule
/// `arsenal_rules` already documents; it is now [`rules::cargo_garment`] alone, so the readout
/// and the block can never disagree about which garment backs a container.
pub(super) fn cargo_panel(
    cargo: RwSignal<Vec<rules::CargoRow>>,
    picks: RwSignal<HashMap<String, String>>,
    items: StoredValue<Vec<RegistryItem>>,
    names: StoredValue<HashMap<String, String>>,
    on_change: impl Fn(&[RegistryItem]) + Copy + 'static,
) -> AnyView {
    let its = items.get_value();
    let idx = index_by_name(&its);
    let rows_now = cargo.get();
    let picks_now = picks.get();

    // Add-picker options: eligible kinds, concrete (non-abstract, non-variant), name-sorted.
    let mut addable: Vec<(String, String)> = its
        .iter()
        .filter(|it| CARGO_ADD_KINDS.contains(&it.kind.as_str()))
        .filter(|it| !it.r#abstract.unwrap_or(false) && it.variant_of.is_none())
        .map(|it| (it.resource_name.clone(), it.display_name.clone()))
        .collect();
    addable.sort_by(|a, b| a.1.cmp(&b.1));
    let addable = StoredValue::new(addable);

    let groups = rules::CARGO_CONTAINERS
        .iter()
        .map(|container| {
            let container: &'static str = container;
            let garment_rn = rules::cargo_garment(&picks_now, container).map(|(_, rn)| rn);
            let rows: Vec<(usize, rules::CargoRow)> = rows_now
                .iter()
                .enumerate()
                .filter(|(_, r)| r.container == container)
                .map(|(i, r)| (i, r.clone()))
                .collect();
            if garment_rn.is_none() && rows.is_empty() {
                return ().into_any();
            }
            let garment_item = garment_rn.and_then(|rn| idx.get(rn).copied());
            let garment_label = garment_rn
                .map(|rn| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.to_string())))
                .unwrap_or_else(|| "no garment worn".to_string());
            let only_rows: Vec<rules::CargoRow> = rows.iter().map(|(_, r)| r.clone()).collect();
            let budget = rules::cargo_budget(&idx, garment_item, &only_rows);
            let budget_line = match (budget.max_weight, budget.max_volume) {
                (None, None) if only_rows.is_empty() => None,
                _ => {
                    let kg = match budget.max_weight {
                        Some(m) => format!("{:.1} / {m} kg", budget.weight),
                        None => format!("{:.1} kg", budget.weight),
                    };
                    let vol = match budget.max_volume {
                        Some(m) => format!("{:.0} / {m} cm³", budget.volume),
                        None => format!("{:.0} cm³", budget.volume),
                    };
                    Some((format!("{kg} · {vol}"), budget.over()))
                }
            };
            view! {
                <div class="mb-2 last:mb-0" data-cargo-container=container>
                    <div class="flex items-center justify-between px-1">
                        <span class="text-label-sm font-semibold uppercase tracking-wider text-on-surface">
                            {container} " — " <span class="normal-case font-normal text-on-surface-variant">{garment_label}</span>
                        </span>
                        {budget_line.map(|(text, over)| {
                            let cls = if over {
                                "font-mono text-label-sm tabular-nums normal-case text-error-alert"
                            } else {
                                "font-mono text-label-sm tabular-nums normal-case text-outline"
                            };
                            view! { <span class=cls data-cargo-budget=container>{text}</span> }
                        })}
                    </div>
                    {rows.into_iter().map(|(i, r)| {
                        let label = names.with_value(|n| n.get(&r.item).cloned().unwrap_or_else(|| r.item.clone()));
                        let qty = r.qty;
                        view! {
                            <div class="flex items-center justify-between gap-2 rounded px-2 py-0.5 hover:bg-white/5">
                                <span class="truncate text-label-sm normal-case text-on-surface-variant">{label}</span>
                                <span class="flex shrink-0 items-center gap-1">
                                    <button type="button" aria-label="Fewer" class="rounded px-1 font-mono text-label-sm text-outline hover:bg-white/10 hover:text-on-surface"
                                        on:click=move |_| {
                                            cargo.update(|c| { if let Some(r) = c.get_mut(i) { r.qty = (r.qty - 1).max(1); } });
                                            on_change(&items.get_value());
                                        }
                                    >"−"</button>
                                    <span class="min-w-[2ch] text-center font-mono text-label-sm tabular-nums text-on-surface">{qty}</span>
                                    <button type="button" aria-label="More" class="rounded px-1 font-mono text-label-sm text-outline hover:bg-white/10 hover:text-on-surface"
                                        on:click=move |_| {
                                            cargo.update(|c| { if let Some(r) = c.get_mut(i) { r.qty += 1; } });
                                            on_change(&items.get_value());
                                        }
                                    >"+"</button>
                                    <button type="button" aria-label="Remove" class="rounded px-1 font-mono text-label-sm text-outline hover:bg-white/10 hover:text-error"
                                        on:click=move |_| {
                                            cargo.update(|c| { c.remove(i); });
                                            on_change(&items.get_value());
                                        }
                                    >"✕"</button>
                                </span>
                            </div>
                        }
                    }).collect_view()}
                    <select
                        class="mt-0.5 w-full rounded-md border border-outline-variant/40 bg-surface-container-lowest/60 px-2 py-1 text-label-sm text-on-surface-variant outline-none focus:border-primary/60"
                        aria-label=format!("Add cargo to {container}")
                        prop:value=""
                        on:change=move |ev| {
                            let rn = event_target_value(&ev);
                            if rn.is_empty() { return; }
                            cargo.update(|c| {
                                if let Some(row) = c.iter_mut().find(|r| r.container == container && r.item == rn) {
                                    row.qty += 1;
                                } else {
                                    c.push(rules::CargoRow { container: container.to_string(), item: rn.clone(), qty: 1 });
                                }
                            });
                            on_change(&items.get_value());
                        }
                    >
                        <option value="" selected>"+ Add item…"</option>
                        {addable.with_value(|a| a.iter().map(|(rn, label)| {
                            view! { <option value=rn.clone()>{label.clone()}</option> }
                        }).collect_view())}
                    </select>
                </div>
            }
            .into_any()
        })
        .collect_view();

    view! {
        <p class="px-1 pb-1 font-mono text-[10px] tracking-widest text-outline uppercase">"Cargo"</p>
        {groups}
    }
    .into_any()
}

/// The center doll: `ArsenalDoll` (wgpu) with the SVG `paper_doll` as the create-error fallback
/// (T-154 contract). Native shell: always the SVG (no GPU).
pub(super) fn doll_view(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
    names: StoredValue<HashMap<String, String>>,
    unavailable: RwSignal<bool>,
) -> AnyView {
    #[cfg(target_arch = "wasm32")]
    {
        if !unavailable.get() {
            return view! {
                <crate::editor::arsenal::arsenal_doll::ArsenalDoll
                    picks
                    active_key
                    names
                    unavailable
                    on_select=Callback::new(move |key: String| active_key.set(key))
                />
            }
            .into_any();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (names, unavailable);
    paper_doll(picks, active_key).into_any()
}

/// T-197 — the **ATTACHMENTS** block of the compat panel: the `attachment_on_weapon` set the active
/// weapon accepts, rendered as toggles. This is the Arsenal's one multi-select surface, because it
/// is the one slot a weapon holds several of at once.
///
/// Returns `None` when the active region is not a weapon, when no weapon is picked, or when the
/// graph offers nothing **and** nothing is picked — so a family with no edges (vanilla
/// launcher/handgun/throwable all have zero) adds no empty section to the panel.
fn attachments_panel(
    active: &str,
    map: &HashMap<String, String>,
    feed: &CompatFeed,
    names: StoredValue<HashMap<String, String>>,
    items: StoredValue<Vec<RegistryItem>>,
    pick_item: impl Fn(String, String) + Copy + 'static,
) -> Option<AnyView> {
    let &(weapon_key, _, _) = rules::WEAPON_SLOTS.iter().find(|(k, _, _)| *k == active)?;
    let host = map.get(weapon_key).filter(|s| !s.is_empty())?.clone();
    let its = items.get_value();
    let idx = index_by_name(&its);
    // Synthesised here rather than added as a 15th `LOADOUT_ROWS` entry: the set must stay out of
    // the single-value row machinery (weight, validation, the doll rail all key off that table),
    // while still reusing the row RULES verbatim — graph-fed, abstract/variant filtered,
    // display-name sorted. `depends_on` is the weapon key, so the graph lookup is host-agnostic.
    let row = rules::LoadoutRow {
        key: "attachments",
        label: "Attachments",
        source: rules::RowSource::Edge {
            edge: ATTACHMENT_EDGE,
            depends_on: weapon_key,
        },
    };
    let mut opts = row_options(&row, "", map, &its, &idx, feed.ready_graph());
    let picked = attachments_of(map, weapon_key);
    let display =
        |rn: &str| names.with_value(|n| n.get(rn).cloned().unwrap_or_else(|| rn.to_string()));
    // A pick the option list dropped stays VISIBLE — deselecting it is the only way to remove it.
    // It is flagged only when the graph actually REJECTS it: an `abstract`/variant prefab the
    // filter hid is still a compatible pick, and an outage is not evidence of anything at all.
    for rn in &picked {
        if opts.iter().any(|o| &o.value == rn) {
            continue;
        }
        let ok = feed
            .ready_graph()
            .is_none_or(|g| g.accepts(&host, rn, ATTACHMENT_EDGE));
        opts.push(rules::RowOption {
            value: rn.clone(),
            label: if ok {
                display(rn)
            } else {
                format!("{} — incompatible", display(rn))
            },
            incompatible: !ok,
        });
    }
    if opts.is_empty() {
        return None;
    }
    let rows = opts
        .into_iter()
        .map(|o| {
            let selected = picked.contains(&o.value);
            let cls = match (selected, o.incompatible) {
                (true, true) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm bg-error/10 text-error",
                (true, false) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm bg-primary/15 text-primary",
                (false, true) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm text-error transition-colors hover:bg-white/10",
                (false, false) => "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface",
            };
            // The toggled set is computed HERE, not in the handler: `pick_item` is the one
            // persist path (`insert`-or-`remove` + one undo step), so a toggle is just a normal
            // pick whose value happens to be the packed set.
            let mut next = picked.clone();
            match next.iter().position(|p| *p == o.value) {
                Some(at) => {
                    next.remove(at);
                }
                None => next.push(o.value.clone()),
            }
            let packed = pack_attachments(&next);
            let akey = attachments_key(weapon_key);
            // `data-value` keeps the panel's uniform click contract (the smoke harness in
            // `tbd-tools` sweeps `[data-value]`); `data-attachment` additionally marks this as a
            // TOGGLE, since a second click removes rather than replaces. `resource_name` is unique
            // per registry row, so the extra nodes cannot shadow a weapon/optic lookup.
            let data_value = o.value.clone();
            let data_attachment = o.value.clone();
            view! {
                <button
                    type="button"
                    data-value=data_value
                    data-attachment=data_attachment
                    aria-pressed=selected.to_string()
                    class=cls
                    on:click=move |_| pick_item(akey.clone(), packed.clone())
                >
                    <span class="truncate normal-case">{o.label}</span>
                    {selected.then(|| view! { <MaterialCheck /> })}
                </button>
            }
        })
        .collect_view();
    Some(
        view! {
            <p class="mt-3 font-mono text-[10px] tracking-widest text-outline uppercase">
                "Attachments"
            </p>
            {rows}
        }
        .into_any(),
    )
}

/// The right compat panel: the active pick's display name, each edge slot that depends on the
/// active region (screen 04: OPTIC "Nothing compatible." / MAGAZINE list), and — for a weapon
/// region — the T-197 multi-select attachment set. Rows click-pick.
pub(super) fn compat_panel(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
    compat: RwSignal<CompatFeed>,
    names: StoredValue<HashMap<String, String>>,
    items: StoredValue<Vec<RegistryItem>>,
    pick_item: impl Fn(String, String) + Copy + 'static,
) -> AnyView {
    let key = active_key.get();
    let map = picks.get();
    let host = map.get(key.as_str()).cloned().unwrap_or_default();
    let head = if host.is_empty() {
        format!("{} — empty", region_title(&key))
    } else {
        names.with_value(|n| n.get(&host).cloned().unwrap_or_else(|| host.clone()))
    };
    let dependents: Vec<&'static rules::LoadoutRow> = rules::LOADOUT_ROWS
        .iter()
        .filter(
            |r| matches!(r.source, rules::RowSource::Edge { depends_on, .. } if depends_on == key),
        )
        .collect();
    let feed = compat.get();
    let attachments = attachments_panel(&key, &map, &feed, names, items, pick_item);
    let body = if dependents.is_empty() {
        // "No dependent slots." is a claim about the whole panel, so it must not survive an
        // attachment set — a modded launcher has no edge ROWS but can still have attachments.
        if attachments.is_none() {
            view! {
                <p class="mt-2 text-label-sm normal-case text-outline">"No dependent slots."</p>
            }
            .into_any()
        } else {
            ().into_any()
        }
    } else {
        dependents
            .into_iter()
            .map(|row| {
                let rules::RowSource::Edge { edge, .. } = row.source else {
                    unreachable!()
                };
                let section = view! {
                    <p class="mt-3 font-mono text-[10px] tracking-widest text-outline uppercase">
                        {row.label}
                    </p>
                };
                let content = if host.is_empty() {
                    view! {
                        <p class="text-label-sm normal-case text-outline">
                            {format!("Pick a {} first.", region_title(&key).to_lowercase())}
                        </p>
                    }
                    .into_any()
                } else if let Some(g) = feed.ready_graph() {
                    let options = g.items_for(&host, edge);
                    if options.is_empty() {
                        view! {
                            <p class="text-label-sm normal-case text-outline">"Nothing compatible."</p>
                        }
                        .into_any()
                    } else {
                        let current = map.get(row.key).cloned().unwrap_or_default();
                        let row_key = row.key;
                        options
                            .into_iter()
                            .map(|rn| {
                                let label = names
                                    .with_value(|n| n.get(&rn).cloned().unwrap_or_else(|| rn.clone()));
                                let is_current = rn == current;
                                let cls = if is_current {
                                    "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm bg-primary/15 text-primary"
                                } else {
                                    "flex w-full items-center justify-between rounded px-1.5 py-1 text-left text-label-sm text-on-surface-variant transition-colors hover:bg-white/10 hover:text-on-surface"
                                };
                                let data_value = rn.clone();
                                view! {
                                    <button
                                        type="button"
                                        data-value=data_value
                                        class=cls
                                        on:click=move |_| pick_item(row_key.to_string(), rn.clone())
                                    >
                                        <span class="truncate normal-case">{label}</span>
                                        {is_current.then(|| view! { <MaterialCheck /> })}
                                    </button>
                                }
                            })
                            .collect_view()
                            .into_any()
                    }
                } else {
                    view! {
                        <p class="text-label-sm normal-case text-outline">"Compat unavailable."</p>
                    }
                    .into_any()
                };
                view! {
                    {section}
                    {content}
                }
                .into_any()
            })
            .collect::<Vec<_>>()
            .collect_view()
            .into_any()
    };
    view! {
        <p class="text-label-md font-semibold normal-case text-on-surface">{head}</p>
        {body}
        {attachments}
    }
    .into_any()
}

/// The Mode-D 2D **SVG paper-doll** (SoldierSilhouette.tsx port). Keyboard-accessible
/// `<g role="button">` hotspots per `DOLL_REGIONS` (optic/magazine nest on the rifle group); three
/// visual states — empty (dashed), equipped (`primary/15`), active (`primary/25`). A hotspot click
/// sets `active_key` (two-way synced with the row list); it never mutates the loadout itself.
fn paper_doll(
    picks: RwSignal<HashMap<String, String>>,
    active_key: RwSignal<String>,
) -> impl IntoView {
    // (key, label, svg path/rect element) — geometry adapted from the React ref (viewBox 360×640).
    // Each region is one `<g>` hotspot; `shape` is its clickable silhouette.
    struct Region {
        key: &'static str,
        shape: &'static str, // an SVG element string (rect/path) sans fill/stroke.
    }
    // Ordered back-to-front (paint order): backpack, body, wear, then the rifle group last.
    const REGIONS: &[Region] = &[
        Region {
            key: "backpack",
            shape: r#"<rect x="84" y="165" width="44" height="120" rx="12"/>"#,
        },
        Region {
            key: "launcher",
            shape: r#"<rect x="246" y="72" width="18" height="120" rx="6" transform="rotate(28 255 132)"/>"#,
        },
        Region {
            key: "jacket",
            shape: r#"<rect x="140" y="132" width="80" height="150" rx="10"/>"#,
        },
        Region {
            key: "pants",
            shape: r#"<rect x="146" y="282" width="68" height="196" rx="8"/>"#,
        },
        Region {
            key: "boots",
            shape: r#"<rect x="146" y="484" width="68" height="40" rx="6"/>"#,
        },
        Region {
            key: "handwear",
            shape: r#"<path d="M108 288 h22 v22 h-22 z M230 288 h22 v22 h-22 z"/>"#,
        },
        Region {
            key: "vest",
            shape: r#"<rect x="150" y="150" width="60" height="64" rx="6"/>"#,
        },
        Region {
            key: "armoredVest",
            shape: r#"<rect x="142" y="142" width="76" height="110" rx="8"/>"#,
        },
        Region {
            key: "headCover",
            shape: r#"<circle cx="180" cy="92" r="26"/>"#,
        },
        Region {
            key: "throwable",
            shape: r#"<rect x="112" y="326" width="26" height="30" rx="4"/>"#,
        },
        Region {
            key: "handgun",
            shape: r#"<rect x="222" y="312" width="26" height="34" rx="4"/>"#,
        },
    ];
    // The rifle group (primary + nested optic/magazine), drawn front-most.
    const RIFLE: &[Region] = &[
        Region {
            key: "primary",
            shape: r#"<rect x="96" y="322" width="150" height="14" rx="3"/>"#,
        },
        Region {
            key: "optic",
            shape: r#"<rect x="150" y="306" width="26" height="12" rx="3"/>"#,
        },
        Region {
            key: "magazine",
            shape: r#"<path d="M168 336 q6 26 18 30 l6 -4 q-10 -6 -12 -28 z"/>"#,
        },
    ];

    let hotspot = move |r: &'static Region| {
        let key = r.key;
        let cls = move || {
            let equipped = picks.with(|m| m.get(key).map(|v| !v.is_empty()).unwrap_or(false));
            let active = active_key.get() == key;
            let base = "cursor-pointer transition-colors";
            if active {
                format!("{base} fill-primary/25 stroke-primary [stroke-width:2.5]")
            } else if equipped {
                format!("{base} fill-primary/15 stroke-primary/60 [stroke-width:1.5]")
            } else {
                format!("{base} fill-on-surface/5 stroke-outline/50 [stroke-width:1.2] [stroke-dasharray:4_3]")
            }
        };
        let label = rules::row(key).map(|r| r.label).unwrap_or(key);
        // inject the shape verbatim; add the reactive class on the group.
        view! {
            <g
                role="button"
                tabindex="0"
                aria-label=label
                aria-pressed=move || (active_key.get() == key).to_string()
                class=cls
                on:click=move |ev: leptos::ev::MouseEvent| { ev.stop_propagation(); active_key.set(key.to_string()); }
                inner_html=r.shape
            ></g>
        }
    };

    view! {
        <svg viewBox="0 0 360 640" class="mx-auto h-[52vh] w-full" role="group" aria-label="Loadout paper-doll">
            // decorative head/neck (non-clickable)
            <circle cx="180" cy="92" r="22" class="fill-on-surface/10"></circle>
            <rect x="170" y="112" width="20" height="18" class="fill-on-surface/10"></rect>
            {REGIONS.iter().map(hotspot).collect_view()}
            {RIFLE.iter().map(hotspot).collect_view()}
        </svg>
    }
}

#[cfg(test)]
mod tests {
    use super::super::class_r_scrub::{live_code, only_body as fn_body};

    /// The live production surface this pin examines spans two files since T-934.8 —
    /// `ArsenalTab` (mod.rs) wires the commit, `cargo_panel` (this file) owns the mutations.
    /// Each file is scrubbed separately (`cut_test_module` truncates a haystack at its first
    /// cfg-test attribute, so a scrubbed concatenation would end at the first test tail) and
    /// the live halves concatenate into one haystack.
    fn live_production_src() -> String {
        [include_str!("mod.rs"), include_str!("panels.rs")]
            .into_iter()
            .map(live_code)
            .collect()
    }

    /// T-503 Class-R: every cargo mutation in the panel must commit through `on_change`, and the
    /// commit must reach `editor_ops::set_loadout`. Staging — a mutation that updates the local
    /// signal and waits for a Save button — goes red here.
    ///
    /// RED (staging): delete the `on_change(&items.get_value());` after the qty `+` handler in
    /// `cargo_panel` → "every cargo mutation must commit: 4 `cargo.update(` vs 3 `on_change(`".
    /// RED (decoy, `if true == false`): move `crate::editor::state::operations::set_loadout(…)` inside
    /// `if true == false { … }` → "ArsenalTab must reach editor_ops::set_loadout".
    /// RED (decoy, `#[cfg(any())]`): park the call in an `#[cfg(any())] fn dead_persist() { … }`
    /// → same failure.
    /// RED (decoy, `loop { break; … }`): park the call after a bare `break;` → same failure.
    #[test]
    fn cargo_mutations_commit_without_a_staging_gate() {
        let live = live_production_src();
        let panel = fn_body(&live, "fn cargo_panel(");
        let mutations = panel.matches("cargo.update(").count();
        let commits = panel.matches("on_change(").count();
        assert!(
            mutations >= 4,
            "cargo_panel should still own the qty -/+, remove and add mutations; found {mutations}"
        );
        assert!(
            commits >= mutations,
            "every cargo mutation must commit: {mutations} `cargo.update(` vs {commits} `on_change(`"
        );

        let tab = fn_body(&live, "pub fn ArsenalTab(");
        assert!(
            tab.contains("crate::editor::state::operations::set_loadout("),
            "ArsenalTab must reach editor_ops::set_loadout on a live path"
        );
        assert!(
            tab.contains("persist(&picks.get_untracked(), items)"),
            "persist_cargo must forward to the same commit the pick path uses"
        );
    }
}
