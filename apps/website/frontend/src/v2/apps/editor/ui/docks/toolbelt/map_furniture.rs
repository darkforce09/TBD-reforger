//! Map furniture.

use super::*;

/// Renders a metric scale from the live map scale or a camera snapshot.
#[component]
pub fn ScaleBar(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    debug_hud: Option<RwSignal<String>>,
    /// The live editor supplies a scale signal seeded 4.0; the camera_snapshot fallback is dead on the only real caller.
    scale_mpp: Option<RwSignal<f64>>,
) -> impl IntoView {
    let spec = move || -> ScaleBarSpec {
        if let Some(s) = scale_mpp {
            return pick_scale_bar(s.get());
        }
        let _ = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        #[allow(unused_mut)]
        let mut deck_zoom = -2.0_f64;
        #[cfg(target_arch = "wasm32")]
        {
            if let Some((_, _, z)) = website_map_engine::streaming::host::camera_snapshot() {
                deck_zoom = z;
            }
        }
        pick_scale_bar(m_per_px(deck_zoom))
    };
    view! {
        <div
            data-scale-bar
            class="flex select-none flex-col items-center gap-0.5"
            title=move || format!("Map scale — {}", spec().label)
        >
            <span class="leading-none text-outline">{move || spec().label}</span>
            <div
                class="relative border-x border-b border-outline/70"
                style=move || format!("width:{:.1}px;height:5px", spec().width_px)
            ></div>
        </div>
    }
}

/// Renders map grid references along the visible map pane.
#[component]
pub fn MapGridRefs(
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    debug_hud: Option<RwSignal<String>>,
) -> impl IntoView {
    let labels = move || -> (Vec<EdgeLabel>, Vec<EdgeLabel>) {
        let _ = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        #[cfg(target_arch = "wasm32")]
        {
            use crate::v2::apps::editor::shell::layout::{
                DOCK_LEFT_PX, DOCK_RIGHT_PX, STRIP_TOP_PX,
            };
            let Some((tx, ty, zoom)) = website_map_engine::streaming::host::camera_snapshot()
            else {
                return (Vec::new(), Vec::new());
            };
            let Some(win) = web_sys::window() else {
                return (Vec::new(), Vec::new());
            };
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
                return (Vec::new(), Vec::new());
            }
            let cam = selection::frozen_camera(vw, vh, tx, ty, zoom);
            let pane_left = DOCK_LEFT_PX;
            let pane_right = vw - DOCK_RIGHT_PX;
            (
                edge_eastings(&cam, pane_left, pane_right, STRIP_TOP_PX),
                edge_northings(&cam, pane_left, STRIP_TOP_PX, vh),
            )
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            (Vec::new(), Vec::new())
        }
    };
    view! {
        <div
            data-grid-refs
            class="pointer-events-none absolute inset-0 z-10 font-mono text-code-md text-primary/80"
        >
            <For
                each=move || labels().0
                key=|l| l.key.clone()
                let:l
            >
                <span
                    class="absolute -translate-x-1/2 rounded bg-surface-container-lowest/60 px-1 leading-none"
                    style=move || {
                        format!(
                            "left:{:.1}px;top:{:.1}px",
                            l.pos_px,
                            crate::v2::apps::editor::shell::layout::STRIP_TOP_PX + 2.0,
                        )
                    }
                >
                    {l.text.clone()}
                </span>
            </For>
            <For
                each=move || labels().1
                key=|l| l.key.clone()
                let:l
            >
                <span
                    class="absolute -translate-y-1/2 rounded bg-surface-container-lowest/60 px-1 leading-none"
                    style=move || {
                        format!(
                            "left:{:.1}px;top:{:.1}px",
                            crate::v2::apps::editor::shell::layout::DOCK_LEFT_PX + 2.0,
                            l.pos_px,
                        )
                    }
                >
                    {l.text.clone()}
                </span>
            </For>
        </div>
    }
}
