//! Mission Creator editor (/missions/:id/edit) — the wgpu boundary collapse (T-159.15).
//!
//! **T-159.15.0 (foundation):** the Leptos app owns `RenderEngine` DIRECTLY as plain Rust — created
//! from a canvas `NodeRef` via `spawn_local`, no `map-engine-wasm` shim, one wasm module / one
//! linear memory. That slice mounted the canvas and rendered a single frame.
//!
//! **T-159.15.1 (this slice):** a damage-driven continuous render loop + wheel-zoom + resize, engine
//! owned directly (D5). Two things unblock the loop on the second `render()` submit (which panicked
//! `wgpu` "Buffer is already mapped" on the 15.0 foundation):
//!   1. `disable_frame_timing()` — drops the `GpuTimer` timestamp-readback lane. Headless is
//!      **WebGPU/Dawn** (not WebGL2 as first assumed), where that lane's `map_async` double-maps its
//!      16-byte buffer on the 2nd submit. The editor has no fps/GPU-time HUD, so the lane is pure
//!      overhead — dropping it removes the offending map. This is the actual fix.
//!   2. `engine.poll()` per frame after `render()` — drains readback `map_async` callbacks for the
//!      WebGL2-fallback path and the cull-counter lane that later world slices add. A no-op on
//!      real-browser WebGPU (the event loop resolves maps).
//! A `window.__selfChecks` bridge exposes the byte-exact GPU readback gate (calibration + texture)
//! the headless driver awaits — under `?force=webgl`, since `self_check`'s polled readback only
//! resolves on WebGL2 headless.
//!
//! The full Eden docked shell (Top Command Strip, Left Outliner, Right Asset Palette, Bottom
//! Toolbelt, doc host) lands across T-159.16–.22. Route is `chromeless` + `full_bleed` (AppLayout
//! hides the platform nav). Verified by GPU readback (not DOM diff) as the map lane grows.
#![allow(dead_code)]
use leptos::prelude::*;

// T-934.6 — editor_ops moved to `crate::editor::state::operations`; the alias keeps the dozens of
// Class-S source-guard needles (`editor_ops::…`) and the page's own prose stable across the move.
use crate::editor::panels::validation_panel;
#[cfg(target_arch = "wasm32")]
use crate::editor::state::doc_host as mission_doc;
#[cfg(target_arch = "wasm32")]
use crate::editor::state::history as mission_history;
#[cfg(target_arch = "wasm32")]
use crate::editor::state::hydrate as mission_hydrate;
#[cfg(target_arch = "wasm32")]
use crate::editor::state::operations as editor_ops;
#[cfg(target_arch = "wasm32")]
use crate::editor::state::persist as yrs_persist;

// T-934.10 — the pure canvas helper belt (connection/comment/marker lane feeds, the hover state
// machine, route resolution, the selection universe, the crew-hide SoA filter, the paste anchor)
// moved to `editor::canvas::render_sync`. Re-exported `pub(crate)` under the SAME names so the
// page's bare call sites, the `mission_editor::…` paths (`state/history.rs`, the panel test
// modules) and the evacuated pins' `use super::…` imports all keep their exact spelling. The cfg
// split mirrors the consumers: nothing in the native non-test build reads these through here.
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use crate::editor::canvas::render_sync::{
    comment_drag_lane_xy, comment_lane_xy, comment_points, connection_lane_verts,
    connection_segments, dragged_comment_points, hover_cursor_css, hover_due, hover_next,
    hover_suppressed, marker_lane_fields, pick_comment, pick_connection, plain_paste_anchor,
    route_availability, route_target, selectable_ids, ConnSegment, HoverState, RouteTarget,
    COMMENT_PICK_PX,
};
// Names only the wasm side consumes (the doc-bound wrappers below + `state/history.rs`).
#[cfg(target_arch = "wasm32")]
pub(crate) use crate::editor::canvas::render_sync::{
    comment_lane_ids, map_render_slot_soa, CommentPoint, CONN_PICK_PX,
};

// T-934.11 — the floating overlay/dialog components (transform widget + mode hint + snap readout,
// asset picker, comment editor, Connections panel, conflict dialog) moved to
// `editor::canvas::overlays`, together with `AssetPickerState`, `ConflictInfo` and the T-648
// widget-pivot registry. Re-exported under the SAME names so the page's bare mounts
// (`<TransformWidgetOverlay …/>`), the wasm block's `register_widget_pivot(` / `read_widget_pivot()`
// calls, and the `crate::editor::mission_editor::{AssetPickerState, ConflictInfo}` paths in
// `state/operations/context.rs` / `state/hydrate.rs` all keep their exact spelling. The T-797
// toolbar-dispatch registry did NOT move: `eden_top_strip` drives it through
// `crate::editor::mission_editor::…` and it bridges the page to the strip, not to the overlays.
#[cfg(target_arch = "wasm32")]
pub(crate) use crate::editor::canvas::overlays::{read_widget_pivot, register_widget_pivot};
pub(crate) use crate::editor::canvas::overlays::{
    AssetPickerOverlay, CommentEditorOverlay, ConflictDialog, ConnectionsPanelOverlay, SnapReadout,
    TransformWidgetOverlay, WidgetModeHint,
};
pub use crate::editor::canvas::overlays::{AssetPickerState, ConflictInfo};

/// T-750 — apply the `/registry` fetch's terminal failure to the three signals the dock reads.
///
/// Lives OUTSIDE the wasm32 gate on purpose: the Favourites failure arm is host-testable via
/// source pins, and `live_code` kills `#[cfg(target_arch = "wasm32")]` bodies on the native
/// harness — a helper the Err arm *calls* is the only failure write that survives the scrub.
fn mark_registry_fetch_failed(
    catalog: RwSignal<crate::editor::arsenal::asset_catalog::CatalogState>,
    vehicle_catalog: RwSignal<crate::editor::arsenal::asset_catalog::CatalogState>,
    registry_failed: RwSignal<bool>,
) {
    use crate::editor::arsenal::asset_catalog::CatalogState;
    catalog.set(CatalogState::Failed);
    vehicle_catalog.set(CatalogState::Failed);
    registry_failed.set(true);
}

/// T-245 — SPA-session cache for the editor's `/registry` + `/registry/compat` payloads.
/// Survives `MissionEditorPage` remounts inside one SPA session so leaving
/// `/missions/:id/edit` and coming back does **not** re-issue the cold fetches or rebuild
/// the Arsenal compat feed / cargo seed map.
///
/// **T-427** moved the cold path off the unbounded dual dump: registry is assembled from
/// `?limit=` pages, Arsenal edges come from a filtered `edge_type=` list, and cargo seeds
/// come from `?view=cargo_defaults` (server-aggregated — no client walk of ~16k cargo edges).
/// This cache still stores the *assembled* result so remounts stay free.
mod registry_session {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use crate::core::dto::RegistryItem;
    use crate::editor::arsenal::arsenal_rules::{CargoRow, CompatFeed};

    struct CachedCompat {
        feed: CompatFeed,
        cargo: HashMap<String, Vec<CargoRow>>,
    }

    thread_local! {
        static REGISTRY: RefCell<Option<Vec<RegistryItem>>> = const { RefCell::new(None) };
        static COMPAT: RefCell<Option<CachedCompat>> = const { RefCell::new(None) };
    }

    /// `true` when this mount must issue `GET /registry` (no SPA-session hit).
    #[must_use]
    pub fn must_fetch_registry() -> bool {
        REGISTRY.with(|c| c.borrow().is_none())
    }

    /// `true` when this mount must issue `GET /registry/compat` (no SPA-session hit).
    #[must_use]
    pub fn must_fetch_compat() -> bool {
        COMPAT.with(|c| c.borrow().is_none())
    }

    #[must_use]
    pub fn cached_registry() -> Option<Vec<RegistryItem>> {
        REGISTRY.with(|c| c.borrow().clone())
    }

    pub fn store_registry(items: Vec<RegistryItem>) {
        REGISTRY.with(|c| *c.borrow_mut() = Some(items));
    }

    /// Clone of the session-ready compat feed + cargo seed map, if any.
    #[must_use]
    pub fn cached_compat() -> Option<(CompatFeed, HashMap<String, Vec<CargoRow>>)> {
        COMPAT.with(|c| {
            c.borrow()
                .as_ref()
                .map(|hit| (hit.feed.clone(), hit.cargo.clone()))
        })
    }

    pub fn store_compat(feed: CompatFeed, cargo: HashMap<String, Vec<CargoRow>>) {
        COMPAT.with(|c| *c.borrow_mut() = Some(CachedCompat { feed, cargo }));
    }

    #[cfg(test)]
    pub fn clear_for_test() {
        REGISTRY.with(|c| *c.borrow_mut() = None);
        COMPAT.with(|c| *c.borrow_mut() = None);
    }
}

/// T-427 — page size for cold `GET /registry?limit=` (matches API `REGISTRY_PAGE_MAX`).
#[cfg(target_arch = "wasm32")]
const REGISTRY_COLD_PAGE: i64 = 500;

/// T-427 — Arsenal-needed compat families for the cold CompatGraph (excludes the ~16k
/// `character_default_cargo` dump and unused default-loadout/weapon families).
#[cfg(target_arch = "wasm32")]
const EDITOR_COMPAT_EDGE_TYPES: &str = "optic_on_weapon,mag_in_weapon,attachment_on_weapon";

/// Assemble the flat catalog via bounded pages (T-427). Never hits the unpaginated dump.
#[cfg(target_arch = "wasm32")]
async fn fetch_registry_pages(
    auth: crate::core::auth::AuthStore,
) -> Result<Vec<crate::core::dto::RegistryItem>, crate::core::client::ApiErr> {
    use crate::core::dto::{RegistryItem, RegistryResponse};

    let mut all: Vec<RegistryItem> = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let path = format!("/registry?limit={REGISTRY_COLD_PAGE}&offset={offset}");
        let page: RegistryResponse = crate::core::client::api_get(auth, &path).await?;
        let n = page.data.len() as i64;
        let total = page.total.unwrap_or(offset + n);
        all.extend(page.data);
        offset += n;
        if n == 0 || offset >= total {
            break;
        }
        // Safety: never spin if the server ignores pagination and returns the full set.
        if n > REGISTRY_COLD_PAGE {
            break;
        }
    }
    Ok(all)
}

/// Cold compat path (T-427): Arsenal edge families + server-aggregated cargo defaults.
/// Does **not** GET unfiltered `/registry/compat` and does **not** walk raw cargo edges client-side.
#[cfg(target_arch = "wasm32")]
async fn fetch_compat_cold(
    auth: crate::core::auth::AuthStore,
) -> Result<
    (
        crate::editor::arsenal::arsenal_rules::CompatFeed,
        std::collections::HashMap<String, Vec<crate::editor::arsenal::arsenal_rules::CargoRow>>,
    ),
    crate::core::client::ApiErr,
> {
    use crate::core::dto::{RegistryCargoDefaultsResponse, RegistryCompatResponse};
    use crate::editor::arsenal::arsenal_rules::{CargoRow, CompatFeed, CompatGraph, CompatStatus};
    use std::collections::HashMap;

    let edges_path = format!("/registry/compat?edge_type={EDITOR_COMPAT_EDGE_TYPES}");
    let edges: RegistryCompatResponse = crate::core::client::api_get(auth, &edges_path).await?;
    let cargo_resp: RegistryCargoDefaultsResponse =
        crate::core::client::api_get(auth, "/registry/compat?view=cargo_defaults").await?;

    let mut cargo: HashMap<String, Vec<CargoRow>> = HashMap::new();
    for (character, rows) in cargo_resp.data {
        cargo.insert(
            character,
            rows.into_iter()
                .map(|r| CargoRow {
                    container: r.container,
                    item: r.item,
                    qty: r.qty,
                })
                .collect(),
        );
    }

    let feed = CompatFeed {
        status: CompatStatus::Ready,
        graph: CompatGraph::from_edges(&edges.data),
    };
    Ok((feed, cargo))
}

/// Round CSS px → device-pixel backing size (≥1), matching the React oracle's `deviceSize`.
#[cfg(target_arch = "wasm32")]
fn device_size(css_w: f64, css_h: f64, dpr: f64) -> (u32, u32) {
    let r = |v: f64| ((v * dpr + 0.5).floor().max(1.0)) as u32;
    (r(css_w), r(css_h))
}

/// T-175 B5 — editor boot phase driving the loading overlay. Cold open runs two independent async
/// tasks (IDB restore + server hydrate, and engine create + world/map-asset bootstrap); the overlay
/// stays up with an honest phase label until **both** settle, so the operator never stares at a
/// silent half-ready map. (The React editor had a T-060 determinate overlay that never ported.)
///
/// T-628 — the phase no longer decides what the bar shows; [`boot_progress::BootProgress`] does,
/// and it spans the whole boot rather than restarting per stage. What survives here is the single
/// question the overlay still needs a phase for: is the boot over? `Hydrating`/`LoadingMap` are
/// kept because the two boot tasks flip them independently and `Ready` is their rendezvous.
///
/// T-631 — a boot can now END BADLY, not only hang. When `RenderEngine::create` returns `Err`
/// (a WebGPU/GL init failure — `createBuffer size too large`, no adapter, a lost device) the
/// bar used to sit at the last honest reading forever because the world task that flips the
/// overlay down lives inside the *success* branch. `Failed` is the fourth terminal state: the
/// overlay stops being a spinner and names the segment that broke and the REAL reason, with a
/// Retry and a "continue without map". No longer `Copy` — `reason` is an owned `String` (the
/// wgpu message, verbatim), which is the whole point: a `&'static str` could only ever say
/// "engine failed", and "make it wrong on demand" means carrying the actual cause through.
#[derive(Clone, PartialEq, Debug)]
enum BootPhase {
    /// IDB restore + server hydrate in flight.
    Hydrating,
    /// Doc ready; engine/atlas/world residency still settling.
    LoadingMap,
    /// Doc hydrated + world settled — overlay hidden.
    Ready,
    /// A boot segment failed unrecoverably. Terminal: the overlay shows the error state (the
    /// failing segment + the underlying reason), not the bar. `seg` names which
    /// [`boot_progress::BootSeg`] broke so the caption reads "Rendering engine failed" rather
    /// than a generic apology; `reason` is the loader's own error text.
    Failed {
        seg: boot_progress::BootSeg,
        reason: String,
    },
}

impl BootPhase {
    /// Fold a boot transition, with `Failed` **sticky**.
    ///
    /// T-631 — the acceptance clause "a subsequent misleading event does NOT overwrite the
    /// original reason" is enforced HERE, not by call-site luck. The two boot tasks run
    /// concurrently: the engine task can land in `Failed` while the doc-hydrate task, oblivious,
    /// is still on its way to `boot.set(LoadingMap)` / `hand_over → Ready`. If those later writes
    /// won, the overlay would flip from a correct error back to a spinner (or vanish onto a dead
    /// map), and the FIRST panic's real cause — the one thing worth reporting — would be buried by
    /// the second, misleading event. So once a boot is `Failed`, every further transition is a
    /// no-op and the original `{seg, reason}` survives. Every task-driven `boot` write goes
    /// through this; nothing calls `boot.set` on the phase directly on a path that can race a
    /// failure.
    // Ungated so `t631_boot_failure_state` can drive it on the host; its only non-test callers
    // (`hand_over`, the two boot tasks) are wasm-only, so a native shell build sees it as dead.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    #[must_use]
    fn advance(self, next: BootPhase) -> BootPhase {
        match self {
            BootPhase::Failed { .. } => self,
            _ => next,
        }
    }
}

/// Hand-over hold, in ms, between the bar reaching 100% and the overlay coming down.
///
/// Not progress and not a duration guess: every segment has already reported [`BootEvent::Finish`]
/// before this timer is armed. It exists because the last real report and the overlay's removal
/// otherwise land in the same Leptos render, so the operator would never see the bar full — he
/// would see it stop short and the screen change. 220 ms is the 200 ms `.mc-load-fill` ease plus a
/// frame, i.e. exactly long enough for the fill to finish travelling to 100%.
///
/// [`BootEvent::Finish`]: boot_progress::BootEvent::Finish
// Ungated so `t628_boot_progress` can hold it to the CSS ease it exists to outlast; only the
// `set_timeout` that consumes it is wasm-only.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const BOOT_HANDOVER_MS: i32 = 220;

/// T-627/T-628 — the pure arithmetic behind the Mission Creator boot overlay, plus the ordering
/// discipline the concurrent satellite fetch that feeds it has to honour.
///
/// It lives here, next to the overlay, rather than beside the fetch, because `mod world_assets` is
/// `#[cfg(target_arch = "wasm32")]` in `main.rs`: not one line under it compiles on the host, so
/// not one line under it can be unit-tested. Everything the loader can decide *without* a network —
/// how a tile is split into Range requests, how out-of-order completions are put back into tile
/// order, what a byte count reads as, how four differently-sized segments add up to one bar that
/// cannot go backwards — is therefore hoisted into this module and proved by `t628_boot_progress`
/// at the bottom of the file, leaving the wasm side with fetch, decode and upload. The pins in that
/// module also hold `world_assets/*` to actually routing through here.
pub mod boot_progress {
    /// Concurrent HTTP Range requests in flight against the map-asset host.
    ///
    /// Browsers cap ~6 connections per HTTP/1.1 origin, and the world-chunk loader is pulling from
    /// that **same** origin for the whole boot, so the satellite cannot have all six: at 6 it wins
    /// every slot and the chunk stream stalls behind it, which makes the boot the operator is
    /// actually waiting on *slower* even though the texture lands sooner. 4 leaves two slots for
    /// the chunk loader and still keeps the pipe full — everon's level 0 is exactly 4 tiles
    /// (28.3 / 21.6 / 27.6 / 33.0 MB measured from the live index), so 4 in flight covers the
    /// largest level in one wave and nothing is gained by going wider.
    pub const SAT_FETCH_CONCURRENCY: usize = 4;

    /// Bytes per Range request. A tile is fetched as a run of these rather than in one shot, and
    /// the reason is the *bar*: everon's level 0 is four ~25 MB tiles fetched four-up, so per-tile
    /// reporting would leave the bar at 0% for the entire download and then snap to 100% — the
    /// same "conveys nothing" failure as the fixed-width animation this slice replaces. At 4 MiB
    /// the 152.7 MB bundle reports ~37 times (~2.7% a step) and the per-request overhead is noise
    /// beside the body.
    pub const SAT_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

    /// Split one tile's byte extent into consecutive `fetch_range` arguments (**inclusive** ends,
    /// which is what the `Range: bytes=a-b` header wants).
    ///
    /// The contract the caller depends on and `t627_boot_progress` pins: the returned spans are in
    /// ascending order, are contiguous with no gap and no overlap, and cover `[offset, offset +
    /// length)` exactly — concatenating the bodies back in this order reproduces the tile byte for
    /// byte. A zero-length tile yields no requests; a zero `chunk` degrades to 1 byte a request
    /// rather than looping forever.
    #[must_use]
    pub fn split_range(offset: u64, length: u64, chunk: u64) -> Vec<(u64, u64)> {
        if length == 0 {
            return Vec::new();
        }
        let step = chunk.max(1);
        let end = offset + length; // exclusive
        let mut out = Vec::new();
        let mut at = offset;
        while at < end {
            let stop = at.saturating_add(step).min(end); // exclusive
            out.push((at, stop - 1));
            at = stop;
        }
        out
    }

    /// Index-addressed collector for completions that arrive out of order.
    ///
    /// `buffer_unordered` hands back whichever request the network finished first, but the
    /// satellite is consumed **positionally** — tile *n* of the reassembled `Vec` is uploaded at
    /// `mip.tiles[n]`'s `(x, y)`, and chunk *n* of a tile is concatenated at its own offset. A Vec
    /// pushed in completion order is therefore a scrambled texture that looks like a rendering bug.
    /// Every result carries the index of the request that produced it and is written to that slot;
    /// [`Ordered::finish`] refuses to hand back a partially filled run, so a dropped completion is
    /// a `None` rather than a silently shorter, shifted `Vec`.
    pub struct Ordered<T> {
        slots: Vec<Option<T>>,
    }

    impl<T> Ordered<T> {
        #[must_use]
        pub fn new(n: usize) -> Self {
            Self {
                slots: (0..n).map(|_| None).collect(),
            }
        }

        /// Place `v` at `i`. `false` when `i` is out of range — the caller must treat that as a
        /// failed fetch, not skip it, or the run silently loses a chunk.
        pub fn put(&mut self, i: usize, v: T) -> bool {
            match self.slots.get_mut(i) {
                Some(slot) => {
                    *slot = Some(v);
                    true
                }
                None => false,
            }
        }

        /// The run in index order, or `None` if any slot was never filled.
        #[must_use]
        pub fn finish(self) -> Option<Vec<T>> {
            self.slots.into_iter().collect()
        }
    }

    /// Bytes of a streamed body to accumulate before reporting them.
    ///
    /// `fetch`'s `ReadableStream` hands back whatever came off the socket — typically 16–64 KB — so
    /// reporting every chunk would push ~1,100 signal writes through the overlay for the DEM alone.
    /// At 512 KB the terrain segment still reports ~140 times (0.7% of itself a step), which is
    /// finer than the bar can render, and the boot does ~200 signal writes instead of ~2,000.
    pub const STREAM_REPORT_BYTES: u64 = 512 * 1024;

    /// The four stretches of the Mission Creator boot. Every one of them has a **real** budget:
    /// three are byte-metered against a `content-length` (or, for the satellite, the sum of the
    /// tbd-sat index's tile `length`s), and the fourth is metered in **files**, because a world
    /// chunk's size is published nowhere and the only number knowable before the fetch is how many
    /// of them there are.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum BootSeg {
        /// IDB replay + `GET /api/v1/missions/:id` — byte-metered off that response's
        /// `content-length` (the editor's own document can be anything from 700 B to ~142 MB, so it
        /// is emphatically not a rounding error on a big mission).
        Mission,
        /// `dem/everon-dem-16bit.png` — byte-metered off its `content-length`.
        Terrain,
        /// The tbd-sat mips at or below the GPU's `maxTextureDimension2D` — byte-metered off the
        /// index, which lists every tile's `length` before a tile byte moves.
        Satellite,
        /// World chunks, the 8 m density bins, the prefab/road/region/atlas/label files — **file**
        /// metered: each unit is one completed fetch, and every batch's count is declared before it
        /// is requested.
        World,
    }

    impl BootSeg {
        /// Canonical boot order. The overlay names the first segment in this order that has not
        /// finished, which is what makes the caption a stable narrative even though the terrain and
        /// satellite fetches actually overlap.
        pub const ALL: [BootSeg; 4] = [
            BootSeg::Mission,
            BootSeg::Terrain,
            BootSeg::Satellite,
            BootSeg::World,
        ];

        #[must_use]
        const fn idx(self) -> usize {
            match self {
                Self::Mission => 0,
                Self::Terrain => 1,
                Self::Satellite => 2,
                Self::World => 3,
            }
        }

        /// The stage line above the bar.
        #[must_use]
        pub const fn title(self) -> &'static str {
            match self {
                Self::Mission => "Loading mission…",
                Self::Terrain => "Loading terrain…",
                Self::Satellite => "Loading satellite…",
                Self::World => "Loading world objects…",
            }
        }
    }

    /// What a loader is allowed to tell the bar. Note what is **not** here: there is no "elapsed",
    /// no "expected duration", no "step N of M begun". Every variant is either a budget the loader
    /// read off the wire before doing the work, or work it has already finished.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum BootEvent {
        /// This segment's real byte budget, read from a `content-length` header or summed from the
        /// tbd-sat index. Replaces both the segment's unit budget and its pacing weight, so the
        /// bar's speed tracks the transfer that is actually going to happen (a 16384-limit GPU
        /// fetches 152.7 MB of satellite where an 8192-limit one fetches 42.2 MB).
        Budget(BootSeg, u64),
        /// `n` more files added to a file-metered segment's budget. **Declared before the fetch**,
        /// never after: a batch that announced itself only on completion would be a bar that jumps
        /// to 100% and then discovers more work, which is the failure this whole slice is fixing.
        Files(BootSeg, u64),
        /// `n` more units — bytes off the socket, or completed fetches — that have **already**
        /// landed.
        Done(BootSeg, u64),
        /// Nothing further will be reported for this segment. Sent when a loader returns, whether
        /// it succeeded or failed, so a dead network cannot leave the bar short of 100% with the
        /// overlay still up.
        Finish(BootSeg),
    }

    /// How the reporter reaches the loaders. `Rc` because the satellite fetch holds it inside a
    /// future that outlives `bootstrap`'s stack frame.
    pub type ProgressFn = std::rc::Rc<dyn Fn(BootEvent)>;

    /// Terrain DEM, measured on the live stack (2026-08-01):
    /// `content-length` of `/map-assets/everon/dem/everon-dem-16bit.png`.
    pub const PLANNED_TERRAIN_BYTES: u64 = 71_911_548;

    /// Satellite, measured off the live tbd-sat index: the mips from level 1 down sum to
    /// 42,152,810 B, which is what an 8192-limit `maxTextureDimension2D` uploads. A 16384-limit GPU
    /// takes level 0 as well (152,710,470 B) — the [`BootEvent::Budget`] the loader sends after
    /// reading the index replaces this with whichever of the two it is actually going to fetch, so
    /// this constant only paces the first ~2 round trips.
    pub const PLANNED_SATELLITE_BYTES: u64 = 42_152_810;

    /// World objects, measured on the live stack: 315 chunk files totalling 15,320,508 B, the 625
    /// density bins totalling 10,582,750 B, and ~1.08 MB of prefab / road / region / glyph-atlas /
    /// label files. Unlike the other three this one is **not** replaced by a measurement, because
    /// no index publishes a chunk's byte size and only the subset of chunks the boot camera pins is
    /// ever fetched — so the segment's *progress* is counted in files (exact) and only its *pacing
    /// weight* is this approximation.
    pub const PLANNED_WORLD_BYTES: u64 = 27_000_000;

    /// One segment's real state. `weight` is pacing only — it never appears in a numerator.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct Segment {
        weight: u64,
        done: u64,
        total: u64,
        finished: bool,
    }

    impl Segment {
        const fn new(weight: u64) -> Self {
            Self {
                weight,
                done: 0,
                total: 0,
                finished: false,
            }
        }

        /// 0..=1. A budget that was never learned reads as 0, not as a guess; a body that overruns
        /// its promised length is clamped rather than allowed to push the bar past the segment.
        fn fraction(self) -> f64 {
            if self.finished {
                return 1.0;
            }
            if self.total == 0 {
                return 0.0;
            }
            #[allow(clippy::cast_precision_loss)]
            let f = self.done as f64 / self.total as f64;
            f.clamp(0.0, 1.0)
        }
    }

    /// T-628 — the whole boot as **one** 0→100% bar.
    ///
    /// T-627 made the satellite determinate and left the other three steps sweeping, and the
    /// operator rejected that: a sweep animates identically at 1%, at 99% and while stalled, so it
    /// carries no information — "you might as well have a black screen". So the bar is now a single
    /// continuous journey with four weighted segments underneath it, and it never resets.
    ///
    /// Three properties, all pinned by `t628_boot_progress`:
    ///
    /// * **Monotonic.** [`Self::percent`] returns a high-water mark. Budgets grow mid-boot — the
    ///   world's file count only becomes known when the residency pins the camera's chunk set, and
    ///   a segment's *weight* changes when it reads its own `content-length` — and either can make
    ///   the freshly computed figure smaller. The bar absorbs that by holding still until real work
    ///   passes it. It never rewinds.
    /// * **Never invented.** Nothing in here is a function of time. The only way a number moves is
    ///   [`BootEvent::Done`], and the only thing that sends one is a loader with the bytes or the
    ///   file already in hand.
    /// * **Reaches 100%.** Every loader sends [`BootEvent::Finish`] when it returns, success or
    ///   failure, and an all-finished progress reads exactly `100.0`.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct BootProgress {
        segs: [Segment; 4],
        /// The high-water mark — see the type doc. This, not the raw ratio, is what the bar draws.
        floor: f64,
    }

    impl Default for BootProgress {
        fn default() -> Self {
            Self::new()
        }
    }

    impl BootProgress {
        #[must_use]
        pub const fn new() -> Self {
            Self {
                segs: [
                    // The mission document starts weightless on purpose. Its size is not knowable
                    // until its response headers arrive, and a placeholder would be exactly the
                    // invention this slice exists to remove — so it joins the bar at its real
                    // `content-length`, one round trip in.
                    Segment::new(0),
                    Segment::new(PLANNED_TERRAIN_BYTES),
                    Segment::new(PLANNED_SATELLITE_BYTES),
                    Segment::new(PLANNED_WORLD_BYTES),
                ],
                floor: 0.0,
            }
        }

        /// Fold one report in and re-arm the high-water mark.
        pub fn apply(&mut self, ev: BootEvent) {
            match ev {
                // A zero budget is not a budget (a response with no `content-length`); taking it
                // would silently delete the segment from the bar's denominator.
                BootEvent::Budget(_, 0) => {}
                BootEvent::Budget(s, bytes) => {
                    let g = &mut self.segs[s.idx()];
                    g.total = bytes;
                    g.weight = bytes;
                }
                BootEvent::Files(s, n) => {
                    let g = &mut self.segs[s.idx()];
                    g.total = g.total.saturating_add(n);
                }
                BootEvent::Done(s, n) => {
                    let g = &mut self.segs[s.idx()];
                    g.done = g.done.saturating_add(n);
                }
                BootEvent::Finish(s) => self.segs[s.idx()].finished = true,
            }
            let raw = self.raw();
            if raw > self.floor {
                self.floor = raw;
            }
        }

        /// The weighted ratio as it stands *right now* — may be lower than [`Self::percent`] after a
        /// budget grew. Exposed for the tests that prove the difference between the two is exactly
        /// the monotonicity guarantee.
        #[must_use]
        pub fn raw(&self) -> f64 {
            if self.is_complete() {
                return 100.0;
            }
            let total_w: u64 = self.segs.iter().map(|s| s.weight).sum();
            if total_w == 0 {
                return 0.0;
            }
            #[allow(clippy::cast_precision_loss)]
            let acc: f64 = self
                .segs
                .iter()
                .map(|s| s.weight as f64 * s.fraction())
                .sum();
            #[allow(clippy::cast_precision_loss)]
            let pct = (acc / total_w as f64) * 100.0;
            // Belt and braces, and deliberately so: `Segment::fraction` already caps each term at
            // its own weight, so with both in place this clamp is unreachable. `t628_boot_progress`
            // proves that — the "cannot exceed 100" pin only goes RED when *both* are removed. Keep
            // them both: one guards a single overrunning segment, the other guards the assembled
            // total, and neither is free to assume the other is still there.
            pct.clamp(0.0, 100.0)
        }

        /// What the bar draws: 0..=100, monotonically non-decreasing for the life of the boot.
        #[must_use]
        pub fn percent(&self) -> f64 {
            self.floor
        }

        /// Every segment has reported in. The overlay may hand over only here.
        #[must_use]
        pub fn is_complete(&self) -> bool {
            self.segs.iter().all(|s| s.finished)
        }

        /// The stage named above the bar: the first unfinished segment in [`BootSeg::ALL`].
        #[must_use]
        pub fn stage(&self) -> BootSeg {
            BootSeg::ALL
                .into_iter()
                .find(|s| !self.segs[s.idx()].finished)
                .unwrap_or(BootSeg::World)
        }

        /// The line under the bar: the overall percentage, then what the current stage has actually
        /// counted. A stage that has not yet read its own budget shows the percentage alone rather
        /// than a denominator nobody measured.
        #[must_use]
        pub fn caption(&self) -> String {
            let s = self.stage();
            let g = self.segs[s.idx()];
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let pct = self.percent().floor() as u64;
            if g.total == 0 {
                return format!("{pct}%");
            }
            let detail = match s {
                BootSeg::World => fmt_files_pair(g.done.min(g.total), g.total),
                _ => fmt_bytes_pair(g.done, g.total),
            };
            format!("{pct}% · {detail}")
        }
    }

    /// `"214 / 834 files"` — the world segment counts completed fetches, so it says so instead of
    /// borrowing the byte formatter and implying a byte budget nothing published.
    #[must_use]
    pub fn fmt_files_pair(done: u64, total: u64) -> String {
        format!("{done} / {total} files")
    }

    /// `done / total` as 0..=100. Clamped at the top so a server that returns one byte more than
    /// the index promised cannot push the bar past the end of its track, and `0` for a zero total
    /// (nothing measured is nothing done, not a division).
    #[must_use]
    pub fn percent(done: u64, total: u64) -> f64 {
        if total == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let pct = (done as f64 / total as f64) * 100.0;
        pct.clamp(0.0, 100.0)
    }

    /// `"47.3 MB / 152.7 MB"` — both sides in whatever unit `total` warrants, so the pair reads as
    /// one measurement instead of switching units under the operator mid-download. Base-10 (MB, not
    /// MiB) to match the manifest's `bytes` field and every browser download UI.
    #[must_use]
    pub fn fmt_bytes_pair(done: u64, total: u64) -> String {
        #[allow(clippy::cast_precision_loss)]
        fn mb(n: u64) -> f64 {
            n as f64 / 1_000_000.0
        }
        if total >= 1_000_000 {
            format!("{:.1} MB / {:.1} MB", mb(done), mb(total))
        } else if total >= 1_000 {
            format!(
                "{} KB / {} KB",
                done.div_ceil(1_000).min(total.div_ceil(1_000)),
                total.div_ceil(1_000)
            )
        } else {
            format!("{done} B / {total} B")
        }
    }
}

/// T-723 — pure armed-placement gesture decisions.
///
/// The wasm pointer handlers in this file call these helpers; the event-SEQUENCE tests below
/// (and in `t723_armed_place`) drive the same functions through press/move/up/Esc chains so a
/// source pin cannot green-wash a regression. Lives here (not in `select_tool`) because that
/// module is `#[cfg(target_arch = "wasm32")]` and a native `cargo test -p website-frontend`
/// would never see it — the same reason `transform` sits in this file.
pub mod armed_place {
    /// What the armed `pointerup` branch should do for a given button + on-canvas bit.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ArmedUp {
        /// LMB over the map — commit the place at the release world point.
        Place,
        /// LMB over chrome / off-canvas — keep the arm (the arming click's own release, or an
        /// aborted drag back onto chrome). Esc / RMB cancel; off-canvas must NOT `cancel_pending`
        /// or click-then-click is arm-then-cancel inside one click (wave-106 MAJOR-1).
        KeepArmed,
        /// RMB — Eden stamp-mode cancel.
        Disarm,
        /// MMB — do not place; fall through so pan-end cleanup can run (wave-106 MAJOR-3).
        FallThroughPan,
        /// Any other button — ignore.
        Ignore,
    }

    /// Decide the armed pointerup action. `button` is `PointerEvent.button` (0/1/2).
    pub fn decide_armed_pointerup(button: i16, on_canvas: bool) -> ArmedUp {
        match button {
            0 if on_canvas => ArmedUp::Place,
            0 => ArmedUp::KeepArmed,
            1 => ArmedUp::FallThroughPan,
            2 => ArmedUp::Disarm,
            _ => ArmedUp::Ignore,
        }
    }

    /// A Pending/Ruler must not promote or commit unless a button is still held.
    /// `buttons` is `PointerEvent.buttons` (bitfield; 0 = none down).
    pub fn may_promote(buttons: u16) -> bool {
        buttons != 0
    }

    /// Whether an LMB press should open a `LeftGesture` while a place is armed.
    /// False: writing Pending/Ruler under an arm strands it (wave-106 MAJOR-2 / wave-108 MINOR-1 /
    /// wave-109 MINOR-5).
    pub fn open_left_gesture_while_armed(armed: bool) -> bool {
        !armed
    }

    // ── Minimal gesture machine for event-SEQUENCE tests (mirrors the host wiring) ──────────

    /// Kind of left-button gesture currently latched (test / decision mirror).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LeftKind {
        Pending,
        Ruler,
        Move,
        Marquee,
    }

    /// Observable effects a step may emit.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Effect {
        Place,
        Disarm,
        PromoteMove,
        CommitMove,
        CommitRulerVertex,
        PanDelta,
        ClearLeft,
    }

    /// Compact host state the sequence runner steps.
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub struct Host {
        pub armed: bool,
        pub left: Option<LeftKind>,
        pub pan: bool,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum Ev {
        /// Palette / picker / composition arm.
        Arm,
        PointerDown {
            button: i16,
            on_canvas: bool,
        },
        /// `buttons` = buttons bitfield after the move; `past_threshold` ≈ travel ≥ 4 px.
        PointerMove {
            buttons: u16,
            past_threshold: bool,
        },
        PointerUp {
            button: i16,
            on_canvas: bool,
        },
        Escape,
    }

    /// One step of the T-723-corrected machine. Production handlers call the same decide_* helpers.
    pub fn step(host: &mut Host, ev: Ev) -> Vec<Effect> {
        let mut out = Vec::new();
        match ev {
            Ev::Arm => {
                host.armed = true;
            }
            Ev::PointerDown {
                button,
                on_canvas: _,
            } => {
                if button == 1 {
                    host.pan = true;
                } else if button == 0 {
                    if open_left_gesture_while_armed(host.armed) {
                        // Ruler vs Pending is a tool-mode concern; sequences that care set Ruler
                        // explicitly via a prior state. Default open is Pending.
                        if host.left.is_none() {
                            host.left = Some(LeftKind::Pending);
                        }
                    }
                    // else: armed — do not latch left (T-723)
                }
            }
            Ev::PointerMove {
                buttons,
                past_threshold,
            } => {
                if host.pan {
                    out.push(Effect::PanDelta);
                    return out;
                }
                if host.armed {
                    // ghost only — no promote while armed (host returns early)
                    return out;
                }
                if let Some(LeftKind::Pending) = host.left {
                    if past_threshold {
                        if may_promote(buttons) {
                            host.left = Some(LeftKind::Move);
                            out.push(Effect::PromoteMove);
                        } else {
                            // button-less move: drop the stranded Pending (do not promote)
                            host.left = None;
                            out.push(Effect::ClearLeft);
                        }
                    }
                }
            }
            Ev::PointerUp { button, on_canvas } => {
                if host.armed {
                    // Always clear a stranded left on armed up (Pending/Ruler/…).
                    if host.left.take().is_some() {
                        out.push(Effect::ClearLeft);
                    }
                    match decide_armed_pointerup(button, on_canvas) {
                        ArmedUp::Place => {
                            out.push(Effect::Place);
                            // one-shot disarm unless caller re-arms (Ctrl keep is outside this pure step)
                            host.armed = false;
                        }
                        ArmedUp::KeepArmed => {}
                        ArmedUp::Disarm => {
                            host.armed = false;
                            out.push(Effect::Disarm);
                        }
                        ArmedUp::FallThroughPan => {
                            if host.pan {
                                host.pan = false;
                            }
                        }
                        ArmedUp::Ignore => {}
                    }
                    return out;
                }
                // unarmed
                if host.pan && button == 1 {
                    host.pan = false;
                    return out;
                }
                if button != 0 {
                    // non-LMB must not commit a left gesture
                    if let Some(k) = host.left.take() {
                        out.push(Effect::ClearLeft);
                        let _ = k;
                    }
                    return out;
                }
                match host.left.take() {
                    Some(LeftKind::Move) => out.push(Effect::CommitMove),
                    Some(LeftKind::Ruler) => out.push(Effect::CommitRulerVertex),
                    Some(LeftKind::Pending) | Some(LeftKind::Marquee) | None => {}
                }
            }
            Ev::Escape => {
                if host.armed {
                    host.armed = false;
                    out.push(Effect::Disarm);
                }
                if host.left.take().is_some() {
                    out.push(Effect::ClearLeft);
                }
            }
        }
        out
    }

    /// Run a full event sequence; returns the final host + flattened effects.
    pub fn run(mut host: Host, events: &[Ev]) -> (Host, Vec<Effect>) {
        let mut all = Vec::new();
        for &ev in events {
            all.extend(step(&mut host, ev));
        }
        (host, all)
    }
}

