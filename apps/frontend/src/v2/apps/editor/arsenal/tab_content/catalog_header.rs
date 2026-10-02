//! Compatibility, capacity, and weight badges above the Arsenal catalog.

use super::*;

/// Renders the live catalog badges.
pub(super) fn catalog_header(
    state: ArsenalTabState,
    items: StoredValue<Vec<RegistryItem>>,
) -> impl IntoView {
    let ArsenalTabState {
        compat,
        active_key,
        picks,
        ..
    } = state;
    view! {
                        // Top badges: compat status (left) + live weight (right).
                        <div class="flex items-center justify-between">
                            {move || {
                                let s = compat.get().status;
                                let (cls, label) = match s {
                                    rules::CompatStatus::Ready => (
                                        "rounded border border-success/40 bg-success/10 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-success",
                                        "Compat active",
                                    ),
                                    rules::CompatStatus::Loading => (
                                        "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-on-surface-variant",
                                        "Compat loading…",
                                    ),
                                    rules::CompatStatus::Unavailable => (
                                        "rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm uppercase tracking-wider text-outline",
                                        "Compat unavailable",
                                    ),
                                };
                                view! { <span class=cls data-compat-badge>{label}</span> }
                            }}
                            <div class="flex items-center gap-3">
                                // per-container capacity readout (registry-only:
                                // max kg + grid W×H; absent values simply don't render).
                                {move || {
                                    let key = active_key.get();
                                    if !rules::CAPACITY_KEYS.contains(&key.as_str()) {
                                        return ().into_any();
                                    }
                                    let rn = picks.with(|m| m.get(key.as_str()).cloned()).filter(|v| !v.is_empty());
                                    let Some(rn) = rn else { return ().into_any() };
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let Some(it) = idx.get(rn.as_str()) else { return ().into_any() };
                                    let mut parts: Vec<String> = Vec::new();
                                    if let Some(kg) = it.max_weight_kg {
                                        parts.push(format!("max {kg} kg"));
                                    }
                                    if let (Some(w), Some(h)) = (it.cargo_grid_w, it.cargo_grid_h) {
                                        parts.push(format!("{w}\u{00d7}{h} grid"));
                                    }
                                    if parts.is_empty() {
                                        return ().into_any();
                                    }
                                    view! {
                                        <span
                                            data-capacity-badge
                                            class="rounded border border-outline-variant/40 bg-surface-variant/30 px-2 py-0.5 font-mono text-label-sm tabular-nums normal-case text-on-surface-variant"
                                        >
                                            {parts.join(" · ")}
                                        </span>
                                    }.into_any()
                                }}
                                {move || {
                                    let its = items.get_value();
                                    let idx = index_by_name(&its);
                                    let map = picks.get();
                                    let mut w = loadout_weight(&map, &idx);
                                    // attachments hang off a weapon, not off a row, so
                                    // `loadout_weight` (which walks LOADOUT_ROWS) cannot see them.
                                    // A suppressor is 0.68 kg of real carried mass; omitting it
                                    // would make an "honest weight" readout quietly dishonest.
                                    // Scoped to weapons that are actually picked — that is exactly
                                    // the set `picks_to_loadout` persists attachments for.
                                    for &(key, _, _) in rules::WEAPON_SLOTS {
                                        if map.get(key).is_none_or(String::is_empty) {
                                            continue;
                                        }
                                        for rn in attachments_of(&map, key) {
                                            w.item_count += 1;
                                            match idx.get(rn.as_str()).and_then(|it| it.weight_kg) {
                                                Some(kg) => w.known_kg += kg,
                                                None => w.unknown_count += 1,
                                            }
                                        }
                                    }
                                    let w = format_loadout_weight(&w);
                                    view! {
                                        <p class="font-mono text-label-sm tabular-nums normal-case text-on-surface-variant">{w}</p>
                                    }
                                }}
                            </div>
                        </div>
    }
}
