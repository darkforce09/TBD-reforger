//! Transform widget for editor overlays.
use super::*;

thread_local! {
    static WIDGET_PIVOT: std::cell::RefCell<Option<std::rc::Rc<dyn Fn() -> Option<(f64, f64)>>>> =
        const { std::cell::RefCell::new(None) };
}

/// Registers a getter for the current selection pivot.
#[cfg(target_arch = "wasm32")]
pub(crate) fn register_widget_pivot(f: std::rc::Rc<dyn Fn() -> Option<(f64, f64)>>) {
    WIDGET_PIVOT.with(|c| *c.borrow_mut() = Some(f));
}

/// Returns the registered selection pivot when available.
#[must_use]
pub(crate) fn read_widget_pivot() -> Option<(f64, f64)> {
    WIDGET_PIVOT.with(|c| c.borrow().as_ref().and_then(|f| f()))
}

/// Renders translation and rotation handles over the selection.
#[component]
pub(crate) fn TransformWidgetOverlay(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    debug_hud: Option<RwSignal<String>>,
    tick: RwSignal<u64>,
    variant: RwSignal<transform::WidgetVariant>,
) -> impl IntoView {
    let projected = move || -> Option<(f64, f64, transform::WidgetVariant)> {
        let _ = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        let _ = tick.get();
        let var = variant.get();
        let (wx, wy) = read_widget_pivot()?;
        #[cfg(target_arch = "wasm32")]
        {
            let (tx, ty, zoom) = website_map_engine::streaming::host::camera_snapshot()?;
            let win = web_sys::window()?;
            let vw = win
                .inner_width()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let vh = win
                .inner_height()
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if vw <= 0.0 || vh <= 0.0 {
                return None;
            }
            let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
            let p = cam.project([wx, wy, 0.0]);
            if !p[0].is_finite() || !p[1].is_finite() {
                return None;
            }
            Some((p[0], p[1], var))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (wx, wy, var);
            None
        }
    };
    view! {
        <svg
            data-transform-widget
            class="pointer-events-none absolute inset-0 z-10"
            width="100%"
            height="100%"
        >
            {move || projected().map(|(cx, cy, var)| {
                const R: f64 = transform::WIDGET_RADIUS_PX;
                const HEAD: f64 = 7.0;
                match var {
                    transform::WidgetVariant::None => view! { <g></g> }.into_any(),
                    transform::WidgetVariant::Translate => view! {
                        <g>
                            <line x1=move || format!("{cx:.1}") y1=move || format!("{cy:.1}")
                                  x2=move || format!("{:.1}", cx + R) y2=move || format!("{cy:.1}")
                                  class="stroke-primary" stroke-width="2" />
                            <polygon
                                points=move || format!(
                                    "{x0:.1},{y0:.1} {x1:.1},{y1:.1} {x1:.1},{y2:.1}",
                                    x0 = cx + R, y0 = cy,
                                    x1 = cx + R - HEAD, y1 = cy - HEAD * 0.7,
                                    y2 = cy + HEAD * 0.7)
                                class="fill-primary" />
                            <line x1=move || format!("{cx:.1}") y1=move || format!("{cy:.1}")
                                  x2=move || format!("{cx:.1}") y2=move || format!("{:.1}", cy - R)
                                  class="stroke-primary" stroke-width="2" />
                            <polygon
                                points=move || format!(
                                    "{x0:.1},{y0:.1} {x1:.1},{y1:.1} {x2:.1},{y1:.1}",
                                    x0 = cx, y0 = cy - R,
                                    x1 = cx - HEAD * 0.7, y1 = cy - R + HEAD,
                                    x2 = cx + HEAD * 0.7)
                                class="fill-primary" />
                            <line x1=move || format!("{cx:.1}") y1=move || format!("{cy:.1}")
                                  x2=move || format!("{cx:.1}") y2=move || format!("{:.1}", cy - crate::v2::apps::editor::bridge::gizmo_z::Z_ARM_LENGTH)
                                  class="stroke-primary" stroke-width="2" />
                            <polygon
                                points=move || format!(
                                    "{x0:.1},{y0:.1} {x1:.1},{y1:.1} {x2:.1},{y1:.1}",
                                    x0 = cx, y0 = cy - crate::v2::apps::editor::bridge::gizmo_z::Z_ARM_LENGTH,
                                    x1 = cx - HEAD * 0.7, y1 = cy - crate::v2::apps::editor::bridge::gizmo_z::Z_ARM_LENGTH + HEAD,
                                    x2 = cx + HEAD * 0.7)
                                class="fill-primary" />
                            {move || {
                                crate::v2::apps::editor::bridge::overlays::read_z_drag_readout().map(|text| view! {
                                    <text x=move || format!("{:.1}", cx + 15.0) y=move || format!("{:.1}", cy - crate::v2::apps::editor::bridge::gizmo_z::Z_ARM_LENGTH * 0.5) class="fill-primary font-mono text-[11px]">
                                        {text}
                                    </text>
                                })
                            }}

                            <circle cx=move || format!("{cx:.1}") cy=move || format!("{cy:.1}")
                                    r="3" class="fill-primary" />
                        </g>
                    }.into_any(),
                    transform::WidgetVariant::Rotate => view! {
                        <g>
                            <circle cx=move || format!("{cx:.1}") cy=move || format!("{cy:.1}")
                                    r=move || format!("{R:.1}")
                                    fill="none" class="stroke-primary" stroke-width="2" />
                            <circle cx=move || format!("{cx:.1}") cy=move || format!("{cy:.1}")
                                    r="3" class="fill-primary" />
                        </g>
                    }.into_any(),
                }
            })}
        </svg>
    }
}

/// Displays the active transform mode beside the pointer.
#[component]
pub(crate) fn WidgetModeHint(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    variant: RwSignal<transform::WidgetVariant>,
) -> impl IntoView {
    view! {
        <div data-widget-mode-hint class="pointer-events-none absolute inset-0 z-10">
            {move || {
                let (x, y, _) = cursor.get()?;
                let label = variant.get().label();
                Some(view! {
                    <div
                        class="absolute rounded bg-surface/80 px-1.5 py-0.5 font-mono text-[11px] \
                               tabular-nums text-on-surface-variant shadow-sm"
                        style=move || format!("left:{:.0}px;top:{:.0}px", x + 16.0, y + 16.0)
                    >
                        {label}
                    </div>
                })
            }}
        </div>
    }
}

/// Displays the current snap ladder and enablement.
#[component]
pub(crate) fn SnapReadout(snap: RwSignal<transform::SnapState>) -> impl IntoView {
    view! {
        <div
            data-snap-readout
            class="pointer-events-none absolute bottom-11 right-3 z-20 rounded bg-surface/70 px-2 \
                   py-0.5 font-mono text-[11px] tabular-nums text-on-surface-variant"
        >
            {move || snap.get().status_readout()}
        </div>
    }
}