/// T-648 — the TRANSFORM primitives: the snap-grid quantiser, the Shift-rotate face-cursor bearing,
/// and the transformation-widget state machine. All pure (no `web_sys`, no engine, no doc), so they
/// live in this UNGATED module and are proved by `t648_transform` at the bottom of the file — the
/// same reason `boot_progress` above is a pure module (`mod select_tool` / `mod editor_ops` are both
/// `#[cfg(target_arch = "wasm32")]` in `main.rs`, so a native `cargo test -p website-frontend` never
/// compiles a test placed beside `drag_delta`; the ticket says "the quantiser goes beside
/// `drag_delta`" as a locality hint, and this is the nearest home whose behaviour a native test can
/// actually execute rather than only source-pin). The wasm gesture code in this same file calls
/// straight into here; the eventual commit still rides the existing `editor_ops::attrs_update_position`
/// field write (T-648 "a GESTURE on an existing field"), which is what this module deliberately does
/// NOT do — it only decides the numbers.
pub mod transform {
    /// The TRANSLATION snap ladder in world metres. Index 0 is **OFF** (free move — the drag delta
    /// passes through unquantised); the rest are the increasing cell sizes the ticket names
    /// (`off / 1 / 5 / 10 m`). Held as a ladder rather than a free number so `[`/`]` step between
    /// named rungs and the readout can name the active one, matching Eden's discrete grid sizes.
    pub const TRANSLATE_LADDER_M: [f64; 4] = [0.0, 1.0, 5.0, 10.0];
    /// The ROTATION snap ladder in degrees. Index 0 is **OFF** (free rotate); the rest are the
    /// ticket's `off / 5 / 15 / 45°` rungs. A Shift-rotate (or a Shift+ring drag) quantises the
    /// face-cursor bearing to the active rung; OFF commits the exact bearing.
    pub const ROTATE_LADDER_DEG: [f64; 4] = [0.0, 5.0, 15.0, 45.0];

    /// Which snap ladder a step key ([`step`]) or a quantise ([`snap_translate`]/[`snap_rotate`])
    /// acts on. The two ladders are independent (translation and rotation each carry their own live
    /// rung index), so the increase/decrease keys and the readout both name an [`Axis`].
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Axis {
        Translate,
        Rotate,
    }

    impl Axis {
        /// The ladder for this axis.
        #[must_use]
        pub const fn ladder(self) -> &'static [f64] {
            match self {
                Axis::Translate => &TRANSLATE_LADDER_M,
                Axis::Rotate => &ROTATE_LADDER_DEG,
            }
        }
        /// Unit suffix for the readout (`m` for translation, `°` for rotation).
        #[must_use]
        pub const fn unit(self) -> &'static str {
            match self {
                Axis::Translate => "m",
                Axis::Rotate => "°",
            }
        }
    }

    /// Move one rung along a ladder of `len` rungs, CLAMPED at both ends (Eden's grid keys do not
    /// wrap — pressing `]` at the coarsest rung is a no-op, not a jump back to OFF). `+1` is
    /// "increase" (a bigger/coarser step), `-1` is "decrease" (finer, toward OFF at index 0).
    /// `delta` is the raw key direction; only its sign matters.
    #[must_use]
    pub fn step(cur: usize, len: usize, delta: i32) -> usize {
        if len == 0 {
            return 0;
        }
        let last = len - 1;
        if delta > 0 {
            (cur + 1).min(last)
        } else if delta < 0 {
            cur.saturating_sub(1)
        } else {
            cur.min(last)
        }
    }

    /// Quantise `value` to the nearest multiple of `step`. `step <= 0` (the OFF rung) is a
    /// **passthrough** — the value is returned exactly, which is how "snap off" reads at the call
    /// site with no branch. Round-half-away-from-zero so a delta exactly between two cells lands on
    /// the farther one symmetrically for + and −. Non-finite `step` is treated as OFF.
    #[must_use]
    pub fn snap_value(value: f64, step: f64) -> f64 {
        if !step.is_finite() || step <= 0.0 {
            return value;
        }
        (value / step).round() * step
    }

    /// Quantise a TRANSLATION delta component (metres) at ladder rung `rung`. Rung 0 = OFF =
    /// passthrough. Applied per-axis to `(dx, dy)` so a snapped drag lands the entity on the grid
    /// lattice while a free drag (rung 0) is byte-for-byte the old `drag_delta`.
    #[must_use]
    pub fn snap_translate(value: f64, rung: usize) -> f64 {
        snap_value(value, *TRANSLATE_LADDER_M.get(rung).unwrap_or(&0.0))
    }

    /// Quantise a ROTATION (degrees) at ladder rung `rung`, then normalise to `[0,360)` (the same
    /// range `update_slot_position` stores). Rung 0 = OFF = the exact bearing, still normalised.
    #[must_use]
    pub fn snap_rotate(deg: f64, rung: usize) -> f64 {
        let snapped = snap_value(deg, *ROTATE_LADDER_DEG.get(rung).unwrap_or(&0.0));
        norm_deg(snapped)
    }

    /// Normalise degrees into `[0,360)` — the canonical rotation range (matches
    /// `MissionDocCore::update_slot_position`, which does `((r % 360)+360)%360`). Non-finite → 0.
    #[must_use]
    pub fn norm_deg(deg: f64) -> f64 {
        if !deg.is_finite() {
            return 0.0;
        }
        ((deg % 360.0) + 360.0) % 360.0
    }

    /// The face-cursor BEARING (XFORM-SHIFT-001): the yaw a slot at `(from_x, from_y)` must take to
    /// point at the cursor `(to_x, to_y)`, in the document's convention — **yaw clockwise from north
    /// (+Y)**, the exact convention `world::glyph_math::deck_angle_for_rotation_deg` inverts for the
    /// screen and the spawn export reads as `headingDeg`. Compass bearing = `atan2(east, north) =
    /// atan2(dx, dy)`, normalised to `[0,360)`:
    ///   * cursor due north (dx=0, dy>0) → 0°
    ///   * due east  (dx>0, dy=0) → 90°
    ///   * due south (dy<0)       → 180°
    ///   * due west  (dx<0, dy=0) → 270°  (the wrap case)
    /// A degenerate aim (cursor exactly on the pivot, or a non-finite input) returns `None` — the
    /// caller leaves the rotation unchanged rather than committing a meaningless 0°.
    #[must_use]
    pub fn bearing_to_face(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> Option<f64> {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        if !dx.is_finite() || !dy.is_finite() || (dx == 0.0 && dy == 0.0) {
            return None;
        }
        Some(norm_deg(dx.atan2(dy).to_degrees()))
    }

    /// Format a ladder step for the readout without a trailing `.0` on a whole number (`5.0 → "5"`,
    /// `2.5 → "2.5"`). Small and local so the readout has no `{:g}`-style dependency.
    #[must_use]
    pub fn fmt_step(v: f64) -> String {
        if (v - v.round()).abs() < 1e-9 {
            format!("{}", v.round() as i64)
        } else {
            let s = format!("{v:.2}");
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
    }

    /// The transformation-widget VARIANT (`WIDGET-CYCLE-001` / `WIDGET-TRANS-001`). Eden cycles
    /// these with `Space`; TBD keeps `Space` as flyTo (`center_on_selection`) and uses the free
    /// `1`/`2`/`3` direct keys instead (the ticket's collision decision — Eden's `1`-`5` are unbound
    /// here). Three variants exist, numbered to MATCH Eden's widget row exactly (T-795, pixel-verified
    /// against Eden frames 164038-164107 — Eden reads `No Widget (1) / Translation (2) / Rotation (3)
    /// / Area Scaling (4) / Area (5)`): **None** (no gizmo — a bare drag still translates, Eden's
    /// widget-less semantics), **Translate** (axis arrows, axis-constrained drag) and **Rotate** (a
    /// ring, drag on the ring = rotate, Shift+drag = snap to the rotation ladder). An Eden author's
    /// muscle memory — 1 drops the gizmo, 2 arms translate, 3 arms rotate — now lands right; the whole
    /// point of taking Eden's keys is that they mean what Eden means.
    ///
    /// **No area-scale variant (4/5 reserved-unbound), and that is scoped honestly.** The widget acts
    /// on the live SELECTION, which the select machine only ever fills with slot + vehicle ids
    /// (`pick_slot_or_vehicle` / `marquee_ids_with_vehicles`). Neither a slot nor a vehicle carries
    /// a scalar size — only zones and triggers have a radius, and those live in their own
    /// collections edited by the zone-draw tool, never in `selection`. So `4`/`5` (area-scale / area)
    /// have nothing in a transform selection to scale; offering them would be a dead key. They stay
    /// RESERVED-UNBOUND (no keydown arm, no help row) for a later slice with a scalable target — the
    /// numbering matches Eden so that later slice can bind them without renumbering again.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub enum WidgetVariant {
        /// **No Widget** (Eden's `1`) — no gizmo is drawn. A drag on the selection still translates
        /// it (the LG::Move path is variant-independent), which is Eden's widget-less drag-move.
        None,
        /// **Translate** (Eden's `2`) — axis arrows; a drag on an arrow is the axis-constrained move.
        /// The default on mount so a fresh selection shows a handle rather than nothing.
        #[default]
        Translate,
        /// **Rotate** (Eden's `3`) — a ring; a drag on the ring rotates the selection about its
        /// centre, Shift+drag snaps to the rotation ladder.
        Rotate,
    }

    impl WidgetVariant {
        /// The `1`/`2`/`3` direct-key selection (Eden's variant keys, minus Space). Matches Eden's
        /// widget row exactly: `1` → No Widget, `2` → Translate, `3` → Rotate; any other digit leaves
        /// the variant unchanged (returns `self`). Digit keys `4`/`5` are deliberately inert here —
        /// reserved-unbound (see the type doc on area-scale). Also the seam T-799's toolbar buttons
        /// drive through the `set_widget` dispatch, so a click and a chord agree on the mapping.
        #[must_use]
        pub fn from_digit(self, digit: u8) -> Self {
            match digit {
                1 => WidgetVariant::None,
                2 => WidgetVariant::Translate,
                3 => WidgetVariant::Rotate,
                _ => self,
            }
        }
        /// The `1`/`2`/`3` digit that SELECTS this variant — the inverse of [`from_digit`]. Drives the
        /// toolbar's three-way toggle-plate active state (T-797/T-799 read it through the dispatch) and
        /// the cursor-adjacent mode hint. `None → 1`, `Translate → 2`, `Rotate → 3`.
        #[must_use]
        pub const fn to_digit(self) -> u8 {
            match self {
                WidgetVariant::None => 1,
                WidgetVariant::Translate => 2,
                WidgetVariant::Rotate => 3,
            }
        }
        /// The short label for the cursor-adjacent mode hint / any chrome that names the active mode.
        #[must_use]
        pub const fn label(self) -> &'static str {
            match self {
                WidgetVariant::None => "No Widget",
                WidgetVariant::Translate => "Translate",
                WidgetVariant::Rotate => "Rotate",
            }
        }
        /// Whether a drag on this variant's ring rotates (only Rotate has a ring). Translate's arrows
        /// and the None widget-less drag both move instead. Used by the widget gesture to decide
        /// whether a press on the ring band opens a rotate, and which ladder a Shift constrains.
        #[must_use]
        pub const fn is_rotate(self) -> bool {
            matches!(self, WidgetVariant::Rotate)
        }
        /// The snap [`Axis`] this variant's step keys (`[`/`]`) tune: Rotate → the rotation ladder,
        /// Translate AND None → the translation ladder (None has no widget of its own, but a bare
        /// drag still translates, so the translation grid is the meaningful one to step). One mapping
        /// so the keydown and the readout agree on "which grid am I stepping".
        #[must_use]
        pub const fn snap_axis(self) -> Axis {
            match self {
                WidgetVariant::None | WidgetVariant::Translate => Axis::Translate,
                WidgetVariant::Rotate => Axis::Rotate,
            }
        }
    }

    /// T-795 — the transform widget's fixed screen radius (px). The gizmo is a screen affordance, not
    /// a world object, so it stays a constant size like a cursor (Eden's widget does too). ONE const so
    /// the SVG overlay render and the gesture ring hit-test agree on where the ring is — the whole
    /// reason the ring was "pure decoration" before was that nothing on the gesture side knew its
    /// geometry, so a drag on the drawn ring fell through to the marquee. They must not diverge again.
    pub const WIDGET_RADIUS_PX: f64 = 42.0;
    /// T-795 — how far (px) a press may sit from the ring's stroke and still count as ON the ring. A
    /// forgiving band (the stroke is 2 px; a bare pixel-exact test is un-hittable), symmetric so a
    /// press just inside or just outside the drawn circle both grab it.
    pub const RING_HIT_TOL_PX: f64 = 10.0;

    /// T-795 — is a press at `(px, py)` on the rotate RING drawn around the projected pivot
    /// `(cx, cy)`? True when the press is within [`RING_HIT_TOL_PX`] of the ring's radius
    /// [`WIDGET_RADIUS_PX`]. This is the hit-test the gesture runs (in Rotate mode, with a live
    /// selection) BEFORE the marquee arm, so a drag on the ring rotates instead of marquee-ing away
    /// the selection. Pure geometry — DOM-free — so it is natively unit-tested.
    #[must_use]
    pub fn press_on_ring(px: f64, py: f64, cx: f64, cy: f64) -> bool {
        let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
        (d - WIDGET_RADIUS_PX).abs() <= RING_HIT_TOL_PX
    }

    /// The live snap-grid state: one rung index per ladder plus a grid-ENABLED latch (KEY-GRID-001,
    /// the `G` toggle). `enabled=false` forces both quantisers to passthrough regardless of rung, so
    /// `G` is a single master switch over "is the grid on at all" while `[`/`]` tune the rung the
    /// switch gates. Copy so the wasm host can hold it in a `Cell` and the readout can snapshot it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct SnapState {
        pub enabled: bool,
        pub translate_rung: usize,
        pub rotate_rung: usize,
    }

    impl Default for SnapState {
        /// Default OFF: grid disabled, both ladders parked at OFF (rung 0). An operator who never
        /// touches `G`/`[`/`]` gets the exact pre-T-648 free move + free rotate.
        fn default() -> Self {
            Self {
                enabled: false,
                translate_rung: 0,
                rotate_rung: 0,
            }
        }
    }

    impl SnapState {
        /// The EFFECTIVE translation rung: the tuned rung when the grid is enabled, else OFF (0).
        /// One place decides "grid off ⇒ passthrough" so the gesture never has to special-case it.
        #[must_use]
        pub fn effective_translate_rung(self) -> usize {
            if self.enabled {
                self.translate_rung
            } else {
                0
            }
        }
        /// The EFFECTIVE rotation rung (grid-gated exactly like translation).
        #[must_use]
        pub fn effective_rotate_rung(self) -> usize {
            if self.enabled {
                self.rotate_rung
            } else {
                0
            }
        }
        /// Toggle the grid master latch (the `G` key). Rungs are preserved so toggling off then on
        /// restores the operator's chosen steps.
        #[must_use]
        pub fn toggled(self) -> Self {
            Self {
                enabled: !self.enabled,
                ..self
            }
        }
        /// Step one axis's rung by `delta` (the `[`/`]` keys), clamped at both ends. Turning a rung
        /// does NOT flip `enabled`: adjusting a step while the grid is off just parks the new rung
        /// for when it is switched on (Eden keeps the two controls orthogonal).
        #[must_use]
        pub fn stepped(self, axis: Axis, delta: i32) -> Self {
            match axis {
                Axis::Translate => Self {
                    translate_rung: step(self.translate_rung, TRANSLATE_LADDER_M.len(), delta),
                    ..self
                },
                Axis::Rotate => Self {
                    rotate_rung: step(self.rotate_rung, ROTATE_LADDER_DEG.len(), delta),
                    ..self
                },
            }
        }
        /// Human readout of one rung for the status bar (the T-636 readout idiom), e.g. `"5 m"`,
        /// `"15°"`, or `"off"`. `off` is spelled out rather than `0` so the operator reads intent.
        /// The number is printed without a trailing `.0` (the ladders are whole numbers today, but
        /// [`fmt_step`] keeps it clean if a fractional rung is ever added).
        #[must_use]
        pub fn rung_label(self, axis: Axis) -> String {
            let rung = match axis {
                Axis::Translate => self.effective_translate_rung(),
                Axis::Rotate => self.effective_rotate_rung(),
            };
            let step = axis.ladder().get(rung).copied().unwrap_or(0.0);
            if step <= 0.0 {
                "off".to_string()
            } else if axis == Axis::Rotate {
                format!("{}{}", fmt_step(step), axis.unit())
            } else {
                format!("{} {}", fmt_step(step), axis.unit())
            }
        }
        /// The full status-bar readout: `"SNAP  move 5 m · rot 15°"` when enabled, `"SNAP  off"`
        /// when the master latch is off. One string so the overlay is a single text node (the
        /// scale-bar / ruler-status idiom).
        ///
        /// O-10 — the chip is labelled `SNAP`, not `GRID`. It reports the SNAP grid (the move/rot
        /// step this latch quantises to), and it sits in the same band as the map-grid labels; naming
        /// it `GRID` made an operator read "the map grid is off" when only snapping was.
        #[must_use]
        pub fn status_readout(self) -> String {
            if !self.enabled {
                return "SNAP  off".to_string();
            }
            format!(
                "SNAP  move {} \u{b7} rot {}",
                self.rung_label(Axis::Translate),
                self.rung_label(Axis::Rotate),
            )
        }
    }
}

/// T-797 — the transformation-widget / snap-grid dispatch the row-2 toolbar + Edit menu drive.
///
/// The live state (`widget_variant` / `snap`) and the canvas `container` (whose CSS rect Select All
/// needs) are `!Send` `RwSignal`s / a DOM handle owned by `MissionEditorPage`'s wasm keydown closure
/// (T-648/T-795). `eden_top_strip` is native-compiled and another slice's `owns`, so it cannot reach
/// them directly — the exact shape [`register_widget_pivot`] already solves for the transform-widget
/// pivot. This is its peer: the editor registers callable INVOKERS at mount; the strip's buttons call
/// them (write path), and reads two GETTERS for the toggle-active plate. Native builds / pre-mount see
/// `None` and the strip's buttons no-op, exactly like `read_widget_pivot`.
///
/// The five invokers mirror the keydown arms one-for-one so a click and the chord do the same thing:
/// `set_widget` (Digit1/2 → `from_digit`), `toggle_snap` (`G`), `snap_step` (`[`/`]`), and
/// `select_all` (Ctrl+A — the closure captures the container, so the button need not measure it).
#[cfg(target_arch = "wasm32")]
type ToolbarDispatch = std::rc::Rc<EditorToolbarDispatch>;

#[cfg(target_arch = "wasm32")]
pub(crate) struct EditorToolbarDispatch {
    /// Select the widget variant from its `1`/`2`/`3` digit (mirror of the Digit1/Digit2/Digit3
    /// arms): `1` → No Widget, `2` → Translate, `3` → Rotate (T-795). This is the seam T-799's
    /// No-Widget button in the row-2 cluster calls — it invokes `set_widget(1)` to arm No Widget,
    /// exactly as the `1` chord does. `4`/`5` are reserved-unbound and a no-op through here too.
    pub set_widget: Box<dyn Fn(u8)>,
    /// Toggle the snap-grid master latch (mirror of the `G` arm).
    pub toggle_snap: Box<dyn Fn()>,
    /// Step the ACTIVE widget's snap ladder by ±1 (mirror of the `[`/`]` arms).
    pub snap_step: Box<dyn Fn(i32)>,
    /// Select every entity in the viewport (mirror of the Ctrl+A arm — the closure owns the rect).
    pub select_all: Box<dyn Fn()>,
    /// The ACTIVE widget variant's selecting digit — `1` No Widget / `2` Translate / `3` Rotate
    /// (`WidgetVariant::to_digit`). T-795: the row-2 cluster is now THREE mutually-exclusive buttons,
    /// so a single `is_rotate` boolean can no longer light the right one; each button lights its plate
    /// when this equals its own digit. Tracked, so a chord-driven flip re-renders every plate.
    pub widget_digit: Box<dyn Fn() -> u8>,
    /// Is the ACTIVE widget variant Rotate? Retained for the T-797 Rotate plate that reads it directly;
    /// `widget_digit() == 3` is the same predicate. (Translate/None are the complement.)
    pub widget_is_rotate: Box<dyn Fn() -> bool>,
    /// Is the snap grid enabled? Drives the snap-grid button's active state.
    pub snap_enabled: Box<dyn Fn() -> bool>,
}

thread_local! {
    /// T-797 — the registered toolbar dispatch (set once at mount). Peer of [`WIDGET_PIVOT`]; the
    /// native strip reads it through the `read_*` shims below and simply sees `None` off-wasm.
    #[cfg(target_arch = "wasm32")]
    static TOOLBAR_DISPATCH: std::cell::RefCell<Option<ToolbarDispatch>> =
        const { std::cell::RefCell::new(None) };

    /// T-797 wave-202 — the dispatch's REACTIVE PRESENCE, so the strip's toggle-plate closures can
    /// subscribe to it from frame one — BEFORE any dispatch is registered — and re-run when one lands.
    ///
    /// Why this exists: the strip renders (and runs its `class=move || …` plate closures) BEFORE this
    /// `on_load` registers the dispatch. Those closures read `widget_variant` / `snap` ONLY *through*
    /// [`with_editor_toolbar_dispatch`], which is a no-op while the dispatch is `None` — so on their
    /// first, and (without a signal to depend on) ONLY, run they touch no tracked signal and Leptos
    /// never re-runs them. The plate froze at its first-render default: Translate stuck lit, Snap dark.
    ///
    /// The fix is a signal the closure reads FIRST, unconditionally: an [`ArcRwSignal<u32>`]
    /// generation counter, bumped every time the dispatch is registered OR cleared. `ArcRwSignal`
    /// (not `RwSignal`) is deliberate — it is reference-counted and owner-INDEPENDENT, so it lives for
    /// the process and survives the editor's unmount/remount (a mission switch) without being disposed
    /// under the plate closures. When a bump lands, the closures re-run; NOW the dispatch is present,
    /// so they call the getters, which do a TRACKED `.get()` on `widget_variant` / `snap` and subscribe
    /// to the real state — after which every chord-driven flip propagates on its own.
    ///
    /// Created lazily on first access (an `ArcRwSignal` cannot be a `const` initializer). The signal
    /// itself carries no wasm dependency, so it exists on both targets; only register/unregister (which
    /// need the `!Send` host state) are wasm-gated.
    static TOOLBAR_DISPATCH_GEN: std::cell::RefCell<Option<ArcRwSignal<u32>>> =
        const { std::cell::RefCell::new(None) };
}

/// T-797 wave-202 — the toolbar-dispatch generation signal (see [`TOOLBAR_DISPATCH_GEN`]), created on
/// first access. The strip's plate closures read this BEFORE [`with_editor_toolbar_dispatch`] so they
/// subscribe from frame one and re-run when a dispatch registers/unregisters; the bridge bumps it.
pub(crate) fn toolbar_dispatch_generation() -> ArcRwSignal<u32> {
    TOOLBAR_DISPATCH_GEN.with(|c| {
        c.borrow_mut()
            .get_or_insert_with(|| ArcRwSignal::new(0))
            .clone()
    })
}

/// Bump the generation → re-run every subscribed plate closure. Untracked read of the current value
/// so the bump itself never subscribes the bumper.
#[cfg(target_arch = "wasm32")]
fn bump_toolbar_dispatch_generation() {
    let gen = toolbar_dispatch_generation();
    gen.set(gen.get_untracked().wrapping_add(1));
}

/// T-797 — register the row-2 / Edit-menu dispatch (called at each mount, wasm-only: only the host
/// has the `!Send` signals + the container to close over). Peer of [`register_widget_pivot`]. Bumping
/// the generation is what makes the plate reactive: the strip's closures — subscribed to the
/// generation from their first render — re-run now that a dispatch is present and subscribe to the
/// tracked widget/snap getters themselves.
#[cfg(target_arch = "wasm32")]
fn register_editor_toolbar_dispatch(d: ToolbarDispatch) {
    TOOLBAR_DISPATCH.with(|c| *c.borrow_mut() = Some(d));
    bump_toolbar_dispatch_generation();
}

/// T-797 wave-202 — drop the registered dispatch on route-leave (editor unmount) and bump, so a
/// re-mount (mission switch) never leaves the plate closures reading a stale, disposed dispatch: the
/// bump re-runs them against `None` (all plates dark) until the fresh `on_load` re-registers and bumps
/// again. Wired via `on_cleanup` at the register site — the T-189 unload-guard teardown pattern.
#[cfg(target_arch = "wasm32")]
fn unregister_editor_toolbar_dispatch() {
    TOOLBAR_DISPATCH.with(|c| *c.borrow_mut() = None);
    bump_toolbar_dispatch_generation();
}

/// T-797 — the strip's INVOKE seam. Runs `f` against the registered dispatch when it is present
/// (wasm, post-mount); a no-op otherwise. `eden_top_strip` calls the four verbs through this.
#[cfg(target_arch = "wasm32")]
pub(crate) fn with_editor_toolbar_dispatch(f: impl FnOnce(&EditorToolbarDispatch)) {
    TOOLBAR_DISPATCH.with(|c| {
        if let Some(d) = c.borrow().as_ref() {
            f(d);
        }
    });
}

/* ═══════ T-934.10 — the pure render-sync helper belt moved to `editor/canvas/render_sync.rs` ════
 *
 * The T-780 connection lane, the T-784/T-796 comment lane + picks, the T-760/T-790 marker lane
 * parse, the T-802 hover state machine, the T-754/wave-129 route resolution, the wave-145
 * selection universe, the T-819 crew-hide SoA filter and the T-743 paste anchor all live there
 * now (re-exported above under their old names). What remains below are the wasm-side wrappers
 * that bind those pure helpers to the live document and DOM — each one reads `editor_ops`'
 * OPS_CTX or `web_sys`, which is the line the split is drawn on.
 */

/// The live document's drawable edges: [`connection_segments`] fed from `MissionDocCore` itself.
///
/// **This is the document read the lane and the pick share** — one function, called by the `doc_tick`
/// Effect that binds the lane AND by the pointer arm that hit-tests it, so what is on screen and what
/// a click can find are the same set by construction. Positions come from the materialized SoA
/// (slots) and `editor_ops::vehicle_points` (vehicles) — the same two sources the slot/vehicle pick
/// uses, and the same set the engine actually draws, so an edge to a slot hidden by the T-665 layer
/// filter has no line and no hit box, matching the entity it points at.
///
/// Only x/y are read. `SlotSoa::xy` is f32 and that is correct here: these coordinates are on their
/// way to a f32 vertex buffer and to a screen-space distance test, not to a `position` write — the
/// wave-127 "read z off `slots_json`, not the SoA" rule is about the z this function never touches.
#[cfg(target_arch = "wasm32")]
#[must_use]
fn live_connection_segments(core: &map_engine_core::doc::MissionDocCore) -> Vec<ConnSegment> {
    let soa = core.materialize();
    let mut positions: std::collections::HashMap<String, (f64, f64)> =
        std::collections::HashMap::with_capacity(soa.ids.len());
    for (i, id) in soa.ids.iter().enumerate() {
        positions.insert(
            id.clone(),
            (f64::from(soa.xy[i * 2]), f64::from(soa.xy[i * 2 + 1])),
        );
    }
    for (id, x, y) in editor_ops::vehicle_points() {
        positions.insert(id, (x, y));
    }
    connection_segments(&core.connection_rows_json(), &positions)
}

/// T-802 — write the map cursor onto the CANVAS (not the container): the canvas is the element the
/// pointer actually hits over the map — the chrome above it is `pointer-events-none` — so the claim
/// lands on the hit element and cannot be inherited by a chrome panel the pointer moves onto.
///
/// `web_sys::HtmlElement::style` is called UFCS-style on purpose: Leptos's `ElementExt::style(S)` is
/// also in scope on this type and wins ordinary method resolution, which is a compile error rather
/// than a silent one — but naming the intended trait keeps it that way for the next editor too.
#[cfg(target_arch = "wasm32")]
fn set_map_cursor(canvas: &web_sys::HtmlCanvasElement, pickable: bool) {
    let _ = web_sys::HtmlElement::style(canvas).set_property("cursor", hover_cursor_css(pickable));
}

/// T-802 — the pick's point sets, materialised ONCE per document generation.
///
/// `tick` is `doc_tick`, the counter `editor_ops::refresh_docks` bumps at the end of every commit
/// (and the render lanes re-bind on). Cache-per-generation rather than cache-per-move is what keeps
/// a 25 Hz hover off the T-057 cliff: the expensive half of a pick is `materialize()` (a full Y.Doc
/// read), not the radius query.
#[cfg(target_arch = "wasm32")]
pub(crate) struct HoverPoints {
    tick: u64,
    soa: map_engine_core::doc::SlotSoa,
    vehicles: Vec<(String, f64, f64)>,
    comments: Vec<CommentPoint>,
}

/// T-802 — is something PICKABLE under screen pixel `(px, py)`? Refreshes `cache` when `tick` has
/// moved, then runs the click path's own pick over it.
///
/// Precedence does not matter here (the answer is a bool, not an id), but the SOURCES do: slots +
/// vehicles through `select_tool::pick_slot_or_vehicle`, then comments through [`pick_comment`]
/// with the tolerance derived by unprojecting two points [`COMMENT_PICK_PX`] apart — byte-for-byte
/// the fold the T-796 drag arm and the T-784 click path run. Markers are deliberately ABSENT: they
/// have no selection route at all (see the `route_target` notes), so a pointer cursor over one
/// would be precisely the `cursor-pointer`-over-a-dead-click lie T-754 was filed for.
#[cfg(target_arch = "wasm32")]
pub(crate) fn hover_hit(
    cache: &mut Option<HoverPoints>,
    tick: u64,
    doc: &mission_doc::DocHandle,
    cam: &map_engine_core::camera::OrthoCamera,
    px: f64,
    py: f64,
) -> bool {
    if cache.as_ref().is_none_or(|c| c.tick != tick) {
        // Both reads are shared borrows of the same `RefCell` (`vehicle_points` goes through
        // `OPS_CTX` to this very doc) — exactly how the click path already nests them.
        let fresh = doc.borrow().as_ref().map(|c| HoverPoints {
            tick,
            // T-819 — map pick cannot hit a crewed figure (nothing rendered).
            soa: map_render_slot_soa(c),
            vehicles: editor_ops::vehicle_points(),
            comments: comment_points(&c.comments_json()),
        });
        let Some(fresh) = fresh else { return false };
        *cache = Some(fresh);
    }
    let Some(pts) = cache.as_ref() else {
        return false;
    };
    if crate::editor::tools::select_tool::pick_slot_or_vehicle(cam, &pts.soa, &pts.vehicles, px, py)
        .is_some()
    {
        return true;
    }
    let w = cam.unproject_xy(px, py);
    let w2 = cam.unproject_xy(px + COMMENT_PICK_PX, py);
    let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
    pick_comment(&pts.comments, w[0], w[1], tol).is_some()
}

/// Wave 129 — [`route_target`] bound to the LIVE document and its world centre: "what would a click
/// on this subject id find, and where?". Shared as one `Rc` by the click (which acts on the answer)
/// and by `validation_panel`'s affordance probe (which only asks), so the two cannot drift apart.
#[cfg(target_arch = "wasm32")]
type SubjectResolver = std::rc::Rc<dyn Fn(&str) -> Option<(RouteTarget, f64, f64)>>;

