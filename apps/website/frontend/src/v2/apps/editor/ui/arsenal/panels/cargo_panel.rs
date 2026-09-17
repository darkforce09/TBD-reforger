//! Cargo rows, add picker, and catalogued capacity readout.

use super::*;

/// Registry kinds offered by the cargo "add" picker (worn/held gear stays on the wear
/// and weapon rows — cargo is what goes *inside* containers).
const CARGO_ADD_KINDS: &[&str] = &[
    "magazine",
    "ammo",
    "gear_item",
    "gear_throwable",
    "gear_explosive",
];

/// Edits cargo rows and shows their budget against the garment's catalogued capacity.
/// [`rules::cargo_garment`] chooses the garment for both the readout and refusal.
pub(crate) fn cargo_panel(
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
