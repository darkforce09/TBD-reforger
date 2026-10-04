//! Tree panel for the ORBAT manager.

#[cfg(target_arch = "wasm32")]
use super::*;

/// Renders the ORBAT tree with virtualized rows.
#[cfg(target_arch = "wasm32")]
pub(super) fn tree_panel(
    squad_nodes: Vec<OutlinerNode>,
    drag_nodes: RwSignal<Vec<OutlinerNode>>,
    signals: OrbatTreeSignals,
    lookups: OrbatTreeLookups,
) -> AnyView {
    let collapsed = signals.collapsed;
    let flat = StoredValue::new(Vec::<FlatRow>::new());
    let rev = RwSignal::new(0u64);
    let nodes_sig = RwSignal::new(squad_nodes);
    Effect::new(move |_| {
        let f = collapsed.with(|c| flatten_visible(&nodes_sig.get(), c));
        flat.set_value(f);
        rev.update(|r| *r = r.wrapping_add(1));
    });
    {
        let f = collapsed.with_untracked(|c| flatten_visible(&nodes_sig.get_untracked(), c));
        flat.set_value(f);
    }
    let scroll_top = RwSignal::new(0.0_f64);
    let lookups = StoredValue::new(lookups);

    (move || {
        rev.track();
        let st = scroll_top.get();
        flat.with_value(|f| {
            let total = f.len();
            if total == 0 {
                set_orbat_stats(0, 0);
                return view! {
                    <p class="px-2 py-6 text-center text-label-sm text-outline">{ORBAT_MANAGER_EMPTY}</p>
                }
                .into_any();
            }
            let render_slice = |rows: &[FlatRow]| -> AnyView {
                view! {
                    <div class="space-y-1">
                        {rows
                            .iter()
                            .map(|r| {
                                stitch_row(r, drag_nodes, signals, lookups.get_value())
                            })
                            .collect::<Vec<_>>()}
                    </div>
                }
                .into_any()
            };
            if total <= VIRTUAL_SLOT_THRESHOLD {
                set_orbat_stats(total, total);
                return render_slice(f);
            }
            let per_screen = (CONTAINER_H / ROW_H).ceil() as usize;
            let start = ((st / ROW_H).floor() as usize).saturating_sub(OVERSCAN);
            let end = (start + per_screen + 2 * OVERSCAN).min(total);
            set_orbat_stats(total, end - start);
            let top = start as f64 * ROW_H;
            let bottom = (total - end) as f64 * ROW_H;
            let slice = render_slice(&f[start..end]);
            view! {
                <div
                    class="overflow-y-auto"
                    style=format!("height:{CONTAINER_H}px")
                    on:scroll=move |ev| {
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev
                                .target()
                                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                            {
                                scroll_top.set(el.scroll_top() as f64);
                            }
                        }
                    }
                >
                    <div style=format!("height:{top}px")></div>
                    {slice}
                    <div style=format!("height:{bottom}px")></div>
                </div>
            }
            .into_any()
        })
    })
    .into_any()
}