#[component]
pub fn MissionEditorPage() -> impl IntoView {
    let container_ref = NodeRef::<leptos::html::Div>::new();
    let canvas_ref = NodeRef::<leptos::html::Canvas>::new();

    // T-159.20 — Save Version + Export controls. The signals live on both targets (the view binds
    // them); the doc-touching command bodies are wasm-gated (native has no `MissionDocCore`).
    let save_semver = RwSignal::new("0.1.0".to_string());
    let save_status = RwSignal::new(String::new());

    // T-159.21 — Eden chrome state. All four are HUD *mirrors* of non-reactive state (the doc's undo
    // stack, its slot count, the leaked selection handle): `MissionDocCore` has no change
    // subscription and the selection is an `Rc<RefCell<…>>`, so `mission_history::refresh_*` pushes
    // onto these at every mutation site instead. `cursor` is fed by the pointer-move unproject.
    // Declared on both targets — the view binds them; only the wasm block ever sets them.
    let can_undo = RwSignal::new(false);
    let can_redo = RwSignal::new(false);
    let obj_count = RwSignal::new(0usize);
    let sel_count = RwSignal::new(0usize);
    let cursor = RwSignal::new(None::<(f64, f64, Option<f64>)>);
    // T-172 B9 — toolbelt SZ estimate (recomputed 500 ms after obj_count settles) + the
    // bottom-right wgpu debug readout (fed ~1 Hz by the rAF loop).
    let sz_bytes = RwSignal::new(None::<usize>);
    let debug_hud = RwSignal::new(String::new());
    // T-635 — the debug HUD (`z … · c… · glyph … · … FPS · rf …ms`) is TELEMETRY: a performance
    // read-out that rides the toolbelt area but is not part of authoring. It defaults HIDDEN and is
    // toggled by Ctrl+Alt+D in the editor keydown (matching the historical FpsCounter binding that
    // died with the React app at T-159.29.3 — recreated here, not reused). When shown it lives in
    // its OWN bottom-right slot (below), never inside the toolbelt wrapper, so it can never overlap
    // the CUR/OBJ readouts again.
    //
    // framework_synthesis §D.4 #7 — this key-gating is correct BECAUSE the HUD is telemetry.
    // Mission-correctness diagnostics (validation errors, coherency warnings) are NEVER gated behind
    // a keypress: the author must not be able to hide the reason their mission is broken. Do not copy
    // this "hide it behind a key" pattern onto validation output.
    let debug_hud_shown = RwSignal::new(false);
    // T-670 (STATUS-ZOOM-001) — the status bar's metres-per-pixel readout, and the single scale
    // number the T-667 scale bar now sizes from. `RenderEngine::zoom()` is reachable ONLY from the
    // rAF sampler (`start_raf`), so the signal has to be born here and be written there; seeded with
    // the editor's default deck zoom (−2 ⇒ 4.00 m/px) so the cell reads a real value before the
    // engine mounts and on native, where there is no engine at all.
    //
    // The sampler runs EVERY FRAME. It writes this signal only when `format_m_per_px` would change
    // (guard in `start_raf`), so a still or merely panning camera writes nothing and the status bar
    // never re-renders per frame — the regression the `rf <ms>` cell above exists to surface.
    let scale_mpp = RwSignal::new(crate::editor::panels::toolbelt::m_per_px(-2.0));
    // T-642/T-643 — the active editor tool (Select ⇆ Ruler ⇆ LoS). The `ModeToolbar` buttons read +
    // set it (the active tool enters TOOL_ACTIVE state, Select returns); the wasm pointer handlers
    // branch on it to choose the point-capture gesture (Ruler AND LoS share `LG::Ruler`) vs the
    // Select machine, and the commit site routes a captured click by `is_ruler()`/`is_los()`. Default
    // Select.
    let tool_mode = RwSignal::new(crate::editor::tools::ruler_tool::EditorTool::Select);
    // T-644 — the LoS SUB-MODE (Ray ⇆ Viewshed). The `ModeToolbar` LoS button reads it (to reflect
    // the active sub-mode in its title/label) and toggles it on a re-click while LoS is already
    // active; the wasm pointer commit reads `get_untracked()` to route a captured LoS click to the
    // ray two-click capture or the one-shot viewshed placement. A plain reactive signal (like
    // `tool_mode`), shared between the toolbar and the pointer handlers — no thread_local. Default Ray.
    let los_mode = RwSignal::new(crate::editor::tools::los_tool::LosMode::default());
    // T-642 — the ruler's status-bar readout (running total + last-leg) and a repaint tick. The
    // `RulerChain` itself is session-local overlay state held in a leaked `RefCell` in the wasm
    // block below (Decision 4 — NOT the Y.Doc); these two signals are the reactive surface the DOM
    // reads: `ruler_status` feeds `StatusBar`, `ruler_tick` is bumped on every chain mutation so the
    // `RulerOverlay` repaints even when a click did not move the pointer (no pointermove to ride).
    let ruler_status = RwSignal::new(None::<String>);
    let ruler_tick = RwSignal::new(0u64);
    // T-643 — LoS repaint tick. The `LosState` (observer/target capture) is session-local overlay
    // state held beside the ruler chain in the wasm block (Decision 4 — NOT the Y.Doc). Unlike the
    // ruler, LoS puts its verdict in an INLINE panel by the target (Decision 2), not the status bar,
    // so there is no `los_status` signal — only this tick, bumped on every capture mutation so the
    // `LosOverlay` repaints even when a click did not move the pointer.
    let los_tick = RwSignal::new(0u64);
    // T-648 — the snap-grid state (translation + rotation ladders + the `G` master latch) and the
    // transform-widget variant (`1` translate / `2` rotate). Both are plain reactive signals read by
    // three places without a thread_local mirror: the window keydown (which toggles the grid, steps
    // the rungs, and cycles the variant), the wasm pointer handlers (which read `get_untracked()` to
    // quantise a Shift-rotate / widget-ring drag), and the DOM overlays (the status-bar SNAP readout
    // + the widget SVG, which re-run on `.get()`). The default `SnapState` is OFF, so an operator who
    // never presses `G`/`[`/`]` gets the exact pre-T-648 free move + free rotate.
    let snap = RwSignal::new(crate::editor::mission_editor::transform::SnapState::default());
    let widget_variant =
        RwSignal::new(crate::editor::mission_editor::transform::WidgetVariant::default());
    // T-648 — repaint tick for the transform widget, bumped whenever the SELECTION changes without a
    // pointermove (a keyboard select-all, an outliner click), so the widget SVG re-projects onto the
    // new selection centroid even with a still pointer — the `ruler_tick`/`los_tick` idiom.
    let widget_tick = RwSignal::new(0u64);
    // T-175 B5 — boot loading overlay phase (set by the wasm boot tasks; the view reads it).
    let boot = RwSignal::new(BootPhase::Hydrating);
    // T-631 — "continue without map": once the render engine fails to start, the map pane is dead
    // for the life of this mount (the engine is `None`, the rAF loop never started, every engine
    // call no-ops). This holds the reason so that, after the operator dismisses the error overlay
    // to keep working on the doc, a persistent labelled badge sits over the dead canvas instead of
    // a blank void that reads as a rendering bug. `Some(reason)` from the instant `create` returns
    // `Err`; never cleared (Retry is a full reload, which rebuilds this fresh). Declared on both
    // targets — the view reads it; only the wasm engine-init `Err` arm sets it.
    let map_disabled = RwSignal::new(None::<String>);
    // T-628 — the one bar. Written only by the two boot tasks, through the `ProgressFn` built below,
    // and only with work that has already completed: there is no timer anywhere on this path, so a
    // stalled network shows a stalled bar, which is the point.
    let progress = RwSignal::new(boot_progress::BootProgress::new());
    #[cfg(target_arch = "wasm32")]
    {
        use std::cell::Cell;
        use std::rc::Rc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        let timer: Rc<Cell<Option<i32>>> = Rc::new(Cell::new(None));
        Effect::new(move |_| {
            let _ = obj_count.get();
            let Some(win) = web_sys::window() else { return };
            if let Some(id) = timer.get() {
                win.clear_timeout_with_handle(id);
            }
            let timer2 = timer.clone();
            let cb = Closure::once_into_js(move || {
                timer2.set(None);
                sz_bytes.set(
                    editor_ops::slots_json()
                        .as_deref()
                        .and_then(crate::editor::mission_size::estimate_compiled_bytes),
                );
            });
            if let Ok(id) = win.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                500,
            ) {
                timer.set(Some(id));
            }
        });
    }

    // T-159.22 — dock state. `outliner_nodes` / `selected_ids` are the same kind of pull-mirror as
    // OBJ/SEL above (pushed by `editor_ops::refresh_docks` from `mission_history::refresh_signals`,
    // i.e. at every mutation site). `active_layer` is the drop target (React's `activeLayerId`);
    // `catalog` holds the `/registry` fetch state and never leaves `Loading` on the native shell,
    // where `api_get` doesn't exist.
    let outliner_nodes = RwSignal::new(Vec::<crate::editor::panels::outliner::OutlinerNode>::new());
    // T-168 — the ORBAT dock tree mirror (faction/squad/slot), rebuilt alongside `outliner_nodes`.
    let orbat_nodes = RwSignal::new(Vec::<crate::editor::panels::outliner::OutlinerNode>::new());
    let selected_ids = RwSignal::new(Vec::<String>::new());
    // T-648 — bump the transform-widget repaint tick whenever the selection changes (any source:
    // outliner click, keyboard, marquee), so the gizmo re-projects onto the new centroid even with a
    // still pointer. `selected_ids` is the reactive selection mirror `refresh_selection_mirrors`
    // updates; a pointermove otherwise drives repaints, this covers the still-pointer case (the
    // `ruler_tick`/`los_tick` idiom, driven declaratively off the mirror instead of a manual bump).
    Effect::new(move |_| {
        let _ = selected_ids.get();
        widget_tick.update(|t| *t = t.wrapping_add(1));
    });
    let active_layer = RwSignal::new(None::<String>);
    // T-180.1 — Eden place side (chips write this in T-180.5); default BLUFOR.
    let active_side = RwSignal::new(String::from("BLUFOR"));
    // T-180.5 — Objects chip stub (place no-op while true).
    let objects_mode = RwSignal::new(false);
    let catalog = RwSignal::new(crate::editor::arsenal::asset_catalog::CatalogState::Loading);
    // T-215 — the Vehicles tab's tree, built from the SAME `/registry` response as `catalog`
    // (`kind == "vehicle"` instead of `"character"`). One fetch, two trees: a second request for
    // rows already in hand would double a ~940 KB payload for nothing.
    let vehicle_catalog =
        RwSignal::new(crate::editor::arsenal::asset_catalog::CatalogState::Loading);
    // T-159.26 — Attributes modal: the open slot id + a doc-change tick the modal re-reads on
    // (`doc_ver` is a plain Rc<Cell>, not reactive; refresh_docks bumps this signal instead).
    let attrs_open = RwSignal::new(None::<String>);
    // T-180.9 — Attributes tab (1 = Identity default; `open_arsenal` sets 3 = Arsenal).
    let attrs_tab = RwSignal::new(1usize);
    let doc_tick = RwSignal::new(0u64);
    // T-780 — the connection edge SELECTED on the map, if any. Session-local overlay state, NOT the
    // Y.Doc — the same rule the slot selection, the ruler chain and the LoS ray follow (a selection
    // is not mission content, and putting it in the document would make it an undo step).
    //
    // It is a signal rather than a `thread_local` because FOUR places must agree about it and two of
    // them are reactive: the `doc_tick` Effect that binds the lane (which re-tints on a selection
    // change), the pointer arm that sets it, the Delete key arm that consumes it, and — since wave
    // 142 — `editor_ops`, which is handed the signal below and RECONCILES it.
    //
    // [wave 142 F-1] That last one is the correction to this ticket's original claim. It said the
    // edge and slot selections were "mutually exclusive by construction" because an edge is only
    // picked when the slot pick already MISSED and a miss clears the slot selection. True of the map
    // pick; false of the editor. Nothing stopped the Outliner row, the marquee, the click-to-select
    // router or a place from raising a slot selection while an edge stayed selected, and Delete then
    // removed the EDGE while the operator looked at a highlighted slot. Nothing reconciled the id
    // against the document either, so an undo left Delete pointing at an edge that no longer existed.
    //
    // Both are now closed in `editor_ops`, where every entity-selection write already funnels through
    // one mirror (`mirror_selection` → `reconcile_connection_selection`): a live entity selection
    // clears the edge, and so does an id the document no longer holds. The exclusivity is a property
    // of the writes, not of one gesture's ordering.
    let selected_connection = RwSignal::new(None::<String>);
    // Both readers (the lane feed and the Delete arm) are wasm-only, so the native view shell never
    // touches it — the file's standard `let _ = …` acknowledgement rather than an `_`-prefixed name,
    // which would make the wasm call sites read as if the value were unused there too.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = selected_connection;
    let settings_open = RwSignal::new(false);
    // T-167 — Faction Manager dialog toggle (launched from the Factions dock "Manage" button).
    let fm_open = RwSignal::new(false);
    // T-177 B2 / T-071.0 — the ORBAT Manager modal open flag (top-strip button ↔ OrbatManagerDialog).
    let orbat_open = RwSignal::new(false);
    // T-664 — the right-click context menu's open state: `Some(MenuState)` = open at that pixel/take,
    // `None` = closed (no DOM). The wasm `contextmenu` handler sets it via `context_menu::open`; the
    // overlay reads it. Mounted BESIDE the ungated dialogs below (not inside the chrome_hidden gate),
    // so a floating menu survives Backspace hide-chrome per the wave-101 verifier.
    let context_menu = RwSignal::new(None::<crate::editor::panels::context_menu::MenuState>);
    // T-647 PLACE-003 — the empty-ground asset picker's open state: `Some(AssetPickerState)` = open
    // at that world/screen point, `None` = closed. The wasm `dblclick` handler sets it via
    // `editor_ops::open_asset_picker` (on a MISS); the picker overlay reads it. Mounted BESIDE the
    // ungated dialogs below (like `context_menu`), so it survives Backspace hide-chrome — the
    // picker is self-contained and does NOT depend on the DockRight catalog, which is exactly why
    // this floating form was chosen over "focus the dock's search" (a hidden dock can't be focused).
    let asset_picker = RwSignal::new(None::<AssetPickerState>);
    // T-651 — the comment editor's open comment id (`None` = closed). Mounted BESIDE the ungated
    // dialogs like the picker and the context menu, so a comment stays editable under Backspace
    // hide-chrome (a floating overlay is not dock chrome — the wave-101 mount rule).
    let comment_editor = RwSignal::new(None::<String>);
    // T-672 — the Connections panel's open flag (the connection graph's SEE + CHECK surface).
    // Declared beside the comment editor and registered with `editor_ops` in the same block below,
    // because the only opener is a context-menu row that has no reactive handle to this signal.
    let connections_panel = RwSignal::new(false);
    // T-662 — Backspace hides the whole Eden chrome (Eden's "hide interface"), leaving the map
    // full-bleed and interactive. Gates the four dock mounts + the strip below; another Backspace
    // brings them back. Declared on both targets — the view reads it, the wasm keydown toggles it.
    let chrome_hidden = RwSignal::new(false);
    // T-638 — per-dock collapse latches (Eden's `E` = left / Entity List, `R` = right / Asset
    // Browser; the tab-strip chevrons flip them too). Session-local (the prefs-store hookup is
    // residue for T-688 — `world_layer_prefs` is out of this ticket's owns). ORTHOGONAL to
    // `chrome_hidden`: it persists through a hide/show cycle, and `chrome_hidden` "wins" while active
    // by zeroing every inset. `mission_editor` OWNS these signals; an Effect below mirrors them (and
    // `chrome_hidden`) into the `eden_layout` inset latch so `select_tool` + the on-canvas gate see one
    // truth, then runs the reflow + centre-hold. The docks read them for the stub swap + chevron glyph.
    let dock_left_collapsed = RwSignal::new(false);
    let dock_right_collapsed = RwSignal::new(false);
    // T-159.27 — the flat registry gear rows for the Attributes Arsenal tab (populated by the same
    // /registry fetch that builds the Factions palette). None until it lands.
    let registry_items = RwSignal::new(None::<Vec<crate::core::dto::RegistryItem>>);
    // T-750 — terminal failure of the `/registry` fetch. Distinct from `registry_items == None`
    // (still in flight): the Favourites panel must not spin on "Resolving…" forever when the
    // catalogue never arrives. `registry_fetch_gen` bumps re-kick the cold fetch (Retry).
    let registry_failed = RwSignal::new(false);
    let registry_fetch_gen = RwSignal::new(0u64);

    // T-255 — Factions palette is side-aware: rebuild whenever Eden chips change `active_side` or
    // the `/registry` rows land. Fetch paths below only write `registry_items` (+ vehicles); this
    // Effect owns `catalog` Ready trees so a BLUFOR→OPFOR chip flip drops NATO and shows USSR.
    {
        use crate::editor::arsenal::asset_catalog::{build_catalog_tree, CatalogState};
        Effect::new(move |_| {
            let side = active_side.get();
            if let Some(items) = registry_items.get() {
                catalog.set(CatalogState::Ready(build_catalog_tree(&items, &side)));
            }
        });
    }
    // T-167 — the compat edge feed for the Smart Arsenal (optic/magazine edge rows + validation).
    // Fetched once alongside /registry; starts Loading, degrades to Unavailable on error.
    let compat = RwSignal::new(crate::editor::arsenal::arsenal_rules::CompatFeed::default());
    // T-159.26 — server hydrate / conflict / dirty (data-safety). `conflict` holds an offered
    // server payload when local IDB content diverges; `dirty` is the unsaved-changes flag;
    // `current_semver` tracks the adopted server version.
    let dirty = RwSignal::new(false);
    let conflict = RwSignal::new(None::<ConflictInfo>);
    let current_semver = RwSignal::new(None::<String>);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = current_semver;

    // T-159.17 — mission id from the `:id` route param (`/missions/:id/edit`; `smoke` on the gate
    // route). One-shot untracked read at mount (id is static per route mount). Fallback `draft`
    // mirrors the React `missionId ?? 'draft'` persistence key. Hoisted out of the wasm block in
    // T-159.21: the chrome's title binds it, and the view compiles on the native target too.
    let mission_id = {
        use leptos_router::hooks::use_params_map;
        use_params_map()
            .get_untracked()
            .get("id")
            .map(|s| s.to_string())
            .unwrap_or_else(|| "draft".to_string())
    };

    // The engine is created + owned on the wasm target only (wgpu is wasm32-gated). Native builds
    // (cargo check) compile the shell without touching the GPU stack.
    #[cfg(target_arch = "wasm32")]
    {
        use leptos::task::spawn_local;
        use std::cell::Cell;
        use std::cell::RefCell;
        use std::rc::Rc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;

        const TERRAIN_W: f64 = 12_800.0;
        const TERRAIN_H: f64 = 12_800.0;
        const INITIAL_TARGET: (f64, f64) = (6_400.0, 6_400.0);
        const INITIAL_ZOOM: f64 = -2.0;
        const WHEEL_ZOOM_PER_PX: f64 = 1.0 / 500.0;
        /// T-159.22 — matches the chrome host div in the view below (and thus every panel inside
        /// it), for the wheel guard's `closest()`. A `data-` attribute, not a class: the class list
        /// is a styling contract that a Tailwind edit could silently change under the guard.
        const CHROME_SEL: &str = "[data-eden-chrome]";

        // T-159.21 — the id is read once in the page body (the chrome's title binds it too).
        let mission_id = mission_id.clone();

        // T-159.20 — auth store for the Save Version POST. Read here in the reactive body (the
        // owner is live); `on_load` is a non-reactive closure, and `AuthStore` is `Copy` so it moves
        // into it cleanly. Provided by `AppLayout` above `<AppRoutes/>`, so present on this route.
        let auth = expect_context::<crate::core::auth::AuthStore>();

        // T-159.22 — the Factions palette catalog. Engine-independent so the dock fills even if
        // wgpu never comes up. `kind == "character"` rows only — `build_catalog_tree` is the
        // T-068.3 `buildCatalogTree` port (T-255: filtered by `active_side` in the Effect above).
        //
        // T-245 — gate the network path on the SPA-session cache. Remounts apply the cached
        // rows synchronously (no network, no second tree rebuild from a fresh download).
        //
        // T-427 — cold path pages `GET /registry?limit=500&offset=…` until `total` is covered
        // (never the unbounded single-shot dump). Page size matches the API `REGISTRY_PAGE_MAX`.
        //
        // T-750 — on Err also raise `registry_failed` (Favourites used to treat bare `None` as
        // forever-loading). Bumping `registry_fetch_gen` re-enters the cold path (Retry).
        {
            use crate::editor::arsenal::asset_catalog::{build_vehicle_catalog_tree, CatalogState};
            Effect::new(move |_| {
                let gen = registry_fetch_gen.get();
                if gen == 0 {
                    if !registry_session::must_fetch_registry() {
                        if let Some(items) = registry_session::cached_registry() {
                            registry_items.set(Some(items.clone()));
                            registry_failed.set(false);
                            vehicle_catalog
                                .set(CatalogState::Ready(build_vehicle_catalog_tree(&items)));
                        }
                        return;
                    }
                } else {
                    // Retry: clear the terminal failure, put palettes back into Loading, and
                    // leave `registry_items` at None so Favourites shows Resolving… again —
                    // never mark the whole collection stale while a retry is in flight (T-695).
                    registry_failed.set(false);
                    catalog.set(CatalogState::Loading);
                    vehicle_catalog.set(CatalogState::Loading);
                }
                spawn_local({
                    async move {
                        match fetch_registry_pages(auth).await {
                            Ok(items) => {
                                registry_session::store_registry(items.clone());
                                registry_items.set(Some(items.clone()));
                                registry_failed.set(false);
                                // T-255 — character `catalog` Ready tree is owned by the
                                // active_side Effect (rebuilds on chip flip).
                                // T-215 — the Vehicles tab, off the same rows.
                                vehicle_catalog
                                    .set(CatalogState::Ready(build_vehicle_catalog_tree(&items)));
                            }
                            Err(_) => {
                                mark_registry_fetch_failed(
                                    catalog,
                                    vehicle_catalog,
                                    registry_failed,
                                );
                            }
                        }
                    }
                });
            });
        }

        // T-167 — compat edge feed for the Smart Arsenal (optic/magazine rows + validation). Own
        // fetch so a compat outage degrades the Arsenal to dumb dropdowns without touching /registry.
        //
        // T-245 — same session gate: a remount reuses the cached CompatFeed + cargo seed map.
        //
        // T-427 — cold path no longer GETs the unfiltered ~20k-edge dump. Two bounded requests:
        //   1. Arsenal edge families only (`optic_on_weapon,mag_in_weapon,attachment_on_weapon`)
        //   2. `?view=cargo_defaults` aggregated cargo seed map (server-side collapse)
        {
            use crate::editor::arsenal::arsenal_rules::{CompatFeed, CompatGraph, CompatStatus};
            if registry_session::must_fetch_compat() {
                spawn_local({
                    async move {
                        match fetch_compat_cold(auth).await {
                            Ok((feed, cargo)) => {
                                registry_session::store_compat(feed.clone(), cargo.clone());
                                editor_ops::set_cargo_defaults(cargo);
                                compat.set(feed);
                            }
                            Err(_) => {
                                // Do not cache a hard failure — a later remount may retry.
                                compat.set(CompatFeed {
                                    status: CompatStatus::Unavailable,
                                    graph: CompatGraph::default(),
                                });
                            }
                        }
                    }
                });
            } else if let Some((feed, cargo)) = registry_session::cached_compat() {
                editor_ops::set_cargo_defaults(cargo);
                compat.set(feed);
            }
        }

        canvas_ref.on_load(move |canvas: web_sys::HtmlCanvasElement| {
            let Some(container) = container_ref.get_untracked() else {
                return;
            };
            let container: web_sys::HtmlDivElement = container;
            let win = web_sys::window().expect("window");

            // Backend override for the headless readback gate: `?force=webgl` → WebGL2/SwiftShader,
            // where the byte-exact self_check readback resolves via `device.poll` (on webgpu/Dawn
            // headless the offscreen map never fires). Default (no query) = prefer WebGPU, matching
            // prod. Mirrors the React `WgpuCanvas` spike's `?force=webgl`.
            let force_webgl = win
                .location()
                .search()
                .map(|s| s.contains("force=webgl"))
                .unwrap_or(false);

            // Size the backing store BEFORE create (the engine reads canvas.width/height).
            let dpr0 = win.device_pixel_ratio();
            let rect0 = container.get_bounding_client_rect();
            let (dw, dh) = device_size(rect0.width(), rect0.height(), dpr0);
            canvas.set_width(dw);
            canvas.set_height(dh);

            let engine: Rc<RefCell<Option<map_engine_render::RenderEngine>>> =
                Rc::new(RefCell::new(None));
            // T-166 — shared map-asset host (camera-settle refresh after wheel/pan).
            let map_host = crate::editor::world_assets::new_host_handle();
            // T-172 B2 — DEM grid handle for the CUR Z sample (published by bootstrap).
            let dem_grid = crate::editor::world_assets::new_dem_grid_handle();
            let disposed = Arc::new(AtomicBool::new(false));

            // T-159.16 — MissionDoc host. Built + seeded + bridged synchronously (before the async
            // engine create), so the `window.__missionDoc` Class R gate does not depend on the wgpu
            // engine coming up. The doc leaks on route-leave like the engine (`!Send` `Rc`, and
            // `on_cleanup` is `Send`-bound) — no double-free (plain Rust `Drop`). The optional
            // doc→engine bind (D5) happens below once the engine is `Some`.
            let doc = mission_doc::new_seeded_doc();
            // T-651 — THE NEW-MISSION TEMPLATE SEEDS COMMENTS. Two of them, under the `INIT` origin
            // so the template is not an undo step.
            //
            // This is the right instant and the only one: the doc is freshly minted and nothing has
            // loaded into it yet. Both later boot steps REPLACE the document wholesale — the IDB
            // restore swaps in a different core, and `hydrate_from_server`'s adopt path reloads from
            // the saved payload — so a restored or downloaded mission keeps exactly the comments it
            // was saved with and never gets a second template. Seeding after either step would be
            // the duplicate-notes bug; `seed_template_comments` also declines on a non-empty
            // comments map, so the property holds twice over.
            //
            // Two, because that is what the evidence supports. FNF v4 deleted a 219-line config
            // guide and a 421-file template, and the onboarding that survived is literally two
            // Comment entities. FNF v3 had 28. **This is ONE community across TWO eras — WOG and
            // OFCRA ship no comment equivalent at all** — so the seed copies what survived a
            // rewrite, not what a single era once had, and it is not a four-way convergence.
            editor_ops::seed_new_mission_template(&doc);
            let doc_ver = Rc::new(Cell::new(1u32));
            mission_doc::register_mission_doc(doc.clone(), doc_ver.clone());

            // T-159.20 — editor commands (Save/Export) context + the `__editorCommands` smoke bridge
            // (peer of `__missionDoc`). `set_ctx` shares the same `Rc` the persistence swap targets,
            // so both the buttons and the bridge see an IDB-restored doc.
            crate::editor::state::commands_hotkeys::set_ctx(
                doc.clone(),
                auth,
                mission_id.clone(),
                current_semver,
            );
            crate::editor::state::commands_hotkeys::register_editor_commands(doc.clone());

            // T-159.18 — LMB select foundation. Selection is app-side state (NOT the Y.Doc — it never
            // lived in the document, matching React's Zustand), held in the editor's leaked-handle
            // idiom so the `window.__editorSelection` smoke bridge (peer of __missionDoc) never reads
            // reactive-owner state a route change could dispose. `left` carries the in-flight LMB
            // gesture (T-159.19 `LeftGesture`: Pending → Move | Marquee — a frozen ortho camera copied
            // at the press drives every unproject) between pointerdown/move/up. Registered
            // synchronously (engine still `None` here — `probe()` reads it lazily; `pick_selfcheck()`
            // needs only the synchronously-seeded doc).
            let selection: crate::editor::tools::select_tool::SelectionHandle =
                Rc::new(RefCell::new(Vec::new()));
            let left: Rc<RefCell<Option<crate::editor::tools::select_tool::LeftGesture>>> =
                Rc::new(RefCell::new(None));
            // T-642 — the persistent ruler polyline. Session-local OVERLAY state (Decision 4 — NOT
            // the Y.Doc, exactly like the selection set above), held in a leaked `Rc<RefCell<…>>` so
            // both the pointer handlers (which mutate it) and the `RulerOverlay`'s `read_chain`
            // closure (which clones it to project) share one source of truth without touching
            // reactive-owner state a route change could dispose.
            let ruler: Rc<RefCell<crate::editor::tools::ruler_tool::RulerChain>> = Rc::new(
                RefCell::new(crate::editor::tools::ruler_tool::RulerChain::new()),
            );
            // Push the chain's current summary onto the reactive surface (status bar + repaint tick).
            // One helper so every mutation site (click / Esc / dbl-click / tool-switch clear) updates
            // both signals identically and can never drift.
            let sync_ruler = {
                let ruler = ruler.clone();
                move || {
                    ruler_status.set(ruler.borrow().status_readout());
                    ruler_tick.update(|t| *t = t.wrapping_add(1));
                }
            };
            // T-642 — tool-switch dismissal (Decision 3): switching the tool back to Select clears
            // the PLACED ruler (the "second-Esc-equivalent" the spec names). One Effect observes
            // `tool_mode`; when it is not Ruler it clears the chain and re-syncs the overlay + status
            // bar. Idempotent — the first run (default Select, empty chain) is a harmless no-op, and
            // switching Select→Ruler leaves an already-empty chain untouched.
            {
                let ruler = ruler.clone();
                let sync_ruler = sync_ruler.clone();
                Effect::new(move |_| {
                    if !tool_mode.get().is_ruler() && !ruler.borrow().is_empty() {
                        ruler.borrow_mut().clear();
                        sync_ruler();
                    }
                });
            }
            // T-642 — hand the leaked chain to the `ruler_tool` thread_local so the `RulerOverlay`
            // (mounted in the shared view, outside this block) can read + project it (the
            // `context_menu::set_menu_signal` handoff idiom).
            crate::editor::tools::ruler_tool::register_ruler_chain(ruler.clone());

            // T-643 — the Line-of-Sight capture. Session-local OVERLAY state (Decision 4 — NOT the
            // Y.Doc, exactly like the selection set + the ruler chain above), a leaked
            // `Rc<RefCell<LosState>>` shared by the pointer handlers (which mutate it) and the
            // `LosOverlay` (which clones it to project + build the profile).
            let los: Rc<RefCell<crate::editor::tools::los_tool::LosState>> =
                Rc::new(RefCell::new(crate::editor::tools::los_tool::LosState::new()));
            // Bump the repaint tick on every LoS mutation (click / Esc / tool-switch clear) so the
            // overlay repaints even on a still-pointer click. (No status-bar readout — Decision 2's
            // verdict lives in the inline panel, so unlike the ruler there is no status signal here.)
            let sync_los = {
                move || {
                    los_tick.update(|t| *t = t.wrapping_add(1));
                }
            };
            // T-644 — the VIEWSHED sub-mode's session-local OVERLAY state (Decision 4 — NOT the Y.Doc,
            // exactly like the ruler chain + the LoS ray above), a leaked `Rc<RefCell<ViewshedState>>`.
            // Unlike the ray (a DOM overlay), the viewshed WASH is a GPU texture lane: the pointer
            // commit computes the raster + uploads it (`place_viewshed` + `engine.viewshed_upload`),
            // and the state holds the placed observer + raster so a pan re-projects the same rect
            // without recompute. Registered into `los_tool`'s thread_local (peer of the LoS state) so
            // the overlay/engine bridge reads it; the compute itself runs through the registered DEM
            // sampler set below (the same 8 m grid).
            let viewshed: Rc<RefCell<crate::editor::tools::los_tool::ViewshedState>> = Rc::new(
                RefCell::new(crate::editor::tools::los_tool::ViewshedState::new()),
            );
            // T-643/T-644 — tool-switch dismissal (Decision 3): switching the tool away from LoS clears
            // BOTH the ray shot AND the viewshed wash (the "second-Esc-equivalent"). One Effect observes
            // `tool_mode` AND `los_mode`; when the tool is not LoS it clears the ray state, and when the
            // active LoS lane is not the viewshed (tool left LoS, or the sub-mode toggled away from
            // Viewshed) it clears the viewshed state + drops the engine lane (`viewshed_clear`). This is
            // the peer of the ruler's clear-on-switch, EXTENDED for the viewshed's GPU lane — the
            // documented "clear-on-switch and Esc route through the existing shared seam". Idempotent:
            // the default-Select first run on empty state is a harmless no-op, and an empty viewshed's
            // `viewshed_clear()` just removes an absent lane.
            {
                let los = los.clone();
                let viewshed = viewshed.clone();
                let engine = engine.clone();
                let sync_los = sync_los;
                Effect::new(move |_| {
                    let is_los = tool_mode.get().is_los();
                    let viewshed_active = is_los && los_mode.get().is_viewshed();
                    if !is_los && !los.borrow().is_empty() {
                        los.borrow_mut().clear();
                        sync_los();
                    }
                    // The viewshed lane lives while the operator is in LoS-viewshed sub-mode; leaving
                    // LoS OR toggling the sub-mode to Ray drops it (state + GPU lane).
                    if !viewshed_active && !viewshed.borrow().is_empty() {
                        viewshed.borrow_mut().clear();
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.viewshed_clear();
                        }
                    }
                });
            }
            // T-643 — hand the leaked state to the `los_tool` thread_local so the `LosOverlay` can
            // read + project it (peer of `register_ruler_chain`), and hand it a DEM point-sampler so
            // the overlay can rebuild the terrain profile after a pan. The sampler closes over the
            // SAME 8 m downsampled `dem_grid` the ruler's per-vertex Z read uses (the reachable DEM
            // in the editor) — `los_tool` takes no compile-time dependency on the grid-handle type.
            crate::editor::tools::los_tool::register_los_state(los.clone());
            // T-644 — hand the leaked viewshed state to `los_tool`'s thread_local (peer of
            // `register_los_state`) so `place_viewshed` can store the computed raster into it and a
            // pan re-projects the same rect. The compute reuses the SAME DEM sampler registered just
            // below (the ray's sampler); `place_viewshed` calls `compute_viewshed_for`, which reads it.
            crate::editor::tools::los_tool::register_viewshed_state(viewshed.clone());
            {
                let dem_grid = dem_grid.clone();
                crate::editor::tools::los_tool::register_los_sampler(std::rc::Rc::new(
                    move |x: f64, y: f64| {
                        dem_grid.borrow().as_ref().and_then(|g| {
                            map_engine_core::dem::downsample::sample_grid_meters(g, x, y)
                        })
                    },
                ));
            }

            crate::editor::tools::select_tool::register_editor_selection(
                selection.clone(),
                doc.clone(),
                engine.clone(),
                container.clone(),
            );

            // T-648 — hand the leaked doc + selection to a pivot getter the `TransformWidgetOverlay`
            // (mounted in the shared view, outside this wasm block) reads to place its gizmo. Peer of
            // `register_ruler_chain` / `register_editor_selection`: the overlay is native-compiled, so
            // it cannot hold the `!Send` `Rc`s directly — it calls `read_widget_pivot()`, which is the
            // registered closure here (or `None` natively / pre-mount). The pivot is the SELECTION
            // CENTROID over slots (SoA) + vehicles (`vehiclesById`), the same average
            // `center_on_selection` flies to, so the widget sits where Space would centre.
            {
                let doc = doc.clone();
                let selection = selection.clone();
                register_widget_pivot(std::rc::Rc::new(move || {
                    let sel = selection.borrow();
                    if sel.is_empty() {
                        return None;
                    }
                    let d = doc.borrow();
                    let core = d.as_ref()?;
                    let soa = core.materialize();
                    let veh =
                        serde_json::from_str::<serde_json::Value>(&core.small_maps_json()).ok();
                    let (mut sx, mut sy, mut n) = (0.0f64, 0.0f64, 0.0f64);
                    for id in sel.iter() {
                        if let Some(row) = soa.ids.iter().position(|s| s == id) {
                            sx += f64::from(soa.xs[row]);
                            sy += f64::from(soa.ys[row]);
                            n += 1.0;
                        } else if let Some(pos) = veh
                            .as_ref()
                            .and_then(|r| r.get("vehiclesById")?.get(id)?.get("position").cloned())
                        {
                            if let (Some(vx), Some(vy)) = (
                                pos.get("x").and_then(serde_json::Value::as_f64),
                                pos.get("y").and_then(serde_json::Value::as_f64),
                            ) {
                                sx += vx;
                                sy += vy;
                                n += 1.0;
                            }
                        }
                    }
                    if n == 0.0 {
                        None
                    } else {
                        Some((sx / n, sy / n))
                    }
                }));
            }

            // T-797 — hand the row-2 toolbar + Edit menu their dispatch. Peer of the
            // `register_widget_pivot` block above: `widget_variant` / `snap` (T-648/T-795) are
            // `RwSignal`s local to this `!Send` `on_load` closure and `container` is the DOM handle
            // whose CSS rect Select All needs, none reachable from the native-compiled strip. Each
            // invoker is the KEYDOWN ARM's body verbatim (so a click and the chord agree), and the two
            // getters read the signals TRACKED (`.get()`), so the strip's toggle-plate closures
            // subscribe across the thread_local and re-render when a chord flips the state.
            //
            // wave-202 — `register_…` also BUMPS the generation signal ([`TOOLBAR_DISPATCH_GEN`]) the
            // strip's plate closures subscribe to from frame one, which is what makes them reactive:
            // they render BEFORE this `on_load` runs, so their first pass reads no getter (the dispatch
            // is `None`) and would never re-run — the bump re-runs them now that the dispatch exists,
            // and THAT run subscribes them to the tracked widget/snap getters. `on_cleanup` mirrors the
            // T-189 unload-guard teardown: on route-leave (unmount) it clears the dispatch and bumps
            // again, so a re-mount (mission switch) never freezes the plate on a stale, disposed
            // dispatch — the plates go dark until the fresh `on_load` re-registers and re-bumps.
            {
                let container = container.clone();
                register_editor_toolbar_dispatch(std::rc::Rc::new(EditorToolbarDispatch {
                    // Digit1/2/3 arm: `from_digit` (1 → No Widget, 2 → Translate, 3 → Rotate; 4/5 and
                    // any other = no-op). T-799's No-Widget button calls this with `1`.
                    set_widget: Box::new(move |d: u8| {
                        widget_variant.set(widget_variant.get_untracked().from_digit(d));
                    }),
                    // `G` arm: flip the snap-grid master latch.
                    toggle_snap: Box::new(move || {
                        snap.set(snap.get_untracked().toggled());
                    }),
                    // `[`/`]` arms: step the ACTIVE widget variant's ladder (its `snap_axis`).
                    snap_step: Box::new(move |delta: i32| {
                        let axis = widget_variant.get_untracked().snap_axis();
                        snap.set(snap.get_untracked().stepped(axis, delta));
                    }),
                    // Ctrl+A arm: measure the live container and delegate to `select_all_in_view`
                    // (the closure OWNS the rect, so the button hands over nothing).
                    select_all: Box::new(move || {
                        let rect = container.get_bounding_client_rect();
                        editor_ops::select_all_in_view(rect.width(), rect.height());
                    }),
                    widget_digit: Box::new(move || widget_variant.get().to_digit()),
                    widget_is_rotate: Box::new(move || widget_variant.get().is_rotate()),
                    snap_enabled: Box::new(move || snap.get().enabled),
                }));
                // Route-leave teardown (T-189 pattern): drop the dispatch and bump so a mission
                // switch's fresh mount cannot inherit a plate frozen on this mount's disposed getters.
                on_cleanup(unregister_editor_toolbar_dispatch);
            }

            // T-655 — the validation panel's payload source. The engine (`mission::validate`) is pure
            // core with no access to this `!Send` doc or the `registry_session` catalogue, and the
            // panel view is native-compiled, so — peer of `register_widget_pivot` / the ruler/LoS
            // registrations — we hand it a getter that reads the LIVE doc + registry each re-eval.
            // It returns the Save-shape compiled payload (`compile_payload(small, slots, false)`,
            // which carries `editor.{factions,squads,slots}` + top-level `vehicles`/`entities` the
            // rules walk) plus the T-658 known-asset-id catalogue (this is where the T-658 SPA
            // boundary lands — the ticket's W111 wiring). `registry_items` is `Copy` (a signal); an
            // untracked read keeps the getter side-effect-free. `None` while the doc/registry are not
            // ready ⇒ the panel is simply empty (and ASSET-RESOLVES skips via its own gate when the
            // catalogue is `None`, the conservative default).
            {
                let doc = doc.clone();
                validation_panel::register_payload_source(std::rc::Rc::new(move || {
                    let d = doc.borrow();
                    let core = d.as_ref()?;
                    let payload = map_engine_core::mission::compile::compile_payload(
                        &core.small_maps_json(),
                        &core.slots_json(),
                        false,
                    );
                    // Catalogue: the live registry rows if loaded, else `None` (rule skips). Built
                    // from `resource_name`s + object prop:/comp: aliases — the ids the payload uses.
                    let known_asset_ids = registry_items
                        .get_untracked()
                        .map(|items| validation_panel::known_asset_ids_from_registry(&items));
                    Some(validation_panel::PayloadSource {
                        payload,
                        known_asset_ids,
                    })
                }));
            }

            // T-655 — the validation panel's CLICK-TO-SELECT router. A finding click routes its
            // `subject_id` (the T-657 stable entity id) → the editor selection, so the offender is
            // PINNED on the map + in the trees (not a clipboard dump). Lives HERE, closing over the
            // `!Send` doc / selection / engine `Rc`s the native-compiled panel cannot hold (peer of
            // the payload-source getter above and `register_widget_pivot`). Mirrors `open_attributes`
            // (replace selection → engine `set_selection` → refresh mirrors) MINUS opening the modal,
            // and additionally CENTRES the camera on the entity (the maker clicked to find it). A
            // STALE finding whose entity was deleted since the last re-eval resolves to no position
            // and no-ops (returns false) rather than clearing the current selection — the centroid
            // math mirrors the transform-widget pivot (slot SoA position, else the vehicle row's
            // `position`).
            //
            // T-754 — WIDENED TO ZONES, and the resolution moved OUT into the pure [`route_target`].
            // Two things follow. (1) A zone id now SELECTS: not into `select_tool`'s selection (a
            // zone is not a slot — `SEL 1` with nothing highlighted is the reason `zone_selected` is
            // its own signal), but through `eden_dock_right::route_select_zone`, which drives the
            // Zones panel's OWN selection signal and raises that tab. The camera still centres, so a
            // zone click behaves like every other click-to-select. (2) Because the resolution is now
            // a pure function, a surface can ASK whether a click has a target before drawing the
            // affordance — which is the wave-115 MAJOR (`cursor-pointer` over a dead click) fixed at
            // its cause rather than papered over. This is STILL THE ONE ROUTER: no second selection
            // path was invented, the closure simply grew an arm.
            //
            // WAVE 129 — the panel now ASKS, and asks THIS resolution. `resolve` below is the whole
            // of the router's question ("what would a click on this subject id find, and where?");
            // the click ACTS on its answer and `register_route_probe` hands the same answer to
            // `validation_panel`'s row so the affordance is the router's own resolution rather than
            // a guess off `subject_id.is_some()`. ONE `Rc`, two callers — they cannot drift apart,
            // which is the correspondence T-754 pinned on the settings surface, held here.
            //
            // The `Entity` arm is the wave-129 widening: `placed_asset_refs` emits an ASSET-RESOLVES
            // finding per `entities[]` row keyed by that row's id, so every placed-object finding
            // used to resolve to `None` under a `cursor-pointer` row. It rides the Vehicle path
            // (selection + centre) because a placed object sits off the slot SoA exactly as a
            // vehicle does.
            {
                let doc = doc.clone();
                let selection = selection.clone();
                let engine = engine.clone();
                // Pure question, live document. `None` ⇒ nothing to select (a stale finding, or a
                // row this editor owns no selection surface for) — the click keeps the current
                // selection intact and the row renders inert.
                let resolve: SubjectResolver = std::rc::Rc::new(move |subject_id: &str| {
                    let d = doc.borrow();
                    let core = d.as_ref()?;
                    // The one fact the small-maps root does not carry: slot-SoA membership.
                    let soa = core.materialize();
                    let slot_row = soa.ids.iter().position(|s| s == subject_id);
                    let root = serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
                        .unwrap_or(serde_json::Value::Null);
                    let target = route_target(&root, subject_id, &|_| slot_row.is_some())?;
                    let (cx, cy) = match target {
                        RouteTarget::Slot => {
                            let row = slot_row.expect("Slot arm implies the SoA matched");
                            (f64::from(soa.xs[row]), f64::from(soa.ys[row]))
                        }
                        RouteTarget::Vehicle { x, y }
                        | RouteTarget::Entity { x, y }
                        | RouteTarget::Zone { x, y }
                        // T-784 — a comment centres like a vehicle; its selection lands in the same
                        // editor selection `Vec`, which is what makes it composable with entities.
                        | RouteTarget::Comment { x, y } => (x, y),
                    };
                    Some((target, cx, cy))
                });
                // The AFFORDANCE seam: "would this click select anything?", answered by the same
                // resolution the click runs. Registered before the actor so a row can never be
                // painted clickable by a router that is not yet installed.
                //
                // COST, stated: this is asked once per visible finding row, and each ask re-reads
                // the small-maps root + the slot SoA — the read the click already did once. The
                // list only renders while the panel is expanded, behind the 250 ms re-eval
                // debounce. If a large mission ever makes that show up, memoise it on a version
                // counter the RESTORE path also bumps: `doc_ver` is not bumped by the IDB restore
                // (`mission_history`, "does not mark dirty / bump `doc_ver`"), so a cache keyed on
                // it would answer for the pre-restore document — an affordance lying again, which
                // is the one trade this seam may not make.
                //
                // WAVE 129 F6 — and the ask is `available`, not `resolve`. `resolve` answers "which
                // surface owns this id?"; the CLICK also needs the Zones panel to be MOUNTED before a
                // zone selection can land (F2 made `route_select_zone` report that honestly). A probe
                // built from `resolve` alone therefore said `true` for a zone whose panel Backspace
                // had unmounted while the click said `false` — `cursor-pointer` over a dead click,
                // the exact T-754 MAJOR, re-created by two correct fixes disagreeing. So the
                // narrowing lives ONCE, in [`route_availability`], and this is the only place the
                // "is the Zones panel there?" question is asked: both seams below clone THIS `Rc`.
                //
                // The liveness oracle is `chrome_hidden`, which is the very gate the `DockRight`
                // mount is written against further down this component (`(!chrome_hidden.get())
                // .then(|| view! { … DockRight … })`), and `DockRight`'s body is where
                // `install_select_zone` runs. `eden_dock_right` exposes no side-effect-free "is the
                // hook live?" accessor — the only reader is `route_select_zone`, which SELECTS — and
                // that module is outside this change's owns, so the mount gate this file owns is the
                // strongest honest answer available. `t754_router_resolves_zones` pins the two
                // against each other so the mirror cannot drift into a lie.
                //
                // Read REACTIVELY (`.get()`, not `get_untracked`): a row's affordance must repaint
                // when Backspace hides the chrome, or it would be correct only until the next
                // unrelated re-render.
                let available: SubjectResolver = {
                    let resolve = std::rc::Rc::clone(&resolve);
                    std::rc::Rc::new(move |subject_id: &str| {
                        route_availability(resolve(subject_id), &|| !chrome_hidden.get())
                    })
                };
                {
                    let probe = std::rc::Rc::clone(&available);
                    validation_panel::register_route_probe(std::rc::Rc::new(
                        move |subject_id: &str| probe(subject_id).is_some(),
                    ));
                }
                validation_panel::register_select_by_id(std::rc::Rc::new(
                    move |subject_id: &str| {
                        let Some((target, cx, cy)) = available(subject_id) else {
                            return false;
                        };
                        if matches!(target, RouteTarget::Zone { .. }) {
                            // A zone is selected in the Zones panel, never in the slot selection.
                            // If that panel is not mounted there is nothing to select, and the
                            // router says so (false) instead of centring on a phantom selection.
                            //
                            // F6 — that case is already excluded above: `available` refused any
                            // `Zone` whose panel is gone, so this `false` is now a case the probe
                            // has ALSO ruled out rather than one only the click could see. The
                            // check stays because `route_select_zone` is the actor and its report
                            // is the ground truth; if it ever disagrees with the oracle the honest
                            // answer is still `false`, never a `true` over a no-op.
                            if !crate::editor::panels::dock_right::route_select_zone(subject_id) {
                                return false;
                            }
                        } else {
                            *selection.borrow_mut() = vec![subject_id.to_string()];
                            let ids = selection.borrow().clone();
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_selection(ids);
                            }
                            mission_history::refresh_selection();
                        }
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.set_view(cx, cy, e.zoom()); // centre on the offender (React flyTo)
                            e.on_camera_changed();
                        }
                        true
                    },
                ));
            }

            // T-159.21 — undo/redo. The ctx carries every handle a post-change rebind needs (doc +
            // engine + selection + doc_ver + id) plus the HUD signal mirrors, so the toolbar buttons,
            // the keyboard shortcuts, and the `__editorHistory` bridge all drive ONE path. Registered
            // here (after the selection exists, engine still `None` — the rebind reads it lazily).
            // `refresh_hud` seeds the HUD from the freshly-seeded doc: OBJ = 8, SEL = 0, and
            // can_undo = false (the seed runs under INIT origin, so it is not an undo step).
            //
            // T-380 — `restore_settled` is created HERE, above `set_ctx`, rather than beside its
            // T-175 B1 partner `engine_mounted` below: `mission_history` needs it as the boot gate on
            // the debounced persist writer, and `set_ctx` runs synchronously at mount, long before the
            // restore task exists. One flag, two readers — the engine rebind handshake and the persist
            // gate ask the same question ("has the document settled?"), so a second flag would only be
            // a second thing to keep in sync.
            let restore_settled = Rc::new(Cell::new(false));
            mission_history::set_ctx(
                doc.clone(),
                engine.clone(),
                selection.clone(),
                doc_ver.clone(),
                mission_id.clone(),
                can_undo,
                can_redo,
                obj_count,
                sel_count,
                dirty,
                restore_settled.clone(),
            );
            // T-159.22 — dock commands (outliner select / active layer / palette place). Registered
            // BEFORE `refresh_hud()` below, because that call funnels into
            // `editor_ops::refresh_docks` — without the ctx the outliner would render empty until
            // the first edit.
            editor_ops::set_ctx(
                doc.clone(),
                engine.clone(),
                selection.clone(),
                active_layer,
                active_side,
                objects_mode,
                outliner_nodes,
                orbat_nodes,
                selected_ids,
                attrs_open,
                attrs_tab,
                doc_tick,
            );
            // T-664 — hand the context-menu signal to the module's thread_local so the wasm
            // `contextmenu` closure below (which has no reactive handle) can open the menu.
            crate::editor::panels::context_menu::set_menu_signal(context_menu);
            // T-647 PLACE-003 — same handoff for the empty-ground asset picker: the wasm `dblclick`
            // closure opens it through `editor_ops::open_asset_picker`, which writes this signal.
            editor_ops::set_asset_picker_signal(asset_picker);
            // T-651 — same handoff for the comment editor: the Outliner's comment row (a native
            // view with no reactive handle) opens it through `editor_ops::open_comment_editor`.
            editor_ops::set_comment_editor_signal(comment_editor);
            // T-672 — same idiom: the `Connections...` context-menu row calls
            // `editor_ops::open_connections_panel`, which needs this handle.
            editor_ops::set_connections_panel_signal(connections_panel);
            // T-780 [wave 142 F-1] — the SAME idiom, for the opposite reason: `editor_ops` does not
            // read this one to render an overlay, it reconciles it. Every entity-selection write in
            // the editor lands in `editor_ops::mirror_selection`, so handing the signal over is what
            // makes "an edge selection and a slot selection cannot both be live" true of routes this
            // page never sees (the Outliner row, the click-to-select router, a place), and what lets
            // an undo that removed the edge clear the amber line with it.
            editor_ops::set_connection_selection_signal(selected_connection);

            mission_history::register_editor_history();
            mission_history::register_key_handler();
            // T-189 — the unsaved-work guard (`beforeunload`). Registered after `set_ctx` above,
            // which is what supplies both the `dirty` flag it reads and the mission id it arms on.
            //
            // This is the ONE editor listener that must come down on route-leave: it is installed on
            // `window`, so a leaked copy would keep prompting on every later tab close — off a
            // `dirty` signal whose owner this route's teardown disposed. The `sse.rs` trap is why
            // that is not the usual `.forget()`: `on_cleanup` is `Send + Sync`-bound and a `Closure`
            // is `!Send`, so the cleanup cannot own the handle. It doesn't have to — the closure
            // parks in a `mission_history` thread_local and the cleanup below is a zero-capture fn
            // item (`Send + Sync`) that removes and drops it.
            mission_history::register_unload_guard();
            on_cleanup(mission_history::unregister_unload_guard);

            // ═══════════ T-780 — the CONNECTION lane feed, and it reads the DOCUMENT ═══════════
            //
            // Registered HERE, above `refresh_hud()`, for the same reason `editor_ops::set_ctx` is:
            // that call funnels into `refresh_docks`, which bumps `doc_tick` — so the seed bind
            // happens on the very first tick rather than one edit later.
            //
            // WHY THIS EFFECT AND NOT A CALL SITE. T-069 and T-672 independently established that a
            // lane fed from a slice's own authoring call sites goes STALE after undo / redo /
            // restore: those paths replace the document without re-entering the code that drew. So
            // the lane is bound from ONE place that re-reads the live `MissionDocCore` whenever the
            // document may have changed, and every path bumps that one channel:
            //
            //   * a committed edit, an undo, a redo   → `mission_history::after_doc_change`
            //   * the mount seed / server hydrate     → `mission_history::refresh_hud`
            //   * the IDB restore swap + the engine   → `mission_history::rebind_engine_from_doc`
            //     mount handshake
            //
            // …and all three end in `refresh_signals` → `editor_ops::refresh_docks` → `doc_tick`.
            // `rebind_engine_from_doc` being on that list is what makes the restore case work in the
            // order it actually happens: the restore can settle BEFORE the engine exists, and the
            // engine-mount handshake re-runs the rebind, which bumps the tick again, and this Effect
            // binds against an engine that is finally there.
            //
            // The `selected_connection` read is tracked deliberately: picking an edge re-runs this
            // and re-packs the lane with that edge tinted. Nothing else needs a "highlight" path —
            // and since wave 142 it is also how the reconcile becomes visible: `editor_ops` clearing
            // a stale or superseded selection is a write to that signal, so the amber drops off the
            // line in the same pass that made it stale.
            //
            // [wave 142 F-4 — RECORDED, DELIBERATE, NOT A BUG TO FILE AGAIN.] This lane re-binds on
            // `doc_tick`, i.e. on COMMIT, so during a slot/vehicle drag an edge stays pinned to the
            // committed endpoint and catches up on pointer-up, while the drag-preview lanes re-pack
            // per pointermove. That is exactly what the sibling hairline lane does: `SquadLinks` is
            // uploaded from the rebind path (`upload_squad_links` on `after_doc_change`) and lags the
            // same way, for the same reason. Two hairline lanes over the same entities that disagree
            // about where those entities are DURING a drag would be worse than both lagging — the
            // inconsistency would read as one of them being wrong.
            //
            // Making them live would be ONE change to BOTH, not a change here: the drag preview owns
            // the provisional positions (they never enter the document until commit), so both lanes
            // would have to be re-packed from the preview's position map on each pointermove, with
            // the committed document as the fallback for every entity not being dragged. That is a
            // preview-side feature, not a connection-lane fix, and it is not this ticket's.
            {
                let doc = doc.clone();
                let engine = engine.clone();
                Effect::new(move |_| {
                    let _ = doc_tick.get();
                    let selected = selected_connection.get();
                    let segs = doc
                        .borrow()
                        .as_ref()
                        .map_or_else(Vec::new, live_connection_segments);
                    // An edge whose endpoint went away (undo of a place, an entity delete and its
                    // T-672 cascade) simply stops producing a segment, so a stale selection is
                    // inert: it tints nothing and `connections_bind` clears the lane at zero.
                    let verts = connection_lane_verts(&segs, selected.as_deref());
                    #[allow(clippy::cast_possible_truncation)]
                    let count = segs.len() as u32;
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.connections_bind(&verts, count);
                    }
                });
            }

            mission_history::refresh_hud();

            // T-159.26 — editor keyboard actions (MissionCreatorPage onKeyDown): Delete
            // (remove selection), Space (center on centroid), Ctrl/Cmd+C/V (copy/paste at cursor).
            // T-669 completes the clipboard: Ctrl/Cmd+X cuts (copy then delete) and
            // Ctrl/Cmd+Shift+V pastes at the SOURCE position instead of at the cursor.
            // T-662 — Backspace is here too, but bound to hide-chrome (`chrome_hidden`), NOT delete.
            // A SEPARATE window keydown from the undo/redo one (which owns Ctrl+Z/Y) — each guards
            // its own keys, both skip editable fields. `cursor` feeds the paste anchor (world coords).
            {
                // T-642 — the ruler chain + its reactive sync, so the Esc arm can dismiss it.
                let ruler = ruler.clone();
                let sync_ruler = sync_ruler.clone();
                // T-643 — the LoS capture + its reactive sync SHARE this same Esc seam (Decision 3):
                // rather than add a second window keydown listener (T-726, the window-Esc pile-up, is
                // pending — a new UNGUARDED listener would make it worse), LoS hooks the ruler's
                // existing Escape arm below, so the eventual T-726 fix covers both tools at once.
                let los = los.clone();
                let sync_los = sync_los;
                // T-644 — the VIEWSHED sub-mode joins the SAME Esc seam (no new window listener — T-726
                // is pending): the keydown arm below also calls `viewshed.escape()` and, on a real
                // dismissal, drops the engine wash lane. `engine` is cloned in so the arm can call
                // `viewshed_clear()` when the wash is dismissed.
                let viewshed = viewshed.clone();
                let engine = engine.clone();
                // T-649 SEL-ALL-001 — the Ctrl/Cmd+A arm needs the canvas CSS size, because Eden
                // scopes Select All to what is ON SCREEN. The container is the same element every
                // pointer gesture measures for its frozen camera, so Ctrl+A and a full-canvas
                // marquee drag are measured against the identical rect.
                let container = container.clone();
                let onkeydown = Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
                    move |ev: web_sys::KeyboardEvent| {
                        if mission_history::in_editable_field() {
                            return;
                        }
                        let modk = ev.ctrl_key() || ev.meta_key();
                        let (cx, cy) = match cursor.get_untracked() {
                            Some((x, y, _)) => (Some(x), Some(y)),
                            None => (None, None),
                        };
                        // Each arm returns whether it acted; prevent the browser default once.
                        let handled = match ev.code().as_str() {
                            // T-642/T-643 — Esc is the SHARED two-step escalating dismissal (Decision
                            // 3) for BOTH measure tools. The ruler: first Esc drops the in-progress
                            // tail (keeps a legged measure placed), a second clears the placed ruler.
                            // LoS mirrors it: first Esc drops the in-progress observer, a second clears
                            // the placed shot. Only one tool is ever non-empty at a time (switching
                            // tools clears the other's overlay), so calling BOTH `.escape()`s here is
                            // safe — the inactive tool's state is empty and its `.escape()` is a false
                            // no-op. Esc only "acts" (→ prevent_default) when SOMETHING was dismissed;
                            // an Esc with neither tool placed falls through untouched (never swallowed).
                            "Escape" if !modk => {
                                // T-726 — yield to any open overlay (context menu / dialogs /
                                // pickers). Those register with `modal_stack`; without this guard,
                                // Esc closing the menu also steps the measure machines (wave108
                                // MAJOR-2; LoS/viewshed victims in waves 109–110).
                                if crate::core::ui::modal_stack::any_open() {
                                    false
                                } else {
                                    // T-723 — Esc disarms an armed place BEFORE the measure-tool seam.
                                    // `cancel_pending` was unreachable from the keyboard; Eden stamp
                                    // cancel is Esc (and RMB on pointerup). Clear the ghost with the arm.
                                    let place_acted = if editor_ops::has_pending() {
                                        editor_ops::cancel_pending();
                                        if let Some(e) = engine.borrow_mut().as_mut() {
                                            e.clear_place_preview();
                                        }
                                        true
                                    } else {
                                        false
                                    };
                                    // T-792 — Esc during an in-progress ZONE/TRIGGER draw (circle
                                    // centre placed, or polygon with ≥1 vertex) abandons the pending
                                    // geometry. A zone draw is multi-click and deliberately SURVIVES
                                    // `cancel_pending` (so the panel's own Close/Undo/Cancel buttons,
                                    // whose pointerup bubbles to the map, don't destroy the ring) —
                                    // `has_pending()` above is therefore false-negative for a draw,
                                    // and F-31 was: arm Circle, click centre, Esc → the "click the
                                    // rim" hint STAYED and the next click still completed the circle.
                                    // `cancel_zone_draw` is the ONE cancel a draw honours; it clears
                                    // the draft and bumps the dock tick so the rim/vertex hint (gated
                                    // on `zone_draft()` under `doc_tick`) vanishes — the same effect
                                    // as the panel Cancel button, now on the keyboard. The draft is
                                    // SHARED by the trigger tool (`begin_zone_draw(.., Trigger)` sets
                                    // the identical `Pending::Zone`), so this one call also cancels an
                                    // in-progress trigger draw. It "acts" only when a draw was in
                                    // flight, so an Esc with no draw falls through untouched — and
                                    // when it DOES act, feeding it to the `||` below prevents the
                                    // browser default and stops any lower Esc layer (dialog/menu/tab)
                                    // consuming the SAME press (one-Esc-one-layer, T-813/T-814).
                                    let zone_draw_acted = editor_ops::cancel_zone_draw();
                                    // T-768 — Esc disarms an armed connect the same way it disarms an
                                    // armed place (T-723). Completing stays LMB pick / RMB Complete.
                                    let connect_acted = if editor_ops::pending_connect().is_some() {
                                        editor_ops::cancel_connect();
                                        true
                                    } else {
                                        false
                                    };
                                    let ruler_acted = ruler.borrow_mut().escape();
                                    if ruler_acted {
                                        sync_ruler();
                                    }
                                    let los_acted = los.borrow_mut().escape();
                                    if los_acted {
                                        sync_los();
                                    }
                                    // T-644 — the viewshed's Esc is one step (clear the placed
                                    // observer + raster); on a real dismissal also drop the engine wash
                                    // lane. Like the ray, only one LoS lane is ever non-empty at a time
                                    // (the sub-mode toggle clears the other), so calling it unconditionally
                                    // is safe — an empty viewshed's `.escape()` is a false no-op.
                                    let viewshed_acted = viewshed.borrow_mut().escape();
                                    if viewshed_acted {
                                        if let Some(e) = engine.borrow_mut().as_mut() {
                                            e.viewshed_clear();
                                        }
                                    }
                                    place_acted
                                        || zone_draw_acted
                                        || connect_acted
                                        || ruler_acted
                                        || los_acted
                                        || viewshed_acted
                                }
                            }
                            "KeyC" if modk && !ev.alt_key() && !ev.shift_key() => {
                                editor_ops::copy_selection()
                            }
                            // T-669 ACTION-CUT-001 — Ctrl/Cmd+X is COPY, then DELETE, in that order
                            // and SHORT-CIRCUITED. `copy_selection` returns false when there was
                            // nothing to put on the clipboard (empty selection, or the ops context /
                            // doc is not up yet), and a cut that could not copy must NOT delete —
                            // that would be a silent destructive Delete wearing an X. `&&` is exactly
                            // that guarantee: `delete_selection` never runs unless the clipboard took
                            // the snapshot first. Both halves are pre-existing `editor_ops`
                            // primitives, so this arm adds no new doc write and no new undo step
                            // beyond the one `delete_selection` already files.
                            //
                            // Census: X was bound by NEITHER window-level editor keydown before this
                            // slice (this file's nor `mission_history`'s Ctrl+Z/Y one) — pinned by
                            // `t669_cut_key_census`. It carries the same guard shape as the C / V
                            // arms it sits between, so the top-of-closure `in_editable_field()` guard
                            // keeps Ctrl+X meaning "cut the text" while the operator is typing in an
                            // Attributes field.
                            "KeyX" if modk && !ev.alt_key() && !ev.shift_key() => {
                                editor_ops::copy_selection() && editor_ops::delete_selection()
                            }
                            // T-743 — THE PLAIN PASTE ALWAYS CARRIES AN ANCHOR. It used to hand
                            // `paste_at_cursor` the raw `cx`/`cy`, which are `None` whenever the
                            // pointer is off the map (over any chrome panel, or before the first
                            // pointermove) — and a `None` anchor is now the paste-at-original
                            // instruction, which is emphatically not what a plain Ctrl/Cmd+V means.
                            // So the fallback is resolved HERE, where the camera is, instead of in
                            // the document core where the two intents used to share one branch:
                            // cursor if there is one, otherwise the centre of the visible map. See
                            // `plain_paste_anchor` for why that is the fallback and not a no-op.
                            //
                            // The centre is unprojected against `frozen_camera` — the SAME camera the
                            // CUR read-out and every pick use — so "the middle of the view" means the
                            // same world point here as it would if the operator had put the pointer
                            // there and pressed Ctrl+V. `try_borrow` rather than `borrow`: this is a
                            // window-level listener that can fire during a frame that already holds
                            // the engine, and a panic in a keydown takes the whole editor with it.
                            "KeyV" if modk && !ev.alt_key() && !ev.shift_key() => {
                                let rect = container.get_bounding_client_rect();
                                let view_centre = engine
                                    .try_borrow()
                                    .ok()
                                    .and_then(|g| {
                                        g.as_ref().map(|e| {
                                            crate::editor::tools::select_tool::frozen_camera(
                                                rect.width(),
                                                rect.height(),
                                                e.target_x(),
                                                e.target_y(),
                                                e.zoom(),
                                            )
                                            .unproject_xy(rect.width() / 2.0, rect.height() / 2.0)
                                        })
                                    })
                                    .filter(|c| c[0].is_finite() && c[1].is_finite())
                                    .map(|c| (c[0], c[1]));
                                match plain_paste_anchor(cx.zip(cy), view_centre) {
                                    Some((ax, ay)) => {
                                        editor_ops::paste_at_cursor(Some(ax), Some(ay))
                                    }
                                    None => false,
                                }
                            }
                            // T-669 ACTION-PASTE-ORIG-001 — Ctrl/Cmd+Shift+V pastes with NO cursor
                            // anchor. `paste_at_cursor`'s anchor is `Option`al and that option IS the
                            // feature: `Some(ax, ay)` translates the clip's centroid onto that point
                            // (the plain paste arm above), `None` leaves every slot on its SOURCE
                            // coordinates.
                            //
                            // T-743 — AND IT NOW MEANS THAT EXACTLY. This comment used to carry a
                            // wrinkle: `Doc::paste_slots`' no-anchor arm added a 20 m `PASTE_NUDGE`
                            // to both axes for byte-parity with the JS `ydoc.pasteSlots`, so the
                            // command named "paste at the source position" put every slot 20 m off
                            // it. The operator retired that parity (it was a migration safety net,
                            // not a contract); the arm translates by nothing, and the help row's
                            // "source position" is now a literal statement rather than an
                            // approximation. The off-map plain paste that used to share the nudged
                            // branch is answered in the arm above.
                            //
                            // MUTUAL EXCLUSION with the plain paste arm: that arm guards
                            // `!ev.shift_key()`, this one guards `ev.shift_key()`, and both require
                            // `modk && !ev.alt_key()`. One `KeyboardEvent` has exactly one `shiftKey`
                            // value, so at most one of the pair can ever match — they partition the
                            // Ctrl+V space rather than overlapping it, and the order they appear in
                            // is therefore irrelevant. Pinned by
                            // `the_two_paste_arms_are_mutually_exclusive`.
                            "KeyV" if modk && !ev.alt_key() && ev.shift_key() => {
                                editor_ops::paste_at_cursor(None, None)
                            }
                            // T-649 SEL-ALL-001 — Ctrl/Cmd+A selects everything IN VIEW. Eden scopes
                            // Select All to the viewport, not to the whole mission, so this hands the
                            // container's live CSS size to `select_all_in_view`, which runs the
                            // marquee's own `pick_rect` over the on-screen rect — an entity parked
                            // off-screen is deliberately NOT selected.
                            //
                            // Census: `KeyA` was bound by NEITHER window-level editor keydown before
                            // this slice (this file's nor `mission_history`'s Ctrl+Z/Y one) — pinned
                            // by `t649_ctrl_a_census`. It sits beside `KeyC` / `KeyV` because it is
                            // the same modifier family and the same top-of-closure
                            // `in_editable_field()` guard is what keeps Ctrl+A meaning "select the
                            // text" while the operator is typing in an Attributes field.
                            //
                            // Returning "acted" is load-bearing: `prevent_default` below is what
                            // stops the browser's own Select All blue-washing the editor chrome.
                            "KeyA" if modk && !ev.alt_key() && !ev.shift_key() => {
                                let rect = container.get_bounding_client_rect();
                                editor_ops::select_all_in_view(rect.width(), rect.height())
                            }
                            // T-635 — Ctrl/Cmd+Alt+D toggles the telemetry HUD (default hidden).
                            // Behind the same `in_editable_field()` guard at the top of this closure,
                            // so it never fires while typing in an Attributes field. It always "acts"
                            // (flips the signal) → `prevent_default` below. This gates TELEMETRY only;
                            // mission-correctness diagnostics stay always-on (see the `debug_hud_shown`
                            // declaration note, framework_synthesis §D.4 #7).
                            "KeyD" if modk && ev.alt_key() && !ev.shift_key() => {
                                debug_hud_shown.set(!debug_hud_shown.get_untracked());
                                true
                            }
                            "Space" if !modk => editor_ops::center_on_selection(),
                            // T-662 — Delete still removes the selection. Backspace is NO LONGER an
                            // alias for Delete; it toggles the Eden chrome (hide/show interface), so
                            // the two keys are now split arms. Backspace always "acts" (it flips the
                            // signal), so `prevent_default` fires below to keep the browser from
                            // treating it as a Back navigation.
                            // T-780 — `CONN-DEL-001` ON THE MAP. Eden deletes a connection by
                            // selecting its line and pressing Del; T-672 could only offer the
                            // panel's per-row button because there was no line. There is now, so
                            // Del over a selected edge removes it.
                            //
                            // It calls `editor_ops::delete_connection` — the EXACT function the
                            // panel's Delete button calls, which is the whole reason this arm is
                            // three lines. A map-side `core.remove_connection` here would be a
                            // second deletion path: a second place to keep the `after_local_edit`
                            // tail, the `connection_count` guard and the one-txn-one-Ctrl+Z promise
                            // correct, and the second one is always the one that rots (T-241).
                            //
                            // A BRANCH INSIDE THE ARM, not a second `"Delete"` arm with a guard.
                            // Both would behave identically, but a new guard TERM has to be readable
                            // by `eden_help`'s keymap census (`parse_guard`) — the census refuses to
                            // guess at a term it cannot parse, because a misread guard would silently
                            // widen or narrow the binding it reports collisions for. `Delete` keeps
                            // its one arm and its one censused `!modk` guard; what changed is what
                            // the arm DOES, which is not a keymap fact.
                            //
                            // [wave 142 F-1] THE ARM RESOLVES ITS SELECTION AGAINST THE DOCUMENT
                            // before it fires. `editor_ops` reconciles the signal on every selection
                            // and every document change (see the declaration), but the branch here
                            // must not DEPEND on that having run: what an armed id is worth is a
                            // question about the live graph, and asking it one expression before the
                            // delete costs nothing. An id the document no longer holds falls through
                            // to the entity delete rather than being handed to a verb that can only
                            // answer `false` — which would swallow the keypress in silence.
                            //
                            // `connection_exists` is the SAME question `delete_connection` gates on,
                            // so the branch taken here and the write that follows cannot disagree.
                            //
                            // The armed id is dropped either way. On the delete it is the disarm (the
                            // edge is gone; leaving it selected would leave Del pointing at a row that
                            // no longer exists, and a Ctrl+Z would silently re-arm it). On the stale
                            // branch it is the same disarm one keypress earlier than the reconcile.
                            "Delete" if !modk => {
                                let armed = selected_connection.try_get_untracked().flatten();
                                if armed.is_some() {
                                    selected_connection.set(None);
                                }
                                match armed.filter(|id| editor_ops::connection_exists(id)) {
                                    Some(id) => editor_ops::delete_connection(&id),
                                    None => editor_ops::delete_selection(),
                                }
                            }
                            "Backspace" if !modk => {
                                chrome_hidden.set(!chrome_hidden.get_untracked());
                                true
                            }
                            // T-638 — E toggles the LEFT dock (Entity List), R the RIGHT (Asset
                            // Browser). Bare keys only (no Ctrl/Cmd/Alt/Shift) so Ctrl+R stays a
                            // browser reload and Alt/Shift combos are untouched; the top-of-closure
                            // `in_editable_field()` guard already keeps them from firing while typing
                            // in an Attributes field. Each always "acts" (flips its latch) → the
                            // reflow + centre-hold run off the Effect that observes the signal.
                            "KeyE" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                dock_left_collapsed.set(!dock_left_collapsed.get_untracked());
                                true
                            }
                            "KeyR" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                dock_right_collapsed.set(!dock_right_collapsed.get_untracked());
                                true
                            }
                            // ══════════════════════ T-648 — the snap grid + transform widget ══════
                            // KEY-GRID-001 — `G` toggles the snap-grid MASTER latch. Census: `KeyG`
                            // is bound by NOTHING in this editor keydown or `mission_history`'s (the
                            // only two window-level editor keydowns) — see the census pin
                            // `t648_keydown_census`. Bare key only (no Ctrl/Cmd/Alt/Shift), behind
                            // the top-of-closure `in_editable_field()` guard like E/R, so it never
                            // fires while typing. Chosen over Eden's `odiaeresis`/`;` keysym
                            // artefacts (the ticket's instruction) — a plain letter mnemonic for
                            // "grid". Always acts (flips the latch) → prevent_default below.
                            "KeyG" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                snap.set(snap.get_untracked().toggled());
                                true
                            }
                            // TOOLBAR-GRID-MOVE-001 — `[` / `]` DECREASE / INCREASE the active snap
                            // step. Census: `BracketLeft`/`BracketRight` are bound by nothing in
                            // either editor keydown. They step the ladder of the CURRENT widget
                            // variant (translate variant → translation ladder, rotate variant →
                            // rotation ladder), so the one pair of keys tunes whichever grid the
                            // operator is working in. Clamped at both ends by `SnapState::stepped`.
                            // Only "act" (→ prevent_default) when a keypress at a ladder end still
                            // reports a change is unnecessary — we always return true because the
                            // key is ours regardless, and `[`/`]` have no browser default worth
                            // preserving inside the editor.
                            "BracketLeft" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                let axis = widget_variant.get_untracked().snap_axis();
                                snap.set(snap.get_untracked().stepped(axis, -1));
                                true
                            }
                            "BracketRight" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                let axis = widget_variant.get_untracked().snap_axis();
                                snap.set(snap.get_untracked().stepped(axis, 1));
                                true
                            }
                            // WIDGET-CYCLE-001 — `1` / `2` / `3` select the widget VARIANT, numbered
                            // to MATCH Eden's widget row EXACTLY (T-795, pixel-verified): `1` No Widget
                            // / `2` Translate / `3` Rotate. This is the Space-collision decision: Eden
                            // cycles variants on Space, but TBD's Space stays flyTo
                            // (`center_on_selection`, the arm above), and Eden's `1`-`5` direct keys
                            // are free here (census: no `Digit*` binding anywhere in the frontend), so
                            // `1`/`2`/`3` dissolve the clash without touching Space. Before T-795 these
                            // were OFF BY ONE (1=Translate, 2=Rotate, 3=nothing) — an Eden author's
                            // muscle memory armed the wrong mode three ways over; the renumber makes
                            // the keys mean what Eden means. `4`/`5` (Area Scaling / Area) stay
                            // RESERVED-UNBOUND — no area-scale variant yet (a transform selection is
                            // slots + vehicles, neither of which scales; see `WidgetVariant`'s doc);
                            // the numbering matches Eden so a later slice binds them without renumbering.
                            // Bare digit only (behind the `in_editable_field()` guard at the top of this
                            // closure, so a digit typed into an attribute field never flips a mode).
                            // `from_digit` always changes the variant for 1/2/3 → act.
                            "Digit1" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                widget_variant.set(widget_variant.get_untracked().from_digit(1));
                                true
                            }
                            "Digit2" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                widget_variant.set(widget_variant.get_untracked().from_digit(2));
                                true
                            }
                            "Digit3" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                widget_variant.set(widget_variant.get_untracked().from_digit(3));
                                true
                            }
                            _ => false,
                        };
                        if handled {
                            ev.prevent_default();
                        }
                    },
                );
                if let Some(win) = web_sys::window() {
                    let _ = win.add_event_listener_with_callback(
                        "keydown",
                        onkeydown.as_ref().unchecked_ref(),
                    );
                }
                onkeydown.forget();
            }

            // T-638 — the collapse REFLOW + CENTRE-HOLD. One Effect observes the three chrome-layout
            // signals (`chrome_hidden`, both dock collapse latches) and is the SINGLE writer of the
            // `eden_layout` inset latch, so the wasm hot-path readers (`select_tool::farthest_empty_px`,
            // the palette-drop `on_canvas` gate) and the DOM chrome never disagree about the live inset.
            //
            // Reflow: our canvas is FULL-BLEED (it always spans the window; the docks overlay it and the
            // insets define the chrome-free MAP PANE), so a collapse does NOT reallocate the device
            // buffer the way Eden's inset-shrunk canvas does — the pane simply grows into the freed
            // width. We still route the change through `e.resize` (identical dims) to mark damage and
            // keep "a layout change goes through resize" true; the visible motion is the centre-hold.
            //
            // Centre-hold DECISION (the ticket's STILL-OPEN item): hold the world point under the MAP
            // PANE CENTRE across a DOCK-COLLAPSE reflow, so the map appears to SLIDE into the freed
            // space, not jump (Eden's behaviour). Implemented as a target nudge computed from the
            // pane-centre delta (`eden_layout::centre_hold_target`) applied via `set_view` (which
            // clamps to bounds) — the engine's own `resize` never moves the target, so without this the
            // world point under the pane centre would shift by the half-inset change.
            //
            // The centre-hold is DELIBERATELY scoped to dock toggles while the chrome is SHOWN. Toggling
            // T-662's `chrome_hidden` also changes the insets (to/from full-bleed), but Backspace
            // hide-interface must not slide the map (its wave-101 behaviour — the chrome just vanishes
            // over a still camera), so a run where `chrome_hidden` is set on either side skips the nudge.
            // The inset MIRROR always runs (the accessors must be correct even while hidden); only the
            // camera nudge is gated. That is the concrete "chrome_hidden × collapse are orthogonal"
            // interaction on the camera: hidden zeroes the insets but never moves the world.
            {
                let engine = engine.clone();
                let container = container.clone();
                Effect::new(move |_| {
                    // Track all three so any toggle re-runs this.
                    let hidden = chrome_hidden.get();
                    let left = dock_left_collapsed.get();
                    let right = dock_right_collapsed.get();
                    // The pre-mirror hidden state (the Cell still holds it) — used to gate the nudge so
                    // an un-hide (was_hidden → shown) also skips the slide.
                    let was_hidden = crate::editor::layout::chrome_hidden();

                    let rect = container.get_bounding_client_rect();
                    let (w, h) = (rect.width(), rect.height());
                    if !(w > 0.0 && h > 0.0) {
                        // Still mirror the state so the accessors are correct before first layout.
                        crate::editor::layout::set_chrome_hidden(hidden);
                        crate::editor::layout::set_dock_left_collapsed(left);
                        crate::editor::layout::set_dock_right_collapsed(right);
                        return;
                    }

                    // Pane centre with the PREVIOUS insets (the Cells still hold the pre-toggle state).
                    let before = crate::editor::layout::pane_center_px(w, h);
                    // Commit the new inset state, then read the pane centre AFTER.
                    crate::editor::layout::set_chrome_hidden(hidden);
                    crate::editor::layout::set_dock_left_collapsed(left);
                    crate::editor::layout::set_dock_right_collapsed(right);
                    let after = crate::editor::layout::pane_center_px(w, h);

                    let dpr = web_sys::window()
                        .map(|win| win.device_pixel_ratio())
                        .unwrap_or(1.0);
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        // Full-bleed: dims unchanged, but resize marks damage + keeps the contract.
                        let _ = e.resize(w, h, dpr);
                        // Nudge ONLY for a dock reflow while the chrome is shown on both sides.
                        let dock_reflow = !was_hidden && !hidden;
                        if dock_reflow
                            && ((before.0 - after.0).abs() > f64::EPSILON
                                || (before.1 - after.1).abs() > f64::EPSILON)
                        {
                            let scale = e.zoom().exp2();
                            let (nx, ny) = crate::editor::layout::centre_hold_target(
                                e.target_x(),
                                e.target_y(),
                                scale,
                                before,
                                after,
                            );
                            e.set_view(nx, ny, e.zoom());
                        }
                    }
                });
            }

            // T-159.17 — persistence layer (additive; the SYNCHRONOUS seed above keeps the doc smoke
            // synchronous — `smoke_doc_editor` still sees 8 slots immediately on its own cold origin).
            // The `window.__missionPersist` bridge is installed synchronously (so the gate can wait on
            // it); the IDB load / initial-persist / warm-mark run async below and flip `ready` last.
            let persist_ready = Rc::new(Cell::new(false));
            let persist_loaded = Rc::new(Cell::new(false));
            yrs_persist::register_mission_persist(
                doc.clone(),
                mission_id.clone(),
                persist_ready.clone(),
                persist_loaded.clone(),
            );
            // T-175 B1 — two-party handshake so restored/hydrated slot positions always reach the
            // GPU regardless of which async task (IDB restore / server hydrate vs engine create)
            // finishes first. Whichever sets its flag second runs the single authoritative
            // `rebind_engine_from_doc` from the settled doc — no seed→restore flash, no double bind.
            // (`restore_settled` itself is declared above `set_ctx` — see the T-380 note there.)
            let engine_mounted = Rc::new(Cell::new(false));
            // T-175 B5 — the two boot-readiness flags the loading overlay clears on: doc settled
            // (restore + hydrate) and world/map-asset residency settled. `boot` → Ready when both.
            let world_ready = Rc::new(Cell::new(false));
            // T-628 — the single reporter both boot tasks fold their measurements into. `RwSignal`
            // is `Copy`, so the closure captures it by value and the `Rc` can be cloned into futures
            // that outlive this scope.
            let report: boot_progress::ProgressFn =
                Rc::new(move |ev| progress.update(|p| p.apply(ev)));
            spawn_local({
                let doc = doc.clone();
                let id = mission_id.clone();
                let ready = persist_ready.clone();
                let loaded = persist_loaded.clone();
                let restore_settled = restore_settled.clone();
                let engine_mounted = engine_mounted.clone();
                let world_ready = world_ready.clone();
                let report = report.clone();
                async move {
                    // 1. Restore from IDB if a blob exists — SWAP a fresh core (mirrors React's
                    //    empty-shell + apply; rests on the tested fresh-peer path + persist_roundtrip_ok,
                    //    NOT on reapply-idempotence). The swap is a synchronous block: no `borrow`/
                    //    `borrow_mut` is ever held across an `.await` (the engine task shares this `Rc`).
                    if let Some(blob) = yrs_persist::load_state(&id).await {
                        if !blob.is_empty() {
                            let fresh = map_engine_core::doc::MissionDocCore::new();
                            fresh.set_origin_init(true);
                            let ok = fresh.apply_update(&blob).is_ok();
                            fresh.set_origin_init(false);
                            if ok {
                                *doc.borrow_mut() = Some(fresh);
                                loaded.set(true);
                                // T-159.21 — the restored core is a DIFFERENT document: its slot
                                // count may differ and its undo stack is empty (the replay ran under
                                // INIT). Re-seed the HUD mirrors off it, or the toolbelt would show
                                // the pre-restore counts. Not `after_local_edit`: nothing was edited,
                                // and re-arming the persist writer here would echo the restore back.
                                mission_history::refresh_hud();
                                // T-189 — but DO tell the truth about `dirty`. The restored blob is
                                // local work that has never been through a Save (a Save is the only
                                // thing that clears the flag), and leaving `dirty` at its `false`
                                // mount default meant the strip showed no modified dot and the
                                // unload guard stayed silent on exactly the state it exists to
                                // protect. This is the flag ONLY — still not `after_local_edit`, for
                                // the reason above.
                                //
                                // The hydrate below is what can prove local is clean: both adopt
                                // paths (`adopt_payload` → `set_dirty(false)`) correct this to
                                // false. The one path that neither proves nor disproves — local
                                // derives from the exact server semver, so hydrate trusts it — keeps
                                // dirty=true, which is that path's own stated premise ("the delta is
                                // the user's own unsaved edits"). A save-then-immediately-reopen
                                // therefore shows the dot with a zero delta: the adopted marker
                                // records a semver, not a document digest, so nothing on this path
                                // can tell that case apart — and over-warning is the safe side of
                                // the failure it replaces (silent loss).
                                mission_history::set_dirty(true);
                            }
                        }
                    }
                    // 1.5 T-159.26 — server hydrate / conflict / dirty (UUID missions only; the
                    //     `smoke` gate route is non-UUID and skips this, so the editor smokes are
                    //     untouched). Replaces the seed with the saved version, or prompts on a
                    //     genuine local-vs-server conflict — the data-safety guarantee.
                    mission_hydrate::hydrate_from_server(
                        doc.clone(),
                        id.clone(),
                        auth,
                        loaded.get(),
                        current_semver,
                        conflict,
                        report.clone(),
                    )
                    .await;
                    // T-761 — Export Compiled parks findings in a thread_local; client-side
                    // `/missions/:id/edit` remounts reuse the wasm instance, so clear on hydrate
                    // or mission B inherits A's build report (wave-116 finding 3).
                    validation_panel::clear_compile_findings();
                    // T-628 — the mission segment is over the instant the hydrate returns, on every
                    // one of its paths (adopted / trusted-local / conflict / 404 / offline). Closing
                    // it here rather than inside the hydrate is what stops a network failure from
                    // parking the bar short of 100% with the overlay still up.
                    report(boot_progress::BootEvent::Finish(
                        boot_progress::BootSeg::Mission,
                    ));
                    // 1.75 T-175 B1 — the doc is now settled (IDB restore + server hydrate). Mark it
                    //      and, if the engine already mounted + first-bound the seed, rebind it from
                    //      the settled doc so restored slot positions render (not the seed).
                    //
                    //      T-380 — this same flip OPENS THE PERSIST GATE (`mission_history`'s
                    //      `after_doc_change`). It is deliberately here and not at `ready.set(true)`
                    //      below: this is the first instant at which an edit-driven write can no
                    //      longer clobber a better record, and everything between here and `ready` is
                    //      await-free, so waiting would buy nothing and would delay a real edit's
                    //      persist. Both awaits above are unconditional, so the gate cannot be opened
                    //      by a path that skipped the restore — and if either await never returns the
                    //      gate stays shut, which is the correct side: the doc is still the fixture,
                    //      the overlay is still up, and a write would be the corruption itself.
                    restore_settled.set(true);
                    if engine_mounted.get() {
                        mission_history::rebind_engine_from_doc();
                    }
                    // T-175 B5 — doc is hydrated: advance the loading overlay (→ Ready if the world
                    // already settled, else keep the overlay up until the world task finishes).
                    // T-631 — but not past a `Failed`: if the engine task has already reported a
                    // GPU-init failure, this task is the "misleading event" that must not bury the
                    // reason. `hand_over` self-guards; the `LoadingMap` write goes through `advance`
                    // so a hydrate that finishes after an engine failure cannot re-spinner the
                    // overlay on top of the error.
                    if world_ready.get() {
                        hand_over(boot);
                    } else {
                        boot.update(|b| *b = b.clone().advance(BootPhase::LoadingMap));
                    }
                    // 2. Initial persist through the debounced writer (get_bytes read at write time;
                    //    cancel when the doc Option is cleared). No mutator hook exists yet, so this
                    //    post-seed/post-load encode is the writer's trigger this slice.
                    {
                        let doc_get = doc.clone();
                        let doc_cancel = doc.clone();
                        yrs_persist::save_state_debounced(
                            &id,
                            Box::new(move || {
                                doc_get
                                    .borrow()
                                    .as_ref()
                                    .map(|c| c.encode_state())
                                    .unwrap_or_default()
                            }),
                            Box::new(move || doc_cancel.borrow().is_none()),
                            yrs_persist::debounce_ms(),
                        );
                    }
                    // 3. Warm-session marker after the doc is ready.
                    let n = doc
                        .borrow()
                        .as_ref()
                        .map(|c| c.slot_count() as u32)
                        .unwrap_or(0);
                    crate::editor::state::session::mark_ready(&id, n, None);
                    // 4. Flush-on-hide listeners (visibilitychange/hidden + pagehide).
                    yrs_persist::register_flush_on_hide(id.clone());
                    // 5. Ready LAST — the gate waits on this before asserting.
                    ready.set(true);
                }
            });

            spawn_local({
                let engine = engine.clone();
                let disposed = disposed.clone();
                let doc = doc.clone();
                let canvas = canvas.clone();
                let map_host = map_host.clone();
                let dem_grid = dem_grid.clone();
                let restore_settled = restore_settled.clone();
                let engine_mounted = engine_mounted.clone();
                let world_ready = world_ready.clone();
                let report = report.clone();
                let (cw, ch) = (rect0.width(), rect0.height());
                async move {
                    match map_engine_render::RenderEngine::create(canvas, force_webgl).await {
                        Ok(mut eng) => {
                            if disposed.load(Ordering::Relaxed) {
                                return;
                            }
                            let _ = eng.resize(cw, ch, dpr0);
                            eng.set_camera_bounds(0.0, 0.0, TERRAIN_W, TERRAIN_H);
                            eng.set_view(INITIAL_TARGET.0, INITIAL_TARGET.1, INITIAL_ZOOM);
                            eng.hide_calibration();
                            // Drop the GpuTimer readback lane: no fps HUD in the editor yet, and on
                            // headless WebGPU its map_async double-maps the 16-byte buffer on the 2nd
                            // submit ("Buffer is already mapped"). `poll()` (below, per frame) keeps
                            // the WebGL2-fallback + future cull-counter readback honest.
                            eng.disable_frame_timing();
                            eng.set_continuous_render(false); // damage-driven, matches the prod oracle
                                                              // T-172 B4 — upload the slot atlas BEFORE the first SoA bind: the whole
                                                              // slot lane (bind / selection tint / drag overlay) is gated on
                                                              // `atlas_ready`, and no frontend ever called this — placed slots were
                                                              // selectable but invisible.
                                                              // T-790 — the atlas is now the WIDENED marker atlas: cells 0/1
                                                              // (ring/disc) stay byte-identical to the old two-cell `build_slot_atlas`
                                                              // (so slots / vehicles / comments are pixel-unchanged), and cells 2..
                                                              // add the per-icon marker glyph shapes `markers_bind` selects.
                            {
                                let (rgba, width, height, uv) =
                                    map_engine_render::scene::build_marker_slot_atlas();
                                if let Err(e) = eng.ensure_slot_atlas(&rgba, width, height, &uv) {
                                    leptos::logging::error!("ensure_slot_atlas: {e:?}");
                                }
                            }
                            *engine.borrow_mut() = Some(eng);
                            register_self_checks(engine.clone());
                            register_editor_cam(engine.clone(), map_host.clone());
                            register_slot_stats(engine.clone());
                            // T-173 P6 — let the Mission Settings render-pref controls reach the
                            // live engine + host.
                            crate::editor::world_assets::register_render_ctx(
                                engine.clone(),
                                map_host.clone(),
                            );
                            // T-159.16 — doc→engine bind (D5): with the atlas up, this first bind
                            // materializes + draws the seeded slot set.
                            //
                            // T-808 — through the SYMBOLOGY bind, like the two `mission_history`
                            // feeds. This is the bind the operator sees FIRST (and the only one a
                            // mission that is never edited ever gets), so leaving it on
                            // `slots_bind_soa` would have opened every mission as a field of
                            // north-pointing riflemen until the first commit re-bound it.
                            // T-819 — map-render SoA (crewed slots derived-hidden; materialize untouched).
                            let soa = doc.borrow().as_ref().map(map_render_slot_soa);
                            if let (Some(soa), Some(e)) =
                                (soa.as_ref(), engine.borrow_mut().as_mut())
                            {
                                let tints = map_engine_core::slots_gpu::side_tints_rgba_bytes(
                                    &soa.side_keys,
                                );
                                e.slots_bind_symbology(
                                    soa.ids.clone(),
                                    &soa.xy,
                                    &tints,
                                    mission_history::soa_roles(soa),
                                    &soa.rotations,
                                );
                            }
                            // T-175 B1 — engine is mounted + first-bound. If the IDB restore + hydrate
                            // already settled, rebind now from the settled doc (the first bind above
                            // may have drawn the pre-restore seed); otherwise the restore task will
                            // rebind once it settles. Exactly one authoritative rebind runs.
                            engine_mounted.set(true);
                            if restore_settled.get() {
                                mission_history::rebind_engine_from_doc();
                            }
                            start_raf(engine.clone(), disposed.clone(), debug_hud, scale_mpp);
                            // T-166 — full map-asset host (hillshade + sat + DEM vectors + world +
                            // forest). Terrain from doc meta (seed/hydrate; default everon).
                            {
                                let terrain = doc
                                    .borrow()
                                    .as_ref()
                                    .and_then(|c| {
                                        serde_json::from_str::<serde_json::Value>(
                                            &c.small_maps_json(),
                                        )
                                        .ok()?
                                        .get("meta")?
                                        .get("terrain")?
                                        .as_str()
                                        .map(str::to_string)
                                    })
                                    .unwrap_or_else(|| "everon".to_string());
                                let host = map_host.clone();
                                // T-175 B5 — mark the world settled + clear the loading overlay once
                                // the map-asset / residency bootstrap finishes (→ Ready if the doc
                                // already hydrated, else the hydrate task flips Ready).
                                // T-628 — the bootstrap folds the DEM's, the satellite's and the
                                // world's real measurements into the same one bar through this
                                // reporter, and closes all three of its segments before it returns.
                                let boot_fut = crate::editor::world_assets::bootstrap(
                                    engine.clone(),
                                    terrain,
                                    host,
                                    dem_grid.clone(),
                                    report.clone(),
                                );
                                let world_ready = world_ready.clone();
                                let restore_settled = restore_settled.clone();
                                spawn_local(async move {
                                    boot_fut.await;
                                    world_ready.set(true);
                                    if restore_settled.get() {
                                        hand_over(boot);
                                    }
                                });
                            }
                        }
                        Err(e) => {
                            // T-631 — the failure path. `RenderEngine::create` returns `Err` on a
                            // WebGPU/GL init failure (no adapter, a lost device, `createBuffer size
                            // too large` on a swiftshader/blocklisted GPU). Before this slice the
                            // `Err` arm only logged and returned: the world task that flips the
                            // overlay down never ran (it is INSIDE the `Ok` arm), so the bar sat at
                            // its last honest reading forever — no error, no reason, no retry.
                            //
                            // Now the boot is driven into `Failed`, carrying the REAL reason. `e` is
                            // a `JsError`; its text is only reachable through JS (`JsError: Display`
                            // prints "JsValue(...)"), so read it off the underlying `Error.message`.
                            let reason = js_sys::Error::from(wasm_bindgen::JsValue::from(e))
                                .message()
                                .as_string()
                                .filter(|s| !s.is_empty())
                                .unwrap_or_else(|| "the render engine failed to start".to_string());
                            leptos::logging::error!("RenderEngine::create: {reason}");
                            if disposed.load(Ordering::Relaxed) {
                                return;
                            }
                            // Name the segment the operator was actually watching when it died —
                            // the first unfinished one, i.e. the "Loading terrain… 50%" the ticket
                            // reproduced — rather than a generic apology. `advance` makes this
                            // sticky: if the doc-hydrate task later reaches for `LoadingMap`/`Ready`
                            // it is a no-op and this reason survives (the "misleading event does not
                            // overwrite" guarantee).
                            let seg = progress.get_untracked().stage();
                            // The map is dead from this instant, independent of whether the operator
                            // is still looking at the error overlay or has dismissed it to keep
                            // working — so the labelled-dead-pane badge is armed here, not on the
                            // Continue click.
                            map_disabled.set(Some(reason.clone()));
                            boot.update(|b| {
                                *b = b.clone().advance(BootPhase::Failed { seg, reason })
                            });
                        }
                    }
                }
            });

            // T-159.15.2 — pan gesture state: `Some((last_client_x, last_client_y))` while an
            // MMB/RMB drag-pan is in flight, else `None`. The pan feeds INCREMENTAL client-px deltas
            // to `engine.pan` (the camera does `target -= dΧ/scale` at the LIVE scale — Rust owns the
            // ortho math; this mirrors the `WgpuCanvas` oracle, NOT the Deck frozen-viewport path
            // that `useSelectTool` uses and the language gate forbids here). `(f64, f64)` is `Copy`,
            // so a `Cell` suffices (no `RefCell`); JS is single-threaded, so these pointer handlers
            // never reenter the rAF loop's `borrow_mut`.
            let pan_px: Rc<Cell<Option<(f64, f64)>>> = Rc::new(Cell::new(None));

            // T-802 (O-8) — the HOVER CURSOR's two pieces of session-local state, born beside
            // `pan_px`/`left` and owned by nothing else. `HoverState` is `Copy` (throttle clock +
            // pickable claim + hysteresis anchor) so a `Cell` suffices; `HoverPoints` is the pick's
            // point sets cached per `doc_tick`, so it needs a `RefCell`. NEITHER is gesture state:
            // the hover read never takes, writes or consumes anything the T-723/T-795/T-796 arms own.
            let hover_state: Rc<Cell<HoverState>> = Rc::new(Cell::new(HoverState::default()));
            let hover_points: Rc<RefCell<Option<HoverPoints>>> = Rc::new(RefCell::new(None));
            // Assert the RESTING cursor once, at mount. Without this the canvas reads the UA `auto`
            // until the first pointer move that finds nothing — so "over empty ground" would be two
            // different computed values depending on whether the pointer had ever been over a glyph.
            // `hover_cursor_css(false)` (not a literal) so there is exactly one source for the value.
            set_map_cursor(&canvas, false);

            // Wheel → zoom_at (engine self-clamps zoom to [-6, 6]). Capture + non-passive so we can
            // preventDefault and beat any child handler. CSS origin = the container rect (same basis
            // as the pan/pick math).
            let onwheel = Closure::<dyn FnMut(web_sys::WheelEvent)>::new({
                let engine = engine.clone();
                let container = container.clone();
                let pan_px = pan_px.clone();
                let map_host = map_host.clone();
                move |ev: web_sys::WheelEvent| {
                    // T-159.22 — the wheel is capture-phase on the CONTAINER, so it fires before any
                    // dock could stop it (that is deliberate: it is what lets `prevent_default` beat
                    // a child, and the panels are descendants). The chrome therefore can't opt out
                    // by listener order — this handler has to look at the target and decline.
                    // Returning BEFORE `prevent_default` is the whole point: it leaves the event
                    // native, so a dock's `overflow-y-auto` scrolls instead of the map zooming
                    // (T-159.21 deferred item #1). A wheel over the free canvas is untouched.
                    if ev
                        .target()
                        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
                        .is_some_and(|el| el.closest(CHROME_SEL).ok().flatten().is_some())
                    {
                        return;
                    }
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        ev.prevent_default();
                        let rect = container.get_bounding_client_rect();
                        e.zoom_at(
                            -ev.delta_y() * WHEEL_ZOOM_PER_PX,
                            ev.client_x() as f64 - rect.left(),
                            ev.client_y() as f64 - rect.top(),
                        );
                        // P5 mid-pan rebase (T-151.11.6): keep an in-flight pan alive across a
                        // mid-pan zoom. Under the single-pointer invariant a `pointermove` precedes
                        // any `wheel`, so `wheel.client == last_px`; this refresh is a provable no-op
                        // that also defensively re-syncs the start px. The next incremental
                        // `engine.pan` then rides the LIVE post-zoom scale, so panning continues
                        // seamlessly with no re-press. (The incremental model has no frozen zoom to
                        // go stale — the Deck bug T-151.11.6 fixed does not exist here.)
                        if pan_px.get().is_some() {
                            pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                        }
                        // T-172 H5 — keep the slot ring px→m sizing + cluster gate in step with
                        // the camera (never called before; stale once the atlas exists).
                        e.on_camera_changed();
                        crate::editor::world_assets::schedule_camera_settle(
                            map_host.clone(),
                            engine.clone(),
                        );
                    }
                }
            });
            let wheel_opts = web_sys::AddEventListenerOptions::new();
            wheel_opts.set_passive(false);
            wheel_opts.set_capture(true);
            let _ = container.add_event_listener_with_callback_and_add_event_listener_options(
                "wheel",
                onwheel.as_ref().unchecked_ref(),
                &wheel_opts,
            );

            // T-159.15.2 — MMB drag-pan (LMB deferred to the doc host / .16: no marquee / slot
            // move yet). T-662 narrowed this to the middle button only; RMB is no longer a pan, so
            // the browser context menu is only suppressed (never blanket-eaten) by `oncontextmenu`
            // below, leaving RMB reachable for T-664. Pointer capture keeps deltas flowing if the
            // drag leaves the div. All five closures leak like the wheel/resize ones above (the
            // engine leaks too; `on_cleanup` only stops the loop — a `!Send` drop handle is later
            // polish).
            let onpointerdown = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let engine = engine.clone();
                let left = left.clone();
                move |ev: web_sys::PointerEvent| {
                    // T-662 — ONLY the middle button (1) pans. RMB (2) used to pan here too, which
                    // ate the right-click before any handler downstream could see it; the button is
                    // now free for T-664's context menu (and the six tickets behind it). MMB-pan is
                    // unchanged.
                    if ev.button() == 1 {
                        ev.prevent_default();
                        let _ = container.set_pointer_capture(ev.pointer_id());
                        pan_px.set(Some((ev.client_x() as f64, ev.client_y() as f64)));
                        // T-176 B2 — mark the pan active so a settle fired mid-drag (incl. by a
                        // simultaneous wheel-zoom) defers the heavy zoom-band recompute (DEM
                        // contours + 8 m forest mass) until the gesture ends.
                        crate::editor::world_assets::set_camera_gesture(true);
                    } else if ev.button() == 0 {
                        // T-723 — while a place is armed, do NOT open LG::Pending / LG::Ruler.
                        // A canvas press under the arm used to latch left; the armed pointerup then
                        // returned without take(), stranding Pending (phantom Move) or Ruler/LoS
                        // (phantom vertex / observer). `armed_place::open_left_gesture_while_armed`
                        // is the one decision; the armed pointerup still take()s as belt-and-braces.
                        if editor_ops::has_pending() {
                            return;
                        }
                        // T-159.18/.19 — LMB pending-left: freeze the ortho camera at press (X-05: the
                        // live engine unproject is deleted; a live unproject would feedback-loop
                        // mid-pan). No pointer capture yet — a sub-threshold release is a click; the
                        // first past-threshold `pointermove` (T-159.19) promotes to Move|Marquee and
                        // captures then. `engine.borrow()` is safe: JS is single-threaded, so this never
                        // reenters the rAF loop's `borrow_mut`.
                        if let Some(e) = engine.borrow().as_ref() {
                            let rect = container.get_bounding_client_rect();
                            let cam = crate::editor::tools::select_tool::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            );
                            let sx = ev.client_x() as f64 - rect.left();
                            let sy = ev.client_y() as f64 - rect.top();
                            // T-642 — TOOL-MODE ARBITRATION (the third mode's entry point). With the
                            // Ruler tool active, an LMB press opens `LG::Ruler` INSTEAD of
                            // `LG::Pending`, so the gesture never enters the Select machine's
                            // pick/marquee/move path and never reaches those doc commits. Constraint
                            // (c) button-0 is enforced by `should_begin_ruler` (this arm is already
                            // button 0, so it always passes here); the constraint matters for the
                            // predicate's other callers. `should_begin_ruler` is false under Select,
                            // so the existing Pending path is byte-for-byte unchanged there.
                            *left.borrow_mut() = Some(
                                if crate::editor::tools::ruler_tool::should_begin_ruler(
                                    tool_mode.get_untracked(),
                                    ev.button(),
                                ) {
                                    crate::editor::tools::select_tool::LeftGesture::Ruler {
                                        start_x: sx,
                                        start_y: sy,
                                        cam,
                                    }
                                } else {
                                    crate::editor::tools::select_tool::LeftGesture::Pending(
                                        crate::editor::tools::select_tool::PendingLeft {
                                            start_x: sx,
                                            start_y: sy,
                                            cam,
                                        },
                                    )
                                },
                            );
                        }
                    }
                }
            });
            let onpointermove = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let engine = engine.clone();
                let left = left.clone();
                let doc = doc.clone();
                let selection = selection.clone();
                let container = container.clone();
                let dem_grid = dem_grid.clone();
                let map_host = map_host.clone();
                // T-802 — the hover cursor's own state: the throttle clock + hysteresis anchor, and
                // the per-`doc_tick` point-set cache. Cloned in like every other handle here; the
                // `pointerleave` closure below clones the state cell (and the canvas) so leaving
                // the map resets the claim. The point CACHE is not reset there on purpose — it is
                // keyed on `doc_tick`, so it is still valid when the pointer comes back.
                let hover_state = hover_state.clone();
                let hover_points = hover_points.clone();
                let canvas = canvas.clone();
                move |ev: web_sys::PointerEvent| {
                    use crate::editor::tools::select_tool::{self as st, LeftGesture as LG};
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    // T-159.21 — CUR read-out. FIRST: both the pan branch and the no-gesture case
                    // below return early, and the cursor must keep tracking through both. Unprojects
                    // against the same `frozen_camera` the pick uses, so CUR always names the world
                    // point a click would hit. The borrow is scoped — the pan branch takes
                    // `borrow_mut` two lines down, and an overlapping borrow would panic.
                    // Un-throttled by design: React rAF-throttles because its cursor write
                    // re-rendered the page, whereas this feeds two text nodes through Leptos's
                    // fine-grained bindings. NaN (singular matrix) reads as off-map.
                    //
                    // T-802 — the camera is now HELD rather than discarded after the unproject, so
                    // the hover hit-test below reads the SAME frozen camera this read-out and the
                    // pick already use. One construction per move, not two: keeping it is strictly
                    // less work than building a second one, and — the wave-201 lesson — a third
                    // copy of the transform would be its own defect class.
                    let hover_cam = {
                        let g = engine.borrow();
                        g.as_ref().map(|e| {
                            st::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            )
                        })
                    };
                    let world = hover_cam.as_ref().map(|c| c.unproject_xy(px, py));
                    cursor.set(
                        world
                            .filter(|c| c[0].is_finite() && c[1].is_finite())
                            .map(|c| {
                                // T-172 B2 — DEM-fed Z beside X/Y; None (em-dash) until the grid
                                // publishes or when the point is outside DEM coverage.
                                let z = dem_grid.borrow().as_ref().and_then(|g| {
                                    map_engine_core::dem::downsample::sample_grid_meters(
                                        g, c[0], c[1],
                                    )
                                });
                                (c[0], c[1], z)
                            }),
                    );
                    // T-723 — MMB pan must work WHILE a place is armed (wave-106 MAJOR-3). The
                    // prior order returned at `has_pending` before the pan branch, so an MMB drag
                    // never moved the camera and the armed pointerup then stole the place. Pan
                    // first whenever `pan_px` is latched; the place ghost yields for the gesture.
                    if let Some((lx, ly)) = pan_px.get() {
                        let (cx, cy) = (ev.client_x() as f64, ev.client_y() as f64);
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.pan(cx - lx, cy - ly);
                            e.on_camera_changed();
                        }
                        pan_px.set(Some((cx, cy)));
                        crate::editor::world_assets::schedule_camera_settle(
                            map_host.clone(),
                            engine.clone(),
                        );
                        return;
                    }
                    // T-175 B2 — palette place ghost: while an asset is being dragged from the
                    // palette (`begin_place` armed `pending`), show a live translucent slot ring at
                    // the cursor's world point so the operator sees where it will land (the drop
                    // commits at pointerup). Mutually exclusive with a map drag/marquee (`left` is
                    // None during a palette place), so this returns before the gesture machine.
                    if editor_ops::has_pending() {
                        if let Some(c) = world.filter(|c| c[0].is_finite() && c[1].is_finite()) {
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_place_preview(c[0] as f32, c[1] as f32);
                            }
                        }
                        return;
                    }

                    // T-802 (O-8) — THE HOVER CURSOR. Strictly READ-ONLY with respect to every
                    // gesture on this handler: it borrows `left` immutably to ask one question
                    // ("is a gesture in flight?"), drops that borrow on the same line, and owns
                    // nothing but its own `hover_state` cell + `hover_points` cache. It sits HERE
                    // — after the pan and armed-place early returns, before the gesture machine
                    // takes `left` — because those two returns are themselves suppression cases and
                    // this is the last point at which "no gesture is running" is still knowable.
                    //
                    // Cost per pointer move when the throttle is closed (the common case): one
                    // `Date::now`, one `Cell` read, one shared `RefCell` borrow. When it is open:
                    // one radius query over the cached SoA. Never a document read unless `doc_tick`
                    // moved. See the T-802 block above for why each of those matters (T-057).
                    {
                        let now_ms = js_sys::Date::now();
                        let gesture_active = left.borrow().is_some();
                        let prev = hover_state.get();
                        if hover_suppressed(
                            gesture_active,
                            editor_ops::has_pending(),
                            tool_mode.get_untracked().captures_points(),
                        ) {
                            // Drop the claim rather than freeze it: a drag that began on a glyph
                            // must not leave the cursor saying "pickable" over open ground, and the
                            // reset means the first move after release re-tests immediately.
                            if prev.pickable {
                                set_map_cursor(&canvas, false);
                            }
                            hover_state.set(HoverState::default());
                        } else if hover_due(prev, now_ms) {
                            let hit = hover_cam.as_ref().is_some_and(|cam| {
                                hover_hit(
                                    &mut hover_points.borrow_mut(),
                                    doc_tick.get_untracked(),
                                    &doc,
                                    cam,
                                    px,
                                    py,
                                )
                            });
                            let next = hover_next(prev, hit, px, py, now_ms);
                            // Write ONLY on a transition — the churn guard. A per-tick write would
                            // be 25 style mutations a second for a value that did not change.
                            if next.pickable != prev.pickable {
                                set_map_cursor(&canvas, next.pickable);
                            }
                            hover_state.set(next);
                        }
                    }

                    // T-159.19 — LMB drag gesture. Own the gesture across the update (take → compute →
                    // put back) so a Pending→Move/Marquee transition never aliases a `&mut`, and so no
                    // `left` borrow is held across the inner `left.borrow_mut()` put-back (the `if let`
                    // temporary-lifetime footgun). Frozen cam (M2/X-05 — no live unproject). Live preview
                    // via `engine.set_drag` (drag) / `engine.upload_marquee` (marquee rect).
                    let taken = left.borrow_mut().take();
                    let Some(g0) = taken else { return };
                    // Promote a Pending press once it clears the threshold; else keep the active drag.
                    let active = match g0 {
                        LG::Pending(p) => {
                            let moved =
                                ((px - p.start_x).powi(2) + (py - p.start_y).powi(2)).sqrt();
                            if moved < st::DRAG_THRESHOLD_PX {
                                *left.borrow_mut() = Some(LG::Pending(p));
                                return;
                            }
                            // T-723 — button-less pointermove must NOT promote a stranded Pending
                            // into Move (wave-106 MAJOR-2). `buttons == 0` drops the gesture.
                            if !st::may_promote_pending(ev.buttons()) {
                                return;
                            }
                            // Real drag now: capture so it survives leaving the canvas (React :200).
                            let _ = container.set_pointer_capture(ev.pointer_id());
                            let sw = p.cam.unproject_xy(p.start_x, p.start_y);
                            // T-795 WIDGET-ROTATE-RING — a drag STARTING on the rotate ring rotates the
                            // selection about its centre, WITHOUT Shift and REGARDLESS of what (if
                            // anything) is under the press. This hit-test runs BEFORE the pick/marquee
                            // arm below so a drag on the ring can never fall through to `None =>
                            // LG::Marquee` and destroy the selection (the F-16 defect: the ring was
                            // pure decoration; dragging its edge started a marquee). Gated on Rotate
                            // mode + a live selection (the ring is only drawn then) and on the press
                            // pixel sitting in the ring band around the PROJECTED pivot — the same
                            // `WIDGET_RADIUS_PX` the overlay draws, so the drawn ring IS the draggable
                            // ring. Commits through the identical `LG::Rotate` arm as Shift+drag: one
                            // txn on release, one undo step, live selection re-read at release.
                            let on_ring = widget_variant.get_untracked().is_rotate()
                                && !selection.borrow().is_empty()
                                && read_widget_pivot()
                                    .map(|(wx, wy)| p.cam.project([wx, wy, 0.0]))
                                    .filter(|pv| pv[0].is_finite() && pv[1].is_finite())
                                    .is_some_and(|pv| {
                                        transform::press_on_ring(p.start_x, p.start_y, pv[0], pv[1])
                                    });
                            if on_ring {
                                LG::Rotate {
                                    start_x: p.start_x,
                                    start_y: p.start_y,
                                    cam: p.cam,
                                }
                            } else {
                                let hit = doc.borrow().as_ref().and_then(|c| {
                                    st::pick_slot_or_vehicle(
                                        &p.cam,
                                        &map_render_slot_soa(c),
                                        &editor_ops::vehicle_points(),
                                        p.start_x,
                                        p.start_y,
                                    )
                                });
                                // T-796 — pick the COMMENT GLYPH so a drag STARTING on a note grabs
                                // it, the same fold the click path (T-784) does. Precedence: the
                                // rotate ring (T-795) short-circuited above, then slot/vehicle wins
                                // its own pixels here, then a comment, then `None => LG::Marquee`. A
                                // note parked on a unit can never steal the unit's drag, and a drag
                                // on a note can never fall through to a marquee that destroys the
                                // selection — the F-16 failure class the ring pin already names,
                                // applied to the note. Against the FROZEN press camera, tolerance
                                // derived by unprojecting two points `COMMENT_PICK_PX` apart, exactly
                                // as the click-path comment pick does — so the drag grabs a note over
                                // the identical hit box the click selects it with.
                                let hit = hit.or_else(|| {
                                    let w = p.cam.unproject_xy(p.start_x, p.start_y);
                                    let w2 =
                                        p.cam.unproject_xy(p.start_x + COMMENT_PICK_PX, p.start_y);
                                    let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                                    doc.borrow().as_ref().and_then(|c| {
                                        pick_comment(
                                            &comment_points(&c.comments_json()),
                                            w[0],
                                            w[1],
                                            tol,
                                        )
                                    })
                                });
                                match hit {
                                    // T-648 XFORM-SHIFT-001 — SHIFT + drag grabbing an ALREADY-SELECTED
                                    // entity rotates the whole selection to face the cursor instead of
                                    // moving it. Shift is free in this drag path (T-053 left it unbound;
                                    // the T-073 cancel note confirms it), so this steals no existing
                                    // gesture. Gated on the grabbed entity being in the CURRENT selection
                                    // so a Shift+drag on empty ground or an unselected entity still falls
                                    // through to the normal pick/marquee below (a rotate needs something
                                    // to rotate). No pointer preview: the render engine's `set_drag` is a
                                    // TRANSLATION lane only, so — like the ruler — the rotate shows its
                                    // result on release; the widget ring (mounted in the view) is the
                                    // live affordance. `LG::Rotate` carries no ids: the commit re-reads
                                    // the live selection at release.
                                    Some(ref id)
                                        if ev.shift_key()
                                            && selection.borrow().iter().any(|s| s == id) =>
                                    {
                                        LG::Rotate {
                                            start_x: p.start_x,
                                            start_y: p.start_y,
                                            cam: p.cam,
                                        }
                                    }
                                    Some(id) => {
                                        // Drag an already-selected slot → move the whole selection; else
                                        // replace the selection with the dragged slot (React :204).
                                        let cur = selection.borrow().clone();
                                        let ids = st::compute_move_ids(&id, &cur);
                                        if !cur.iter().any(|s| *s == id) {
                                            *selection.borrow_mut() = ids.clone();
                                            if let Some(e) = engine.borrow_mut().as_mut() {
                                                // Slot tint only — vehicle glyphs have no selection lane.
                                                let slot_ids: Vec<String> = ids
                                                    .iter()
                                                    .filter(|i| !editor_ops::is_vehicle_id(i))
                                                    .cloned()
                                                    .collect();
                                                e.set_selection(slot_ids);
                                            }
                                        }
                                        LG::Move {
                                            ids,
                                            start_wx: sw[0],
                                            start_wy: sw[1],
                                            cam: p.cam,
                                            dx: 0.0,
                                            dy: 0.0,
                                        }
                                    }
                                    None => LG::Marquee {
                                        start_x: p.start_x,
                                        start_y: p.start_y,
                                        start_wx: sw[0],
                                        start_wy: sw[1],
                                        cam: p.cam,
                                    },
                                }
                            } // end `else` — non-ring drag (T-795 WIDGET-ROTATE-RING short-circuits above)
                        }
                        other => other,
                    };
                    // Drive the live preview for the (possibly just-promoted) state, coalescing the
                    // world delta / marquee rect into `active` for the pointerup commit.
                    let next = match active {
                        LG::Move {
                            ids,
                            start_wx,
                            start_wy,
                            cam,
                            ..
                        } => {
                            let (dx, dy) = st::drag_delta(&cam, start_wx, start_wy, px, py);
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                // T-573 — preview the WHOLE selection. The T-425 pre-filter fed
                                // `set_drag` slot ids only and nothing previewed the vehicles, so a
                                // mixed drag drew the slots moving and the vehicles standing while
                                // the pointerup commit moved both: an overlay lying about its drop.
                                st::push_drag_preview(
                                    e,
                                    &ids,
                                    &editor_ops::vehicle_points(),
                                    dx,
                                    dy,
                                );
                                // T-796 — the COMMENT half of the preview. A comment is not in the
                                // slot overlay lane (`set_drag`) nor the vehicle re-pack, so
                                // `push_drag_preview` cannot move it; its lane is re-bound here so a
                                // dragged note's glyph FOLLOWS THE CURSOR (the O-7 preview-parity
                                // note). `comment_drag_lane_xy` re-packs EVERY note — the ones not in
                                // the drag stay drawn at rest — with the dragged ids offset by the
                                // live delta. On a mixed drag with no comment in `ids` this re-binds
                                // the lane to its authored positions (all offsets zero), which is a
                                // harmless identity re-upload, so there is no branch to keep in step.
                                //
                                // T-808 — the ids ride along: the lane's selection treatment is
                                // per-row and the engine looks it up BY ID, so a preview bound
                                // without them would strip the ring off a dragged note for the
                                // whole gesture and hand it back on release. `comment_lane_ids`
                                // packs the same `comment_points` list this re-pack offsets, so
                                // ids[i] still names the bubble at xy[2i] mid-drag.
                                let lane = doc.borrow().as_ref().map(|c| {
                                    let cj = c.comments_json();
                                    (
                                        comment_drag_lane_xy(&cj, &ids, dx, dy),
                                        comment_lane_ids(&cj),
                                    )
                                });
                                if let Some((cxy, cids)) = lane {
                                    e.comments_bind_ids(&cxy, cids);
                                }
                            }
                            LG::Move {
                                ids,
                                start_wx,
                                start_wy,
                                cam,
                                dx,
                                dy,
                            }
                        }
                        LG::Marquee {
                            start_x,
                            start_y,
                            start_wx,
                            start_wy,
                            cam,
                        } => {
                            let end = cam.unproject_xy(px, py);
                            if end[0].is_finite() && end[1].is_finite() {
                                let (min_x, max_x) = (start_wx.min(end[0]), start_wx.max(end[0]));
                                let (min_y, max_y) = (start_wy.min(end[1]), start_wy.max(end[1]));
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.upload_marquee(min_x, min_y, max_x, max_y, true);
                                }
                            }
                            LG::Marquee {
                                start_x,
                                start_y,
                                start_wx,
                                start_wy,
                                cam,
                            }
                        }
                        LG::Pending(p) => LG::Pending(p),
                        // T-642 — a ruler press does NOT promote: it stays `Ruler` until release,
                        // when a sub-threshold pointerup commits ONE vertex. The rubber-band leg to
                        // the cursor is drawn by `RulerOverlay` off the live `cursor` signal (already
                        // updated at the top of this handler), so there is nothing to preview via the
                        // engine here — the arm just carries itself back. No pointer capture, no GPU
                        // upload: a ruler never touches the drag/marquee engine lanes.
                        LG::Ruler {
                            start_x,
                            start_y,
                            cam,
                        } => LG::Ruler {
                            start_x,
                            start_y,
                            cam,
                        },
                        // T-648 — a Shift-rotate, like the ruler, does NOT preview through the engine
                        // (its `set_drag` is translation-only) and does NOT promote: it stays
                        // `Rotate` until release. The live affordance is the widget ring in the view;
                        // the rotate itself is applied on pointerup from the release cursor. Carry the
                        // arm back unchanged.
                        LG::Rotate {
                            start_x,
                            start_y,
                            cam,
                        } => LG::Rotate {
                            start_x,
                            start_y,
                            cam,
                        },
                    };
                    *left.borrow_mut() = Some(next);
                }
            });
            let onpointerup = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let engine = engine.clone();
                let left = left.clone();
                let doc = doc.clone();
                let selection = selection.clone();
                let map_host = map_host.clone();
                // T-642 — the ruler chain + its reactive sync + the DEM grid (per-vertex Z sample).
                let ruler = ruler.clone();
                let dem_grid = dem_grid.clone();
                let sync_ruler = sync_ruler.clone();
                // T-643 — the LoS capture + its reactive sync. The commit arm below routes a captured
                // click to the ruler OR the LoS state by `tool_mode`, since both tools share the
                // `LG::Ruler` gesture (the "mode field on the ruler arm").
                let los = los.clone();
                let sync_los = sync_los;
                // T-644 — the viewshed state, for the one-shot placement branch of the LoS commit (the
                // `engine` clone above carries the wash upload). `los_mode` is a Copy RwSignal read
                // directly below (`get_untracked`) to route ray vs viewshed.
                let viewshed = viewshed.clone();
                // T-159.21 — no `mission_id` capture: the persist tail now runs inside
                // `mission_history::after_local_edit`, which reads the id from its ctx.
                move |ev: web_sys::PointerEvent| {
                    // T-159.22 / T-723 — palette place. FIRST: a place is armed by a palette /
                    // picker / composition surface. The ARMED state (`has_pending()`) is checked
                    // before any gesture branch below. This branch used to assume `left`/`pan_px`
                    // were both None here — that was FALSE (canvas pointerdown while armed still
                    // wrote LG::Pending/Ruler; MMB still latched pan_px) and is the root of
                    // wave-106 MAJOR-1/2/3. Corrected contract:
                    //   * button 0 only places; button 2 disarms; button 1 falls through to pan;
                    //   * always `left.take()` so Pending/Ruler cannot strand;
                    //   * off-canvas LMB keeps the arm (arming click's own release) — Esc/RMB cancel.
                    // The host still stops `pointerdown` only, so a release over a dock bubbles here;
                    // the chrome insets decide Place vs KeepArmed.
                    if editor_ops::has_pending() {
                        // T-723 — clear ANY stranded left gesture before deciding (Pending → phantom
                        // Move; Ruler → phantom vertex; LoS capture under the same LG::Ruler arm →
                        // phantom observer/target). Belt-and-braces with the pointerdown skip.
                        let _ = left.borrow_mut().take();

                        let button = ev.button();
                        // MMB: do not place — fall through to pan-end cleanup below.
                        if button == 1 {
                            // keep armed; pan_px cleanup runs next
                        } else if button == 2 {
                            // RMB — Eden stamp-mode cancel
                            editor_ops::cancel_pending();
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.clear_place_preview();
                            }
                            return;
                        } else if button != 0 {
                            return;
                        } else {
                            let rect = container.get_bounding_client_rect();
                            let (px, py) = (
                                ev.client_x() as f64 - rect.left(),
                                ev.client_y() as f64 - rect.top(),
                            );
                            // T-638 — the LIVE insets (dock collapse + chrome_hidden folded in).
                            let on_canvas = px >= crate::editor::layout::dock_left_px()
                                && px <= rect.width() - crate::editor::layout::dock_right_px()
                                && py >= crate::editor::layout::strip_top_px()
                                && py <= rect.height() - crate::editor::layout::toolbelt_band_px();
                            let world = if on_canvas {
                                let g = engine.borrow();
                                g.as_ref().map(|e| {
                                    crate::editor::tools::select_tool::frozen_camera(
                                        rect.width(),
                                        rect.height(),
                                        e.target_x(),
                                        e.target_y(),
                                        e.zoom(),
                                    )
                                    .unproject_xy(px, py)
                                })
                            } else {
                                None
                            };
                            let world_ok = world.filter(|c| c[0].is_finite() && c[1].is_finite());
                            match armed_place::decide_armed_pointerup(button, world_ok.is_some()) {
                                armed_place::ArmedUp::Place => {
                                    // ══════════════════════ T-647 — the Ctrl state machine (arm ↔ Ctrl) ═══════
                                    // Ctrl is OVERLOADED across this ticket and its meaning is decided by the
                                    // ARMED state, resolved in exactly two places:
                                    //   (1) HERE, with a placement ARMED — Ctrl on release = MULTI-PLACE: land
                                    //       the entity but KEEP the pending armed so the next click drops another
                                    //       (`place_at_keep`). Without Ctrl the arm is one-shot (`place_at`
                                    //       take()s it). Eden's Ctrl-stamp behaviour.
                                    //   (2) In the LMB drag-commit (pointerup, `LG::Move` below), with NO
                                    //       placement armed — Ctrl + drag character→character = REGROUP.
                                    // The two can never fire at once: `has_pending()` gates this branch and the
                                    // drag branch runs only when it is false. That mutual exclusion is the whole
                                    // reason PLACE-004 and CONN-GROUP-001 are one row — see the pin
                                    // `t647_ctrl_state_machine`.
                                    //
                                    // T-647 PLACE-CREW-001 — Alt on release = place an EMPTY vehicle: the
                                    // per-gesture override of the DockRight crew toggle (which is the default).
                                    // Threaded to `place_at*` as `alt_empty`; for a Vehicle arm it forces
                                    // `crewed: false`, for a character/object arm it is inert.
                                    let ctrl_multi = ev.ctrl_key() || ev.meta_key();
                                    let alt_empty = ev.alt_key();
                                    let c = world_ok.expect("Place implies finite world");
                                    if ctrl_multi {
                                        editor_ops::place_at_keep(c[0], c[1], alt_empty);
                                    } else {
                                        editor_ops::place_at_alt(c[0], c[1], alt_empty);
                                    }
                                }
                                armed_place::ArmedUp::KeepArmed => {
                                    // Off-canvas LMB: the arming click's own release (dock /
                                    // composition) — do NOT cancel_pending (wave-106 MAJOR-1 /
                                    // wave-108 composition tooltip). Esc / RMB disarm.
                                }
                                // FallThroughPan / Disarm / Ignore are handled above by button.
                                armed_place::ArmedUp::FallThroughPan
                                | armed_place::ArmedUp::Disarm
                                | armed_place::ArmedUp::Ignore => {}
                            }
                            // T-175 B2 — drop the ghost after a place attempt or a chrome release.
                            // Ctrl multi-place that KEPT the pending re-shows on the next move.
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.clear_place_preview();
                            }
                            return;
                        }
                    }
                    // Pan end (MMB/RMB).
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                        // T-176 B2 — pan ended: clear the gesture flag BEFORE scheduling so this
                        // settle runs the full zoom-band recompute (contours + forest) once.
                        crate::editor::world_assets::set_camera_gesture(false);
                        crate::editor::world_assets::schedule_camera_settle(
                            map_host.clone(),
                            engine.clone(),
                        );
                    }
                    // LMB gesture end. `take()` into a `let` first so the RefMut drops before the
                    // per-branch re-borrows below (the `if let` temporary-lifetime footgun). If a pan
                    // just ended, `left` is None ⇒ this returns.
                    let taken = left.borrow_mut().take();
                    let Some(g) = taken else { return };
                    use crate::editor::tools::select_tool::{self as st, LeftGesture as LG};
                    // T-723 — only button 0 commits a left gesture. A phantom Move (stranded
                    // Pending promoted after disarm) used to commit on RMB/MMB pointerup and
                    // teleport the just-placed entity. Wrong-button releases abandon the gesture.
                    if ev.button() != 0 {
                        match &g {
                            LG::Move { .. } => {
                                if container.has_pointer_capture(ev.pointer_id()) {
                                    let _ = container.release_pointer_capture(ev.pointer_id());
                                }
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    st::clear_drag_preview(e, &editor_ops::vehicle_points());
                                    // T-796 — a wrong-button release is never a commit; put a dragged
                                    // note's lane back at its authored position too (identity when the
                                    // drag held no comment). T-808 — with its ids, or the restore
                                    // would drop the selection ring the abandoned drag was carrying.
                                    if let Some((cxy, cids)) = doc.borrow().as_ref().map(|c| {
                                        (
                                            comment_lane_xy(&c.comments_json()),
                                            comment_lane_ids(&c.comments_json()),
                                        )
                                    }) {
                                        e.comments_bind_ids(&cxy, cids);
                                    }
                                }
                            }
                            LG::Marquee { .. } => {
                                if container.has_pointer_capture(ev.pointer_id()) {
                                    let _ = container.release_pointer_capture(ev.pointer_id());
                                }
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    e.upload_marquee(0.0, 0.0, 0.0, 0.0, false);
                                }
                            }
                            LG::Rotate { .. } => {
                                if container.has_pointer_capture(ev.pointer_id()) {
                                    let _ = container.release_pointer_capture(ev.pointer_id());
                                }
                            }
                            _ => {}
                        }
                        return;
                    }
                    let rect = container.get_bounding_client_rect();
                    let up_x = ev.client_x() as f64 - rect.left();
                    let up_y = ev.client_y() as f64 - rect.top();
                    match g {
                        // T-159.18/.53 — sub-threshold press = a click: pick against the FROZEN press
                        // camera (X-05) and toggle/replace/clear the selection.
                        LG::Pending(p) => {
                            let moved =
                                ((up_x - p.start_x).powi(2) + (up_y - p.start_y).powi(2)).sqrt();
                            if moved < st::DRAG_THRESHOLD_PX {
                                let additive = ev.ctrl_key() || ev.meta_key();
                                let hit = doc.borrow().as_ref().and_then(|c| {
                                    st::pick_slot_or_vehicle(
                                        &p.cam,
                                        &map_render_slot_soa(c),
                                        &editor_ops::vehicle_points(),
                                        p.start_x,
                                        p.start_y,
                                    )
                                });
                                // T-768 — Eden CONN-START-001 final half: when a connect is armed
                                // (RMB ▸ Connect ▸ kind), an LMB pick on a target calls the SAME
                                // `complete_connect` the RMB "Complete Connection" row uses. A miss
                                // keeps the arm (Esc / RMB Cancel / panel Cancel disarm). The arm is
                                // consumed on attempt inside complete_connect — no stranded mode.
                                if editor_ops::pending_connect().is_some() {
                                    if let Some(ref id) = hit {
                                        let _ = editor_ops::complete_connect(id);
                                    }
                                }
                                // ══════════ T-784 — pick the COMMENT GLYPH ══════════════════════
                                //
                                // On an entity MISS only: a slot or vehicle always wins its own
                                // pixels, so a note parked on top of a unit can never steal the
                                // unit's click.
                                //
                                // FOLDED INTO `hit`, not handled beside it. Everything downstream —
                                // `apply_click`'s replace/toggle, the additive Ctrl branch, the SEL
                                // readout, `refresh_selection` — then treats a comment exactly as it
                                // treats a slot, with no new selection route to keep in step. That
                                // is what makes Ctrl+click COMPOSE a comment with entities (the
                                // T-781 capture reads one selection `Vec`) and what makes the map's
                                // edge selection drop: a non-empty entity selection is the condition
                                // `editor_ops::reconcile_connection_selection` already tests inside
                                // `mirror_selection`, so this arm adds no clear of its own.
                                //
                                // DELIBERATELY AFTER the connect arm above: `complete_connect` must
                                // keep seeing entity hits only, or an armed connection would take a
                                // comment as an endpoint — an edge to a thing that never compiles.
                                //
                                // …and DELIBERATELY BEFORE the edge pick below, which is NOT a draw
                                // -order argument (wave 145 F-3: `draw_order.rs` pins
                                // `MissionConnections` ABOVE `MissionComments`, so at an exact
                                // overlap the hairline is the topmost pixel and the note still wins
                                // the click). The reason is the SHAPE of the two targets. A note is
                                // a point with a `COMMENT_PICK_PX` radius — a few pixels, and if a
                                // line crossing it could take the click it would be unclickable at
                                // that spot with no way for the operator to tell why. An edge is a
                                // long segment that stays clickable along its whole length, so it
                                // loses nothing by yielding the handful of pixels around a note.
                                //
                                // Against the FROZEN press camera (X-05), tolerance derived by
                                // unprojecting two points `COMMENT_PICK_PX` apart, exactly as the
                                // connection pick below derives its own. NO affordance is painted on
                                // the strength of this pick and that is on purpose (wave 129): it is
                                // a SELECTION gesture like a slot click, not a route to an
                                // inspector, so there is no "can this be clicked" question here and
                                // therefore no second answer to it.
                                let hit = hit.or_else(|| {
                                    let w = p.cam.unproject_xy(p.start_x, p.start_y);
                                    let w2 =
                                        p.cam.unproject_xy(p.start_x + COMMENT_PICK_PX, p.start_y);
                                    let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                                    doc.borrow().as_ref().and_then(|c| {
                                        pick_comment(
                                            &comment_points(&c.comments_json()),
                                            w[0],
                                            w[1],
                                            tol,
                                        )
                                    })
                                });
                                // ══════ T-780 — pick the CONNECTION line the operator drew ══════
                                //
                                // Runs ONLY on a miss: an entity always wins its own pixels, so an
                                // edge can never steal a click from the slot it ends at. That
                                // ordering is also what keeps the two selections mutually exclusive
                                // — a hit clears the edge, and a plain miss is the only thing that
                                // can set one (`apply_click(None, false)` clears the slot selection
                                // in the very same breath).
                                //
                                // Against the FROZEN press camera `p.cam` (X-05), like the slot pick
                                // above, so a click during an inertial pan tests the geometry the
                                // operator was actually looking at. The tolerance is derived by
                                // unprojecting two points `CONN_PICK_PX` apart rather than reading a
                                // scale off the camera: it stays a constant SCREEN radius at every
                                // zoom without this file learning the projection's internals.
                                //
                                // NO affordance is painted anywhere on the strength of this pick,
                                // and that is on purpose (wave 129). This is a SELECTION gesture,
                                // the same kind a slot click is — not a route to an inspector — so
                                // there is no "can this subject be clicked" question to ask, and
                                // therefore no second answer to it. The tint the picked edge gets is
                                // the CONSEQUENCE of the click, applied after the fact by the lane
                                // Effect, never a promise made before it. Nothing here consults
                                // `route_target` or a hardcoded kind list.
                                if hit.is_some() {
                                    selected_connection.set(None);
                                } else if !additive {
                                    let w = p.cam.unproject_xy(p.start_x, p.start_y);
                                    let w2 =
                                        p.cam.unproject_xy(p.start_x + CONN_PICK_PX, p.start_y);
                                    let tol = (w2[0] - w[0]).hypot(w2[1] - w[1]);
                                    let edge = doc.borrow().as_ref().and_then(|c| {
                                        pick_connection(
                                            &live_connection_segments(c),
                                            w[0],
                                            w[1],
                                            tol,
                                        )
                                    });
                                    selected_connection.set(edge);
                                }
                                {
                                    let mut sel = selection.borrow_mut();
                                    // T-788 F-27 — a PLAIN click that lands INSIDE the current
                                    // multi-selection must NOT collapse it to `[hit]`. Two natural
                                    // gestures depend on this: a group drag (the `LG::Move` arm
                                    // below moves the whole selection via `compute_move_ids`), and a
                                    // double-click to multi-edit (the first click's pointerup used to
                                    // re-select the one slot BEFORE `dblclick`/`open_attributes`
                                    // fired, so SEL9 became SEL1 and the modal opened single-edit).
                                    // Eden keeps the marquee alive on a click within it. A plain
                                    // click OUTSIDE the selection still REPLACES (Eden semantics), and
                                    // Ctrl/Cmd (`additive`) still toggles — both flow through
                                    // `apply_click` untouched, so a lone-slot click is unchanged.
                                    let keep_multi = !additive
                                        && sel.len() > 1
                                        && hit.as_ref().is_some_and(|h| sel.iter().any(|s| s == h));
                                    if !keep_multi {
                                        st::apply_click(&mut sel, hit, additive);
                                    }
                                }
                                let ids = selection.borrow().clone();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    let slot_ids: Vec<String> = ids
                                        .iter()
                                        .filter(|i| !editor_ops::is_vehicle_id(i))
                                        .cloned()
                                        .collect();
                                    e.set_selection(slot_ids); // tint lane (slots only)
                                }
                                // T-159.21 — SEL readout only: a click changes the selection, not the
                                // document (no rebind / persist / undo step / tree rebuild).
                                mission_history::refresh_selection();
                            }
                        }
                        // T-159.19 M4/M5 — drag-move commit. Release capture; if it actually moved,
                        // commit ONE `move_entities` txn (one undo step), re-bind the moved glyphs, keep
                        // the moved slots selected, and schedule the first edit-driven persist.
                        LG::Move {
                            ids, dx, dy, cam, ..
                        } => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            // ══════════ T-647 CONN-GROUP-001 (map half) — Ctrl+drag = regroup ══════
                            // The second half of the Ctrl state machine (see the arm ↔ Ctrl block in
                            // the place branch above). This branch runs only with NO placement armed
                            // (`has_pending()` short-circuited the whole pointerup before here), so
                            // Ctrl here can only mean "regroup", never "multi-place". A SINGLE
                            // CHARACTER slot dragged onto ANOTHER character slot moves the dragged
                            // one into the target's squad (`regroup_slot_onto`), and the positional
                            // move is SKIPPED — the drop was a group gesture, not a reposition.
                            // Anything else under Ctrl (a vehicle in the drag, a multi-selection, a
                            // drop onto empty ground or onto a vehicle) falls through to the normal
                            // move, so Ctrl+drag keeps its move meaning everywhere regroup does not
                            // apply. The preview lanes are dropped back either way (regroup commits
                            // no position, so nothing re-binds the glyphs from a move).
                            // T-796 — a comment id can never be a regroup SOURCE: regroup moves a
                            // character slot into another slot's squad (`regroup_slot_onto`), and a
                            // note is in neither the slot SoA nor a squad. Excluding it here (asked
                            // of the document's comment map, the `delete_selection` membership rule —
                            // not a `cmt-` prefix) means a Ctrl+drag of a lone note falls straight
                            // through to the positional move below instead of probing for a phantom
                            // regroup target under the drop.
                            let single_comment_drag = ids.len() == 1
                                && doc.borrow().as_ref().is_some_and(|c| {
                                    editor_ops::comment_details(c)
                                        .iter()
                                        .any(|d| d.id == ids[0])
                                });
                            let regrouped = if (ev.ctrl_key() || ev.meta_key())
                                && ids.len() == 1
                                && !editor_ops::is_vehicle_id(&ids[0])
                                && !single_comment_drag
                            {
                                let target = doc.borrow().as_ref().and_then(|c| {
                                    st::pick(&cam, &map_render_slot_soa(c), up_x, up_y)
                                });
                                match target {
                                    Some(tid) if tid != ids[0] => {
                                        // `regroup_slot_onto` runs the shared dirty tail itself
                                        // (via `refile_slot`), so this branch must NOT also call
                                        // `after_local_edit` — it only drops the stale drag preview.
                                        let ok = editor_ops::regroup_slot_onto(&ids[0], &tid);
                                        if ok {
                                            if let Some(e) = engine.borrow_mut().as_mut() {
                                                st::clear_drag_preview(
                                                    e,
                                                    &editor_ops::vehicle_points(),
                                                );
                                            }
                                        }
                                        ok
                                    }
                                    _ => false,
                                }
                            } else {
                                false
                            };
                            if regrouped {
                                return;
                            }
                            if dx != 0.0 || dy != 0.0 {
                                // T-796 — split the COMMENTS out of the drag FIRST, asked of the
                                // document's own comment map (`comment_details`) exactly as
                                // `delete_selection` asks it — the prefix `cmt-` is a minting
                                // convention, not a document invariant. A note is in neither the slot
                                // SoA nor `vehiclesById`, so left in `ids` it would land in
                                // `slot_ids` and be handed to `move_entities`, which reads the slot
                                // SoA, finds nothing, and moves it NOWHERE — the exact defect O-6
                                // pixel-verified (a 90px drag left the stored position unchanged).
                                let comment_ids: Vec<String> = doc
                                    .borrow()
                                    .as_ref()
                                    .map(|c| {
                                        let members: std::collections::HashSet<String> =
                                            editor_ops::comment_details(c)
                                                .into_iter()
                                                .map(|d| d.id)
                                                .collect();
                                        ids.iter()
                                            .filter(|id| members.contains(*id))
                                            .cloned()
                                            .collect()
                                    })
                                    .unwrap_or_default();
                                // T-796 — commit each dragged note to base + delta through
                                // `move_comment`, ONE core transaction each ⇒ one Ctrl+Z. A
                                // SINGLE-comment drag (the O-6 case, and what `compute_move_ids`
                                // produces for a drag of an unselected note) is therefore exactly one
                                // undo step, the ticket's "moving must be ONE step". A drag of
                                // SEVERAL notes is N steps — the SAME accepted per-txn class as the
                                // `delete_selection` comment loop (the one-txn batch would be minted
                                // core-side in `store.rs`, out of this slice), never lost work: undo
                                // restores every moved note. The base x/z is read from the same
                                // `comment_details` list, so the stored position and the previewed
                                // glyph came from ONE read. `z` here is the note's NORTHING (a
                                // comment is `{x, z}`, two horizontals) — `dy` is a plane delta, so
                                // this passes the northing through verbatim and never zeroes an axis.
                                if !comment_ids.is_empty() {
                                    let moves: Vec<(String, f64, f64)> = doc
                                        .borrow()
                                        .as_ref()
                                        .map(|c| {
                                            dragged_comment_points(
                                                &comment_points(&c.comments_json()),
                                                &comment_ids,
                                            )
                                            .into_iter()
                                            .map(|p| (p.id, p.x + dx, p.y + dy))
                                            .collect()
                                        })
                                        .unwrap_or_default();
                                    for (id, x, z) in moves {
                                        editor_ops::move_comment(id, x, z);
                                    }
                                }
                                // T-491 — one LOCAL yrs txn for mixed slot+vehicle drag (T-425 split
                                // `move_entities` then `move_vehicles` needed two Ctrl+Z). The comment
                                // half is already committed above; this partitions the REMAINDER.
                                let (veh_ids, slot_ids): (Vec<String>, Vec<String>) = ids
                                    .iter()
                                    .filter(|id| !comment_ids.iter().any(|c| c == *id))
                                    .cloned()
                                    .partition(|id| editor_ops::is_vehicle_id(id));
                                if !slot_ids.is_empty() || !veh_ids.is_empty() {
                                    let guard = doc.borrow();
                                    let Some(core) = guard.as_ref() else {
                                        return;
                                    };
                                    // wave-127 F-6 — the drag carries each slot's CURRENT z.
                                    // `move_entities_in_txn` reads the existing z and DISCARDS it,
                                    // writing `zs[i]` verbatim, so the `vec![0.0; n]` that used to
                                    // sit here flattened every dragged slot to the deck inside one
                                    // txn — while VEHICLES in the same drag kept theirs
                                    // (`move_vehicles_in_txn` never touches z). Nothing re-samples
                                    // afterwards to hide it: `terrainZ` did not survive the React
                                    // deletion, so that `0.0` was the final stored value, not a
                                    // placeholder for a DEM lookup. Same defect, and same fix, as
                                    // the Attributes tab (F-2) and Align/Distribute (F-5); the z is
                                    // resolved through their `keep_z_rows`/`slot_z` pair so there is
                                    // one z-resolution vocabulary in the editor, not three.
                                    //
                                    // ORDER: the core indexes `zs` by each id's position in the
                                    // `ids` slice, so `zs[i]` must be `slot_ids[i]`'s z. `zs` is
                                    // built by mapping over the very `slot_ids` Vec that is then
                                    // passed as `ids` — same length, same order, no re-sort between
                                    // the two — so the correspondence is structural, not a
                                    // convention two call sites have to agree on.
                                    //
                                    // `raw_slot_rows` is an O(document) JSON parse, so it is read
                                    // ONCE for the whole drag rather than per slot, and not at all
                                    // for a vehicle-only drag. `keep_z_rows` is asked with the write
                                    // shape a translate always has (x and y written, z absent — the
                                    // deltas stand in for the coordinates, since it only asks WHICH
                                    // fields are written), so it answers `Some` for every drag.
                                    let z_rows = (!slot_ids.is_empty())
                                        .then(|| {
                                            editor_ops::keep_z_rows(core, Some(dx), Some(dy), None)
                                        })
                                        .flatten();
                                    let zs: Vec<f64> = slot_ids
                                        .iter()
                                        .map(|id| {
                                            z_rows
                                                .as_ref()
                                                .and_then(|rows| editor_ops::slot_z(rows, id))
                                                .unwrap_or(0.0)
                                        })
                                        .collect();
                                    core.move_entities_and_vehicles(slot_ids, &veh_ids, dx, dy, zs);
                                    drop(guard);
                                    mission_history::after_local_edit();
                                }
                            } else if let Some(e) = engine.borrow_mut().as_mut() {
                                // No move ⇒ no commit, so nothing else re-binds: drop BOTH preview
                                // lanes back to the authored positions (T-573 — the vehicle lane is
                                // a live re-pack now, not a passive bind).
                                st::clear_drag_preview(e, &editor_ops::vehicle_points());
                                // T-796 — and the comment lane: a zero-delta release still ran the
                                // preview re-pack above, so re-bind the notes to their authored
                                // positions (no committed move re-binds them here). Identity when the
                                // drag held no note. T-808 — ids ride along (see the preview arm).
                                if let Some((cxy, cids)) = doc.borrow().as_ref().map(|c| {
                                    (
                                        comment_lane_xy(&c.comments_json()),
                                        comment_lane_ids(&c.comments_json()),
                                    )
                                }) {
                                    e.comments_bind_ids(&cxy, cids);
                                }
                            }
                        }
                        // T-159.19 M3 — marquee commit. Release capture; a ≥1×1 px box replaces the
                        // selection with the enclosed slots (`pick_rect` over the frozen-cam world AABB);
                        // hide the rect.
                        LG::Marquee {
                            start_x,
                            start_y,
                            start_wx,
                            start_wy,
                            cam,
                        } => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if (up_x - start_x).abs() >= 1.0 && (up_y - start_y).abs() >= 1.0 {
                                let ids = doc
                                    .borrow()
                                    .as_ref()
                                    .map(|c| {
                                        st::marquee_ids_with_vehicles(
                                            &cam,
                                            &map_render_slot_soa(c),
                                            &editor_ops::vehicle_points(),
                                            start_wx,
                                            start_wy,
                                            up_x,
                                            up_y,
                                        )
                                    })
                                    .unwrap_or_default();
                                *selection.borrow_mut() = ids.clone();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    let slot_ids: Vec<String> = ids
                                        .iter()
                                        .filter(|i| !editor_ops::is_vehicle_id(i))
                                        .cloned()
                                        .collect();
                                    e.set_selection(slot_ids);
                                }
                                // T-159.21 — SEL readout only (selection change, not a doc edit).
                                mission_history::refresh_selection();
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.upload_marquee(0.0, 0.0, 0.0, 0.0, false); // hide
                            }
                        }
                        // T-642 — RULER vertex commit. This arm is only reached with NO palette place
                        // armed (the `has_pending()` branch at the top of pointerup already returned)
                        // and no pan in flight — so it deliberately sits OUTSIDE the T-723 armed-place
                        // branch (constraint (a)), and because the ruler pointerdown wrote `LG::Ruler`
                        // into `left`, this `take()` (constraint (b)) is what clears it. A sub-threshold
                        // release is a click → commit ONE point; the tool stays armed for the next.
                        // (Past-threshold would be a drag; neither measure tool has a drag gesture, so
                        // an accidental micro-drag simply drops without committing.) The point records
                        // its DEM elevation at click time (Decision 2) from the SAME grid CUR-Z reads,
                        // unprojected against the FROZEN press camera so it lands where CUR pointed.
                        //
                        // T-643 — BOTH measure tools share this `LG::Ruler` arm (the "mode field on
                        // the ruler arm"): a captured click routes by `tool_mode` — a ruler VERTEX
                        // (`chain.press`) under Ruler, or a LoS observer/target (`state.click`) under
                        // LoS. The unproject + Z-sample + threshold are identical; only the
                        // destination differs, so the two tools can never disagree about where a click
                        // landed. Neither destination is a doc write (Decision 4 for both).
                        LG::Ruler {
                            start_x,
                            start_y,
                            cam,
                        } => {
                            let moved =
                                ((up_x - start_x).powi(2) + (up_y - start_y).powi(2)).sqrt();
                            if moved < st::DRAG_THRESHOLD_PX {
                                let w = cam.unproject_xy(start_x, start_y);
                                if w[0].is_finite() && w[1].is_finite() {
                                    let z = dem_grid.borrow().as_ref().and_then(|g| {
                                        map_engine_core::dem::downsample::sample_grid_meters(
                                            g, w[0], w[1],
                                        )
                                    });
                                    if tool_mode.get_untracked().is_los() {
                                        if los_mode.get_untracked().is_viewshed() {
                                            // T-644 VIEWSHED sub-mode — a SINGLE click places the
                                            // observer and shades the whole disc (one-shot, not a
                                            // drag: this shares the ray's sub-threshold click arm, so
                                            // the T-723 button-0/no-armed-place/take discipline is
                                            // already met). Follows `place_viewshed`'s documented
                                            // host-wiring example: store the observer + click-time Z in
                                            // the session state, then `place_viewshed` (compute the
                                            // raster + stash it for pan re-projection) and upload the
                                            // returned texture to the engine's viewshed lane. Session-
                                            // local overlay state + a GPU wash — never a doc write
                                            // (Decision 4). NO-ENGINE GUARD (mirrors the ray's engine
                                            // guard / Boot-Failed): `place_viewshed` returns `None`
                                            // when no DEM sampler is registered, and the upload only
                                            // runs when the engine is live — a dead map draws nothing.
                                            viewshed.borrow_mut().place(w[0], w[1], z);
                                            if let Some(tex) =
                                                crate::editor::tools::los_tool::place_viewshed(
                                                    w[0], w[1],
                                                )
                                            {
                                                if let Some(e) = engine.borrow_mut().as_mut() {
                                                    let _ = e.viewshed_upload(
                                                        tex.min_x,
                                                        tex.min_y,
                                                        tex.max_x,
                                                        tex.max_y,
                                                        tex.tex_w,
                                                        tex.tex_h,
                                                        &tex.rgba,
                                                        tex.stride_bytes,
                                                    );
                                                }
                                            }
                                        } else {
                                            // LoS RAY: first click sets the observer, second completes
                                            // the shot (Decision 2's two-click capture). Session-local
                                            // overlay state, never a doc write (Decision 4).
                                            los.borrow_mut().click(w[0], w[1], z);
                                            sync_los();
                                        }
                                    } else {
                                        ruler.borrow_mut().press(w[0], w[1], z);
                                        sync_ruler();
                                    }
                                }
                            }
                        }
                        // T-648 XFORM-SHIFT-001 — SHIFT-ROTATE commit. Reached only with NO palette
                        // place armed (the `has_pending()` branch at the top of pointerup already
                        // returned — the T-723 discipline: this arm sits OUTSIDE that branch) and no
                        // pan in flight, and because the promotion wrote `LG::Rotate` into `left`,
                        // this `take()` above is what clears it (nothing is left armed). Release the
                        // capture the promotion grabbed, then rotate the LIVE selection to face the
                        // release cursor (unprojected against the frozen press `cam`), quantised to
                        // the effective rotation rung. One history/persist tail via
                        // `rotate_selection_to_face`. A drop with no finite aim (cursor off-map, or
                        // on the pivot) is a silent no-op inside the commit.
                        LG::Rotate { cam, .. } => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            let aim = cam.unproject_xy(up_x, up_y);
                            if aim[0].is_finite() && aim[1].is_finite() {
                                let rung = snap.get_untracked().effective_rotate_rung();
                                let acted =
                                    editor_ops::rotate_selection_to_face(aim[0], aim[1], rung);
                                if acted {
                                    // A rotate changes the doc but not the selection; keep the tint
                                    // lane in sync (glyphs re-bind off the history tail) and refresh
                                    // the SEL readout, mirroring the Move commit's bookkeeping.
                                    mission_history::refresh_selection();
                                }
                            }
                        }
                    }
                }
            });
            // T-662 → T-664 — RMB no longer pans (see onpointerdown), so the browser menu is only
            // *suppressed* here, never propagation-eaten. `prevent_default` stops the BROWSER's
            // native menu (still the first thing this does, and all it did under T-662); it does NOT
            // `stop_propagation` — that is the invariant the T-662 pin protects, and it holds: this
            // handler attaches to the SAME `contextmenu` event and, having stopped the native menu,
            // opens OUR menu at the event pixel. `prevent_default`'s only meaning is "suppress the
            // browser menu" — it is NOT a "someone handled this" flag (wave-101 verifier note 2), so
            // there is no `default_prevented()` gate here: this handler always acts on the click.
            //
            // Hit-target (T-664, selection-aware): pick the entity under the cursor with a fresh
            // frozen camera at the event px (the same pick the click / dbl-click paths run), then
            // `resolve_target` decides the take — empty ground vs on-entity, retargeting to the hit
            // entity when it is not already selected (Eden's rule). `open` commits any retarget to
            // the live selection and shows the menu. Do not add `stop_propagation` here.
            let oncontextmenu = Closure::<dyn FnMut(web_sys::MouseEvent)>::new({
                let container = container.clone();
                let engine = engine.clone();
                let doc = doc.clone();
                let selection = selection.clone();
                move |ev: web_sys::MouseEvent| {
                    ev.prevent_default();
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    // Frozen camera at the event px (borrow scoped so it drops before the pick's
                    // doc borrow; JS is single-threaded so this never reenters the rAF borrow_mut).
                    let cam = {
                        let g = engine.borrow();
                        let Some(e) = g.as_ref() else { return };
                        crate::editor::tools::select_tool::frozen_camera(
                            rect.width(),
                            rect.height(),
                            e.target_x(),
                            e.target_y(),
                            e.zoom(),
                        )
                    };
                    // Slot OR vehicle under the cursor — the same pick the left-click uses, so the
                    // menu's notion of "the entity here" matches selection's.
                    let hit = doc.borrow().as_ref().and_then(|c| {
                        crate::editor::tools::select_tool::pick_slot_or_vehicle(
                            &cam,
                            &map_render_slot_soa(c),
                            &editor_ops::vehicle_points(),
                            px,
                            py,
                        )
                    });
                    let sel = selection.borrow().clone();
                    // T-651 (`PLACE-COMMENT-001`) — the PLACE GESTURE, and it is deliberately not an
                    // armed one. The world point is unprojected HERE, against the same frozen camera
                    // the pick above used, and rides `MenuTarget` to the dispatch; "Place Comment"
                    // then writes the annotation immediately at that point.
                    //
                    // Why no arm: comments do not need an arm to be correct. Unlike a palette place,
                    // the gesture that chooses the point (the right-click) and the gesture that
                    // confirms the action (the menu row) are already two events, so the point is
                    // captured once and consumed once, with no in-flight state to strand. (T-723
                    // repaired the armed pointerup machine — button filter, left.take(), Esc/RMB
                    // disarm — but Place Comment still adds ZERO new state to LeftGesture.)
                    let world = cam.unproject_xy(px, py);
                    let target =
                        crate::editor::panels::context_menu::resolve_target(hit.as_deref(), &sel)
                            .at_world(world[0], world[1]);
                    crate::editor::panels::context_menu::open(
                        ev.client_x() as f64,
                        ev.client_y() as f64,
                        target,
                    );
                }
            });
            // T-159.21 — pointer off the map ⇒ the CUR read-out shows the em-dash cells (React's
            // `onPointerLeave → null`). Fires when the pointer enters a chrome panel too, which is
            // correct: those px are not map coordinates.
            let onpointerleave = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let engine = engine.clone();
                // T-802 — the hover cursor is reset here for the same reason the CUR read-out is
                // blanked: these pixels are no longer map pixels. Without it a pointer that left the
                // map WHILE over a glyph would strand `pointer` on the canvas, and since `cursor` is
                // an inherited property the stale claim would sit under the chrome the pointer moved
                // onto. Dropping the state (not just the CSS) also makes the re-entry move re-test.
                let hover_state = hover_state.clone();
                let canvas = canvas.clone();
                move |_ev: web_sys::PointerEvent| {
                    cursor.set(None);
                    hover_state.set(HoverState::default());
                    set_map_cursor(&canvas, false);
                    // T-175 B2 — hide the palette place ghost when the cursor leaves the map (a
                    // still-armed place re-shows it on re-entry; an off-canvas release cancels).
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        e.clear_place_preview();
                    }
                }
            });
            // T-159.18/.19 — pointercancel ends BOTH a pan and any LMB gesture, but (unlike pointerup)
            // is NOT a commit: it drops the gesture without picking / moving / selecting, and clears any
            // live preview (drag overlay / marquee rect) + releases capture.
            let onpointercancel = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let pan_px = pan_px.clone();
                let container = container.clone();
                let left = left.clone();
                let engine = engine.clone();
                let doc = doc.clone(); // T-796 — to re-bind the comment lane on a cancelled drag
                move |ev: web_sys::PointerEvent| {
                    // T-159.22 — a cancelled pointer drops an armed place, like every other
                    // in-flight gesture below (pointercancel is never a commit).
                    // T-768 — same for an armed connect (never a commit on cancel).
                    editor_ops::cancel_pending();
                    editor_ops::cancel_connect();
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                    }
                    use crate::editor::tools::select_tool::{self as st, LeftGesture as LG};
                    let taken = left.borrow_mut().take();
                    match taken {
                        Some(LG::Move { .. }) => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                // T-573 — a cancel is never a commit, so nothing downstream
                                // re-binds: the previewed vehicle rows would otherwise stay parked
                                // at the last offset while the document says they never moved.
                                st::clear_drag_preview(e, &editor_ops::vehicle_points());
                                // T-796 — the comment lane, same reasoning: a cancelled drag that
                                // held a note left its glyph at the previewed offset. Re-bind the
                                // authored positions (identity when no note was dragged).
                                // T-808 — ids ride along (see the preview arm).
                                if let Some((cxy, cids)) = doc.borrow().as_ref().map(|c| {
                                    (
                                        comment_lane_xy(&c.comments_json()),
                                        comment_lane_ids(&c.comments_json()),
                                    )
                                }) {
                                    e.comments_bind_ids(&cxy, cids);
                                }
                            }
                        }
                        Some(LG::Marquee { .. }) => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.upload_marquee(0.0, 0.0, 0.0, 0.0, false);
                            }
                        }
                        // T-648 — a cancelled Shift-rotate: release the capture the promotion grabbed
                        // and drop the gesture with NO rotation committed (cancel is never a commit).
                        // No engine preview to clear — a rotate never touched the drag/marquee lanes.
                        Some(LG::Rotate { .. }) => {
                            if container.has_pointer_capture(ev.pointer_id()) {
                                let _ = container.release_pointer_capture(ev.pointer_id());
                            }
                        }
                        _ => {}
                    }
                }
            });
            let _ = container.add_event_listener_with_callback(
                "pointerdown",
                onpointerdown.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "pointermove",
                onpointermove.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "pointerup",
                onpointerup.as_ref().unchecked_ref(),
            );
            // pointercancel ends the pan + a pending LMB without a click (T-159.18).
            let _ = container.add_event_listener_with_callback(
                "pointercancel",
                onpointercancel.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "contextmenu",
                oncontextmenu.as_ref().unchecked_ref(),
            );
            let _ = container.add_event_listener_with_callback(
                "pointerleave",
                onpointerleave.as_ref().unchecked_ref(),
            );
            // T-159.26 A1 / T-647 ATTR-OPEN-001 / PLACE-003 — native dblclick, left button only.
            // Picks with a FRESH frozen camera at the event px (the same pick the click / context
            // menu paths use). Two outcomes:
            //   * HIT an entity → open Attributes. The pick is `pick_slot_or_vehicle`, NOT the
            //     slot-only `pick` this handler used before T-647: Attributes must open for a
            //     VEHICLE (and any glyph on the vehicle lane) as well as a slot, which is exactly
            //     the ATTR-OPEN-001 "not just slots" swap. `open_attributes` still owns the
            //     multi-select suppression (>1 selected ⇒ no-op).
            //   * MISS (empty ground) → open the asset PICKER at the world point (PLACE-003).
            //     Picking an asset there arms a place (`begin_place*`), and the very next canvas
            //     click lands it (the click-then-click contract, PLACE-001). This is the LEFT
            //     button; right-click is T-664's context menu, so the two never collide.
            // The chrome subtree stops pointerdown, so a dblclick over a dock never reaches here;
            // and a boot that ended `Failed` has no engine, so the `engine.borrow()` guard below
            // returns before either branch — no engine, no placement (and no picker).
            let ondblclick = Closure::<dyn FnMut(web_sys::MouseEvent)>::new({
                let container = container.clone();
                let engine = engine.clone();
                let doc = doc.clone();
                // T-642 — the ruler chain + its reactive sync, so a dbl-click can END the chain.
                let ruler = ruler.clone();
                let sync_ruler = sync_ruler.clone();
                move |ev: web_sys::MouseEvent| {
                    if ev.button() != 0 {
                        return;
                    }
                    // T-642 — with the Ruler tool active, a double-click ENDS the chain and KEEPS it
                    // placed (Decision 3), instead of opening Attributes / the asset picker. The two
                    // pointerups of the dbl-click already committed two coincident final vertices, so
                    // `dedup_tail` drops the duplicate before `double_click` stops the draw — the kept
                    // ruler ends on the real penultimate vertex. Returns before the pick below so a
                    // dbl-click in ruler mode never opens an editor dialog. (Select mode is unchanged:
                    // this guard is skipped and the pick path runs exactly as before.)
                    if tool_mode.get_untracked().is_ruler() {
                        let mut r = ruler.borrow_mut();
                        // 0.5 m dedupe: far below a click's pixel footprint at any editor zoom, so
                        // only the dbl-click's own coincident second vertex is removed.
                        r.dedup_tail(0.5);
                        r.double_click();
                        drop(r);
                        sync_ruler();
                        return;
                    }
                    // T-643 — with the LoS tool active, a double-click must NOT open Attributes / the
                    // asset picker either. LoS captures TWO single clicks (observer then target); a
                    // fast double-click's two pointerups already ran `LosState::click` twice via the
                    // shared `LG::Ruler` arm — which is exactly a completed shot — so this handler just
                    // swallows the `dblclick` event so it opens no dialog. (Select mode is unchanged:
                    // both measure-tool guards are skipped and the pick path runs as before.)
                    if tool_mode.get_untracked().is_los() {
                        return;
                    }
                    let rect = container.get_bounding_client_rect();
                    let (px, py) = (
                        ev.client_x() as f64 - rect.left(),
                        ev.client_y() as f64 - rect.top(),
                    );
                    let cam = {
                        let g = engine.borrow();
                        let Some(e) = g.as_ref() else { return };
                        crate::editor::tools::select_tool::frozen_camera(
                            rect.width(),
                            rect.height(),
                            e.target_x(),
                            e.target_y(),
                            e.zoom(),
                        )
                    };
                    // T-647 ATTR-OPEN-001 — slot OR vehicle under the cursor, matching the click and
                    // context-menu picks so "the entity here" means one thing editor-wide.
                    let hit = doc.borrow().as_ref().and_then(|c| {
                        crate::editor::tools::select_tool::pick_slot_or_vehicle(
                            &cam,
                            &map_render_slot_soa(c),
                            &editor_ops::vehicle_points(),
                            px,
                            py,
                        )
                    });
                    match hit {
                        Some(id) => editor_ops::open_attributes(id),
                        // T-647 PLACE-003 — empty ground: open the asset picker at the world point
                        // the dblclick names (same frozen-cam unproject the place ghost/CUR use, so
                        // the picker's eventual drop lands where the dblclick was). A singular
                        // unproject (NaN) is off-map and opens nothing.
                        None => {
                            let world = cam.unproject_xy(px, py);
                            if world[0].is_finite() && world[1].is_finite() {
                                editor_ops::open_asset_picker(
                                    world[0],
                                    world[1],
                                    ev.client_x() as f64,
                                    ev.client_y() as f64,
                                );
                            }
                        }
                    }
                }
            });
            let _ = container
                .add_event_listener_with_callback("dblclick", ondblclick.as_ref().unchecked_ref());

            let onresize = Closure::<dyn FnMut()>::new({
                let engine = engine.clone();
                let canvas = canvas.clone();
                let container = container.clone();
                move || {
                    let dpr = web_sys::window()
                        .map(|w| w.device_pixel_ratio())
                        .unwrap_or(1.0);
                    let rect = container.get_bounding_client_rect();
                    let (dw, dh) = device_size(rect.width(), rect.height(), dpr);
                    canvas.set_width(dw);
                    canvas.set_height(dh);
                    if let Some(e) = engine.borrow_mut().as_mut() {
                        let _ = e.resize(rect.width(), rect.height(), dpr);
                    }
                }
            });
            let _ =
                win.add_event_listener_with_callback("resize", onresize.as_ref().unchecked_ref());

            // The engine + these listeners intentionally leak on route-leave: `on_cleanup` is
            // `Send`-bound and can't hold the `!Send` engine, so we only move `disposed` (Send) into
            // it. Stopping the loop is what prevents a runaway render; a proper `!Send` drop handle
            // is a later polish. The T-189 `beforeunload` guard is the one exception — it is torn
            // down (see its `on_cleanup` above), because a leaked unload prompt is user-visible.
            onwheel.forget();
            onresize.forget();
            onpointerdown.forget();
            onpointermove.forget();
            onpointerup.forget();
            onpointercancel.forget();
            oncontextmenu.forget();
            onpointerleave.forget();
            ondblclick.forget();
            on_cleanup(move || disposed.store(true, Ordering::Relaxed));
        });
    }

    view! {
        <div
            node_ref=container_ref
            class="relative h-screen w-screen overflow-hidden bg-background"
        >
            <canvas node_ref=canvas_ref class="absolute inset-0 z-0 block h-full w-full"></canvas>
            // T-159.21 — Eden chrome host (React MissionCreatorPage:272). The container class is
            // deliberately UNCHANGED and the canvas stays full-bleed underneath: every `select_tool`
            // probe derives its camera from this container's bounding rect, so shrinking it would
            // silently invalidate the pan/select/marquee/move gates.
            //
            // `pointer-events-none` hands the whole rect to the map; each panel re-enables
            // `pointer-events-auto` for itself. The panels are DESCENDANTS of the gesture container,
            // so without `stop_propagation` a chrome click would bubble into `onpointerdown` and open
            // an LMB map gesture — clicking Undo would also deselect. Its corollary is the
            // chrome-free inset in `select_tool::farthest_empty_px`.
            // T-159.22 — `data-eden-chrome` marks the whole chrome subtree for the wheel guard's
            // `closest()` (see CHROME_SEL): a wheel whose target is inside it must scroll the dock,
            // not zoom the map.
            <div
                data-eden-chrome
                class="pointer-events-none absolute inset-0 z-10"
                on:pointerdown=|ev| ev.stop_propagation()
            >
                // T-662 — the four chrome mounts (strip + both docks + toolbelt) are gated on
                // `chrome_hidden` (Backspace toggles it). While hidden they leave the DOM entirely,
                // so the map is full-bleed and every px is a map gesture; the modals below are NOT
                // gated (a Settings/Attributes dialog the operator opened stays put). Re-pressing
                // Backspace remounts them.
                // T-177 A3 — raise the strip's stacking context above the docks (z-20) so its
                // menu dropdowns (`z-50`, scoped to this subtree) paint OVER the left/right docks
                // instead of behind them (the Environment-menu-clipped-by-Outliner bug).
                {
                    let strip_title = mission_id.clone();
                    move || (!chrome_hidden.get()).then(|| view! {
                    <div class="absolute inset-x-0 top-0 z-30 h-12">
                        <crate::editor::eden_chrome::TopCommandStrip
                            title=strip_title.clone()
                            can_undo
                            can_redo
                            save_semver
                            save_status
                            dirty
                            settings_open
                            doc_tick
                            obj_count
                            orbat_open
                        />
                    </div>
                })}
                // T-638 — the wrapper shrinks to a top-corner 24×24 box while the dock is collapsed
                // (drop `bottom-0`/`w-*` so it stops covering the map — the freed strip becomes
                // click-through and the map pane reflows). The `DockLeft`/`DockRight` component renders
                // either the full panel or the stub off the same `collapsed` signal.
                //
                // T-637 — THE WIDTH IS NOT WRITTEN HERE. These four class strings are `eden_layout`
                // consts, next to the `DOCK_LEFT_PX`/`DOCK_RIGHT_PX` they must agree with, because
                // `select_tool` unprojects the pointer by those numbers: a hand-written `w-*` that
                // drifted from the const would map every click in the map pane to the wrong world
                // position while everything still LOOKED right. `t637_dock_geometry` parses the width
                // back out of these consts and unprojects with it.
                {move || (!chrome_hidden.get()).then(|| view! {
                    <div class=move || if dock_left_collapsed.get() {
                        crate::editor::layout::DOCK_LEFT_MOUNT_COLLAPSED
                    } else {
                        crate::editor::layout::DOCK_LEFT_MOUNT
                    }>
                        <crate::editor::eden_chrome::DockLeft
                            nodes=outliner_nodes
                            selected=selected_ids
                            active_layer
                            collapsed=dock_left_collapsed
                        />
                    </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                    <div class=move || if dock_right_collapsed.get() {
                        crate::editor::layout::DOCK_RIGHT_MOUNT_COLLAPSED
                    } else {
                        crate::editor::layout::DOCK_RIGHT_MOUNT
                    }>
                        <crate::editor::eden_chrome::DockRight
                            catalog
                            vehicle_catalog
                            registry_items
                            registry_failed
                            registry_fetch_gen
                            doc_tick
                            fm_open
                            active_side
                            objects_mode
                            collapsed=dock_right_collapsed
                        />
                    </div>
                })}
                // T-636 — the old single ~580 px centred pill split into TWO mounts (Eden's layout:
                // tools on a toolbar, telemetry in a full-width status bar). BOTH stay behind the
                // `chrome_hidden` gate, so Backspace still unmounts the whole bottom belt (wave101
                // N-5 forward constraint — the gate count grows from 4 to 6 here, both new mounts
                // gated).
                //
                // (1) The mode toolbar — Select / Ruler / LoS. Floats just above the status bar,
                // left-of-centre, keeping the operator's "content and feel unchanged": the same
                // three buttons in the same pill, no longer sharing the strip with the readouts.
                {move || (!chrome_hidden.get()).then(|| view! {
                <div class="absolute bottom-11 left-1/2 -translate-x-1/2">
                    <crate::editor::panels::toolbelt::ModeToolbar tool_mode los_mode />
                </div>
                })}
                // (2) The full-width status bar — CUR/OBJ/SEL/SZ readouts, the T-667 map-furniture
                // slot (scale bar + grid refs, wave 106 — reserved, not built), the T-719 debug HUD
                // slot (now a legitimate VISIBLE home in the bar's right section instead of the
                // invisible `right-3 bottom-3` overlay corner DockRight's z-20 painted over), and the
                // §Open primary-action slot. Docked `inset-x-0 bottom-0`, spanning the viewport like
                // Eden's status bar rather than floating centred. The HUD keeps its exact T-635 gate
                // stack: `chrome_hidden` (this wrapper unmounts it) AND `debug_hud_shown` (passed as
                // `hud_shown`) AND a non-empty sampler string (checked inside `StatusBar`).
                {move || (!chrome_hidden.get()).then(|| view! {
                <div class="absolute inset-x-0 bottom-0">
                    <crate::editor::panels::toolbelt::StatusBar
                        cursor
                        sel_count
                        obj_count
                        selected_ids
                        sz_bytes
                        debug_hud
                        hud_shown=debug_hud_shown
                        ruler_status
                        scale_mpp
                    />
                </div>
                })}
                // T-667 — map-pane edge grid references (dispatcher-authorized SINGLE mount line;
                // the overlay component + all its logic live in `eden_toolbelt`, my owned file). It
                // anchors to the MAP-PANE edges (between the docks), so it cannot render from the
                // status-bar furniture slot; it reads the live camera + viewport itself and re-runs
                // off the same `cursor`/`debug_hud` heartbeats (no new rAF loop). Gated by
                // `chrome_hidden` like the other furniture so Backspace hides it too.
                {move || (!chrome_hidden.get()).then(|| view! { <crate::editor::panels::toolbelt::MapGridRefs cursor debug_hud=Some(debug_hud) /> })}
                // T-159.26 — Attributes modal (fixed overlay; no DOM while closed). Inside the
                // chrome subtree so its pointerdowns never open a map gesture. NOT gated by T-662's
                // `chrome_hidden` — a dialog the operator opened must survive a hide-interface toggle.
                <div class="pointer-events-auto">
                    <crate::editor::panels::attributes_modal::AttributesModal attrs_open attrs_tab doc_tick registry_items compat />
                </div>
                <div class="pointer-events-auto">
                    <crate::editor::eden_chrome::MissionSettingsDialog open=settings_open doc_tick />
                    <crate::pages::operations::faction_manager::FactionManagerDialog open=fm_open registry=registry_items />
                    // T-177 B2 / T-071.0 — ORBAT Manager modal shell (browse/select the live ORBAT
                    // faction → squad → slot tree relocated from the left dock).
                    <crate::editor::eden_chrome::OrbatManagerDialog
                        open=orbat_open
                        orbat=orbat_nodes
                        selected=selected_ids
                        active_layer
                        registry=registry_items
                    />
                </div>
                // T-159.26 — local-vs-server conflict prompt (React's ConflictDialog). Renders only
                // when `conflict` is Some (a divergent local doc on cold boot). Data-safety: the
                // user chooses which version wins before any Save.
                <div class="pointer-events-auto">
                    <ConflictDialog conflict conflict_id=mission_id.clone() />
                </div>
                // T-664 — the right-click context menu overlay. Mounted HERE, beside the ungated
                // dialogs (Attributes / Settings / Faction / ORBAT / Conflict) and NOT inside the
                // four `chrome_hidden` gates above, so a menu the operator opened survives a
                // Backspace hide-chrome (wave-101 verifier note 1: a floating overlay is not dock
                // chrome). Renders no DOM while `context_menu` is None; its own backdrop is
                // `pointer-events-auto` so click-away dismissal works even over the map.
                <div class="pointer-events-auto">
                    <crate::editor::panels::context_menu::ContextMenuOverlay menu=context_menu />
                </div>
                // T-647 PLACE-003 — the empty-ground asset picker. Ungated (survives hide-chrome)
                // and self-contained: it reuses the SAME `registry_items` + `active_side` the
                // DockRight catalog is built from, so a picked leaf arms the identical place the
                // dock would (`begin_place`), which the next canvas click lands (PLACE-001). Renders
                // no DOM while `asset_picker` is None.
                // T-651 — the comment editor (all three ATTR-FIELD-CMT-* fields + copy/delete),
                // opened by double-clicking a comment row in the Outliner. Ungated for the same
                // reason as the picker above; renders no DOM while closed.
                <div class="pointer-events-auto">
                    <CommentEditorOverlay open=comment_editor doc_tick />
                </div>
                // T-672 — the Connections panel (every edge, every finding, a delete per row).
                // UNGATED like the context menu and the comment editor: an audit surface the
                // operator deliberately opened is not dock chrome, so it survives Backspace
                // hide-chrome. Renders no DOM while closed.
                <div class="pointer-events-auto">
                    <ConnectionsPanelOverlay open=connections_panel doc_tick />
                </div>
                <div class="pointer-events-auto">
                    <AssetPickerOverlay picker=asset_picker registry=registry_items active_side />
                </div>
                // T-642 — the ruler overlay (dispatcher-authorized SINGLE mount line; the component +
                // all its logic live in `ruler_tool`, my owned file). UNGATED like the context menu /
                // asset picker: a PLACED ruler is a measurement the operator created, so it survives a
                // Backspace hide-chrome (it is not dock furniture — unlike the scale bar / grid refs,
                // which ARE gated). It is `pointer-events-none` (the SVG never eats a map gesture — the
                // click-chain capture is the map's own pointer handlers), reads the live camera + chain
                // itself, and re-runs off the same `cursor`/`debug_hud` heartbeats as the furniture (no
                // new rAF loop) plus `ruler_tick` (repaint on a still-pointer click).
                <crate::editor::tools::ruler_tool::RulerOverlay cursor debug_hud=Some(debug_hud) tick=ruler_tick />
                // T-643 — the Line-of-Sight overlay (dispatcher-authorized SINGLE mount line; the
                // component + all its logic live in `los_tool`, my owned file). UNGATED like the ruler
                // overlay: a placed LoS shot is a measurement the operator created, so it survives a
                // Backspace hide-chrome (it is not dock furniture). `pointer-events-none` (the SVG
                // never eats a map gesture — the two-click capture is the map's own pointer handlers),
                // reads the live camera + state + DEM sampler itself, and re-runs off the same
                // `cursor`/`debug_hud` heartbeats as the ruler (no new rAF loop) plus `los_tick`
                // (repaint on a still-pointer click).
                <crate::editor::tools::los_tool::LosOverlay cursor debug_hud=Some(debug_hud) tick=los_tick />
                // T-648 — the transformation widget (WIDGET-CYCLE-001 / WIDGET-TRANS-001). UNGATED
                // like the ruler/LoS overlays: it draws on the live selection, is `pointer-events-none`
                // (the gestures are the map's own handlers), and re-runs off the same
                // `cursor`/`debug_hud` heartbeats plus `widget_tick` (repaint on a keyboard selection
                // change with a still pointer). Draws nothing when the selection is empty.
                <TransformWidgetOverlay
                    cursor
                    debug_hud=Some(debug_hud)
                    tick=widget_tick
                    variant=widget_variant
                />
                // T-795 — the cursor-adjacent active-mode hint (one of the two mode indicators the
                // review F-16 asked for; the toolbar toggle plate per T-799 is the other). Same
                // `pointer-events-none` overlay band as the widget gizmo.
                <WidgetModeHint cursor variant=widget_variant />
                // T-655 — the validation panel (dispatcher-authorized SINGLE mount line; the component
                // + all its logic live in `validation_panel`, my owned file). A floating collapsible
                // card, bottom-left above the status bar (the overlay idiom — NOT docked; docking
                // collides with the dock files program-wide). UNGATED — it is NOT inside a
                // `chrome_hidden` gate, so a Backspace hide-interface leaves it visible: validation is
                // ALWAYS ON and correctness diagnostics are never gated (T-635's doctrine — "telemetry
                // gates, correctness diagnostics never"), unlike the scale bar / grid refs / snap
                // readout, which ARE dock furniture and gated. Re-evaluates off `doc_tick` (the T-666
                // channel) through its own 250 ms trailing debounce; the engine call is defensively
                // wrapped so a rule panic can never take the editor down.
                <validation_panel::ValidationPanel doc_tick />
                // T-648 — the snap-grid step readout (TOOLBAR-GRID-MOVE-001). GATED on `chrome_hidden`
                // — it is status-bar furniture like the scale bar / grid refs, so Backspace hides it
                // too (this is the SEVENTH chrome-gated mount; the count pin is updated to match).
                {move || (!chrome_hidden.get()).then(|| view! { <SnapReadout snap /> })}
            </div>
            // T-628 — boot loading overlay: ONE bar, 0→100%, across the whole boot. It never resets
            // between stages and there is no sweep anywhere in it — the stage name underneath
            // changes, the bar does not restart. `pointer-events-none` so it never intercepts an
            // operator / editor-smoke click (the map no-ops while the engine is None).
            //
            // Every width this renders came from `BootProgress`, which moves on completed bytes and
            // completed fetches only. The 200 ms ease on `.mc-load-fill` travels *to* the last
            // measured width, so it can lag a real completion and can never lead one; a stalled
            // network is a stalled bar. The overlay comes down `BOOT_HANDOVER_MS` after the bar
            // reaches 100%, never before it.
            {move || {
                let phase = boot.get();
                let p = progress.get();
                match phase {
                    // The boot succeeded and handed over: no overlay. (A dead map after a
                    // "continue without map" still shows its own badge below — that is NOT gated
                    // on the phase, because dismissing the error is exactly reaching `Ready`.)
                    BootPhase::Ready => None,
                    // T-631 — the failure state. The overlay stops being a spinner and becomes a
                    // report: the segment that broke, the REAL reason (verbatim from wgpu), and two
                    // ways out. `pointer-events-auto` HERE (the loading bar is `-none`) because the
                    // operator has to be able to click Retry / Continue. `z-50` keeps it over the
                    // chrome the same way the bar did.
                    // `.into_any()`: the three arms build different concrete `View` element trees
                    // (an error card vs. the bar vs. nothing), which Leptos cannot unify into one
                    // return type — erasing each to `AnyView` is the standard reconciliation.
                    BootPhase::Failed { seg, reason } => Some(view! {
                        <div class="animate-overlay-fade pointer-events-auto absolute inset-0 z-50 flex items-center justify-center bg-background/90 backdrop-blur-sm">
                            <div class="flex w-80 max-w-[90vw] flex-col items-center gap-3 rounded-xl border border-error/40 bg-surface-variant/40 p-6 text-center">
                                <p class="text-sm font-semibold text-error">
                                    {format!("{} failed", seg.title().trim_end_matches('…'))}
                                </p>
                                // The real reason, verbatim. This is the line the ticket is about:
                                // the boot no longer sits silent — it says WHY. `break-words` so a
                                // long wgpu message wraps instead of overflowing the card.
                                <p class="max-h-32 overflow-y-auto break-words font-mono text-[11px] text-on-surface-variant/80">
                                    {reason}
                                </p>
                                <div class="mt-1 flex gap-2">
                                    <button
                                        type="button"
                                        aria-label="Retry"
                                        class="rounded-lg bg-primary px-4 py-2 text-label-md font-medium text-on-primary"
                                        on:click=move |_| {
                                            // A fresh GPU-init attempt = a fresh mount. Reload the
                                            // page so `RenderEngine::create` runs again from a clean
                                            // slate (the failure can be transient — a lost device, a
                                            // GPU still waking). Wasm-only; the native shell has no
                                            // engine to retry.
                                            #[cfg(target_arch = "wasm32")]
                                            if let Some(win) = web_sys::window() {
                                                let _ = win.location().reload();
                                            }
                                        }
                                    >
                                        "Retry"
                                    </button>
                                    <button
                                        type="button"
                                        aria-label="Continue without map"
                                        class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                                        on:click=move |_| {
                                            // Dismiss the error onto the live editor: the docs, the
                                            // outliner and the Attributes all work against the doc
                                            // with no engine. Reaching `Ready` takes the overlay
                                            // down; `map_disabled` stays `Some`, so the dead-pane
                                            // badge below replaces the map. `advance` is not needed
                                            // here — this is the one deliberate exit FROM `Failed`,
                                            // driven by the operator, so it sets `Ready` directly.
                                            boot.set(BootPhase::Ready);
                                        }
                                    >
                                        "Continue without map"
                                    </button>
                                </div>
                            </div>
                        </div>
                    }.into_any()),
                    // Still booting: the T-628 bar, unchanged. One 0→100% journey, no sweep.
                    _ => {
                        let pct = p.percent();
                        let title = p.stage().title();
                        let caption = p.caption();
                        Some(view! {
                            <div class="animate-overlay-fade pointer-events-none absolute inset-0 z-50 flex items-center justify-center bg-background/85 backdrop-blur-sm">
                                <div class="flex w-64 flex-col items-center gap-2">
                                    <p class="text-sm font-medium text-on-surface-variant">{title}</p>
                                    // Was `h-1` (a 4 px hairline) when it only ever swept. A bar the
                                    // operator is expected to read a real position off is worth the
                                    // extra 2 px.
                                    <div class="h-1.5 w-56 overflow-hidden rounded-full bg-surface-variant/40">
                                        <div
                                            class="mc-load-fill h-full rounded-full bg-primary"
                                            style=format!("width:{pct:.1}%")
                                        ></div>
                                    </div>
                                    <p class="font-mono text-[11px] tabular-nums text-on-surface-variant/60">
                                        {caption}
                                    </p>
                                </div>
                            </div>
                        }.into_any())
                    }
                }
            }}
            // T-631 — the dead-map badge. After "continue without map" the overlay is gone and the
            // canvas behind the chrome is a black rectangle drawing nothing (the engine never
            // started). This labels it so it reads as a known, chosen degraded state rather than a
            // bug — and it is `pointer-events-none`, low in the corner, so it never fights the docks
            // or the toolbelt the operator is still using. Shown only once the error overlay is down
            // (`boot == Ready`) so it does not double up with the report above.
            {move || {
                let disabled = map_disabled.get();
                let down = boot.get() == BootPhase::Ready;
                (down)
                    .then_some(disabled)
                    .flatten()
                    .map(|reason| view! {
                        <div class="pointer-events-none absolute bottom-16 left-1/2 z-40 -translate-x-1/2 rounded-lg border border-error/30 bg-background/80 px-3 py-1.5 text-center backdrop-blur-sm">
                            <p class="text-label-md font-medium text-error">"Map unavailable"</p>
                            <p class="max-w-xs truncate font-mono text-[10px] text-on-surface-variant/70">
                                {reason}
                            </p>
                        </div>
                    })
            }}
        </div>
    }
}

