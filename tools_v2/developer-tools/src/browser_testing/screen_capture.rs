//! Headless editor capture using the shared CDP browser harness.
//!
//! This is the operator's headless screenshot rig for the live Mission Creator: it drives the
//! running editor over CDP and captures both the DOM chrome (`Page.captureScreenshot`) and the
//! `wgpu` map (`canvas.toDataURL`). Three entry points:
//!
//!   * [`shot`]      — navigate the steps, poll the boot overlay out, dump diagnostics, capture
//!   * [`zoomsweep`] — per-zoom `__editorCamSet` plus a canvas read
//!   * [`crop`]      — crop, and optionally upscale, a region of a capture
//!
//! Chrome launch (ANGLE/Vulkan on the real device), the font-cache workaround, the CDP wait and
//! the teardown are `cdp::launch_with_gpu` + `Browser::shutdown`; this module carries the driver
//! logic on top of them.
//!
//! The hard-won environment knowledge lives in `docs/tools/editor_capture.md` — the three
//! non-obvious things (writable `XDG_CACHE_HOME`, `--use-angle=vulkan`, read the map off the canvas
//! not the compositor) are preserved here. `cdp::launch_with_gpu(_, GpuBackend::Vulkan, _)` carries
//! the vulkan flags and pins `XDG_CACHE_HOME` on the chromium child (KB-002); this module carries
//! the driver logic.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex as StdMutex};

use anyhow::{Context, Result, anyhow};
use base64::Engine as _;
use serde_json::{Value, json};

use crate::browser_testing::cdp::{self, GpuBackend, Page, sleep_ms};

/// The capture viewport passed to `Emulation.setDeviceMetricsOverride` (1920×1080, dsf=1).
/// Deliberately larger than the gate harness's 1440×900 — these shots frame the whole editor.
const CAPTURE_VIEWPORT: (u32, u32) = (1920, 1080);

/// Debug port for the capture chromium. Distinct from the gate harness ports (9337/9341/9399) so a
/// capture never collides with a concurrent gate run; 9222 is chromium's own default.
const CAPTURE_DEBUG_PORT: u16 = 9222;

/// The boot-overlay selector. The editor boots behind a
/// full-bleed loading overlay; screenshotting before it clears captures the spinner, not the map.
const OVERLAY_SELECTOR: &str =
    r#"[class*="animate-overlay-fade"], [class*="z-50"][class*="backdrop-blur"]"#;

/// An all-black canvas still encodes to a valid PNG — just a tiny one. Byte count is the tell:
/// ~45 KB is a black rectangle, ~3.7 MB is the real map. Refuse to write below this.
const CANVAS_MIN_BYTES: usize = 20_000;

/// One capture "step": a URL to navigate to and how long (ms) to settle afterwards. The CLI
/// spells these as positional pairs `<url> <waitMs> [url waitMs ...]`.
#[derive(Clone, Debug)]
pub struct Step {
    pub url: String,
    pub wait_ms: u64,
}

/// Behaviour switches the `capture` CLI exposes as flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShotOptions {
    /// `CANVAS_CAPTURE=1` — also write `<out>_canvas.png` via `toDataURL`. **Required to see the
    /// map**: headless chrome's compositor returns a black GPU layer even when the engine renders.
    pub canvas_capture: bool,
    /// `FORCE_HIDE_OVERLAY=1` — remove the boot overlay from the DOM before capturing, to read the
    /// chrome behind a stuck boot. The map may be blank — that is itself the finding.
    pub force_hide_overlay: bool,
}

/// Console/log/exception lines the page emitted, captured by a raw-WS message listener
/// (for the "last 40" diagnostics dump). Kept behind an `Arc<Mutex>` because CDP events arrive on
/// the reader task.
type ConsoleLog = Arc<StdMutex<Vec<String>>>;

#[path = "screen_capture/eval_js.rs"]
mod eval_js;
pub use eval_js::shot;
pub use eval_js::zoomsweep;

#[path = "screen_capture/crop.rs"]
mod crop;
pub use crop::crop;
