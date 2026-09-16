//! Role: Module boundary for the editor's interactive map tools.
//! Position: `editor` in the frontend editor.
//! Signals & state: DOM overlays, browser transports, and the mount-scoped installs behind them.
//! Invariants: the decidable half of every tool here — its state machine, geometry and verdicts —
//! lives in `website_map_engine::editing::tools`. What remains is the browser's half.

/// The Line-of-Sight overlay: the SVG sight line and its inline profile panel.
pub mod los_tool;

/// The ruler / measure tool.
pub mod ruler_tool;

/// Select and LMB pick: pointer routing, the marquee, and the selection smoke bridge.
#[cfg(target_arch = "wasm32")]
pub mod select_tool;

/// The object layer's browser seam onto the streamed world occluder.
#[cfg(target_arch = "wasm32")]
pub mod los_world_wasm;

/// The browser half of the viewshed job scheduler: the clock, the frame pump, and the log line.
pub mod viewshed_scheduler;