/// T-628 — take the overlay down, [`BOOT_HANDOVER_MS`] after the last segment reported in.
///
/// Called from the two boot tasks' rendezvous, so by the time it runs every segment has already
/// sent `Finish` and the bar reads exactly 100%. The delay is the hand-over, not the work: without
/// it the final report and the overlay's removal are folded into one Leptos render and the operator
/// sees the bar stop short and the screen change under it. A window that has gone away simply skips
/// the timer and the overlay stays — the same thing that already happens if a boot task never
/// returns.
#[cfg(target_arch = "wasm32")]
fn hand_over(boot: RwSignal<BootPhase>) {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;
    // T-631 — `Ready` takes the overlay down; a `Failed` boot must keep it up (the error state is
    // the overlay). Both the timer callback and the two immediate fallbacks route through `advance`
    // so a rendezvous that fires after an engine-init failure cannot dismiss the error onto a dead
    // map. In the normal boot `self` is `LoadingMap`/`Hydrating`, so `advance(Ready)` == `Ready`.
    // `Copy` closure (its only capture, the `RwSignal`, is `Copy`), so it is both callable inline
    // in the fallbacks and movable into the timer closure.
    let go_ready = move || boot.update(|b| *b = b.clone().advance(BootPhase::Ready));
    let Some(win) = web_sys::window() else {
        go_ready();
        return;
    };
    let cb = Closure::once_into_js(go_ready);
    if win
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            cb.as_ref().unchecked_ref(),
            BOOT_HANDOVER_MS,
        )
        .is_err()
    {
        boot.update(|b| *b = b.clone().advance(BootPhase::Ready));
    }
}

