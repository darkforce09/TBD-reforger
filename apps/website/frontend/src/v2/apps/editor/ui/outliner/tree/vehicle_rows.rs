//! Vehicle rows for editor outliner trees.

use super::*;

// The authoring outliner shows map-placed vehicles with the same selection and Attributes actions
// as slots. ORBAT-only vehicles have no map position and stay in the ORBAT tree. The engine read is
// wasm-only, so native builds render no vehicle footer.

/// Renders map-placed vehicles as selectable rows in the authoring outliner.
#[cfg(target_arch = "wasm32")]
pub(super) fn placed_vehicle_rows(authoring: bool, selected: RwSignal<Vec<String>>) -> AnyView {
    if !authoring {
        return ().into_any();
    }
    let rows: Vec<website_map_engine::editing::hosted_commands::VehicleRow> =
        website_map_engine::editing::hosted_commands::vehicle_rows()
            .into_iter()
            .filter(|v| v.xy.is_some()) // on-the-map vehicles only
            .collect();
    if rows.is_empty() {
        return ().into_any();
    }
    view! {
        // A small-caps section marker (ROW_FACTION idiom) sets the placed vehicles off from the layer
        // tree above without pretending to be a collapsible folder.
        <div class=ROW_FACTION aria-hidden="true">
            <span class="size-4 shrink-0"></span>
            <MaterialIcon name="directions_car" class="block text-sm leading-none" />
            <span class="truncate">"Placed vehicles"</span>
        </div>
        {rows
            .into_iter()
            .map(|v| {
                let id = v.id.clone();
                let id_click = id.clone();
                let id_dbl = id.clone();
                // Label the row by the vehicle's classname tail (`resourceName` is a GUID-headed path);
                // the outliner shows an author-legible name, not a raw prefab path.
                let label = {
                    let tail = crate::v2::apps::editor::arsenal::asset_catalog::classname_tail(&v.resource_name);
                    if tail.is_empty() { v.id.clone() } else { tail.to_string() }
                };
                let aria = label.clone();
                let is_sel = {
                    let id = id.clone();
                    move || selected.get().iter().any(|s| s == &id)
                };
                view! {
                    <button
                        type="button"
                        aria-label=aria
                        class=move || if is_sel() { ROW_ACTIVE } else { ROW }
                        on:click=move |_| {
                            // Same single-click contract a slot row has: select through the
                            // kind-agnostic `select_slot` (sets selection + engine tint).
                            entity_selection::select_slot(id_click.clone());
                        }
                        on:dblclick=move |_| {
                            // SEL-ORBAT-DBL-001 — activate opens Attributes, exactly like a slot.
                            crate::v2::apps::editor::bridge::host_state::editor_context::open_attributes(id_dbl.clone());
                        }
                    >
                        // A leading spacer keeps these rows aligned with the tree's guide column.
                        <span class="size-4 shrink-0"></span>
                        <MaterialIcon name="directions_car" class="block text-sm leading-none" />
                        <span class="truncate">{label}</span>
                    </button>
                }
                .into_any()
            })
            .collect::<Vec<_>>()}
    }
    .into_any()
}

/// Native shell: `editor_ops` is wasm32-only, so there is no document to read — no placed-vehicle
/// rows on the native build (the `placed_vehicles_panel` stub rule).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn placed_vehicle_rows(_authoring: bool, _selected: RwSignal<Vec<String>>) -> AnyView {
    ().into_any()
}

/// publish `window.__outlinerStats[key] = {total, rendered, threshold}` for the gate.
#[cfg(target_arch = "wasm32")]
pub(super) fn set_outliner_stats(key: &str, total: usize, rendered: usize) {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else { return };
    let stats = match js_sys::Reflect::get(&win, &JsValue::from_str("__outlinerStats")) {
        Ok(v) if v.is_object() => v,
        _ => {
            let o = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__outlinerStats"), &o);
            o.into()
        }
    };
    let entry = js_sys::Object::new();
    let set = |k: &str, n: usize| {
        let _ = js_sys::Reflect::set(&entry, &JsValue::from_str(k), &JsValue::from_f64(n as f64));
    };
    set("total", total);
    set("rendered", rendered);
    set("threshold", VIRTUAL_SLOT_THRESHOLD);
    let _ = js_sys::Reflect::set(&stats, &JsValue::from_str(key), &entry);
}
#[cfg(not(target_arch = "wasm32"))]
/// Native no-op for the browser outliner statistics hook.
pub(super) fn set_outliner_stats(_key: &str, _total: usize, _rendered: usize) {}
