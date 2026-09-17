//! Tree panel for the ORBAT manager.

use super::*;

/// Renders the ORBAT tree with virtualized rows.
pub(super) fn tree_panel(
    squad_nodes: Vec<OutlinerNode>,
    drag_nodes: RwSignal<Vec<OutlinerNode>>,
    detail_by_id: HashMap<String, SlotDetail>,
    vehicle_by_squad: HashMap<String, usize>,
    selected: RwSignal<Vec<String>>,
    collapsed: RwSignal<HashSet<String>>,
    rename_squad: RwSignal<Option<String>>,
    rename_draft: RwSignal<String>,
    add_vehicle_squad: RwSignal<Option<String>>,
    vehicle_options: Vec<(String, String)>,
) -> AnyView {
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
    let detail_by_id = StoredValue::new(detail_by_id);
    let vehicle_by_squad = StoredValue::new(vehicle_by_squad);
    let vehicle_options = StoredValue::new(vehicle_options);

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
                                stitch_row(
                                    r,
                                    drag_nodes,
                                    selected,
                                    collapsed,
                                    rename_squad,
                                    rename_draft,
                                    detail_by_id.get_value(),
                                    vehicle_by_squad.get_value(),
                                    add_vehicle_squad,
                                    vehicle_options.get_value(),
                                )
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
                        #[cfg(target_arch = "wasm32")]
                        {
                            use wasm_bindgen::JsCast;
                            if let Some(el) = ev
                                .target()
                                .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                            {
                                scroll_top.set(el.scroll_top() as f64);
                            }
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        let _ = &ev;
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