/// The rAF render loop. Each frame renders then polls the device (see `RenderEngine::poll`) so
/// readback `map_async` callbacks drain on the WebGL2-fallback + cull-counter path. (The timer
/// double-map that panicked the 15.0 loop is handled upstream by `disable_frame_timing`.) Stops
/// (and drops itself) once `disposed` is set.
#[cfg(target_arch = "wasm32")]
fn start_raf(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    disposed: std::sync::Arc<std::sync::atomic::AtomicBool>,
    debug_hud: RwSignal<String>,
    scale_mpp: RwSignal<f64>,
) {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::atomic::Ordering;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    // T-172 B9 — ~1 Hz debug readout sample (screen-05 bottom-right HUD): zoom, drawn world
    // chunks, tree glyphs, FPS. Counting frames between samples measures real rAF cadence.
    let mut frames = 0u32;
    let mut last_sample = 0.0f64;
    // T-670 — last PUBLISHED scale readout. The camera zoom is only reachable from inside this
    // per-frame closure, so this is the guard that keeps a 60 fps read from becoming a 60 fps
    // Leptos write: the frame formats the scale and calls `set` ONLY when the formatted string
    // differs from what the status bar is already showing. Without it every frame would dirty
    // `scale_mpp`, re-rendering the status bar (and the scale bar) 60×/s for a value that changes a
    // handful of times per wheel gesture and never at all while panning or idle. Empty until the
    // first frame publishes, so the seeded default is replaced as soon as the engine is live.
    let mut last_scale_text = String::new();

    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        if disposed.load(Ordering::Relaxed) {
            f.borrow_mut().take(); // drop the loop closure — no further frames
            return;
        }
        // T-631 — the double-panic fix. This was `engine.borrow_mut()`, which PANICS if the cell
        // is already borrowed. When `e.render()` panicked (the observed `createBuffer size too
        // large` → wasm `unreachable`), the abort re-entered the editor while the first panic was
        // unwinding; the next frame's `borrow_mut` then found the cell still held and panicked a
        // SECOND time with "RefCell already borrowed", and that second panic — not the render
        // failure — is what surfaced, burying the real cause. `try_borrow_mut` makes a contended
        // frame a no-op instead of a panic, so a re-entrant borrow can never overwrite the first,
        // true panic. It is also the correct steady-state behaviour: a frame that cannot get the
        // engine simply waits for the next rAF rather than taking the tab down.
        let Ok(mut guard) = engine.try_borrow_mut() else {
            // Contended: skip this frame, keep the loop alive.
            let cb_ref = f.borrow();
            if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
                let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
            }
            return;
        };
        if let Some(e) = guard.as_mut() {
            let _ = e.render();
            e.poll(); // ★ T-159.15.1: drain readback map_async so the next submit can't double-map
            frames += 1;
            // T-670 — publish the screen scale for the status-bar readout (and, through it, the
            // T-667 scale bar). Read every frame so a wheel-zoom shows on the very next frame
            // rather than waiting up to a second for the ~1 Hz HUD sample below; WRITTEN only when
            // the displayed string changes, so an idle or panning camera costs zero re-renders.
            // The `m_per_px`/`format_m_per_px` pair is `eden_toolbelt`'s — the same conversion the
            // scale bar uses and the same `2^(−deckZoom)` convention T-639's contour ladder takes.
            {
                let mpp = crate::editor::panels::toolbelt::m_per_px(e.zoom());
                let text = crate::editor::panels::toolbelt::format_m_per_px(mpp);
                if text != last_scale_text {
                    last_scale_text = text;
                    scale_mpp.set(mpp);
                }
            }
            {
                // js_sys::Date over web_sys Performance — no extra web-sys feature needed, and
                // ms precision is plenty for a 1 Hz FPS sample.
                let now = js_sys::Date::now();
                if last_sample == 0.0 {
                    last_sample = now;
                } else if now - last_sample >= 1000.0 {
                    let fps = (f64::from(frames) * 1000.0 / (now - last_sample)).round();
                    let stats: serde_json::Value =
                        serde_json::from_str(&e.stats()).unwrap_or_default();
                    let chunks = stats["chunks"].as_u64().unwrap_or(0);
                    let glyphs = stats["tree_glyphs"].as_u64().unwrap_or(0);
                    // T-173 — frame-cost cell: submitted-frame CPU EMA + its FPS-equivalent
                    // (1000/ms — the off-vsync headroom number; rAF FPS stays vsync-capped).
                    let rf_ms = stats["render_cpu_ms_ema"].as_f64().unwrap_or(0.0);
                    let rf_eq = if rf_ms > 0.0 { 1000.0 / rf_ms } else { 0.0 };
                    debug_hud.set(format!(
                        "z {:.2} · c{chunks} · glyph {glyphs} · {fps:.0} FPS · rf {rf_ms:.2}ms ({rf_eq:.0} eq)",
                        e.zoom()
                    ));
                    frames = 0;
                    last_sample = now;
                }
            }
        }
        let cb_ref = f.borrow();
        if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
            let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
        }
    }) as Box<dyn FnMut()>));
    let cb_ref = g.borrow();
    if let (Some(cb), Some(win)) = (cb_ref.as_ref(), web_sys::window()) {
        let _ = win.request_animation_frame(cb.as_ref().unchecked_ref());
    }
}

/// Expose the byte-exact GPU readback self-checks on `window.__selfChecks` — the map-lane gate the
/// headless driver awaits (see [[wgpu-headless-gpu-verify]]). Both checks are scene-independent
/// (`self_check` renders its own fixed calibration probe scene; `texture_self_check` a synthetic
/// 2×2 texture) and `&self`: they clone their GPU handles up front, so the shared `borrow()` here is
/// released before the async readback runs — no contention with the rAF loop's `borrow_mut` (JS is
/// single-threaded). Each resolves to a JSON string with a `pass` field.
#[cfg(target_arch = "wasm32")]
fn register_self_checks(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let obj = js_sys::Object::new();

    let calibration = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };
    let texture = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move || {
            engine
                .borrow()
                .as_ref()
                .map(|e| e.texture_self_check())
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut() -> js_sys::Promise>)
    };

    let _ = js_sys::Reflect::set(
        &obj,
        &JsValue::from_str("calibration"),
        calibration.as_ref(),
    );
    let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("texture"), texture.as_ref());
    // T-173 — `window.__editorBench(n)` off-vsync frame-cost bench (perf gates G-A/G-B): resolves
    // the engine's `render_bench` JSON. Registered here (not in `__selfChecks`) so the perf probe
    // and the operator console both reach it by one name.
    let bench = {
        let engine = engine.clone();
        Closure::wrap(Box::new(move |n: f64| {
            engine
                .borrow_mut()
                .as_mut()
                .map(|e| {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    e.render_bench(n.max(1.0) as u32)
                })
                .unwrap_or_else(|| js_sys::Promise::reject(&JsValue::from_str("engine not ready")))
        }) as Box<dyn FnMut(f64) -> js_sys::Promise>)
    };
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__selfChecks"), &obj);
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorBench"), bench.as_ref());
    }
    // The harness reads these across the page lifetime; leak them (the engine leaks too).
    calibration.forget();
    texture.forget();
    bench.forget();
}

/// Expose the camera view-state on `window.__editorCam()` for the headless pan smoke (T-159.15.2 /
/// spec P6): a JSON string `{"tx","ty","z","backend"}` read from the `&self` getters `target_x()` /
/// `target_y()` / `zoom()` / `backend()`. (`#[wasm_bindgen(getter)]` fns are plain method calls from
/// Rust.) All are `&self` behind a shared `borrow()`, released before return — no contention with the
/// rAF loop's `borrow_mut` (JS is single-threaded). Registered once the engine is `Some`; the closure
/// leaks like the self-checks. The smoke drives pan via getter deltas (never `unproject_xy`, X-05).
///
/// T-166 — also installs `window.__editorCamSet(tx, ty, z)` so `smoke_fullmap` can Class-R probe
/// tree glyphs at zoom ≥ 0 without relying on CDP `mouseWheel` → DOM `wheel` delivery.
#[cfg(target_arch = "wasm32")]
fn register_editor_cam(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
    map_host: crate::editor::world_assets::HostHandle,
) {
    use wasm_bindgen::prelude::*;

    let cam = Closure::wrap(Box::new({
        let engine = engine.clone();
        move || -> JsValue {
            engine
                .borrow()
                .as_ref()
                .map(|e| {
                    JsValue::from_str(&format!(
                        r#"{{"tx":{},"ty":{},"z":{},"backend":"{}"}}"#,
                        e.target_x(),
                        e.target_y(),
                        e.zoom(),
                        e.backend()
                    ))
                })
                .unwrap_or_else(|| JsValue::from_str("null"))
        }
    }) as Box<dyn FnMut() -> JsValue>);

    let cam_set = Closure::wrap(Box::new({
        let engine = engine.clone();
        let map_host = map_host.clone();
        move |tx: f64, ty: f64, z: f64| {
            if let Some(e) = engine.borrow_mut().as_mut() {
                e.set_view(tx, ty, z);
                e.on_camera_changed(); // T-172 H5
            }
            // Immediate flush so smoke_fullmap A_trees_on does not race the 120 ms debounce.
            crate::editor::world_assets::flush_viewport(map_host.clone(), engine.clone());
        }
    }) as Box<dyn FnMut(f64, f64, f64)>);

    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCam"), cam.as_ref());
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__editorCamSet"), cam_set.as_ref());
    }
    cam.forget();
    cam_set.forget();
}

/// T-172 B4 — automated slot-lane proof hook: `window.__wgpuSlotStats()` returns the engine's
/// `slot_stats_json` (atlas_ready / slot_len / cluster_mode / …) for the doc smoke.
#[cfg(target_arch = "wasm32")]
fn register_slot_stats(
    engine: std::rc::Rc<std::cell::RefCell<Option<map_engine_render::RenderEngine>>>,
) {
    use wasm_bindgen::prelude::*;

    let stats = Closure::wrap(Box::new(move || -> JsValue {
        engine
            .borrow()
            .as_ref()
            .map(|e| JsValue::from_str(&e.slot_stats_json()))
            .unwrap_or_else(|| JsValue::from_str("null"))
    }) as Box<dyn FnMut() -> JsValue>);
    if let Some(win) = web_sys::window() {
        let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__wgpuSlotStats"), stats.as_ref());
    }
    stats.forget();
}

// T-934.10 — names only the evacuated `#[cfg(test)]` pins still reach through `super::…` (their
// shipping callers moved into `render_sync` with the belt). This `use` lives HERE, at the file's
// test boundary, because the Class-R scrubber and the keymap census both treat the FIRST literal
// `#[cfg(test)]` as "everything after this is test fixture" — a test-gated import up top would
// truncate every scrub of this file to nothing.
#[cfg(test)]
pub(crate) use crate::editor::canvas::render_sync::{
    crewed_slot_ids, map_render_keep_indices, CONN_LINE_RGBA, CONN_LINE_SELECTED_RGBA,
    HOVER_CURSOR_PICKABLE, HOVER_CURSOR_PLAIN, HOVER_RELEASE_PX, HOVER_THROTTLE_MS,
};

#[cfg(test)]
#[path = "mission_editor_tests/t245_registry_session.rs"]
mod t245_registry_session;

/// T-750 — registry fetch Err raises a terminal failure signal the Favourites panel can read.
///
/// Wave-114 MINOR-2: Err only set `catalog`/`vehicle_catalog` to Failed and left `registry_items`
/// at None, so Favourites spun on "Resolving…" forever. Pins run on `live_code` (comments +
/// string literals blanked; test module cut) so a hollow note cannot green them. The helper is
/// host-visible on purpose — the wasm32 Err arm is scrubbed on native, but the call site in the
/// raw page still names it (asserted separately with a fragment-assembled needle).
#[cfg(test)]
#[path = "mission_editor_tests/t750_registry_fetch_failure_signal.rs"]
mod t750_registry_fetch_failure_signal;

/// T-573 — the mixed-drag preview wiring.
///
/// **Why a source pin here and a behavioural test elsewhere.** The proof that the preview moves the
/// right vehicles is a real unit test on the real function —
/// `map_engine_core::slots_gpu::pack_vehicle_drag_preview`, native, driven directly. What cannot be
/// proven that way is the *wiring*: `mod select_tool` is `#[cfg(target_arch = "wasm32")]`
/// (`main.rs`) and `editor_ops` is `#![cfg(target_arch = "wasm32")]`, so no native test can call the
/// drag path, and `RenderEngine` needs a GPU device besides. That leaves "the host hands the WHOLE
/// mixed selection to both lanes" as the one claim only source can carry — so it is carried on the
/// fail-closed scrubber (T-601 `class_r_scrub`), not a grep: `live_code` deletes comments, string
/// literals, `#[cfg(any())]` items, `if false` blocks and code after an unconditional jump, and
/// `only_body` refuses a marker that matches zero or two or more items rather than guessing.
/// `the_preview_pin_rejects_every_dead_code_wrapper` below keeps that honest.
#[cfg(test)]
#[path = "mission_editor_tests/t573_mixed_drag_preview.rs"]
mod t573_mixed_drag_preview;

/// T-427 — cold path must not depend on the unbounded dual dump.
#[cfg(test)]
#[path = "mission_editor_tests/t427_cold_registry_path.rs"]
mod t427_cold_registry_path;

/// T-627/T-628 — the Mission Creator boot bar: one 0→100% journey over four measured segments, and
/// a satellite fetch that is concurrent without being unordered.
///
/// Everything the loader itself does is `#[cfg(target_arch = "wasm32")]` — `fetch_range` is
/// `gloo-net` over `web_sys`, and `mod world_assets` does not exist on the host at all — so no test
/// here fetches anything, and none pretends to. What these pins do cover is the whole class of bug
/// a host test *can* catch cheaply and a browser cannot catch at all until the operator is already
/// watching the wrong bar: the arithmetic that turns four differently-sized, differently-metered
/// segments into one number that never rewinds, and the reassembly that decides whether tile 3's
/// pixels land at tile 3's coordinates. The source pins at the bottom hold the wasm side to actually
/// routing through the code proved here, so it cannot drift back to a sweep, to a whole-file DEM
/// GET, or to a batch that discovers its own size after the fact, while these stay green.
#[cfg(test)]
#[path = "mission_editor_tests/t628_boot_progress.rs"]
mod t628_boot_progress;

/// T-631 — the boot overlay cannot fail SILENTLY. The engine-init failure itself is wasm-side
/// (`RenderEngine::create` needs a real GPU), but the state machine the overlay reads —
/// `BootPhase` and its `advance` fold — is pure and drives entirely here, which is exactly what
/// the acceptance clause allows ("a native test can still drive `BootPhase`/`BootEvent`
/// transitions directly"). These tests inject the failure the way the engine task does, assert the
/// overlay reaches `Failed { seg, reason }` carrying the ORIGINAL reason, and — the part that made
/// the real bug so nasty — assert that the concurrent doc-hydrate task's later, misleading
/// transitions (`LoadingMap`, then `Ready` via the hand-over) do NOT overwrite that reason.
#[cfg(test)]
#[path = "mission_editor_tests/t631_boot_failure_state.rs"]
mod t631_boot_failure_state;

/// T-629 — `world_assets` is `#![cfg(target_arch = "wasm32")]`, so its `mod tbd_sat;` never
/// compiles for the host test runner. `tbd_sat.rs` itself is pure (serde + integer comparisons,
/// no `web_sys`), and the mip level it chooses IS the displayed resolution of the basemap — the
/// most consequential arithmetic in the map host. Mounting the same file a second time under a
/// test-only name is what lets that arithmetic be executed here rather than only grepped for.
/// The two mounts are never both live: this one is `not(target_arch = "wasm32")`.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "world_assets/tbd_sat.rs"]
mod tbd_sat_pure;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "mission_editor_tests/t629_satellite_resolution.rs"]
mod t629_satellite_resolution;

/// T-662 — the two input traps that gated the editor program: RMB eaten by pan, and Backspace
/// aliased to Delete.
///
/// **Why source pins.** Both traps live in code no native test can execute: the pointer/keydown
/// closures and the whole Eden view are `#[cfg(target_arch = "wasm32")]` (they call `RenderEngine`,
/// `web_sys` events, and `editor_ops`, all wasm-only), so the proof that the trap is gone is the
/// proof that the *wiring* changed. String-literal match arms (`"Backspace"`, `"Delete"`) are pinned
/// on the raw file because `live_code` blanks string literals; structural/no-comment claims (the pan
/// button guard, the contextmenu body, the mount gating) are pinned on `live_code`, which blanks
/// comments and dead code so a stale note or an `if false` wrapper cannot satisfy them.
#[cfg(test)]
#[path = "mission_editor_tests/t662_input_traps.rs"]
mod t662_input_traps;

/// T-635 — the debug HUD (telemetry) must (a) toggle behind Ctrl+Alt+D in the editor keydown,
/// honouring the editable-field guard; (b) default HIDDEN; (c) NO LONGER live inside the toolbelt's
/// `bottom-5 left-1/2` wrapper (so it cannot paint over the CUR/OBJ readouts) and stay gated so an
/// overlap is impossible; (d) keep the telemetry-vs-diagnostics distinction explicit in a comment.
#[cfg(test)]
#[path = "mission_editor_tests/t635_debug_hud.rs"]
mod t635_debug_hud;

/// T-647 — placement interactions: the Ctrl state machine (multi-place ↔ regroup), Alt = empty
/// vehicle, the double-click asset picker on empty ground, and the double-click→Attributes swap that
/// now reaches vehicles. All six ids are pinned on live source (comments stripped / string literals
/// blanked), because the doc-mutating half is wasm-only (`editor_ops` runs no native test) — the
/// wiring is the thing a native pin can prove, exactly as the T-573 / T-662 / T-635 modules do.
#[cfg(test)]
#[path = "mission_editor_tests/t647_placement_interactions.rs"]
mod t647_placement_interactions;

/// T-642 — source pins for the RULER click-chain wiring in the wasm pointer/keydown/dblclick
/// handlers (which a native test cannot execute; the pure state machine + math are event-tested in
/// `ruler_tool`). These pin the binding constraints the wave-106 verifier flagged (T-723) plus the
/// mount + the tool-mode arbitration entry, on scrubbed code (comments + strings blanked) so a
/// needle is real code, never a comment. The scrubber KEEPS `#[cfg(target_arch="wasm32")]` blocks
/// (undecided cfg), so the handler tokens are visible — the same reason t662 can pin inside them.
#[cfg(test)]
#[path = "mission_editor_tests/t642_ruler_wiring.rs"]
mod t642_ruler_wiring;

/// T-643 — source pins for the LINE-OF-SIGHT click-capture wiring in the wasm pointer/keydown/
/// dblclick handlers (which a native test cannot execute; the pure state machine + occlusion math are
/// unit-tested in `los_tool`). LoS deliberately REUSES the ruler's `LG::Ruler` gesture arm + Esc seam
/// (the "mode field on the ruler arm" the ticket sanctions, so no third `LeftGesture` variant is
/// added to the un-owned `select_tool`), so these pins prove that reuse is disciplined: the commit
/// routes by `tool_mode`, the Esc is the SHARED arm (not a second window listener — T-726), and the
/// overlay/state/sampler are mounted + registered. Scrubbed code (comments + strings blanked) so a
/// needle is real code; the scrubber keeps `#[cfg(target_arch="wasm32")]` blocks visible.
#[cfg(test)]
#[path = "mission_editor_tests/t643_los_wiring.rs"]
mod t643_los_wiring;

/// T-644 (wave 110) — source pins for the VIEWSHED live entry point: the sub-mode is threaded through
/// the LoS button (toggle) and the pointer commit (route), a viewshed click computes + uploads the
/// wash to the engine lane, and the clear seams (Esc + tool/sub-mode switch) drop BOTH the state and
/// the GPU lane through the EXISTING shared seams — no new window listener (T-726 pending). The pure
/// `LosMode`/`ViewshedState`/`place_viewshed` core is unit-tested in `los_tool`; these prove the wasm
/// wiring a native test cannot execute. Scrubbed code (comments + strings blanked) so a needle is real
/// code; the scrubber keeps `#[cfg(target_arch="wasm32")]` blocks visible.
#[cfg(test)]
#[path = "mission_editor_tests/t644_viewshed_wiring.rs"]
mod t644_viewshed_wiring;

/// T-644 (wave 110) — source pins for the LoS button's SUB-MODE TOGGLE in `eden_toolbelt`: the ONE
/// LoS button re-click toggles Ray ⇆ Viewshed (`LosMode::toggled`) while LoS is already active, and
/// the button's title/label reflect the live sub-mode. The toolbar is a Leptos view (structural), so
/// this is pinned by SOURCE INSPECTION on scrubbed `eden_toolbelt.rs`, mirroring `t643`/`t668`.
#[cfg(test)]
#[path = "mission_editor_tests/t644_los_button_submode.rs"]
mod t644_los_button_submode;

/// T-648 — Transform: Shift-rotate, snap grid, transform widget + the Space collision decision.
///
/// The pure primitives (`transform` module) are proved BEHAVIOURALLY here — it is an ungated module
/// like `boot_progress`, so a native `cargo test -p website-frontend` (the command CI/the wave gate
/// runs) compiles and executes these, unlike a test placed beside `drag_delta` in the wasm-only
/// `select_tool`. The wasm wiring (the Shift-rotate gesture arm, the widget mount, the keydown
/// bindings, the included comment fix) is proved by SOURCE PINS on `live_code` (comments + dead code
/// stripped, so a stale note or an `if false` wrapper cannot satisfy them). The keydown CENSUS reads
/// both window-level editor keydowns (this file + `mission_history`) as raw text.
#[cfg(test)]
#[path = "mission_editor_tests/t648_transform.rs"]
mod t648_transform;

/// T-655 — the validation panel wiring pins: the mount exists, its payload source is registered, it
/// re-evaluates off the `doc_tick` channel, it is ALWAYS ON (no debug flag), and it SURVIVES
/// hide-chrome (mounted OUTSIDE every `chrome_hidden` gate — the diagnostics doctrine). These scan
/// the comment-stripped page source (`live_code`) so the doc prose that mentions `chrome_hidden`
/// cannot false-match the gate check.
#[cfg(test)]
#[path = "mission_editor_tests/t655_validation_panel_wiring.rs"]
mod t655_validation_panel_wiring;

/* ═══════ T-761 — compile findings cleared on editor hydrate (wave-116 finding 3) ═══════════════
 *
 * The behavioural pin lives in validation_panel. This Class-R pin locks the PRODUCTION call site:
 * MissionEditorPage must clear after hydrate_from_server, or a client-side mission switch still
 * inherits the previous mission's build report.
 */
#[cfg(test)]
#[path = "mission_editor_tests/t761_compile_findings_cleared_on_hydrate.rs"]
mod t761_compile_findings_cleared_on_hydrate;

/* ═══════ T-754 — the click-to-select router resolves ZONES, and says so before it is clicked ═════
 *
 * Two families, as the ticket demands: unit tests over the pure resolution (it is `serde_json`-only,
 * so it RUNS natively — this is not a source scan pretending to be a behaviour test), and source pins
 * for the parts that are wiring (the closure is wasm-only and holds `!Send` handles).
 */
#[cfg(test)]
#[path = "mission_editor_tests/t754_router_resolves_zones.rs"]
mod t754_router_resolves_zones;

/* ════ wave 129 F6 — the affordance probe and the click may not answer different questions ════
 *
 * F1 built the probe out of `route_target`'s resolution alone; F2 made `route_select_zone` report
 * honestly that an unmounted Zones panel selects nothing. Each is right. TOGETHER they disagreed:
 * for a zone subject with the panel unmounted the probe said `true` (the row painted
 * `cursor-pointer`) and the click said `false` (nothing happened) — a dead click dressed as an
 * affordance, which is the T-754 MAJOR this wave was opened to kill.
 *
 * The fix is [`route_availability`]: ONE narrowing both seams go through. These pins defend the
 * INVARIANT, not the arm — the table below covers every target kind, so the next divergence (a new
 * arm that needs a seam, say) is red here whether or not it is a zone.
 */
#[cfg(test)]
#[path = "mission_editor_tests/wave129_f6_probe_and_click_cannot_disagree.rs"]
mod wave129_f6_probe_and_click_cannot_disagree;

// ─────────────────────── T-649 — Select All in view + Attributes multi-edit ───────────────────
/// Source pins for T-649. `map-engine-core` is linked natively with the `mission` feature ONLY
/// (`Cargo.toml`: `doc`/`camera` are `cfg(target_arch = "wasm32")` deps), and `select_tool` /
/// `editor_ops` are both wasm32-gated modules — so neither `OrthoCamera`, `SlotSoa` nor
/// `select_all_in_view` can be CALLED from a native `cargo test`. These pin the wiring the way the
/// rest of this file's editor contracts are pinned: on the live source, with string literals
/// blanked (`live_code`) wherever the shape rather than the text is the contract.
#[cfg(test)]
#[path = "mission_editor_tests/t649_select_all_and_multi_edit.rs"]
mod t649_select_all_and_multi_edit;

// ──────────────── T-669 — clipboard completion: cut + paste-at-original ───────────────────────
/// Source pins for T-669 (`ACTION-CUT-001`, `ACTION-PASTE-ORIG-001`). `editor_ops` is a wasm32-only
/// module, so neither `copy_selection` nor `paste_at_cursor` can be CALLED from a native
/// `cargo test`; these pin the WIRING the way the rest of this file's editor contracts are pinned —
/// on the live source, sliced to the keydown arm list so a needle can never self-match inside this
/// module (which lives in the same file as the arms it reads).
#[cfg(test)]
#[path = "mission_editor_tests/t669_clipboard_completion.rs"]
mod t669_clipboard_completion;

/// T-670 (`STATUS-ZOOM-001`) — the editor's half of the metres-per-pixel readout. `RenderEngine::
/// zoom()` is reachable only from the rAF sampler, so the editor owns the signal and the sampler
/// writes it. The sampler runs EVERY FRAME, which makes the write guard the load-bearing part of
/// this ticket: an unguarded `set` would dirty the status bar 60×/s and tank editor performance —
/// the exact class of regression the `rf <ms>` HUD cell exists to surface. These are Leptos view /
/// wasm-closure innards, so they are pinned by SOURCE INSPECTION on scrubbed code (the established
/// `t635`/`t636` pattern here); needles are assembled at run time so this module's own prose can
/// never satisfy them.
#[cfg(test)]
#[path = "mission_editor_tests/t670_scale_signal.rs"]
mod t670_scale_signal;

/// T-723 — event-SEQUENCE regressions for the armed-place root (wave-106 MAJOR-1/2/3,
/// wave-108 composition tooltip + Ruler strand, wave-109 LoS strand).
///
/// These drive `armed_place::step` / `run` — the same decide_* helpers the wasm handlers call.
/// Source pins are forbidden here: they could not see any of the three defects.
#[cfg(test)]
#[path = "mission_editor_tests/t723_armed_place.rs"]
mod t723_armed_place;

/// Wave-130 F1 / T-760 — `mission_history` is `#![cfg(target_arch = "wasm32")]`, so its feed cannot
/// host a native Class-R pin. Pin the two live call sites from here via `include_str!` + scrub, the
/// same way T-573 pins `vehicles_bind` / `select_tool` from this file.
#[cfg(test)]
#[path = "mission_editor_tests/t760_markers_bind_feed.rs"]
mod t760_markers_bind_feed;

/// **T-808 — the FEEDERS, pinned to the symbology signatures.**
///
/// T-808 built the engine half (`slots_bind_symbology`, `vehicles_bind_symbology`,
/// `comments_bind_ids`) but could not wire the callers, and an engine that can draw a medic facing
/// east is worth exactly nothing while the feeder still calls `slots_bind_soa`: every slot reads as
/// a rifleman pointing north, every vehicle as an amber disc, every note as unselected. These pins
/// hold the wiring — the same Class-R shape as the T-760 marker pin above, and for the same reason:
/// `mission_history` is `#![cfg(target_arch = "wasm32")]` end to end, so a scrubbed `include_str!`
/// is a native test's only reach into it.
#[cfg(test)]
#[path = "mission_editor_tests/t808_symbology_feed.rs"]
mod t808_symbology_feed;

/// T-726 — window-Esc pile-up: every editor Esc consumer consults the modal stack.
///
/// Defect (wave106 MINOR-2 / wave108 MAJOR-2 / wave109-110): separate window keydowns all fired on
/// one Esc — stacked prefs+settings both closed, and Esc closing the context menu also cleared a
/// placed ruler/LoS/viewshed. The fix is one design: overlays `register` + `is_topmost_open`; the
/// shared measure arm yields while `any_open()`.
///
/// Hollow-pin discipline: deleting the `any_open` guard or any overlay's `is_topmost_open` gate
/// turns the matching assert RED. Needles are assembled from fragments so this module cannot
/// self-satisfy them.
#[cfg(test)]
#[path = "mission_editor_tests/t726_window_esc_stack.rs"]
mod t726_window_esc_stack;

/// T-768 — Eden CONN-START-001 LMB target pick: the missing half after T-672's RMB arm/complete.
///
/// Hollow-pin discipline: deleting `complete_connect` from the Pending click path, or
/// `cancel_connect` from the Esc arm, turns the matching assert RED. Needles are assembled from
/// fragments so this module cannot self-satisfy them.
#[cfg(test)]
#[path = "mission_editor_tests/t768_connect_lmb_complete.rs"]
mod t768_connect_lmb_complete;

/// T-780 — the connection LINE: geometry, packing, hit test, and the four wiring facts that make
/// `CONN-DEL-001` reachable from the map.
///
/// The first three are behaviour tests over the pure functions. The rest are Class-R pins, because
/// the wiring lives in `#[cfg(target_arch = "wasm32")]` closures no native test can call: they are
/// taken over `live_code` (comments AND string literals blanked, test module cut), so a needle can
/// never be satisfied by the prose that describes it or by this module's own assertion text.
#[cfg(test)]
#[path = "mission_editor_tests/t780_connection_line.rs"]
mod t780_connection_line;

// ═════ T-784 — the comment GLYPH is pickable, and the pick is the lane's own list ════════════════
//
// The defect was total: the glyph had no pick path at all, the Outliner comment row was `ROW_STATIC`
// with no route to the selection, the T-697 selection filter can only NARROW an existing selection
// (so it can never introduce a comment that was not already selected), and `route_target` had no
// comment arm — so even the document-search hit rendered inert. Nothing could put a comment id into
// the selection by clicking, which left T-781's composable-comment lane unreachable.
//
// These pins hold the three properties the fix rests on:
//   1. the lane and the pick are ONE document read (T-780's construction, applied to the glyph);
//   2. a comment RESOLVES, so `subject_id_routes` — the affordance behind the Outliner row and the
//      dock-left search hit — can honestly say yes, and the existing kinds still resolve as before;
//   3. the map click folds the comment into `hit`, so the selection it lands in is the one the
//      composition capture reads, and Delete removes the comment rather than reporting success over
//      an unchanged document.
#[cfg(test)]
#[path = "mission_editor_tests/t784_comment_glyph.rs"]
mod t784_comment_glyph;

// ═════ T-796 — a comment can be DRAGGED: pick → preview → one-txn move ════════════════════════════
//
// O-6 pixel-verified two defects. The DRAG half is fixed here (the glyph RESTYLE — neutral shape +
// non-selection colour + selection-on-top — is the `comments_bind` glyph in `map-engine-render`'s
// `engine.rs`, a co-owned symbology surface (T-808/T-790) outside this slice's one file; it is
// reported found-not-fixed, not touched from here).
//
// Before this ticket a drag STARTING on a note resolved `pick_slot_or_vehicle` → None → LG::Marquee,
// so the note never entered a move; and even if a comment id HAD reached the LG::Move commit it would
// have fallen into `slot_ids` and been handed to `move_entities`, which reads the slot SoA, finds no
// such row, and moves it nowhere — a 90px drag left the stored position unchanged (verified twice).
//
// These pins + the two pure-function tests hold the four properties the fix rests on:
//   1. the drag-start FOLDS the comment pick into `hit` (the T-784 click precedent, not a fork), so a
//      note grabs like a slot — after slot/vehicle, before the marquee fallthrough;
//   2. the commit PARTITIONS comments out by asking the document's own map (`comment_details`, the
//      `delete_selection` rule) and routes them to `move_comment` — never to `move_entities`;
//   3. each note is base + delta, ONE txn per note, so a single-note drag is one Ctrl+Z (the ticket's
//      "ONE step"; multi-note inherits `delete_selection`'s accepted per-txn class);
//   4. the mid-drag preview re-binds the note's own lane so the glyph follows the cursor, and every
//      non-commit exit (zero delta, wrong button, cancel) re-binds it to the authored positions.
#[cfg(test)]
#[path = "mission_editor_tests/t796_comment_drag.rs"]
mod t796_comment_drag;

// ══ T-790 — the authored icon + caption reach the marker lane (F-03 write-half) ══════════════════
//
// Before this ticket the feeder read only x/z/factionId and dropped `icon`/`label`, so every placed
// marker drew as one pale disc with no caption. `marker_lane_fields` now parses all four parallel
// arrays from `briefing_marker_rows_json`, mapping `icon` → the canonical glyph (so DIFFERENT icons
// draw DIFFERENT shapes) and carrying `label` as the on-map caption. These pins hold that; the last
// is a Class-R pin binding `mission_history`'s two feed sites to the widened `markers_bind` shape
// (that module is `#![cfg(wasm32)]`, so a source pin is a native test's only reach into it).
#[cfg(test)]
#[path = "mission_editor_tests/t790_marker_glyph_caption.rs"]
mod t790_marker_glyph_caption;

// ══ Wave 145 F-1 — the selection prune keeps what EXISTS and drops what is GONE, both ways ═══════
//
// The prune in `mission_history` had one job — drop ids the document no longer holds — and was
// sourced from the slot SoA, a universe that contains no vehicle, no placed object and no comment.
// So it did the opposite of its job for three of the four selectable kinds: every document change
// (a drag commit, an undo, a layer toggle, a loadout write, an IDB restore) silently deleted them
// from the selection and cleared the Outliner highlight.
//
// The correction is only half a correction if it stops there. The prune is ALSO what has been
// guaranteeing that a stale id can never reach Delete — the wave-129 and wave-142 MAJORs both grew
// out of an id outliving the thing it named. So these pins fire BOTH directions against
// `selectable_ids`, the widened universe:
//
//   * a comment / vehicle / object / hidden slot that IS in the document SURVIVES a change;
//   * one that is NOT in the document — deleted, undone away, or never minted — still FALLS OUT.
//
// Plus the two the widening could have got wrong: a HIDDEN slot is in the universe (it exists; the
// SoA's T-665/T-701 drop is a visibility view, not an existence test), and a ZONE or MARKER is NOT
// (neither has a route into `ctx.selection`, so admitting them would widen the universe past the
// set being pruned). The Class-R pin at the end binds `mission_history`'s single prune site to this
// function through `include_str!` — that module is `#![cfg(target_arch = "wasm32")]` end to end, so
// no test placed in it would ever execute, and a source pin is the only reach a native test has.
#[cfg(test)]
#[path = "mission_editor_tests/w145_selection_prune.rs"]
mod w145_selection_prune;

// ── T-802 (O-8) — the hover cursor ───────────────────────────────────────────────────────────────
//
// Two halves, and both are load-bearing for a different reason.
//
// The STATE MACHINE (`hover_due` / `hover_next` / `hover_cursor_css`) is pure, so the throttle and
// the hysteresis are proved here rather than eyeballed in a browser — including the acceptance's
// own churn property, which is a statement about a SEQUENCE of hit-tests and therefore exactly the
// kind of thing a screenshot cannot check.
//
// The SOURCE PINS are the T-057 half. The reason hover picking was removed in the first place was
// cost, and every one of the four things that keeps this cheap (throttle first, no second point
// set, one document read per `doc_tick`, write only on change) is a property of the CALL SITE, not
// of any function these tests can call. So the call site is read back out of the file: an edit that
// hoists the pick above the throttle, or re-derives its own geometry, or writes the style every
// tick, breaks a named assertion instead of quietly re-creating the regression that deleted this
// feature once already.
#[cfg(test)]
#[path = "mission_editor_tests/t802_hover_cursor.rs"]
mod t802_hover_cursor;

/* ═══════════════════════ T-819 — crewed slots leave the map render SoA ═══════════════════════
 *
 * Eden hides a unit that is boarded into a vehicle. The hide is DERIVED from `vehicle.crew` —
 * never a new document flag, never a `materialize()` drop (those are T-665/T-701 for operator
 * hide). Figures + labels leave the map; outliner / selection / compile still see the slots.
 */

#[cfg(test)]
#[path = "mission_editor_tests/t819_crewed_render_hide.rs"]
mod t819_crewed_render_hide;
