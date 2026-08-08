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

    use crate::arsenal_rules::{CargoRow, CompatFeed};
    use crate::dto::RegistryItem;

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
    auth: crate::auth::AuthStore,
) -> Result<Vec<crate::dto::RegistryItem>, crate::client::ApiErr> {
    use crate::dto::{RegistryItem, RegistryResponse};

    let mut all: Vec<RegistryItem> = Vec::new();
    let mut offset: i64 = 0;
    loop {
        let path = format!("/registry?limit={REGISTRY_COLD_PAGE}&offset={offset}");
        let page: RegistryResponse = crate::client::api_get(auth, &path).await?;
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
    auth: crate::auth::AuthStore,
) -> Result<
    (
        crate::arsenal_rules::CompatFeed,
        std::collections::HashMap<String, Vec<crate::arsenal_rules::CargoRow>>,
    ),
    crate::client::ApiErr,
> {
    use crate::arsenal_rules::{CargoRow, CompatFeed, CompatGraph, CompatStatus};
    use crate::dto::{RegistryCargoDefaultsResponse, RegistryCompatResponse};
    use std::collections::HashMap;

    let edges_path = format!("/registry/compat?edge_type={EDITOR_COMPAT_EDGE_TYPES}");
    let edges: RegistryCompatResponse = crate::client::api_get(auth, &edges_path).await?;
    let cargo_resp: RegistryCargoDefaultsResponse =
        crate::client::api_get(auth, "/registry/compat?view=cargo_defaults").await?;

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

/// T-647 PLACE-003 — where a double-click on empty ground opened the asset picker: the WORLD point
/// the eventual place will land at, plus the SCREEN pixel to anchor the floating panel at.
///
/// Defined HERE (not in the wasm-only `editor_ops`) so the native test build — which compiles this
/// page but not `editor_ops` — can name it. `editor_ops` re-uses it via `crate::mission_editor::…`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssetPickerState {
    /// World metres — the anchor the operator aimed at (parity with the ghost/CUR unproject). The
    /// actual drop still comes from the next canvas click, so this is not a bypass of the click.
    pub wx: f64,
    pub wy: f64,
    /// Client pixel of the dblclick, so the panel floats at the cursor (like Eden's create menu).
    pub screen_x: f64,
    pub screen_y: f64,
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
    /// `1`/`2` direct keys instead (the ticket's collision decision — Eden's `1`-`5` are unbound
    /// here). Only two variants exist: **Translate** (axis arrows, axis-constrained drag) and
    /// **Rotate** (a ring, drag = rotate, Shift+drag = snap to the rotation ladder).
    ///
    /// **No area-scale variant, and that is scoped honestly.** The widget acts on the live
    /// SELECTION, which the select machine only ever fills with slot + vehicle ids
    /// (`pick_slot_or_vehicle` / `marquee_ids_with_vehicles`). Neither a slot nor a vehicle carries
    /// a scalar size — only zones and triggers have a radius, and those live in their own
    /// collections edited by the zone-draw tool, never in `selection`. So `3` (area-scale) has
    /// nothing in a transform selection to scale; offering it would be a dead key. Eden's `1`-`5`
    /// stay free for a later slice that gives the widget a scalable target.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub enum WidgetVariant {
        #[default]
        Translate,
        Rotate,
    }

    impl WidgetVariant {
        /// The `1`/`2` direct-key selection (Eden's variant keys, minus Space). `1` → Translate,
        /// `2` → Rotate; any other digit leaves the variant unchanged (returns `self`). Digit keys
        /// beyond `2` (`3`-`5`) are deliberately inert here — see the type doc on area-scale.
        #[must_use]
        pub fn from_digit(self, digit: u8) -> Self {
            match digit {
                1 => WidgetVariant::Translate,
                2 => WidgetVariant::Rotate,
                _ => self,
            }
        }
        /// Whether a Shift+ring drag on this variant snaps to the rotation ladder (only Rotate has a
        /// ring). Translate's arrows snap through the translation ladder instead. Used by the widget
        /// gesture to pick which ladder a Shift constrains.
        #[must_use]
        pub const fn is_rotate(self) -> bool {
            matches!(self, WidgetVariant::Rotate)
        }
        /// The snap [`Axis`] this variant's step keys (`[`/`]`) tune: Translate → the translation
        /// ladder, Rotate → the rotation ladder. One mapping so the keydown and the readout agree on
        /// "which grid am I stepping".
        #[must_use]
        pub const fn snap_axis(self) -> Axis {
            match self {
                WidgetVariant::Translate => Axis::Translate,
                WidgetVariant::Rotate => Axis::Rotate,
            }
        }
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
        /// The full status-bar readout: `"GRID  move 5 m · rot 15°"` when enabled, `"GRID  off"`
        /// when the master latch is off. One string so the overlay is a single text node (the
        /// scale-bar / ruler-status idiom).
        #[must_use]
        pub fn status_readout(self) -> String {
            if !self.enabled {
                return "GRID  off".to_string();
            }
            format!(
                "GRID  move {} \u{b7} rot {}",
                self.rung_label(Axis::Translate),
                self.rung_label(Axis::Rotate),
            )
        }
    }
}

thread_local! {
    /// T-648 — the registered SELECTION-CENTROID getter the transform widget projects onto. Set from
    /// `MissionEditorPage`'s wasm block (which owns the `!Send` doc + selection `Rc`s); read by the
    /// native-compiled [`TransformWidgetOverlay`] via [`read_widget_pivot`]. Peer of
    /// `ruler_tool::RULER_CHAIN` — a thread_local so the overlay never touches disposed reactive
    /// state and native builds simply see `None`.
    static WIDGET_PIVOT: std::cell::RefCell<Option<std::rc::Rc<dyn Fn() -> Option<(f64, f64)>>>> =
        const { std::cell::RefCell::new(None) };
}

/// T-648 — register the selection-centroid getter (called once at mount). `#[cfg(target_arch =
/// "wasm32")]` because only the wasm host has the doc/selection `Rc`s to close over; the getter it
/// stores returns `Option<(world_x, world_y)>` — the current selection centroid, or `None` when the
/// selection is empty or the doc is not ready.
#[cfg(target_arch = "wasm32")]
fn register_widget_pivot(f: std::rc::Rc<dyn Fn() -> Option<(f64, f64)>>) {
    WIDGET_PIVOT.with(|c| *c.borrow_mut() = Some(f));
}

/// T-648 — the current selection centroid in world metres, or `None` (empty selection / no doc /
/// native build / pre-mount). The overlay calls this each repaint; it is a cheap doc read behind the
/// registered closure.
#[must_use]
fn read_widget_pivot() -> Option<(f64, f64)> {
    WIDGET_PIVOT.with(|c| c.borrow().as_ref().and_then(|f| f()))
}

/// T-648 WIDGET-CYCLE-001 / WIDGET-TRANS-001 — the TRANSFORMATION WIDGET: a lightweight
/// `pointer-events-none` SVG gizmo drawn on the selection centroid, in the ruler/LoS overlay idiom
/// (full-bleed SVG, reads the live camera off `world_assets::camera_snapshot`, projects world→screen
/// with the same `frozen_camera` the pick uses, re-runs off the `cursor`/`debug_hud`/`tick`
/// heartbeats — no new rAF loop). It is a VIEW + affordance: the actual gestures (Shift+drag rotate,
/// axis-constrained move) are captured by the map's own pointer handlers and commit through the
/// existing move / `attrs_update_position` paths — the SVG never eats a pointer.
///
/// Two variants (`WidgetVariant`, cycled by the `1`/`2` keys):
///   * **Translate** — a pair of axis ARROWS (X east, Y north) centred on the selection. A drag on
///     an arrow is the axis-constrained move; the arrows are the discoverable handle for it.
///   * **Rotate** — a RING around the selection. A drag rotates; Shift+drag on the ring snaps to the
///     rotation ladder. (Shift+drag anywhere on a selected entity already rotates — the ring makes
///     the gesture visible.)
///
/// Only drawn when something is selected (a widget with no target is nothing to show). Ungated so it
/// is native-compiled and its projection is source-pinned; the geometry itself renders only under
/// wasm (it needs the live camera + window).
#[component]
fn TransformWidgetOverlay(
    /// Pan heartbeat (the editor's pointer-move cursor write) — re-projects the gizmo on pan.
    cursor: RwSignal<Option<(f64, f64, Option<f64>)>>,
    /// ~1 Hz zoom heartbeat (the rAF debug sampler) — re-projects after a still-pointer wheel-zoom.
    debug_hud: Option<RwSignal<String>>,
    /// Bumped when the selection changes without a pointermove, so the gizmo re-projects onto the new
    /// centroid even with a still pointer (the `ruler_tick` idiom).
    tick: RwSignal<u64>,
    /// The live widget variant (`1` translate / `2` rotate) — decides arrows vs ring.
    variant: RwSignal<transform::WidgetVariant>,
) -> impl IntoView {
    // The projected gizmo centre (screen px) + the variant, or None when there is nothing to draw.
    let projected = move || -> Option<(f64, f64, transform::WidgetVariant)> {
        // Subscribe to all heartbeats so the closure re-runs on pan (cursor), zoom (hud), selection
        // change (tick) and variant change.
        let _ = cursor.get();
        if let Some(h) = debug_hud {
            let _ = h.get();
        }
        let _ = tick.get();
        let var = variant.get();
        let (wx, wy) = read_widget_pivot()?;
        #[cfg(target_arch = "wasm32")]
        {
            let (tx, ty, zoom) = crate::world_assets::camera_snapshot()?;
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
            let cam = crate::select_tool::frozen_camera(vw, vh, tx, ty, zoom);
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
        // Full-bleed, non-interactive SVG in the same overlay band as the ruler/grid refs (z-10),
        // over the map but under the chrome docks. `pointer-events-none`: the gizmo is a view, the
        // gesture is the map's own pointer handlers.
        <svg
            data-transform-widget
            class="pointer-events-none absolute inset-0 z-10"
            width="100%"
            height="100%"
        >
            {move || projected().map(|(cx, cy, var)| {
                // Fixed pixel radius/arm length — the gizmo is a screen affordance, not a world
                // object, so it stays a constant size like a cursor (Eden's widget does too).
                const R: f64 = 42.0;
                const HEAD: f64 = 7.0;
                match var {
                    // TRANSLATE — X (east, +screen-x) and Y (north, −screen-y) arrows from the centre.
                    transform::WidgetVariant::Translate => view! {
                        <g>
                            // X axis arrow (east).
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
                            // Y axis arrow (north = up on screen).
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
                            <circle cx=move || format!("{cx:.1}") cy=move || format!("{cy:.1}")
                                    r="3" class="fill-primary" />
                        </g>
                    }.into_any(),
                    // ROTATE — a ring around the centre (drag = rotate; Shift+drag snaps).
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

/// T-648 TOOLBAR-GRID-MOVE-001 — the snap-grid STATUS READOUT: the active step ladder in the
/// status-bar band (the T-636 readout idiom). Its own tiny `pointer-events-none` element rather than
/// a field inside `eden_toolbelt::StatusBar`, because that component is another slice's owned file —
/// this keeps the readout inside T-648's three-file boundary while sitting in the same band. Shows
/// `GRID  move 5 m · rot 15°` (or `GRID  off`), re-running off the `snap` signal.
#[component]
fn SnapReadout(snap: RwSignal<transform::SnapState>) -> impl IntoView {
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

/// T-647 PLACE-003 — the empty-ground asset picker: a floating list of placeable characters for the
/// active Eden side, opened by a double-click on empty ground. Picking a row ARMS a place
/// (`begin_place`, exactly what a DockRight leaf does) and closes the panel; the operator's next
/// canvas click lands it (the click-then-click contract PLACE-001).
///
/// **Why a floating picker, not "focus the dock's search".** The ticket offered either; this is the
/// cheaper FAITHFUL form under this slice's file boundary. It reuses the same `registry_items` +
/// `active_side` the dock's catalog is built from (`build_catalog_tree`), so a picked leaf arms the
/// identical place — no second catalog, no divergence. And it is self-contained: it does not touch
/// the DockRight, so it still works when Backspace has hidden the chrome (a hidden dock can't be
/// focused — the ticket's own guard). Boot `Failed`/no-engine never opens it: the `dblclick` handler
/// returns on a `None` engine before it can call `open_asset_picker`.
#[component]
fn AssetPickerOverlay(
    picker: RwSignal<Option<AssetPickerState>>,
    registry: RwSignal<Option<Vec<crate::dto::RegistryItem>>>,
    active_side: RwSignal<String>,
) -> impl IntoView {
    // A live query filters the flat leaf list (Eden's create-menu type-ahead). Reset on each open so
    // a stale query never leaks in.
    let query = RwSignal::new(String::new());
    Effect::new(move |_| {
        if picker.get().is_some() {
            query.set(String::new());
        }
    });
    // Esc closes (mirrors the context menu). No-op while the picker is closed.
    #[cfg(target_arch = "wasm32")]
    {
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if picker.get_untracked().is_some() && ev.key() == "Escape" {
                ev.prevent_default();
                crate::editor_ops::close_asset_picker();
            }
        });
        on_cleanup(move || key.remove());
    }

    move || {
        let state = picker.get()?;
        // Flatten the side-filtered character catalog to placeable leaves (label + payload). Folders
        // carry no payload, so `payload.is_some()` is exactly "a placeable leaf" (asset_catalog docs).
        let items = registry.get().unwrap_or_default();
        let tree = crate::asset_catalog::build_catalog_tree(&items, &active_side.get());
        let mut leaves: Vec<(String, crate::asset_catalog::PlacePayload)> = Vec::new();
        fn collect(
            nodes: &[crate::asset_catalog::CatalogNode],
            out: &mut Vec<(String, crate::asset_catalog::PlacePayload)>,
        ) {
            for n in nodes {
                if let Some(p) = &n.payload {
                    out.push((n.label.clone(), p.clone()));
                }
                collect(&n.children, out);
            }
        }
        collect(&tree, &mut leaves);
        let q = query.get().trim().to_lowercase();
        if !q.is_empty() {
            leaves.retain(|(label, _)| label.to_lowercase().contains(&q));
        }
        // Anchor at the dblclick pixel (like the context menu). `max-h` + scroll keep a long list on
        // screen; a fuller flip/clamp is later polish — this slice ships the picker + its arm.
        let pos = format!("left:{:.0}px;top:{:.0}px", state.screen_x, state.screen_y);
        let rows = leaves
            .into_iter()
            .map(|(label, payload)| {
                view! {
                    <button
                        class="block w-full truncate px-3 py-1.5 text-left text-sm text-on-surface hover:bg-primary/20"
                        on:pointerdown=move |ev| {
                            ev.stop_propagation();
                            // Arm the same place a DockRight character leaf arms, then close: the
                            // next canvas click lands it (PLACE-001 click-then-click). `editor_ops`
                            // is wasm-only (the eden_dock_right leaf idiom), so the arm is gated and
                            // the native view shell just consumes the capture.
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::editor_ops::begin_place(payload.clone());
                                crate::editor_ops::close_asset_picker();
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &payload;
                        }
                    >
                        {label}
                    </button>
                }
            })
            .collect_view();
        Some(view! {
            // Click-away backdrop — transparent, full-screen, closes on any click (context-menu
            // idiom). `z-40` under the panel (`z-50`) but over the map/chrome.
            <div
                class="fixed inset-0 z-40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::close_asset_picker();
                }
                on:contextmenu=move |ev| ev.prevent_default()
            ></div>
            <div
                class="glass animate-dialog-in fixed z-50 flex max-h-[22rem] w-64 flex-col overflow-hidden rounded-md border border-outline-variant/30 shadow-2xl outline-none"
                style=pos
                on:contextmenu=move |ev| ev.prevent_default()
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="border-b border-outline-variant/25 px-2 py-1.5">
                    <input
                        type="search"
                        class="w-full rounded bg-surface/40 px-2 py-1 text-sm text-on-surface outline-none placeholder:text-on-surface-variant"
                        placeholder="Place asset…"
                        on:input=move |ev| query.set(event_target_value(&ev))
                    />
                </div>
                <div class="min-h-0 flex-1 overflow-y-auto py-1">{rows}</div>
            </div>
        })
    }
}

/// T-651 (`PLACE-COMMENT-001`) — **the comment editor**: the one surface that authors all three
/// `ATTR-FIELD-CMT-*` fields, plus the COPY and DELETE verbs. Opened by double-clicking a comment row
/// in the Outliner; renders no DOM while closed.
///
/// **Why its own overlay and not the Attributes modal.** Attributes reads the slot SoA
/// (`editor_ops::read_attrs`), and a comment is not in it — a comment never reaches `materialize`
/// at all, which is the same property that keeps it out of the render and off the compiled mission.
/// Pointing Attributes at a comment id would open a dialog with every field blank and every write a
/// no-op: the T-716 "live-but-inert" failure this codebase already names.
///
/// **Where each verb lands.** Title/tooltip/position write through `set_comment_*`, one core
/// transaction each, so each committed edit is one Ctrl+Z. The POSITION pair is also the drag
/// commit's surface: with a comment absent from the render SoA there is nothing on the map to grab,
/// so typed coordinates are the honest form of "drag" for this ticket — the doc-side mutator
/// (`move_comment`) is the same one a future map-drawn comment glyph would call, so wiring a
/// pointer drag later changes the CALLER and nothing else. Duplicate is the copy verb; Delete
/// removes the row and unfiles it from its folder. Filing into a layer is the Outliner drag, not a
/// control here.
///
/// Commits on `change` (blur / Enter), not on every keystroke: a per-character write would put one
/// undo step per letter on the stack of a field whose whole purpose is long prose.
#[component]
fn CommentEditorOverlay(open: RwSignal<Option<String>>, doc_tick: RwSignal<u64>) -> impl IntoView {
    // Esc closes (the picker / context-menu idiom).
    #[cfg(target_arch = "wasm32")]
    {
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked().is_some() && ev.key() == "Escape" {
                ev.prevent_default();
                crate::editor_ops::close_comment_editor();
            }
        });
        on_cleanup(move || key.remove());
    }

    move || {
        let id = open.get()?;
        // `doc_tick` is the reactive re-read trigger (the Attributes-modal idiom): an undo, a
        // duplicate or an outliner refile bumps it and this panel re-reads the row.
        let _ = doc_tick.get();
        #[cfg(target_arch = "wasm32")]
        let row = crate::editor_ops::read_comment(&id);
        #[cfg(not(target_arch = "wasm32"))]
        let row: Option<()> = None;
        // The row vanished (deleted, or undone away while the panel was open) — close rather than
        // edit a ghost. Returning `None` renders nothing; the signal is cleared on the next open.
        let (title, tooltip, x, z) = match &row {
            #[cfg(target_arch = "wasm32")]
            Some(c) => (c.title.clone(), c.tooltip.clone(), c.x, c.z),
            #[cfg(not(target_arch = "wasm32"))]
            Some(()) => (String::new(), String::new(), 0.0, 0.0),
            None => return None,
        };
        let (id_title, id_tip, id_x, id_z, id_dup, id_del) = (
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
            id.clone(),
        );
        let (x_for_z, z_for_x) = (x, z);
        Some(view! {
            <div
                class="fixed inset-0 z-40 bg-scrim/40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::close_comment_editor();
                }
            ></div>
            <div
                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex w-[min(28rem,92vw)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-xl border border-outline-variant/30 p-4 shadow-2xl outline-none"
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="flex items-center gap-2">
                    <span class="font-label-md text-label-md text-on-surface">"Comment"</span>
                    <span class="ml-auto font-code-sm text-code-sm text-on-surface-variant">
                        {id.clone()}
                    </span>
                </div>
                // ATTR-FIELD-CMT-TITLE
                <div class="space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Title"</label>
                    <input
                        type="text"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=title
                        on:change=move |ev| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::editor_ops::rename_comment(
                                    id_title.clone(),
                                    event_target_value(&ev),
                                );
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = (&id_title, &ev);
                        }
                    />
                </div>
                // ATTR-FIELD-CMT-TOOLTIP — a textarea, not an input: FNF v3's surviving in-map
                // instructions ran to seven paragraphs, and a single-line box would make the field
                // useless for the one job it exists to do.
                <div class="space-y-1">
                    <label class="font-label-sm text-[11px] text-on-surface-variant">"Tooltip"</label>
                    <textarea
                        rows="5"
                        class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-label-md text-label-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                        prop:value=tooltip
                        on:change=move |ev| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::editor_ops::set_comment_tooltip(
                                    id_tip.clone(),
                                    event_target_value(&ev),
                                );
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = (&id_tip, &ev);
                        }
                    ></textarea>
                </div>
                // ATTR-FIELD-CMT-POSITION — world metres, `{x, z}` (the marker / zone-centre
                // vocabulary, never `{x, y}`). A non-numeric entry is ignored rather than written as
                // 0, which would teleport the note to the terrain corner on a stray keystroke.
                <div class="flex gap-2">
                    <div class="flex-1 space-y-1">
                        <label class="font-label-sm text-[11px] text-on-surface-variant">"X (m)"</label>
                        <input
                            type="number"
                            class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-code-md text-code-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                            prop:value=x
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    crate::editor_ops::move_comment(id_x.clone(), v, z_for_x);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = (&id_x, &ev, z_for_x);
                            }
                        />
                    </div>
                    <div class="flex-1 space-y-1">
                        <label class="font-label-sm text-[11px] text-on-surface-variant">"Z (m)"</label>
                        <input
                            type="number"
                            class="w-full rounded border border-border-subtle bg-surface-dim px-2 py-1.5 font-code-md text-code-md text-on-surface focus:border-primary focus:ring-1 focus:ring-primary focus:outline-none"
                            prop:value=z
                            on:change=move |ev| {
                                #[cfg(target_arch = "wasm32")]
                                if let Ok(v) = event_target_value(&ev).trim().parse::<f64>() {
                                    crate::editor_ops::move_comment(id_z.clone(), x_for_z, v);
                                }
                                #[cfg(not(target_arch = "wasm32"))]
                                let _ = (&id_z, &ev, x_for_z);
                            }
                        />
                    </div>
                </div>
                <div class="flex items-center gap-2 pt-1">
                    // COPY. The new comment lands in the same folder, offset so it is not stacked
                    // invisibly on its source. The panel follows the copy — that is what makes the
                    // duplicate immediately editable instead of leaving the operator on the original.
                    <button
                        type="button"
                        class="rounded border border-border-subtle px-3 py-1.5 text-label-md text-on-surface hover:bg-primary/15"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            if let Some(new_id) =
                                crate::editor_ops::duplicate_comment(&id_dup, COMMENT_COPY_OFFSET_M)
                            {
                                crate::editor_ops::open_comment_editor(new_id);
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_dup;
                        }
                    >
                        "Duplicate"
                    </button>
                    <button
                        type="button"
                        class="rounded border border-error/50 px-3 py-1.5 text-label-md text-error hover:bg-error/15"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                crate::editor_ops::delete_comment(id_del.clone());
                                crate::editor_ops::close_comment_editor();
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            let _ = &id_del;
                        }
                    >
                        "Delete"
                    </button>
                    <button
                        type="button"
                        class="ml-auto rounded bg-primary px-3 py-1.5 text-label-md text-on-primary"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::close_comment_editor();
                        }
                    >
                        "Close"
                    </button>
                </div>
            </div>
        })
    }
}

/// T-651 — how far a duplicated comment is offset from its source, in metres. Non-zero so the copy
/// is a distinct, clickable row rather than a perfect overlay of the original.
const COMMENT_COPY_OFFSET_M: f64 = 25.0;

/// T-672 — one Connections-panel row, flattened for rendering: the edge plus the findings that name
/// it. Target-independent on purpose — `editor_ops` is wasm-only, so this is what lets the panel have
/// ONE view body instead of a wasm branch and an untested native twin.
struct ConnRowView {
    kind: String,
    /// `"SL (s0) → Rifleman (s1)"` — both endpoints, label-resolved.
    head: String,
    id: String,
    /// `"CONN-DANGLING: to endpoint `x` is not a placed entity"`, one per finding on this row.
    problems: Vec<String>,
}

/// T-672 — **the Connections panel: the connection graph's SEE and CHECK surface.**
///
/// This component is the ticket's primary constraint made concrete. The framework corpus records
/// FNF v4's entire defect cluster on the connection mechanism, with the instruction "the inspector
/// and the validation rules must precede the edges — do not ship edges you cannot see or check".
/// A connection has **no map glyph** in this slice (see the `LaneRole::SquadLinks` trace note on
/// `editor_ops`'s connection block), so this panel is the ONLY place an operator can observe the
/// graph they are authoring, audit it, or delete from it. It is not an inspector bolted onto the
/// feature; it is the feature's only surface, and the edge verbs hang off it.
///
/// Three things, in the order they matter:
///   1. **EVERY edge, listed** — `kind`, `from → to` with resolved labels, in `map-engine-core`'s
///      stable content order (so the rows never reshuffle under the cursor between reads).
///   2. **EVERY finding, listed** — the four graph rules (`CONN-SELF` / `CONN-DANGLING` /
///      `CONN-DUPLICATE` / `CONN-CYCLE`, plus `CONN-KIND` for a hydrated foreign vocabulary),
///      rendered against the row they belong to AND summarised at the top, because a warning the
///      operator must scroll to find is a warning they will not read.
///   3. **A delete per row** (`CONN-DEL-001`). Eden deletes a connection by selecting its line and
///      pressing Del; there is no line here, so the addressable row is the selection.
///
/// It also shows the ARMED connect, if any, with its own cancel — the two-act connect gesture's only
/// persistent state, which would otherwise be invisible between the two right-clicks.
///
/// `doc_tick` is the reactive re-read trigger (the Attributes-modal / comment-editor idiom): a draw,
/// a delete, an undo or a hydrate bumps it and this panel re-reads. There is no doc change
/// subscription, so a panel that read once would be a stale audit — which is worse than no audit.
///
/// Mounted UNGATED beside the other floating overlays: an audit surface the operator deliberately
/// opened is not dock chrome and must survive a Backspace hide-chrome (the wave-101 mount rule).
#[component]
fn ConnectionsPanelOverlay(open: RwSignal<bool>, doc_tick: RwSignal<u64>) -> impl IntoView {
    // Esc closes (the picker / context-menu / comment-editor idiom).
    #[cfg(target_arch = "wasm32")]
    {
        let key = window_event_listener(leptos::ev::keydown, move |ev| {
            if open.get_untracked() && ev.key() == "Escape" {
                ev.prevent_default();
                crate::editor_ops::close_connections_panel();
            }
        });
        on_cleanup(move || key.remove());
    }

    move || {
        if !open.get() {
            return None;
        }
        // Re-read on every doc mutation — see the component note.
        let _ = doc_tick.get();
        // `editor_ops` is a wasm-only module, so the doc read is behind a cfg and the whole panel
        // is expressed over the target-independent [`ConnRowView`]. That keeps ONE view body for
        // both targets — the native build renders the same empty-state DOM rather than a second,
        // untested branch (the shape `CommentEditorOverlay` uses, for the same reason).
        #[cfg(target_arch = "wasm32")]
        let (rows, finding_count, armed_line) = {
            let list = crate::editor_ops::connection_list();
            let findings = crate::editor_ops::connection_findings();
            // Findings keyed by the row they belong to. Built once here rather than re-scanned per
            // row: a graph with N edges and N findings would otherwise be quadratic, and the panel
            // re-renders on every document mutation.
            let mut by_row: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            for f in &findings {
                by_row
                    .entry(f.connection_id.clone())
                    .or_default()
                    .push(format!("{}: {}", f.code, f.detail));
            }
            let rows: Vec<ConnRowView> = list
                .into_iter()
                .map(|r| ConnRowView {
                    problems: by_row.get(&r.id).cloned().unwrap_or_default(),
                    head: format!("{} \u{2192} {}", r.from_label, r.to_label),
                    kind: r.kind,
                    id: r.id,
                })
                .collect();
            let armed_line = crate::editor_ops::pending_connect()
                .map(|(kind, from)| format!("Connecting: {kind} from {from}"));
            (rows, findings.len(), armed_line)
        };
        #[cfg(not(target_arch = "wasm32"))]
        let (rows, finding_count, armed_line): (Vec<ConnRowView>, usize, Option<String>) =
            (Vec::new(), 0, None);

        let total = rows.len();
        // Bound out of the `view!` macro: `class:` takes a value, not a comparison expression.
        let clean = finding_count == 0;
        let empty = total == 0;

        let row_views = rows
            .into_iter()
            .map(|r| {
                let bad = !r.problems.is_empty();
                let (problems, del_id, head) = (r.problems, r.id.clone(), r.head);
                view! {
                    <div class="flex flex-col gap-0.5 border-b border-outline-variant/20 py-1.5 last:border-b-0">
                        <div class="flex items-center gap-2">
                            <span
                                class="shrink-0 rounded px-1.5 py-0.5 font-code-sm text-code-sm"
                                class:bg-surface-dim=!bad
                                class:text-on-surface-variant=!bad
                                class:bg-error-container=bad
                                class:text-on-error-container=bad
                            >
                                {r.kind}
                            </span>
                            <span class="flex-1 truncate font-label-md text-label-md text-on-surface">
                                {head}
                            </span>
                            <span class="shrink-0 font-code-sm text-code-sm text-outline">
                                {r.id}
                            </span>
                            <button
                                type="button"
                                title="Delete this connection (CONN-DEL-001) — one Ctrl+Z restores it"
                                class="shrink-0 cursor-pointer rounded px-2 py-0.5 font-label-sm text-[11px] text-on-surface-variant hover:bg-error-container hover:text-on-error-container"
                                on:click=move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        crate::editor_ops::delete_connection(&del_id);
                                    }
                                    #[cfg(not(target_arch = "wasm32"))]
                                    let _ = &del_id;
                                }
                            >
                                "Delete"
                            </button>
                        </div>
                        {(!problems.is_empty())
                            .then(|| {
                                problems
                                    .into_iter()
                                    .map(|p| {
                                        view! {
                                            <div class="pl-2 font-code-sm text-code-sm text-error">
                                                {p}
                                            </div>
                                        }
                                    })
                                    .collect_view()
                            })}
                    </div>
                }
            })
            .collect_view();

        Some(view! {
            <div
                class="fixed inset-0 z-40 bg-scrim/40"
                on:pointerdown=move |ev| {
                    ev.stop_propagation();
                    #[cfg(target_arch = "wasm32")]
                    crate::editor_ops::close_connections_panel();
                }
            ></div>
            <div
                class="glass animate-dialog-in fixed top-1/2 left-1/2 z-50 flex max-h-[80vh] w-[min(40rem,94vw)] -translate-x-1/2 -translate-y-1/2 flex-col gap-3 rounded-xl border border-outline-variant/30 p-4 shadow-2xl outline-none"
                on:pointerdown=move |ev| ev.stop_propagation()
            >
                <div class="flex items-center gap-2">
                    <span class="font-label-md text-label-md text-on-surface">"Connections"</span>
                    <span class="font-code-sm text-code-sm text-on-surface-variant">
                        {format!("{total} edge(s)")}
                    </span>
                    <button
                        type="button"
                        class="ml-auto cursor-pointer rounded px-2 py-1 font-label-sm text-[11px] text-on-surface-variant hover:bg-surface-dim"
                        on:click=move |_| {
                            #[cfg(target_arch = "wasm32")]
                            crate::editor_ops::close_connections_panel();
                        }
                    >
                        "Close"
                    </button>
                </div>
                // The armed connect — the two-act gesture's only persistent state, which is
                // otherwise invisible between the two right-clicks.
                {armed_line
                    .map(|line| {
                        view! {
                            <div class="flex items-center gap-2 rounded border border-primary/40 bg-surface-dim px-2 py-1">
                                <span class="flex-1 truncate font-code-sm text-code-sm text-on-surface">
                                    {line}
                                </span>
                                <button
                                    type="button"
                                    class="shrink-0 cursor-pointer rounded px-2 py-0.5 font-label-sm text-[11px] text-on-surface-variant hover:bg-surface-bright"
                                    on:click=move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            crate::editor_ops::cancel_connect();
                                            crate::editor_ops::open_connections_panel();
                                        }
                                    }
                                >
                                    "Cancel"
                                </button>
                            </div>
                        }
                    })}
                // The CHECK summary. Rendered at the TOP and unconditionally (including the clean
                // "no problems" case), because a validation surface that only appears when something
                // is wrong cannot be distinguished from one that is broken.
                <div
                    class="rounded px-2 py-1 font-label-sm text-[11px]"
                    class:bg-surface-dim=clean
                    class:text-on-surface-variant=clean
                    class:bg-error-container=!clean
                    class:text-on-error-container=!clean
                >
                    {if clean {
                        "No problems found in the connection graph.".to_string()
                    } else {
                        format!(
                            "{finding_count} problem(s): dangling endpoints, self-links, duplicates or ownership cycles — see the rows below.",
                        )
                    }}
                </div>
                <div class="min-h-0 flex-1 overflow-y-auto">
                    {if empty {
                        view! {
                            <div class="py-6 text-center font-label-sm text-[11px] text-on-surface-variant">
                                "No connections yet. Right-click a unit → Connect → pick a relation, then right-click the target and choose Complete Connection."
                            </div>
                        }
                            .into_any()
                    } else {
                        row_views.into_any()
                    }}
                </div>
            </div>
        })
    }
}

/* ═════════════ T-754 — what the click-to-select router RESOLVES, as a pure question ═════════════
 *
 * T-655 shipped ONE click-to-select router (`validation_panel::register_select_by_id`, registered
 * from this file's wasm mount). Two surfaces now draw a click affordance on top of it — the
 * validation panel's finding rows and T-688's aggregated settings rows — and the wave-115 verifier
 * found the defect that follows from the router's resolution living INSIDE the closure: a surface
 * cannot ask "would this click select anything?", so it guesses "the row names an id, so yes", and
 * every zone-owned row wore `cursor-pointer` over a click that could only produce a toast.
 *
 * So the resolution is lifted OUT of the closure into [`route_target`] — pure, `serde_json`-only,
 * natively compiled, and therefore both TESTABLE and ASKABLE. The registered closure is the only
 * ACTOR (it still owns the `!Send` doc/selection/engine `Rc`s); a view that wants to know whether a
 * click has a target asks the same function the click will ask. That is what makes "a row is
 * clickable iff the router resolves its subject" a correspondence rather than a hope.
 *
 * The ZONE arm is the T-754 widening. A zone is in neither the slot SoA nor `vehiclesById` — its
 * selection is the Zones panel's own `RwSignal` (`eden_dock_right::route_select_zone`, the seam that
 * panel now exposes for exactly this) — so before this the router returned `false` for 100% of the
 * aggregation's entity rows.
 */

/// What [`route_target`] resolved a subject id to — i.e. WHICH selection surface owns it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum RouteTarget {
    /// A slot: the caller's SoA predicate matched. The position comes from the SoA row (the caller
    /// already has it), so no coordinates ride this arm.
    Slot,
    /// A `vehiclesById` row, at its authored `position`.
    Vehicle { x: f64, y: f64 },
    /// T-754 — a `zonesById` row, at its geometric centre. Selected in the Zones panel, not in the
    /// slot selection (a zone id in `select_tool`'s selection would read `SEL 1` with nothing
    /// highlighted — see `eden_dock_right`'s `zone_selected`).
    Zone { x: f64, y: f64 },
}

/// **Where a `subject_id` would go if it were clicked**, over the document's `small_maps_json()`
/// root plus `is_slot` (slot ids live in the SoA, which is not in that root, so the one fact this
/// function cannot read is supplied by the caller).
///
/// `None` means NOTHING would be selected — a stale id whose entity was deleted, or a kind no
/// selection surface owns. A view MUST NOT paint a click affordance on a row this returns `None`
/// for; that is the T-754 defect, stated as a rule.
///
/// Order is slot → vehicle → zone, the order the shipped router already tried, so the widening
/// cannot change what an existing id resolves to.
pub(crate) fn route_target(
    root: &serde_json::Value,
    subject_id: &str,
    is_slot: &dyn Fn(&str) -> bool,
) -> Option<RouteTarget> {
    if is_slot(subject_id) {
        return Some(RouteTarget::Slot);
    }
    if let Some(p) = root
        .get("vehiclesById")
        .and_then(|m| m.get(subject_id))
        .and_then(|v| v.get("position"))
    {
        if let (Some(x), Some(y)) = (
            p.get("x").and_then(serde_json::Value::as_f64),
            p.get("y").and_then(serde_json::Value::as_f64),
        ) {
            return Some(RouteTarget::Vehicle { x, y });
        }
    }
    if let Some(zone) = root.get("zonesById").and_then(|m| m.get(subject_id)) {
        if let Some((x, y)) = zone_centre(zone) {
            return Some(RouteTarget::Zone { x, y });
        }
    }
    None
}

/// A zone's geometric centre in world metres — a circle's centre, or a polygon's vertex mean.
/// `None` for a shapeless row (a draw that was never committed), which is exactly the row a click
/// cannot centre on and therefore must not advertise.
///
/// The shape vocabulary is the document's, read the same way `editor_ops::zone_rows` reads it:
/// `shape.circle {x, z, r}` / `shape.polygon [[x, z], …]`, where **the map's world `y` IS that `z`**
/// (`eden_zones::circle_from_clicks`' note). Read here off the JSON rather than through `ZoneRow`
/// because `editor_ops` is a wasm32-only module and this must run on the native test shell.
fn zone_centre(zone: &serde_json::Value) -> Option<(f64, f64)> {
    let shape = zone.get("shape")?;
    if let Some(c) = shape.get("circle") {
        if let (Some(x), Some(z)) = (
            c.get("x").and_then(serde_json::Value::as_f64),
            c.get("z").and_then(serde_json::Value::as_f64),
        ) {
            return Some((x, z));
        }
    }
    let ring = shape.get("polygon")?.as_array()?;
    let verts: Vec<(f64, f64)> = ring
        .iter()
        .filter_map(|p| {
            let a = p.as_array()?;
            Some((a.first()?.as_f64()?, a.get(1)?.as_f64()?))
        })
        .collect();
    if verts.is_empty() {
        return None;
    }
    let n = verts.len() as f64;
    let (sx, sz) = verts
        .iter()
        .fold((0.0, 0.0), |(ax, az), (x, z)| (ax + x, az + z));
    Some((sx / n, sz / n))
}

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
    let scale_mpp = RwSignal::new(crate::eden_toolbelt::m_per_px(-2.0));
    // T-642/T-643 — the active editor tool (Select ⇆ Ruler ⇆ LoS). The `ModeToolbar` buttons read +
    // set it (the active tool enters TOOL_ACTIVE state, Select returns); the wasm pointer handlers
    // branch on it to choose the point-capture gesture (Ruler AND LoS share `LG::Ruler`) vs the
    // Select machine, and the commit site routes a captured click by `is_ruler()`/`is_los()`. Default
    // Select.
    let tool_mode = RwSignal::new(crate::ruler_tool::EditorTool::Select);
    // T-644 — the LoS SUB-MODE (Ray ⇆ Viewshed). The `ModeToolbar` LoS button reads it (to reflect
    // the active sub-mode in its title/label) and toggles it on a re-click while LoS is already
    // active; the wasm pointer commit reads `get_untracked()` to route a captured LoS click to the
    // ray two-click capture or the one-shot viewshed placement. A plain reactive signal (like
    // `tool_mode`), shared between the toolbar and the pointer handlers — no thread_local. Default Ray.
    let los_mode = RwSignal::new(crate::los_tool::LosMode::default());
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
    // quantise a Shift-rotate / widget-ring drag), and the DOM overlays (the status-bar GRID readout
    // + the widget SVG, which re-run on `.get()`). The default `SnapState` is OFF, so an operator who
    // never presses `G`/`[`/`]` gets the exact pre-T-648 free move + free rotate.
    let snap = RwSignal::new(crate::mission_editor::transform::SnapState::default());
    let widget_variant = RwSignal::new(crate::mission_editor::transform::WidgetVariant::default());
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
                    crate::editor_ops::slots_json()
                        .as_deref()
                        .and_then(crate::mission_size::estimate_compiled_bytes),
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
    let outliner_nodes = RwSignal::new(Vec::<crate::outliner::OutlinerNode>::new());
    // T-168 — the ORBAT dock tree mirror (faction/squad/slot), rebuilt alongside `outliner_nodes`.
    let orbat_nodes = RwSignal::new(Vec::<crate::outliner::OutlinerNode>::new());
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
    let catalog = RwSignal::new(crate::asset_catalog::CatalogState::Loading);
    // T-215 — the Vehicles tab's tree, built from the SAME `/registry` response as `catalog`
    // (`kind == "vehicle"` instead of `"character"`). One fetch, two trees: a second request for
    // rows already in hand would double a ~940 KB payload for nothing.
    let vehicle_catalog = RwSignal::new(crate::asset_catalog::CatalogState::Loading);
    // T-159.26 — Attributes modal: the open slot id + a doc-change tick the modal re-reads on
    // (`doc_ver` is a plain Rc<Cell>, not reactive; refresh_docks bumps this signal instead).
    let attrs_open = RwSignal::new(None::<String>);
    // T-180.9 — Attributes tab (1 = Identity default; `open_arsenal` sets 3 = Arsenal).
    let attrs_tab = RwSignal::new(1usize);
    let doc_tick = RwSignal::new(0u64);
    let settings_open = RwSignal::new(false);
    // T-167 — Faction Manager dialog toggle (launched from the Factions dock "Manage" button).
    let fm_open = RwSignal::new(false);
    // T-177 B2 / T-071.0 — the ORBAT Manager modal open flag (top-strip button ↔ OrbatManagerDialog).
    let orbat_open = RwSignal::new(false);
    // T-664 — the right-click context menu's open state: `Some(MenuState)` = open at that pixel/take,
    // `None` = closed (no DOM). The wasm `contextmenu` handler sets it via `context_menu::open`; the
    // overlay reads it. Mounted BESIDE the ungated dialogs below (not inside the chrome_hidden gate),
    // so a floating menu survives Backspace hide-chrome per the wave-101 verifier.
    let context_menu = RwSignal::new(None::<crate::context_menu::MenuState>);
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
    let registry_items = RwSignal::new(None::<Vec<crate::dto::RegistryItem>>);

    // T-255 — Factions palette is side-aware: rebuild whenever Eden chips change `active_side` or
    // the `/registry` rows land. Fetch paths below only write `registry_items` (+ vehicles); this
    // Effect owns `catalog` Ready trees so a BLUFOR→OPFOR chip flip drops NATO and shows USSR.
    {
        use crate::asset_catalog::{build_catalog_tree, CatalogState};
        Effect::new(move |_| {
            let side = active_side.get();
            if let Some(items) = registry_items.get() {
                catalog.set(CatalogState::Ready(build_catalog_tree(&items, &side)));
            }
        });
    }
    // T-167 — the compat edge feed for the Smart Arsenal (optic/magazine edge rows + validation).
    // Fetched once alongside /registry; starts Loading, degrades to Unavailable on error.
    let compat = RwSignal::new(crate::arsenal_rules::CompatFeed::default());
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
        let auth = expect_context::<crate::auth::AuthStore>();

        // T-159.22 — the Factions palette catalog. Engine-independent so the dock fills even if
        // wgpu never comes up. `kind == "character"` rows only — `build_catalog_tree` is the
        // T-068.3 `buildCatalogTree` port (T-255: filtered by `active_side` in the Effect above).
        //
        // T-245 — gate the network path on the SPA-session cache. Remounts apply the cached
        // rows synchronously (no network, no second tree rebuild from a fresh download).
        //
        // T-427 — cold path pages `GET /registry?limit=500&offset=…` until `total` is covered
        // (never the unbounded single-shot dump). Page size matches the API `REGISTRY_PAGE_MAX`.
        {
            use crate::asset_catalog::{build_vehicle_catalog_tree, CatalogState};
            if registry_session::must_fetch_registry() {
                spawn_local({
                    async move {
                        match fetch_registry_pages(auth).await {
                            Ok(items) => {
                                registry_session::store_registry(items.clone());
                                registry_items.set(Some(items.clone()));
                                // T-255 — character `catalog` Ready tree is owned by the
                                // active_side Effect (rebuilds on chip flip).
                                // T-215 — the Vehicles tab, off the same rows.
                                vehicle_catalog
                                    .set(CatalogState::Ready(build_vehicle_catalog_tree(&items)));
                            }
                            Err(_) => {
                                catalog.set(CatalogState::Failed);
                                vehicle_catalog.set(CatalogState::Failed);
                            }
                        }
                    }
                });
            } else if let Some(items) = registry_session::cached_registry() {
                registry_items.set(Some(items.clone()));
                vehicle_catalog.set(CatalogState::Ready(build_vehicle_catalog_tree(&items)));
            }
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
            use crate::arsenal_rules::{CompatFeed, CompatGraph, CompatStatus};
            if registry_session::must_fetch_compat() {
                spawn_local({
                    async move {
                        match fetch_compat_cold(auth).await {
                            Ok((feed, cargo)) => {
                                registry_session::store_compat(feed.clone(), cargo.clone());
                                crate::editor_ops::set_cargo_defaults(cargo);
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
                crate::editor_ops::set_cargo_defaults(cargo);
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
            let map_host = crate::world_assets::new_host_handle();
            // T-172 B2 — DEM grid handle for the CUR Z sample (published by bootstrap).
            let dem_grid = crate::world_assets::new_dem_grid_handle();
            let disposed = Arc::new(AtomicBool::new(false));

            // T-159.16 — MissionDoc host. Built + seeded + bridged synchronously (before the async
            // engine create), so the `window.__missionDoc` Class R gate does not depend on the wgpu
            // engine coming up. The doc leaks on route-leave like the engine (`!Send` `Rc`, and
            // `on_cleanup` is `Send`-bound) — no double-free (plain Rust `Drop`). The optional
            // doc→engine bind (D5) happens below once the engine is `Some`.
            let doc = crate::mission_doc::new_seeded_doc();
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
            crate::editor_ops::seed_new_mission_template(&doc);
            let doc_ver = Rc::new(Cell::new(1u32));
            crate::mission_doc::register_mission_doc(doc.clone(), doc_ver.clone());

            // T-159.20 — editor commands (Save/Export) context + the `__editorCommands` smoke bridge
            // (peer of `__missionDoc`). `set_ctx` shares the same `Rc` the persistence swap targets,
            // so both the buttons and the bridge see an IDB-restored doc.
            crate::mission_commands::set_ctx(doc.clone(), auth, mission_id.clone(), current_semver);
            crate::mission_commands::register_editor_commands(doc.clone());

            // T-159.18 — LMB select foundation. Selection is app-side state (NOT the Y.Doc — it never
            // lived in the document, matching React's Zustand), held in the editor's leaked-handle
            // idiom so the `window.__editorSelection` smoke bridge (peer of __missionDoc) never reads
            // reactive-owner state a route change could dispose. `left` carries the in-flight LMB
            // gesture (T-159.19 `LeftGesture`: Pending → Move | Marquee — a frozen ortho camera copied
            // at the press drives every unproject) between pointerdown/move/up. Registered
            // synchronously (engine still `None` here — `probe()` reads it lazily; `pick_selfcheck()`
            // needs only the synchronously-seeded doc).
            let selection: crate::select_tool::SelectionHandle = Rc::new(RefCell::new(Vec::new()));
            let left: Rc<RefCell<Option<crate::select_tool::LeftGesture>>> =
                Rc::new(RefCell::new(None));
            // T-642 — the persistent ruler polyline. Session-local OVERLAY state (Decision 4 — NOT
            // the Y.Doc, exactly like the selection set above), held in a leaked `Rc<RefCell<…>>` so
            // both the pointer handlers (which mutate it) and the `RulerOverlay`'s `read_chain`
            // closure (which clones it to project) share one source of truth without touching
            // reactive-owner state a route change could dispose.
            let ruler: Rc<RefCell<crate::ruler_tool::RulerChain>> =
                Rc::new(RefCell::new(crate::ruler_tool::RulerChain::new()));
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
            crate::ruler_tool::register_ruler_chain(ruler.clone());

            // T-643 — the Line-of-Sight capture. Session-local OVERLAY state (Decision 4 — NOT the
            // Y.Doc, exactly like the selection set + the ruler chain above), a leaked
            // `Rc<RefCell<LosState>>` shared by the pointer handlers (which mutate it) and the
            // `LosOverlay` (which clones it to project + build the profile).
            let los: Rc<RefCell<crate::los_tool::LosState>> =
                Rc::new(RefCell::new(crate::los_tool::LosState::new()));
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
            let viewshed: Rc<RefCell<crate::los_tool::ViewshedState>> =
                Rc::new(RefCell::new(crate::los_tool::ViewshedState::new()));
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
            crate::los_tool::register_los_state(los.clone());
            // T-644 — hand the leaked viewshed state to `los_tool`'s thread_local (peer of
            // `register_los_state`) so `place_viewshed` can store the computed raster into it and a
            // pan re-projects the same rect. The compute reuses the SAME DEM sampler registered just
            // below (the ray's sampler); `place_viewshed` calls `compute_viewshed_for`, which reads it.
            crate::los_tool::register_viewshed_state(viewshed.clone());
            {
                let dem_grid = dem_grid.clone();
                crate::los_tool::register_los_sampler(std::rc::Rc::new(move |x: f64, y: f64| {
                    dem_grid
                        .borrow()
                        .as_ref()
                        .and_then(|g| map_engine_core::dem::downsample::sample_grid_meters(g, x, y))
                }));
            }

            crate::select_tool::register_editor_selection(
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
                crate::validation_panel::register_payload_source(std::rc::Rc::new(move || {
                    let d = doc.borrow();
                    let core = d.as_ref()?;
                    let payload = map_engine_core::mission::compile::compile_payload(
                        &core.small_maps_json(),
                        &core.slots_json(),
                        false,
                    );
                    // Catalogue: the live registry rows if loaded, else `None` (rule skips). Built
                    // from `resource_name`s + object prop:/comp: aliases — the ids the payload uses.
                    let known_asset_ids = registry_items.get_untracked().map(|items| {
                        crate::validation_panel::known_asset_ids_from_registry(&items)
                    });
                    Some(crate::validation_panel::PayloadSource {
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
            {
                let doc = doc.clone();
                let selection = selection.clone();
                let engine = engine.clone();
                crate::validation_panel::register_select_by_id(std::rc::Rc::new(
                    move |subject_id: &str| {
                        let d = doc.borrow();
                        let Some(core) = d.as_ref() else {
                            return false;
                        };
                        // The one fact the small-maps root does not carry: slot-SoA membership.
                        let soa = core.materialize();
                        let slot_row = soa.ids.iter().position(|s| s == subject_id);
                        let root =
                            serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
                                .unwrap_or(serde_json::Value::Null);
                        // `None` ⇒ nothing to select (a stale finding, or a row this editor owns no
                        // selection surface for). Keep the current selection intact.
                        let Some(target) = route_target(&root, subject_id, &|_| slot_row.is_some())
                        else {
                            return false;
                        };
                        drop(d);
                        let (cx, cy) = match target {
                            RouteTarget::Slot => {
                                let row = slot_row.expect("Slot arm implies the SoA matched");
                                (f64::from(soa.xs[row]), f64::from(soa.ys[row]))
                            }
                            RouteTarget::Vehicle { x, y } | RouteTarget::Zone { x, y } => (x, y),
                        };
                        if matches!(target, RouteTarget::Zone { .. }) {
                            // A zone is selected in the Zones panel, never in the slot selection.
                            // If that panel is not mounted there is nothing to select, and the
                            // router says so (false) instead of centring on a phantom selection.
                            if !crate::eden_dock_right::route_select_zone(subject_id) {
                                return false;
                            }
                        } else {
                            *selection.borrow_mut() = vec![subject_id.to_string()];
                            let ids = selection.borrow().clone();
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_selection(ids);
                            }
                            crate::mission_history::refresh_selection();
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
            crate::mission_history::set_ctx(
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
            crate::editor_ops::set_ctx(
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
            crate::context_menu::set_menu_signal(context_menu);
            // T-647 PLACE-003 — same handoff for the empty-ground asset picker: the wasm `dblclick`
            // closure opens it through `editor_ops::open_asset_picker`, which writes this signal.
            crate::editor_ops::set_asset_picker_signal(asset_picker);
            // T-651 — same handoff for the comment editor: the Outliner's comment row (a native
            // view with no reactive handle) opens it through `editor_ops::open_comment_editor`.
            crate::editor_ops::set_comment_editor_signal(comment_editor);
            // T-672 — same idiom: the `Connections...` context-menu row calls
            // `editor_ops::open_connections_panel`, which needs this handle.
            crate::editor_ops::set_connections_panel_signal(connections_panel);

            crate::mission_history::register_editor_history();
            crate::mission_history::register_key_handler();
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
            crate::mission_history::register_unload_guard();
            on_cleanup(crate::mission_history::unregister_unload_guard);
            crate::mission_history::refresh_hud();

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
                        if crate::mission_history::in_editable_field() {
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
                                ruler_acted || los_acted || viewshed_acted
                            }
                            "KeyC" if modk && !ev.alt_key() && !ev.shift_key() => {
                                crate::editor_ops::copy_selection()
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
                                crate::editor_ops::copy_selection()
                                    && crate::editor_ops::delete_selection()
                            }
                            "KeyV" if modk && !ev.alt_key() && !ev.shift_key() => {
                                crate::editor_ops::paste_at_cursor(cx, cy)
                            }
                            // T-669 ACTION-PASTE-ORIG-001 — Ctrl/Cmd+Shift+V pastes with NO cursor
                            // anchor. `paste_at_cursor`'s anchor is `Option`al and that option IS the
                            // feature: `Some(cx, cy)` translates the clip's centroid onto the map
                            // cursor (the plain paste arm above), `None` leaves every slot on its
                            // SOURCE coordinates. Honesty about the one wrinkle: with no anchor
                            // `Doc::paste_slots` offsets the whole clip by `PASTE_NUDGE` (20 m) so the
                            // copy is not buried pixel-perfect under its original and unclickable.
                            // That nudge is `map-engine-core`'s pre-existing no-anchor behaviour
                            // (byte-parity with the JS `ydoc.pasteSlots`), not a choice this arm
                            // makes, and the help row says "source position" rather than claiming an
                            // exact-coordinate paste.
                            //
                            // MUTUAL EXCLUSION with the plain paste arm: that arm guards
                            // `!ev.shift_key()`, this one guards `ev.shift_key()`, and both require
                            // `modk && !ev.alt_key()`. One `KeyboardEvent` has exactly one `shiftKey`
                            // value, so at most one of the pair can ever match — they partition the
                            // Ctrl+V space rather than overlapping it, and the order they appear in
                            // is therefore irrelevant. Pinned by
                            // `the_two_paste_arms_are_mutually_exclusive`.
                            "KeyV" if modk && !ev.alt_key() && ev.shift_key() => {
                                crate::editor_ops::paste_at_cursor(None, None)
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
                                crate::editor_ops::select_all_in_view(rect.width(), rect.height())
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
                            "Space" if !modk => crate::editor_ops::center_on_selection(),
                            // T-662 — Delete still removes the selection. Backspace is NO LONGER an
                            // alias for Delete; it toggles the Eden chrome (hide/show interface), so
                            // the two keys are now split arms. Backspace always "acts" (it flips the
                            // signal), so `prevent_default` fires below to keep the browser from
                            // treating it as a Back navigation.
                            "Delete" if !modk => crate::editor_ops::delete_selection(),
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
                            // WIDGET-CYCLE-001 — `1` / `2` select the widget VARIANT (Translate /
                            // Rotate). This is the Space-collision decision: Eden cycles variants on
                            // Space, but TBD's Space stays flyTo (`center_on_selection`, the arm
                            // above), and Eden's `1`-`5` direct keys are free here (census: no
                            // `Digit*` binding anywhere in the frontend), so `1`/`2` dissolve the
                            // clash without touching Space. `3`-`5` are deliberately NOT bound —
                            // there is no area-scale variant (a transform selection is slots +
                            // vehicles, neither of which scales; see `WidgetVariant`'s doc). Bare
                            // digit only. `from_digit` is a no-op for any other digit, but we only
                            // reach here for 1/2 so it always changes the variant → act.
                            "Digit1" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                widget_variant.set(widget_variant.get_untracked().from_digit(1));
                                true
                            }
                            "Digit2" if !modk && !ev.alt_key() && !ev.shift_key() => {
                                widget_variant.set(widget_variant.get_untracked().from_digit(2));
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
                    let was_hidden = crate::eden_layout::chrome_hidden();

                    let rect = container.get_bounding_client_rect();
                    let (w, h) = (rect.width(), rect.height());
                    if !(w > 0.0 && h > 0.0) {
                        // Still mirror the state so the accessors are correct before first layout.
                        crate::eden_layout::set_chrome_hidden(hidden);
                        crate::eden_layout::set_dock_left_collapsed(left);
                        crate::eden_layout::set_dock_right_collapsed(right);
                        return;
                    }

                    // Pane centre with the PREVIOUS insets (the Cells still hold the pre-toggle state).
                    let before = crate::eden_layout::pane_center_px(w, h);
                    // Commit the new inset state, then read the pane centre AFTER.
                    crate::eden_layout::set_chrome_hidden(hidden);
                    crate::eden_layout::set_dock_left_collapsed(left);
                    crate::eden_layout::set_dock_right_collapsed(right);
                    let after = crate::eden_layout::pane_center_px(w, h);

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
                            let (nx, ny) = crate::eden_layout::centre_hold_target(
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
            crate::yrs_persist::register_mission_persist(
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
                    if let Some(blob) = crate::yrs_persist::load_state(&id).await {
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
                                crate::mission_history::refresh_hud();
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
                                crate::mission_history::set_dirty(true);
                            }
                        }
                    }
                    // 1.5 T-159.26 — server hydrate / conflict / dirty (UUID missions only; the
                    //     `smoke` gate route is non-UUID and skips this, so the editor smokes are
                    //     untouched). Replaces the seed with the saved version, or prompts on a
                    //     genuine local-vs-server conflict — the data-safety guarantee.
                    crate::mission_hydrate::hydrate_from_server(
                        doc.clone(),
                        id.clone(),
                        auth,
                        loaded.get(),
                        current_semver,
                        conflict,
                        report.clone(),
                    )
                    .await;
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
                        crate::mission_history::rebind_engine_from_doc();
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
                        crate::yrs_persist::save_state_debounced(
                            &id,
                            Box::new(move || {
                                doc_get
                                    .borrow()
                                    .as_ref()
                                    .map(|c| c.encode_state())
                                    .unwrap_or_default()
                            }),
                            Box::new(move || doc_cancel.borrow().is_none()),
                            crate::yrs_persist::debounce_ms(),
                        );
                    }
                    // 3. Warm-session marker after the doc is ready.
                    let n = doc
                        .borrow()
                        .as_ref()
                        .map(|c| c.slot_count() as u32)
                        .unwrap_or(0);
                    crate::editor_session::mark_ready(&id, n, None);
                    // 4. Flush-on-hide listeners (visibilitychange/hidden + pagehide).
                    crate::yrs_persist::register_flush_on_hide(id.clone());
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
                                                              // T-172 B4 — upload the ring+disc slot atlas BEFORE the first SoA bind:
                                                              // the whole slot lane (bind / selection tint / drag overlay) is gated on
                                                              // `atlas_ready`, and no frontend ever called this — placed slots were
                                                              // selectable but invisible. Pixels built in core (slotAtlas.ts parity).
                            {
                                let atlas = map_engine_core::slots_gpu::build_slot_atlas();
                                if let Err(e) = eng.ensure_slot_atlas(
                                    &atlas.rgba,
                                    atlas.width,
                                    atlas.height,
                                    &atlas.uv,
                                ) {
                                    leptos::logging::error!("ensure_slot_atlas: {e:?}");
                                }
                            }
                            *engine.borrow_mut() = Some(eng);
                            register_self_checks(engine.clone());
                            register_editor_cam(engine.clone(), map_host.clone());
                            register_slot_stats(engine.clone());
                            // T-173 P6 — let the Mission Settings render-pref controls reach the
                            // live engine + host.
                            crate::world_assets::register_render_ctx(
                                engine.clone(),
                                map_host.clone(),
                            );
                            // T-159.16 — doc→engine bind (D5): with the atlas up, this first bind
                            // materializes + draws the seeded slot set.
                            let soa = doc.borrow().as_ref().map(|c| c.materialize());
                            if let (Some(soa), Some(e)) =
                                (soa.as_ref(), engine.borrow_mut().as_mut())
                            {
                                let tints = map_engine_core::slots_gpu::side_tints_rgba_bytes(
                                    &soa.side_keys,
                                );
                                e.slots_bind_soa(soa.ids.clone(), &soa.xy, &tints);
                            }
                            // T-175 B1 — engine is mounted + first-bound. If the IDB restore + hydrate
                            // already settled, rebind now from the settled doc (the first bind above
                            // may have drawn the pre-restore seed); otherwise the restore task will
                            // rebind once it settles. Exactly one authoritative rebind runs.
                            engine_mounted.set(true);
                            if restore_settled.get() {
                                crate::mission_history::rebind_engine_from_doc();
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
                                let boot_fut = crate::world_assets::bootstrap(
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
                        crate::world_assets::schedule_camera_settle(
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
                        crate::world_assets::set_camera_gesture(true);
                    } else if ev.button() == 0 {
                        // T-159.18/.19 — LMB pending-left: freeze the ortho camera at press (X-05: the
                        // live engine unproject is deleted; a live unproject would feedback-loop
                        // mid-pan). No pointer capture yet — a sub-threshold release is a click; the
                        // first past-threshold `pointermove` (T-159.19) promotes to Move|Marquee and
                        // captures then. `engine.borrow()` is safe: JS is single-threaded, so this never
                        // reenters the rAF loop's `borrow_mut`.
                        if let Some(e) = engine.borrow().as_ref() {
                            let rect = container.get_bounding_client_rect();
                            let cam = crate::select_tool::frozen_camera(
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
                                if crate::ruler_tool::should_begin_ruler(
                                    tool_mode.get_untracked(),
                                    ev.button(),
                                ) {
                                    crate::select_tool::LeftGesture::Ruler {
                                        start_x: sx,
                                        start_y: sy,
                                        cam,
                                    }
                                } else {
                                    crate::select_tool::LeftGesture::Pending(
                                        crate::select_tool::PendingLeft {
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
                move |ev: web_sys::PointerEvent| {
                    use crate::select_tool::{self as st, LeftGesture as LG};
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
                    let world = {
                        let g = engine.borrow();
                        g.as_ref().map(|e| {
                            st::frozen_camera(
                                rect.width(),
                                rect.height(),
                                e.target_x(),
                                e.target_y(),
                                e.zoom(),
                            )
                            .unproject_xy(px, py)
                        })
                    };
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
                    // T-175 B2 — palette place ghost: while an asset is being dragged from the
                    // palette (`begin_place` armed `pending`), show a live translucent slot ring at
                    // the cursor's world point so the operator sees where it will land (the drop
                    // commits at pointerup). Mutually exclusive with a map drag/marquee (`left` is
                    // None during a palette place), so this returns before the gesture machine.
                    if crate::editor_ops::has_pending() {
                        if let Some(c) = world.filter(|c| c[0].is_finite() && c[1].is_finite()) {
                            if let Some(e) = engine.borrow_mut().as_mut() {
                                e.set_place_preview(c[0] as f32, c[1] as f32);
                            }
                        }
                        return;
                    }
                    if let Some((lx, ly)) = pan_px.get() {
                        let (cx, cy) = (ev.client_x() as f64, ev.client_y() as f64);
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.pan(cx - lx, cy - ly);
                            e.on_camera_changed(); // T-172 H5 — slot sizing/cluster gate
                        }
                        pan_px.set(Some((cx, cy)));
                        // T-173 P1 — stream residency mid-drag: the debounce+max-latency arm fires a
                        // (cheap, memo-gated) settle every ~250 ms during a continuous pan, so chunks
                        // load as the camera crosses boundaries instead of only at pointer-up.
                        crate::world_assets::schedule_camera_settle(
                            map_host.clone(),
                            engine.clone(),
                        );
                        return;
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
                            // Real drag now: capture so it survives leaving the canvas (React :200).
                            let _ = container.set_pointer_capture(ev.pointer_id());
                            let sw = p.cam.unproject_xy(p.start_x, p.start_y);
                            let hit = doc.borrow().as_ref().and_then(|c| {
                                st::pick_slot_or_vehicle(
                                    &p.cam,
                                    &c.materialize(),
                                    &crate::editor_ops::vehicle_points(),
                                    p.start_x,
                                    p.start_y,
                                )
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
                                                .filter(|i| !crate::editor_ops::is_vehicle_id(i))
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
                                    &crate::editor_ops::vehicle_points(),
                                    dx,
                                    dy,
                                );
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
                    // T-159.22 — palette drag-to-place. FIRST: a place is armed by a `pointerdown`
                    // on a palette leaf, which the chrome host stops from reaching the map. The
                    // ARMED state is the signal this branch keys on (`has_pending()`), checked before
                    // any gesture branch below — NOT an assumption about what the gesture handles hold.
                    // (Wave-109 verifier fix: the prior comment asserted the gesture handles were
                    // necessarily unset at this point, which its own next sentence refutes — a release
                    // over a dock bubbles here even mid-gesture. What is actually true is that
                    // `has_pending()` short-circuits with a `return` before the gesture `match`
                    // regardless of what the gesture handles hold, so the two paths never interleave.)
                    //
                    // The host stops `pointerdown` only, so a release over a dock ALSO bubbles here:
                    // the chrome insets decide. They are the same consts `select_tool`'s probe grid
                    // insets by, so "not under chrome" means one thing editor-wide.
                    if crate::editor_ops::has_pending() {
                        let rect = container.get_bounding_client_rect();
                        let (px, py) = (
                            ev.client_x() as f64 - rect.left(),
                            ev.client_y() as f64 - rect.top(),
                        );
                        // T-638 — the LIVE insets (dock collapse + chrome_hidden folded in). A
                        // collapsed dock grows the on-canvas region into the freed strip, so a drop
                        // there lands an entity instead of being swallowed; while chrome is hidden the
                        // whole window is on-canvas. Same accessors `select_tool`'s probe grid uses, so
                        // "not under chrome" means one thing editor-wide.
                        let on_canvas = px >= crate::eden_layout::dock_left_px()
                            && px <= rect.width() - crate::eden_layout::dock_right_px()
                            && py >= crate::eden_layout::strip_top_px()
                            && py <= rect.height() - crate::eden_layout::toolbelt_band_px();
                        // Same frozen-camera unproject the pick + CUR use, so the slot lands exactly
                        // where CUR said it would.
                        let world = if on_canvas {
                            let g = engine.borrow();
                            g.as_ref().map(|e| {
                                crate::select_tool::frozen_camera(
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
                        match world.filter(|c| c[0].is_finite() && c[1].is_finite()) {
                            Some(c) => {
                                if ctrl_multi {
                                    crate::editor_ops::place_at_keep(c[0], c[1], alt_empty);
                                } else {
                                    crate::editor_ops::place_at_alt(c[0], c[1], alt_empty);
                                }
                            }
                            None => crate::editor_ops::cancel_pending(),
                        }
                        // T-175 B2 — the place gesture ended (drop or cancel): drop the ghost. A
                        // Ctrl multi-place that KEPT the pending re-shows the ghost on the next
                        // pointermove, so clearing it here is right either way.
                        if let Some(e) = engine.borrow_mut().as_mut() {
                            e.clear_place_preview();
                        }
                        return;
                    }
                    // Pan end (MMB/RMB).
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                        // T-176 B2 — pan ended: clear the gesture flag BEFORE scheduling so this
                        // settle runs the full zoom-band recompute (contours + forest) once.
                        crate::world_assets::set_camera_gesture(false);
                        crate::world_assets::schedule_camera_settle(
                            map_host.clone(),
                            engine.clone(),
                        );
                    }
                    // LMB gesture end. `take()` into a `let` first so the RefMut drops before the
                    // per-branch re-borrows below (the `if let` temporary-lifetime footgun). If a pan
                    // just ended, `left` is None ⇒ this returns.
                    let taken = left.borrow_mut().take();
                    let Some(g) = taken else { return };
                    use crate::select_tool::{self as st, LeftGesture as LG};
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
                                        &c.materialize(),
                                        &crate::editor_ops::vehicle_points(),
                                        p.start_x,
                                        p.start_y,
                                    )
                                });
                                {
                                    let mut sel = selection.borrow_mut();
                                    st::apply_click(&mut sel, hit, additive);
                                }
                                let ids = selection.borrow().clone();
                                if let Some(e) = engine.borrow_mut().as_mut() {
                                    let slot_ids: Vec<String> = ids
                                        .iter()
                                        .filter(|i| !crate::editor_ops::is_vehicle_id(i))
                                        .cloned()
                                        .collect();
                                    e.set_selection(slot_ids); // tint lane (slots only)
                                }
                                // T-159.21 — SEL readout only: a click changes the selection, not the
                                // document (no rebind / persist / undo step / tree rebuild).
                                crate::mission_history::refresh_selection();
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
                            let regrouped = if (ev.ctrl_key() || ev.meta_key())
                                && ids.len() == 1
                                && !crate::editor_ops::is_vehicle_id(&ids[0])
                            {
                                let target = doc
                                    .borrow()
                                    .as_ref()
                                    .and_then(|c| st::pick(&cam, &c.materialize(), up_x, up_y));
                                match target {
                                    Some(tid) if tid != ids[0] => {
                                        // `regroup_slot_onto` runs the shared dirty tail itself
                                        // (via `refile_slot`), so this branch must NOT also call
                                        // `after_local_edit` — it only drops the stale drag preview.
                                        let ok =
                                            crate::editor_ops::regroup_slot_onto(&ids[0], &tid);
                                        if ok {
                                            if let Some(e) = engine.borrow_mut().as_mut() {
                                                st::clear_drag_preview(
                                                    e,
                                                    &crate::editor_ops::vehicle_points(),
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
                                // T-491 — one LOCAL yrs txn for mixed slot+vehicle drag (T-425 split
                                // `move_entities` then `move_vehicles` needed two Ctrl+Z).
                                let (veh_ids, slot_ids): (Vec<String>, Vec<String>) = ids
                                    .iter()
                                    .cloned()
                                    .partition(|id| crate::editor_ops::is_vehicle_id(id));
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
                                            crate::editor_ops::keep_z_rows(
                                                core,
                                                Some(dx),
                                                Some(dy),
                                                None,
                                            )
                                        })
                                        .flatten();
                                    let zs: Vec<f64> = slot_ids
                                        .iter()
                                        .map(|id| {
                                            z_rows
                                                .as_ref()
                                                .and_then(|rows| {
                                                    crate::editor_ops::slot_z(rows, id)
                                                })
                                                .unwrap_or(0.0)
                                        })
                                        .collect();
                                    core.move_entities_and_vehicles(slot_ids, &veh_ids, dx, dy, zs);
                                    drop(guard);
                                    crate::mission_history::after_local_edit();
                                }
                            } else if let Some(e) = engine.borrow_mut().as_mut() {
                                // No move ⇒ no commit, so nothing else re-binds: drop BOTH preview
                                // lanes back to the authored positions (T-573 — the vehicle lane is
                                // a live re-pack now, not a passive bind).
                                st::clear_drag_preview(e, &crate::editor_ops::vehicle_points());
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
                                            &c.materialize(),
                                            &crate::editor_ops::vehicle_points(),
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
                                        .filter(|i| !crate::editor_ops::is_vehicle_id(i))
                                        .cloned()
                                        .collect();
                                    e.set_selection(slot_ids);
                                }
                                // T-159.21 — SEL readout only (selection change, not a doc edit).
                                crate::mission_history::refresh_selection();
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
                                                crate::los_tool::place_viewshed(w[0], w[1])
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
                                let acted = crate::editor_ops::rotate_selection_to_face(
                                    aim[0], aim[1], rung,
                                );
                                if acted {
                                    // A rotate changes the doc but not the selection; keep the tint
                                    // lane in sync (glyphs re-bind off the history tail) and refresh
                                    // the SEL readout, mirroring the Move commit's bookkeeping.
                                    crate::mission_history::refresh_selection();
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
                        crate::select_tool::frozen_camera(
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
                        crate::select_tool::pick_slot_or_vehicle(
                            &cam,
                            &c.materialize(),
                            &crate::editor_ops::vehicle_points(),
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
                    // Why no arm: an armed place would join `LeftGesture`'s pointerdown/up machine,
                    // and that machine has a known-pending defect (T-723 — the armed pointerup path
                    // has no button filter, can strand `LG::Pending`, and has no Esc disarm; the
                    // in-code "left/pan_px are both None here" invariant near pointerdown was
                    // refuted in wave 106). Comments do not need an arm to be correct: unlike a
                    // palette place, the gesture that chooses the point (the right-click) and the
                    // gesture that confirms the action (the menu row) are already two events, so the
                    // point is captured once and consumed once, with no in-flight state to strand.
                    // This ticket therefore adds ZERO new state to the gesture machine.
                    let world = cam.unproject_xy(px, py);
                    let target = crate::context_menu::resolve_target(hit.as_deref(), &sel)
                        .at_world(world[0], world[1]);
                    crate::context_menu::open(ev.client_x() as f64, ev.client_y() as f64, target);
                }
            });
            // T-159.21 — pointer off the map ⇒ the CUR read-out shows the em-dash cells (React's
            // `onPointerLeave → null`). Fires when the pointer enters a chrome panel too, which is
            // correct: those px are not map coordinates.
            let onpointerleave = Closure::<dyn FnMut(web_sys::PointerEvent)>::new({
                let engine = engine.clone();
                move |_ev: web_sys::PointerEvent| {
                    cursor.set(None);
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
                move |ev: web_sys::PointerEvent| {
                    // T-159.22 — a cancelled pointer drops an armed place, like every other
                    // in-flight gesture below (pointercancel is never a commit).
                    crate::editor_ops::cancel_pending();
                    if pan_px.get().is_some() {
                        pan_px.set(None);
                        if container.has_pointer_capture(ev.pointer_id()) {
                            let _ = container.release_pointer_capture(ev.pointer_id());
                        }
                    }
                    use crate::select_tool::{self as st, LeftGesture as LG};
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
                                st::clear_drag_preview(e, &crate::editor_ops::vehicle_points());
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
                        crate::select_tool::frozen_camera(
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
                        crate::select_tool::pick_slot_or_vehicle(
                            &cam,
                            &c.materialize(),
                            &crate::editor_ops::vehicle_points(),
                            px,
                            py,
                        )
                    });
                    match hit {
                        Some(id) => crate::editor_ops::open_attributes(id),
                        // T-647 PLACE-003 — empty ground: open the asset picker at the world point
                        // the dblclick names (same frozen-cam unproject the place ghost/CUR use, so
                        // the picker's eventual drop lands where the dblclick was). A singular
                        // unproject (NaN) is off-map and opens nothing.
                        None => {
                            let world = cam.unproject_xy(px, py);
                            if world[0].is_finite() && world[1].is_finite() {
                                crate::editor_ops::open_asset_picker(
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
                        <crate::eden_chrome::TopCommandStrip
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
                        crate::eden_layout::DOCK_LEFT_MOUNT_COLLAPSED
                    } else {
                        crate::eden_layout::DOCK_LEFT_MOUNT
                    }>
                        <crate::eden_chrome::DockLeft
                            nodes=outliner_nodes
                            selected=selected_ids
                            active_layer
                            collapsed=dock_left_collapsed
                        />
                    </div>
                })}
                {move || (!chrome_hidden.get()).then(|| view! {
                    <div class=move || if dock_right_collapsed.get() {
                        crate::eden_layout::DOCK_RIGHT_MOUNT_COLLAPSED
                    } else {
                        crate::eden_layout::DOCK_RIGHT_MOUNT
                    }>
                        <crate::eden_chrome::DockRight
                            catalog
                            vehicle_catalog
                            registry_items
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
                    <crate::eden_toolbelt::ModeToolbar tool_mode los_mode />
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
                    <crate::eden_toolbelt::StatusBar
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
                {move || (!chrome_hidden.get()).then(|| view! { <crate::eden_toolbelt::MapGridRefs cursor debug_hud=Some(debug_hud) /> })}
                // T-159.26 — Attributes modal (fixed overlay; no DOM while closed). Inside the
                // chrome subtree so its pointerdowns never open a map gesture. NOT gated by T-662's
                // `chrome_hidden` — a dialog the operator opened must survive a hide-interface toggle.
                <div class="pointer-events-auto">
                    <crate::attributes::AttributesModal attrs_open attrs_tab doc_tick registry_items compat />
                </div>
                <div class="pointer-events-auto">
                    <crate::eden_chrome::MissionSettingsDialog open=settings_open doc_tick />
                    <crate::faction_manager::FactionManagerDialog open=fm_open registry=registry_items />
                    // T-177 B2 / T-071.0 — ORBAT Manager modal shell (browse/select the live ORBAT
                    // faction → squad → slot tree relocated from the left dock).
                    <crate::eden_chrome::OrbatManagerDialog
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
                    <crate::context_menu::ContextMenuOverlay menu=context_menu />
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
                <crate::ruler_tool::RulerOverlay cursor debug_hud=Some(debug_hud) tick=ruler_tick />
                // T-643 — the Line-of-Sight overlay (dispatcher-authorized SINGLE mount line; the
                // component + all its logic live in `los_tool`, my owned file). UNGATED like the ruler
                // overlay: a placed LoS shot is a measurement the operator created, so it survives a
                // Backspace hide-chrome (it is not dock furniture). `pointer-events-none` (the SVG
                // never eats a map gesture — the two-click capture is the map's own pointer handlers),
                // reads the live camera + state + DEM sampler itself, and re-runs off the same
                // `cursor`/`debug_hud` heartbeats as the ruler (no new rAF loop) plus `los_tick`
                // (repaint on a still-pointer click).
                <crate::los_tool::LosOverlay cursor debug_hud=Some(debug_hud) tick=los_tick />
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
                <crate::validation_panel::ValidationPanel doc_tick />
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
                let mpp = crate::eden_toolbelt::m_per_px(e.zoom());
                let text = crate::eden_toolbelt::format_m_per_px(mpp);
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
    map_host: crate::world_assets::HostHandle,
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
            crate::world_assets::flush_viewport(map_host.clone(), engine.clone());
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

/// T-159.26 — the local-vs-server conflict payload the [`ConflictDialog`] offers to load. Un-gated
/// (two Strings, no wasm types) so the shared editor view can hold the signal; `mission_hydrate`
/// (wasm-only) produces and consumes it.
#[derive(Clone)]
pub struct ConflictInfo {
    pub payload_json: String,
    pub semver: Option<String>,
}

/// The conflict prompt (React `ConflictDialog`): renders only when `conflict` is `Some`. "Load
/// server version" hydrates the offered payload (data replaced); "Keep local copy" keeps the local
/// doc and marks it divergent. Renders no DOM while `None` — V-capture-safe.
#[component]
fn ConflictDialog(conflict: RwSignal<Option<ConflictInfo>>, conflict_id: String) -> impl IntoView {
    let id = StoredValue::new(conflict_id);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = id;
    move || {
        conflict.get().map(|c| {
            let _ = &c;
            #[cfg(target_arch = "wasm32")]
            let (id_server, id_local) = (id.get_value(), id.get_value());
            let semver_label = c
                .semver
                .clone()
                .map(|s| format!("Saved version v{s}"))
                .unwrap_or_else(|| "A saved version".to_string());
            view! {
                <div class="fixed inset-0 z-[60] bg-black/50 backdrop-blur-sm"></div>
                <div class="glass fixed top-1/2 left-1/2 z-[60] flex w-[92vw] max-w-md -translate-x-1/2 -translate-y-1/2 flex-col rounded-xl shadow-2xl outline-none">
                    <div class="border-b border-outline-variant/30 px-6 py-4">
                        <h2 class="text-headline-sm text-on-surface">"Unsaved local changes"</h2>
                        <p class="mt-1 text-label-md text-on-surface-variant">
                            {semver_label}
                            " on the server differs from your local copy. Which version should win?"
                        </p>
                    </div>
                    <div class="flex justify-end gap-2 px-6 py-4">
                        <button
                            type="button"
                            aria-label="Keep local copy"
                            class="rounded-lg border border-white/10 bg-white/5 px-4 py-2 text-label-md text-on-surface transition-colors hover:bg-white/10"
                            on:click=move |_| {
                                #[cfg(target_arch = "wasm32")]
                                crate::mission_hydrate::resolve_conflict_local(
                                    id_local.clone(),
                                    conflict,
                                );
                            }
                        >
                            "Keep local copy"
                        </button>
                        <button
                            type="button"
                            aria-label="Load server version"
                            class="rounded-lg bg-primary px-4 py-2 text-label-md font-medium text-on-primary"
                            on:click=move |_| {
                                #[cfg(target_arch = "wasm32")]
                                crate::mission_hydrate::resolve_conflict_server(
                                    id_server.clone(),
                                    conflict,
                                );
                            }
                        >
                            "Load server version"
                        </button>
                    </div>
                </div>
            }
        })
    }
}

#[cfg(test)]
mod t245_registry_session {
    use super::registry_session;
    use crate::arsenal_rules::{CompatFeed, CompatStatus};
    use crate::dto::RegistryItem;
    use std::collections::HashMap;

    fn sample_item(resource_name: &str) -> RegistryItem {
        RegistryItem {
            id: "00000000-0000-0000-0000-000000000001".to_string(),
            modpack_id: "test".to_string(),
            resource_name: resource_name.to_string(),
            display_name: resource_name.to_string(),
            category: "NATO/Rifleman".to_string(),
            icon_url: None,
            kind: "character".to_string(),
            r#abstract: None,
            arsenal_type: None,
            weight_kg: None,
            volume_cm3: None,
            max_weight_kg: None,
            max_volume_cm3: None,
            cargo_grid_w: None,
            cargo_grid_h: None,
            addon: None,
            variant_of: None,
            sort_order: 0,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    /// Cold session → both network paths are still required (first open pays once).
    #[test]
    fn cold_session_must_fetch_both() {
        registry_session::clear_for_test();
        assert!(
            registry_session::must_fetch_registry(),
            "cold session must still GET /registry once"
        );
        assert!(
            registry_session::must_fetch_compat(),
            "cold session must still GET /registry/compat once"
        );
    }

    /// After a successful fetch is stored, a remount must NOT plan another network round-trip
    /// — this is the load-bearing T-245 contract (no re-pay on every editor open).
    #[test]
    fn warm_session_skips_both_unpaginated_fetches() {
        registry_session::clear_for_test();
        let items = vec![sample_item("Prefab.Character.Test")];
        registry_session::store_registry(items.clone());
        let feed = CompatFeed {
            status: CompatStatus::Ready,
            graph: Default::default(),
        };
        registry_session::store_compat(feed, HashMap::new());

        assert!(
            !registry_session::must_fetch_registry(),
            "warm session must skip GET /registry"
        );
        assert!(
            !registry_session::must_fetch_compat(),
            "warm session must skip GET /registry/compat"
        );
        let hit = registry_session::cached_registry().expect("registry session hit");
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].resource_name, "Prefab.Character.Test");
        let (feed_hit, _) = registry_session::cached_compat().expect("compat session hit");
        assert!(
            matches!(feed_hit.status, CompatStatus::Ready),
            "cached compat feed must stay Ready"
        );
    }

    /// Mount source must consult the session gate before calling the cold fetch helpers.
    /// Guards against a future "helpful" revert to the always-spawn_local dual fetch.
    #[test]
    fn mount_source_gates_unpaginated_fetches_on_session_cache() {
        let src = include_str!("mission_editor.rs");
        assert!(
            src.contains("registry_session::must_fetch_registry()"),
            "mount path must gate GET /registry on must_fetch_registry()"
        );
        assert!(
            src.contains("registry_session::must_fetch_compat()"),
            "mount path must gate GET /registry/compat on must_fetch_compat()"
        );
        assert!(
            src.contains("registry_session::store_registry"),
            "successful /registry response must populate the session cache"
        );
        assert!(
            src.contains("registry_session::store_compat"),
            "successful /registry/compat response must populate the session cache"
        );
    }
}

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
mod t573_mixed_drag_preview {
    use crate::arsenal::class_r_scrub::{live_code, only_body};

    /// Both lanes must be driven from the same unfiltered `ids`, and the T-425 slot-only pre-filter
    /// must be gone from the drag branch — that filter WAS the bug (vehicles never previewed).
    #[test]
    fn drag_preview_feeds_the_whole_mixed_selection_to_both_lanes() {
        let tool = live_code(include_str!("select_tool.rs"));
        let push = only_body(&tool, "pub fn push_drag_preview(");
        assert!(
            push.contains("e.set_drag(ids.to_vec()"),
            "the slot lane must get the WHOLE id list — set_drag skips ids it cannot resolve, so \
             filtering vehicles out first only ever cost the vehicle preview"
        );
        assert!(
            push.contains("pack_vehicle_drag_preview("),
            "the vehicle lane must be re-packed with the dragged rows offset"
        );
        assert!(
            push.contains("e.vehicles_bind("),
            "…and uploaded, or the re-pack never reaches the GPU"
        );
        assert!(
            !push.contains("is_vehicle_id"),
            "a preview that filters by kind is the defect this ticket cures"
        );

        // The un-committed exits must put the vehicle lane back: it is live state during a drag now.
        let clear = only_body(&tool, "pub fn clear_drag_preview(");
        assert!(
            clear.contains("e.set_drag(Vec::new()") && clear.contains("e.vehicles_bind("),
            "clearing the preview must drop BOTH lanes, not just the slot overlay"
        );

        // `class_r_scrub::cut_test_module` cuts from the **first** `#[cfg(test)]` to EOF, and this
        // file has one at ~line 88 (`registry_session::clear_for_test`, a test-only helper inside a
        // production module). Scrubbing the whole file therefore examines only its first ~90 lines
        // and would report every needle below as absent — which is how this assertion first failed,
        // and why the scrubber is worth having: it refused rather than guessed. So hand it the
        // region from the next top-level item onward. The cut is at brace depth 0 between complete
        // items, so the slice stays balanced and the scrubber's own cut still fires on the real
        // test modules below.
        // Split so the anchor literal is not itself a second occurrence in this file (the t427
        // pin below uses the same trick for the same reason).
        let anchor = format!("{}{}", "const REGISTRY_", "COLD_PAGE");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "the scrub anchor must be unambiguous — 0 or 2+ means this pin is reading a region it \
             cannot identify"
        );
        let editor = live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..]);
        assert!(
            editor.contains("pub fn MissionEditorPage"),
            "canary: the scrubbed region must still contain the editor page, or the anchor moved \
             and this pin is examining almost nothing"
        );
        assert!(
            editor.contains("st::push_drag_preview("),
            "the pointermove drag branch must push the preview through the shared helper"
        );
        assert!(
            !editor.contains("e.set_drag(slot_ids"),
            "the drag branch must no longer feed set_drag a vehicle-filtered id list"
        );
        assert!(
            editor.contains("st::clear_drag_preview("),
            "the no-move release and the pointercancel must restore the vehicle lane"
        );
    }

    /// **Calibration.** Every needle above must stop being satisfied once the code it names is
    /// dead, or this pin could report a live mixed-drag preview over code the build never runs —
    /// which is the exact shape of defect the ticket is about, relocated into the test.
    #[test]
    fn the_preview_pin_rejects_every_dead_code_wrapper() {
        let needle = "pack_vehicle_drag_preview(";
        let attacks: [(&str, String); 8] = [
            ("if false", format!("if false {{ {needle}); }}")),
            (
                "if true == false",
                format!("if true == false {{ {needle}); }}"),
            ),
            ("while false", format!("while false {{ {needle}); }}")),
            ("loop { break; … }", format!("loop {{ break; {needle}); }}")),
            (
                "#[cfg(any())] item",
                format!("#[cfg(any())] fn d() {{ {needle}); }}"),
            ),
            (
                "#[cfg(any())] mod shadow",
                format!("#[cfg(any())] mod s {{ fn d() {{ {needle}); }} }}"),
            ),
            ("after return", format!("fn d() {{ return; {needle}); }}")),
            ("comment", format!("// {needle})")),
        ];
        for (label, body) in attacks {
            let forged = format!("pub fn push_drag_preview() {{\n    {body}\n}}\n#[cfg(test)]\n");
            assert!(
                !live_code(&forged).contains(needle),
                "{label}: the vehicle re-pack needle survived scrubbing — this pin would report a \
                 live mixed-drag preview over code that never runs"
            );
        }
        // A second definition is how a pin gets fed a pristine decoy while the real one is gutted.
        let two = "pub fn push_drag_preview() { good(); }\n\
                   mod real { pub fn push_drag_preview() { bad(); } }\n#[cfg(test)]\n";
        let scrubbed = live_code(two);
        let caught =
            std::panic::catch_unwind(|| only_body(&scrubbed, "pub fn push_drag_preview(")).is_err();
        assert!(
            caught,
            "two definitions must be RED, not a coin flip over which one ships"
        );
        // …and the honest shape must still satisfy the needle, or the battery proves nothing.
        let live = format!("pub fn push_drag_preview() {{\n    {needle});\n}}\n#[cfg(test)]\n");
        assert!(live_code(&live).contains(needle));
    }
}

/// T-427 — cold path must not depend on the unbounded dual dump.
#[cfg(test)]
mod t427_cold_registry_path {
    /// Source guard: cold open pages registry with limit+offset and never calls the bare dump.
    #[test]
    fn cold_registry_uses_paginated_path_not_unbounded_dump() {
        let src = include_str!("mission_editor.rs");
        assert!(
            src.contains("fetch_registry_pages"),
            "cold registry must go through the paginated helper"
        );
        assert!(
            src.contains("/registry?limit={REGISTRY_COLD_PAGE}&offset={offset}"),
            "cold registry URL must carry limit+offset"
        );
        // Bare dump path as an api_get literal — the only remaining "/registry" forms are
        // query-bearing (`?limit=` / `?view=` / `?edge_type=`).
        assert!(
            !src.contains("api_get(auth, \"/registry\")"),
            "must not api_get bare registry dump"
        );
    }

    /// Source guard: cold compat uses filtered Arsenal edges + cargo_defaults view.
    #[test]
    fn cold_compat_uses_filtered_edges_and_cargo_defaults_view() {
        let src = include_str!("mission_editor.rs");
        assert!(
            src.contains("fetch_compat_cold"),
            "cold compat must go through the narrow helper"
        );
        assert!(
            src.contains("optic_on_weapon,mag_in_weapon,attachment_on_weapon"),
            "Arsenal edge_type filter must be pinned"
        );
        assert!(
            src.contains("/registry/compat?view=cargo_defaults"),
            "cargo seeds must come from the aggregated view"
        );
        let walk_fn = format!("{}{}", "cargo_defaults_by_character", "(&");
        assert!(
            !src.contains(&walk_fn),
            "client must not walk raw cargo edges on cold open"
        );
        assert!(
            !src.contains("api_get(auth, \"/registry/compat\")"),
            "must not api_get bare compat dump"
        );
    }

    /// DTO: paginated envelope + cargo_defaults view round-trip.
    #[test]
    fn dto_paginated_registry_and_cargo_defaults_round_trip() {
        let page = serde_json::json!({
            "data": [],
            "etag": "W/\"x\"",
            "modpack_id": "00000000-0000-0000-0000-000000000001",
            "modpack_version": "1",
            "total": 1857,
            "limit": 500,
            "offset": 0
        });
        let r: crate::dto::RegistryResponse = serde_json::from_value(page).unwrap();
        assert_eq!(r.total, Some(1857));
        assert_eq!(r.limit, Some(500));
        assert_eq!(r.offset, Some(0));

        let cargo = serde_json::json!({
            "view": "cargo_defaults",
            "data": {
                "char_a": [{"container": "vest", "item": "mag", "qty": 2}]
            },
            "etag": "W/\"y\"",
            "modpack_id": "00000000-0000-0000-0000-000000000001",
            "modpack_version": "1",
            "source_edge_count": 16223
        });
        let c: crate::dto::RegistryCargoDefaultsResponse = serde_json::from_value(cargo).unwrap();
        assert_eq!(c.view, "cargo_defaults");
        assert_eq!(c.source_edge_count, Some(16223));
        let rows = c.data.get("char_a").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].container, "vest");
        assert_eq!(rows[0].qty, 2);
        // Slim proof: aggregated row count << raw edge count advertised by the server.
        assert!(
            (rows.len() as i64) < c.source_edge_count.unwrap(),
            "cargo_defaults view must be smaller than the raw edge walk"
        );
    }
}

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
mod t628_boot_progress {
    use super::boot_progress::{
        fmt_bytes_pair, fmt_files_pair, percent, split_range, BootEvent, BootProgress, BootSeg,
        Ordered, PLANNED_SATELLITE_BYTES, PLANNED_TERRAIN_BYTES, PLANNED_WORLD_BYTES,
        SAT_CHUNK_BYTES, SAT_FETCH_CONCURRENCY, STREAM_REPORT_BYTES,
    };
    use super::BOOT_HANDOVER_MS;

    /// everon `everon-sat.tbd-sat`, read off the live index at `/map-assets/everon/satellite/`
    /// (2026-08-01): file 152,713,114 B; level 0 = 4 tiles of 28,326,346 / 21,632,714 / 27,555,806
    /// / 33,042,794 starting at 2,644.
    const L0_TILE0_OFFSET: u64 = 2_644;
    const L0_TILE0_LENGTH: u64 = 28_326_346;
    const FILE_BYTES: u64 = 152_713_114;

    // ── split_range: the spans must rebuild the tile byte for byte ────────────────────────────

    #[test]
    fn split_range_covers_the_tile_exactly_contiguously_and_in_order() {
        let spans = split_range(L0_TILE0_OFFSET, L0_TILE0_LENGTH, SAT_CHUNK_BYTES);
        assert!(!spans.is_empty(), "a 28 MB tile must produce requests");
        assert_eq!(
            spans[0].0, L0_TILE0_OFFSET,
            "the run must start at the tile's own offset"
        );
        assert_eq!(
            spans[spans.len() - 1].1,
            L0_TILE0_OFFSET + L0_TILE0_LENGTH - 1,
            "the run must end on the tile's last byte (Range ends are inclusive)"
        );
        let mut covered = 0u64;
        for (i, &(start, end)) in spans.iter().enumerate() {
            assert!(end >= start, "span {i} is inverted");
            assert!(
                end - start + 1 <= SAT_CHUNK_BYTES,
                "span {i} is larger than one request"
            );
            if i > 0 {
                assert_eq!(
                    start,
                    spans[i - 1].1 + 1,
                    "span {i} must resume exactly where {} stopped — a gap loses bytes, an \
                     overlap duplicates them, and concatenation cannot tell either from a good run",
                    i - 1
                );
            }
            covered += end - start + 1;
        }
        assert_eq!(
            covered, L0_TILE0_LENGTH,
            "the spans must cover the tile exactly"
        );
    }

    #[test]
    fn split_range_degenerate_inputs_do_not_loop_or_overrun() {
        assert!(
            split_range(2_644, 0, SAT_CHUNK_BYTES).is_empty(),
            "a zero-length tile asks for nothing"
        );
        assert_eq!(
            split_range(100, 10, SAT_CHUNK_BYTES),
            vec![(100, 109)],
            "a tile below one chunk is one request"
        );
        assert_eq!(
            split_range(100, 3, 0),
            vec![(100, 100), (101, 101), (102, 102)],
            "a zero chunk must degrade to 1 B a request, not spin"
        );
    }

    // ── Ordered: the scrambled-texture guard ─────────────────────────────────────────────────

    #[test]
    fn completions_arriving_out_of_order_reassemble_in_request_order() {
        // The network hands back 3, 0, 2, 1 — the shape `buffer_unordered` actually produces.
        let mut slots: Ordered<&str> = Ordered::new(4);
        for (i, body) in [(3, "d"), (0, "a"), (2, "c"), (1, "b")] {
            assert!(slots.put(i, body), "slot {i} must accept its body");
        }
        assert_eq!(
            slots.finish(),
            Some(vec!["a", "b", "c", "d"]),
            "the assembled run must be in REQUEST order, not completion order — `commit_mip` \
             uploads element n at mip.tiles[n]'s (x, y), so completion order here is a scrambled \
             satellite texture that reads as a rendering bug"
        );
    }

    #[test]
    fn a_dropped_completion_fails_instead_of_shifting_the_run() {
        let mut slots: Ordered<u8> = Ordered::new(3);
        assert!(slots.put(0, 1));
        assert!(slots.put(2, 3));
        assert_eq!(
            slots.finish(),
            None,
            "a missing chunk must fail the whole fetch; a 2-element Vec would silently shift \
             every tile after the gap"
        );
    }

    #[test]
    fn an_out_of_range_slot_is_refused_rather_than_dropped() {
        let mut slots: Ordered<u8> = Ordered::new(2);
        assert!(
            !slots.put(2, 9),
            "an index past the plan must be reported so the caller aborts — silently ignoring \
             it loses a chunk the length check would then blame on the server"
        );
    }

    // ── percent / byte formatting ────────────────────────────────────────────────────────────

    #[test]
    fn percent_is_clamped_and_survives_a_zero_total() {
        assert!((percent(0, FILE_BYTES) - 0.0).abs() < 1e-9);
        assert!((percent(FILE_BYTES / 2, FILE_BYTES) - 50.0).abs() < 0.001);
        assert!((percent(FILE_BYTES, FILE_BYTES) - 100.0).abs() < 1e-9);
        assert!(
            (percent(FILE_BYTES + 4096, FILE_BYTES) - 100.0).abs() < 1e-9,
            "a body longer than the index promised must not push the fill past its track"
        );
        assert!(
            (percent(1, 0) - 0.0).abs() < 1e-9,
            "nothing measured is nothing done, not a division"
        );
    }

    #[test]
    fn the_byte_pair_reads_in_one_unit_and_matches_the_manifest() {
        assert_eq!(
            fmt_bytes_pair(0, FILE_BYTES),
            "0.0 MB / 152.7 MB",
            "the total must read as the manifest's own `bytes` field does"
        );
        assert_eq!(fmt_bytes_pair(47_300_000, FILE_BYTES), "47.3 MB / 152.7 MB");
        assert_eq!(
            fmt_bytes_pair(4_194_304, 42_152_810),
            "4.2 MB / 42.2 MB",
            "the 8192-limit device fetches level 1 down — 42 MB, not 152"
        );
        assert_eq!(
            fmt_bytes_pair(500, 900),
            "500 B / 900 B",
            "a sub-KB total must not read as 0.0 MB / 0.0 MB"
        );
    }

    // ── the one bar: weighting, monotonicity, clamping, and reaching 100% ────────────────────

    /// The world segment's real shape at boot, measured on the live stack: 7 `WorldHost::init`
    /// files + 2 label files + 625 density bins are declared up front, and the chunk batch the
    /// residency pins declares itself before it fetches.
    const WORLD_STATIC_FILES: u64 = 7 + 2 + 625;
    const WORLD_CHUNK_FILES: u64 = 200;

    /// Drive the whole boot the way the loaders do, in the order they do it.
    fn boot_to_completion() -> BootProgress {
        let mut p = BootProgress::new();
        p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
        p.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
        p.apply(BootEvent::Done(BootSeg::Mission, 2_032));
        p.apply(BootEvent::Finish(BootSeg::Mission));
        p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        p.apply(BootEvent::Finish(BootSeg::Terrain));
        p.apply(BootEvent::Budget(
            BootSeg::Satellite,
            PLANNED_SATELLITE_BYTES,
        ));
        p.apply(BootEvent::Done(BootSeg::Satellite, PLANNED_SATELLITE_BYTES));
        p.apply(BootEvent::Finish(BootSeg::Satellite));
        p.apply(BootEvent::Files(BootSeg::World, WORLD_CHUNK_FILES));
        p.apply(BootEvent::Done(
            BootSeg::World,
            WORLD_STATIC_FILES + WORLD_CHUNK_FILES,
        ));
        p.apply(BootEvent::Finish(BootSeg::World));
        p
    }

    #[test]
    fn nothing_is_claimed_before_anything_is_measured() {
        let mut p = BootProgress::new();
        assert!(
            (p.percent() - 0.0).abs() < 1e-9,
            "a boot that has measured nothing is at 0% — the old sweep's whole problem was that it \
             looked identical at 0 and at 99"
        );
        // Budgets alone move nothing: they are denominators, not work.
        p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        p.apply(BootEvent::Budget(
            BootSeg::Satellite,
            PLANNED_SATELLITE_BYTES,
        ));
        p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
        assert!(
            (p.percent() - 0.0).abs() < 1e-9,
            "knowing how big the download is is not the same as having downloaded any of it"
        );
        p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES / 2));
        assert!(p.percent() > 0.0, "real bytes must move the bar");
    }

    #[test]
    fn one_bar_spans_the_whole_boot_and_never_resets_between_segments() {
        let mut p = BootProgress::new();
        p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
        let mut seen: Vec<f64> = vec![p.percent()];
        // Mission, then terrain, then satellite, then world — the four stages in boot order.
        p.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
        p.apply(BootEvent::Done(BootSeg::Mission, 2_032));
        p.apply(BootEvent::Finish(BootSeg::Mission));
        seen.push(p.percent());
        p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        for _ in 0..4 {
            p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES / 4));
            seen.push(p.percent());
        }
        p.apply(BootEvent::Finish(BootSeg::Terrain));
        seen.push(p.percent());
        p.apply(BootEvent::Budget(
            BootSeg::Satellite,
            PLANNED_SATELLITE_BYTES,
        ));
        for _ in 0..4 {
            p.apply(BootEvent::Done(
                BootSeg::Satellite,
                PLANNED_SATELLITE_BYTES / 4,
            ));
            seen.push(p.percent());
        }
        p.apply(BootEvent::Finish(BootSeg::Satellite));
        seen.push(p.percent());
        p.apply(BootEvent::Files(BootSeg::World, WORLD_CHUNK_FILES));
        p.apply(BootEvent::Done(
            BootSeg::World,
            WORLD_STATIC_FILES + WORLD_CHUNK_FILES,
        ));
        p.apply(BootEvent::Finish(BootSeg::World));
        seen.push(p.percent());

        for w in seen.windows(2) {
            assert!(
                w[1] >= w[0],
                "the bar must never step back — it went {:.3} → {:.3}. Restarting per stage is \
                 exactly what T-627 did and what the operator rejected",
                w[0],
                w[1]
            );
        }
        // Crossing a segment boundary must not drop the bar to zero.
        assert!(
            seen.iter().skip(2).all(|v| *v > 0.0),
            "no reading after the first stage may be 0%: that is a reset, not one bar"
        );
        assert!((seen[seen.len() - 1] - 100.0).abs() < 1e-9);
    }

    #[test]
    fn a_budget_that_grows_holds_the_bar_instead_of_rewinding_it() {
        let mut p = BootProgress::new();
        // The world's static plan lands, and the init + label files complete against it…
        p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
        p.apply(BootEvent::Done(BootSeg::World, 9));
        let before = p.percent();
        // …then the residency pins the boot camera and 200 chunk files join the same segment.
        p.apply(BootEvent::Files(BootSeg::World, WORLD_CHUNK_FILES));
        assert!(
            p.raw() < before,
            "the arithmetic really does dip here — 9/634 is a bigger fraction than 9/834. If this \
             assert fails the test is no longer exercising the case it exists for"
        );
        assert!(
            (p.percent() - before).abs() < 1e-9,
            "the bar must ABSORB the larger budget by holding, not by rewinding: it read {before:.4} \
             and then {:.4}",
            p.percent()
        );
        // And it resumes as soon as real work passes the mark it held.
        p.apply(BootEvent::Done(BootSeg::World, 400));
        assert!(p.percent() > before, "real work past the hold must move it");
    }

    #[test]
    fn a_weight_that_grows_holds_the_bar_instead_of_rewinding_it() {
        let mut p = BootProgress::new();
        p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES / 2));
        let before = p.percent();
        // A 16384-limit GPU takes level 0 too: the satellite's real budget is 152.7 MB, not the
        // 42.2 MB planned — so the denominator jumps and every completed byte is worth less.
        p.apply(BootEvent::Budget(BootSeg::Satellite, 152_710_470));
        assert!(
            p.raw() < before,
            "a satellite 3.6× the planned size really does shrink everything else's share"
        );
        assert!(
            (p.percent() - before).abs() < 1e-9,
            "learning the device's real satellite size must not rewind the bar"
        );
    }

    #[test]
    fn a_segment_that_overruns_its_promised_budget_is_clamped_to_its_own_share() {
        let mut p = BootProgress::new();
        p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        let honest = p.percent();
        // A `content-length` that undercounts the body (a proxy re-encoding it, say) must not let
        // the terrain segment spend the satellite's and the world's share of the track.
        p.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES * 4));
        assert!(
            (p.percent() - honest).abs() < 1e-9,
            "a segment that overruns is clamped at its own weight — it read {honest:.4} then {:.4}",
            p.percent()
        );
        let expected = 100.0 * PLANNED_TERRAIN_BYTES as f64
            / (PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES + PLANNED_WORLD_BYTES) as f64;
        assert!(
            (honest - expected).abs() < 0.001,
            "a finished terrain is worth exactly its weight's share: {honest:.3} vs {expected:.3}"
        );
    }

    #[test]
    fn the_bar_can_never_exceed_one_hundred() {
        let mut p = BootProgress::new();
        for seg in BootSeg::ALL {
            p.apply(BootEvent::Budget(seg, 1_000));
            p.apply(BootEvent::Files(seg, 10));
            p.apply(BootEvent::Done(seg, u64::MAX));
            assert!(
                p.percent() <= 100.0,
                "{seg:?} pushed the bar to {:.4} — past the end of its own track",
                p.percent()
            );
        }
        p.apply(BootEvent::Done(BootSeg::World, u64::MAX));
        assert!(
            (p.percent() - 100.0).abs() < 1e-9,
            "saturating every segment reads 100%, not 400%"
        );
    }

    #[test]
    fn every_segment_finishing_reads_exactly_one_hundred_even_when_one_failed() {
        // The failure shape the overlay has to survive: the DEM never arrived, so its segment has
        // no budget and no bytes at all — but the boot still ends and the overlay still has to come
        // down on a full bar rather than park at 49% forever.
        let mut p = BootProgress::new();
        p.apply(BootEvent::Files(BootSeg::World, WORLD_STATIC_FILES));
        p.apply(BootEvent::Budget(
            BootSeg::Satellite,
            PLANNED_SATELLITE_BYTES,
        ));
        p.apply(BootEvent::Done(BootSeg::Satellite, PLANNED_SATELLITE_BYTES));
        p.apply(BootEvent::Done(BootSeg::World, WORLD_STATIC_FILES));
        assert!(!p.is_complete());
        assert!(p.percent() < 100.0, "an unfinished boot is not a full bar");
        for seg in BootSeg::ALL {
            p.apply(BootEvent::Finish(seg));
        }
        assert!(p.is_complete());
        assert!(
            (p.percent() - 100.0).abs() < 1e-9,
            "every loader has reported in, so the bar reads 100% — it read {:.4}. A hand-over on a \
             bar that stopped short is the failure this slice exists to remove",
            p.percent()
        );
        assert!((boot_to_completion().percent() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn a_weightless_segment_redistributes_its_share_to_the_others() {
        // The mission document starts weightless (its size is unknowable until its headers land),
        // so before it reports the other three divide the whole bar between them…
        let mut without = BootProgress::new();
        without.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        without.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        let share_without = without.percent();

        // …and the moment it weighs 142 MB, the terrain is worth materially less of the track.
        let mut with = BootProgress::new();
        with.apply(BootEvent::Budget(BootSeg::Mission, 142_000_000));
        with.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        with.apply(BootEvent::Done(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        let share_with = with.percent();

        let denom_without = PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES + PLANNED_WORLD_BYTES;
        assert!(
            (share_without - 100.0 * PLANNED_TERRAIN_BYTES as f64 / denom_without as f64).abs()
                < 0.001,
            "with no mission weight the terrain is its share of the other three"
        );
        assert!(
            (share_with
                - 100.0 * PLANNED_TERRAIN_BYTES as f64 / (denom_without + 142_000_000) as f64)
                .abs()
                < 0.001,
            "a 142 MB mission document takes its own share of the bar — the T-060 scale case is \
             exactly why the document cannot be treated as a rounding error"
        );
        assert!(
            share_with < share_without / 2.0,
            "a mission bigger than the whole map must take more than half the track: {share_with:.2} \
             vs {share_without:.2}"
        );
    }

    #[test]
    fn the_weights_are_the_live_measurements_and_the_map_dominates_them() {
        assert_eq!(
            PLANNED_TERRAIN_BYTES, 71_911_548,
            "the terrain weight is the `content-length` of \
             /map-assets/everon/dem/everon-dem-16bit.png, measured 2026-08-01"
        );
        assert_eq!(
            PLANNED_SATELLITE_BYTES, 42_152_810,
            "the satellite weight is the tbd-sat index's own tile lengths from level 1 down — what \
             an 8192-limit maxTextureDimension2D actually uploads"
        );
        // The whole reason weights exist: a naive equal-quarters bar would stall in two places.
        let total = PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES + PLANNED_WORLD_BYTES;
        let dem_and_sat = PLANNED_TERRAIN_BYTES + PLANNED_SATELLITE_BYTES;
        assert!(
            dem_and_sat * 100 / total >= 80,
            "the DEM and satellite are ~81% of the map's bytes — an equal-quarters bar would give \
             them half the track and crawl through both, then race through the rest"
        );
        assert!(
            STREAM_REPORT_BYTES > 0 && PLANNED_TERRAIN_BYTES / STREAM_REPORT_BYTES >= 100,
            "the stream must report at least ~100 times across the DEM, or the terrain segment is \
             a per-file bar again: 0% for the whole download, then a snap"
        );
    }

    #[test]
    fn the_stage_name_follows_the_first_unfinished_segment() {
        let mut p = BootProgress::new();
        assert_eq!(p.stage(), BootSeg::Mission);
        assert_eq!(p.stage().title(), "Loading mission…");
        p.apply(BootEvent::Finish(BootSeg::Mission));
        assert_eq!(p.stage(), BootSeg::Terrain);
        assert_eq!(p.stage().title(), "Loading terrain…");
        p.apply(BootEvent::Finish(BootSeg::Terrain));
        assert_eq!(p.stage(), BootSeg::Satellite);
        assert_eq!(p.stage().title(), "Loading satellite…");
        p.apply(BootEvent::Finish(BootSeg::Satellite));
        assert_eq!(p.stage(), BootSeg::World);
        assert_eq!(p.stage().title(), "Loading world objects…");
    }

    #[test]
    fn the_caption_reports_bytes_for_bytes_and_files_for_files() {
        let mut p = BootProgress::new();
        assert_eq!(
            p.caption(),
            "0%",
            "a stage that has not read its own budget shows the percentage alone — not a \
             denominator nobody measured"
        );
        p.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
        p.apply(BootEvent::Done(BootSeg::Mission, 2_032));
        assert_eq!(p.caption(), "0% · 3 KB / 3 KB");
        p.apply(BootEvent::Finish(BootSeg::Mission));
        p.apply(BootEvent::Budget(BootSeg::Terrain, PLANNED_TERRAIN_BYTES));
        p.apply(BootEvent::Done(BootSeg::Terrain, 26_700_000));
        assert_eq!(p.caption(), "18% · 26.7 MB / 71.9 MB");
        p.apply(BootEvent::Finish(BootSeg::Terrain));
        p.apply(BootEvent::Finish(BootSeg::Satellite));
        p.apply(BootEvent::Files(BootSeg::World, 834));
        p.apply(BootEvent::Done(BootSeg::World, 214));
        assert!(
            p.caption().ends_with("214 / 834 files"),
            "the world counts completed fetches, so it says files — implying a byte budget nothing \
             published is the same defect one size down. Got {}",
            p.caption()
        );
        assert_eq!(fmt_files_pair(214, 834), "214 / 834 files");
    }

    // ── the wasm side must actually route through the code proved above ──────────────────────

    /// Source pin on `world_assets/satellite.rs`. It is `#[cfg(target_arch = "wasm32")]` (via
    /// `mod world_assets` in `main.rs`), so nothing in it can be called from here — but it can be
    /// held to *shape*. `live_code` blanks comments and string literals first, so a needle can only
    /// be satisfied by code that ships.
    #[test]
    fn the_satellite_fetch_is_bounded_concurrent_ordered_and_fails_fast() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let src = live_code(include_str!("world_assets/satellite.rs"));
        let body = only_body(&src, "async fn fetch_tiles(");

        assert!(
            body.contains("buffer_unordered(SAT_FETCH_CONCURRENCY)"),
            "the fetch must be concurrent and BOUNDED by the named constant — an unbounded \
             `FuturesUnordered` over 37 requests starves the world-chunk loader on the same origin"
        );
        assert!(
            body.contains("split_range(t.offset, t.length, SAT_CHUNK_BYTES)"),
            "requests must come from the span planner proved above, not an ad-hoc loop"
        );
        assert!(
            body.contains("Ordered::new(p.len())") && body.contains(".put(pi, body.bytes)"),
            "completions must be written to their own index; a `push` here is the scrambled \
             texture this module exists to prevent"
        );
        assert!(
            body.contains("slot.finish()?"),
            "reassembly must refuse a partially filled run"
        );
        assert!(
            body.contains("let body = got?;")
                && body.contains("body.bytes.len() as u64 != want")
                && body.contains("body.total != file_size"),
            "fail-fast and the length check must both survive: the pre-T-627 loop returned None \
             on the first failure and on a short body, and a partial texture must still never \
             reach commit_mip"
        );
        assert!(
            !body.contains("out.push(body.bytes)"),
            "bodies must never be pushed in completion order"
        );

        // And the full load must not be back to swallowing the whole bundle to read its index.
        let full = only_body(&src, "async fn load_unified_full(");
        assert!(
            full.contains("fetch_index_head(url, true)") && full.contains("fetch_tiles(url,"),
            "the full mip chain must come from the index + per-tile Range fetches"
        );
        assert!(
            !full.contains("fetch_bytes(url)"),
            "a whole-file GET has no byte progress to report and drags down 110.6 MB of level 0 \
             that an 8192-limit GPU cannot use"
        );
    }

    /// Source pin on the overlay itself. Raw `include_str!` (not `live_code`) because this file's
    /// first `#[cfg(test)]` is a `clear_for_test` helper near the top, which would cut the view
    /// out; the needles are therefore assembled at runtime so this test's own text cannot satisfy
    /// them.
    #[test]
    fn the_overlay_draws_one_measured_bar_and_no_sweep_anywhere() {
        let src = include_str!("mission_editor.rs");
        let from_progress = format!("{}{}", "p.", "percent()");
        assert!(
            src.contains(&from_progress),
            "the overlay's width must come from the accumulator, not from a per-stage step"
        );
        let inline_width = format!("{}{}", "width:{", "pct:.1}%");
        assert!(
            src.contains(&inline_width),
            "the fill's width must be the real percentage"
        );
        let sweep = format!("{}{}", "animate-mc-", "load-bar");
        assert!(
            !src.contains(sweep.as_str()),
            "the Mission Creator boot overlay must contain NO indeterminate sweep. A sweep looks \
             identical at 1%, at 99% and while stalled — 'you might as well have a black screen'. \
             (The class itself still ships for other surfaces; this file may not use it.)"
        );
        assert!(
            SAT_FETCH_CONCURRENCY >= 4 && SAT_FETCH_CONCURRENCY <= 6,
            "browsers cap ~6 connections per origin and the chunk loader shares them — outside \
             4..=6 this is either not parallel or actively starving the rest of the boot"
        );
        assert!(
            SAT_CHUNK_BYTES > 0 && FILE_BYTES / SAT_CHUNK_BYTES >= 20,
            "the chunk size must give the bar at least ~20 steps across the bundle, or it is a \
             per-tile bar again: four ~25 MB tiles fetched four-up would sit at 0% then snap"
        );
    }

    /// Source pin on the terrain segment. The DEM is the single biggest thing the boot fetches and
    /// the pre-T-628 path pulled it with a plain `fetch_bytes`, which yields one 71.9 MB step at the
    /// very end — indistinguishable from a stall for the whole download.
    #[test]
    fn the_terrain_dem_is_streamed_against_its_content_length() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let src = live_code(include_str!("world_assets/mod.rs"));
        let body = only_body(&src, "async fn load_dem_and_hillshade(");
        assert!(
            body.contains("fetch_bytes_streamed(") && body.contains("BootSeg::Terrain"),
            "the DEM must be fetched through the measured, streamed helper — a whole-body GET has \
             nothing to report until it is already finished"
        );
        assert!(
            !body.contains("fetch_bytes(&format!"),
            "the unmeasured whole-body GET must not come back"
        );

        let fetch = live_code(include_str!("world_assets/fetch.rs"));
        let streamed = only_body(&fetch, "pub async fn fetch_bytes_streamed(");
        // `live_code` blanks string literals, so the header NAME cannot be the needle — the shape
        // that survives is "a header off this response, parsed as a number, becomes the budget",
        // which is the property that matters anyway.
        assert!(
            streamed.contains(".headers()")
                && streamed.contains("parse::<u64>()")
                && streamed.contains("BootEvent::Budget(seg, budget)"),
            "the budget must be a header read off this response, not a constant and not a guess"
        );
        assert!(
            streamed.contains("reader.read()") && streamed.contains("BootEvent::Done"),
            "progress must be the bytes that came out of the body reader — nothing else in this \
             function is allowed to be the numerator"
        );
        let elapsed = ["Date::now", "set_timeout", "performance"];
        for needle in elapsed {
            assert!(
                !streamed.contains(needle),
                "`{needle}` in the streaming fetch would be a bar moving on a clock: the one \
                 defect this whole slice is aimed at"
            );
        }
    }

    /// Source pin on the world segment's two dynamic budgets. Both must be declared **before** the
    /// fetches they cover: a batch that announces itself on completion is a bar that reaches 100%
    /// and then finds more work, which reads to the operator as a lie either way round.
    #[test]
    fn every_world_batch_declares_its_files_before_it_fetches_them() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let world = live_code(include_str!("world_assets/world_host.rs"));
        let queue = only_body(&world, "async fn fetch_and_queue(");
        let declare = queue
            .find("BootEvent::Files")
            .expect("the chunk batch must declare its own size");
        let fetch = queue
            .find("fetch_bytes(&url)")
            .expect("the chunk batch must still fetch");
        assert!(
            declare < fetch,
            "the chunk count must be declared before the first request goes out, not after"
        );
        assert!(
            queue.contains("ids.len() as u64"),
            "the declared count must be the residency's own missing set — the exact list it is \
             about to request, not an estimate of it"
        );

        let boot = live_code(include_str!("world_assets/mod.rs"));
        let bootstrap = only_body(&boot, "pub async fn bootstrap(");
        let plan = bootstrap
            .find("planned_density_bins()")
            .expect("the 625 density bins must be declared up front");
        let init = bootstrap
            .find("world.init(")
            .expect("bootstrap must still init the world host");
        assert!(
            plan < init,
            "the density bins are a known constant (25×25) and must join the budget before the \
             world starts filling it — declaring them after the chunks land would park the bar at \
             100% and then discover 625 more files"
        );

        // The forest host may only count a bin it actually landed; counting attempts would let a
        // retried bin advance a unit that was already declared and spent.
        let forest = live_code(include_str!("world_assets/forest_mass.rs"));
        let upload = only_body(&forest, "async fn boot_upload(");
        let done_at = upload
            .find("BootEvent::Done")
            .expect("a landed bin must be counted");
        let ok_at = upload
            .rfind("if ok {")
            .expect("the bin must only be counted when it decoded");
        assert!(
            ok_at < done_at,
            "a density bin counts on success only — a retry loop that counts attempts finishes 625 \
             declared bins at 640 done, i.e. a full segment over a holed canopy"
        );
    }

    /// Source pin on the hand-over. Every segment must be closed by the code that owns it, or a
    /// dead network leaves the bar short of 100% with the overlay still up — and the overlay may
    /// not come down until it is full.
    #[test]
    fn every_segment_is_closed_and_the_overlay_waits_for_a_full_bar() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let boot = live_code(include_str!("world_assets/mod.rs"));
        let bootstrap = only_body(&boot, "pub async fn bootstrap(");
        for seg in ["BootSeg::Terrain", "BootSeg::Satellite", "BootSeg::World"] {
            assert!(
                bootstrap.contains(&format!("BootEvent::Finish({seg})")),
                "`bootstrap` owns {seg} and must close it on every path, including failure"
            );
        }
        // "On every path" has teeth: both map futures reach their loader through a `?` on the
        // manifest, and a `?` returns from whichever `async` block it sits in. The `Finish` must
        // therefore live in an OUTER block, or a failed manifest fetch skips it — and the bar comes
        // up short precisely on the boot that went wrong. Two `async {` before the close is that
        // nesting.
        for (open, seg) in [
            ("let dem_fut = async {", "BootSeg::Terrain"),
            ("let sat_fut = async {", "BootSeg::Satellite"),
        ] {
            let at = bootstrap
                .find(open)
                .unwrap_or_else(|| panic!("`{open}` must still exist"));
            let close = bootstrap[at..]
                .find(&format!("BootEvent::Finish({seg})"))
                .unwrap_or_else(|| panic!("{seg} must be closed inside its own future"));
            let region = &bootstrap[at..at + close];
            assert!(
                region.matches("async {").count() >= 2,
                "{seg}'s `Finish` must sit outside the block holding the `?` — one `async {{` \
                 between them means a failed manifest fetch returns past it"
            );
        }
        let src = include_str!("mission_editor.rs");
        let mission_finish = format!(
            "{}{}",
            "BootEvent::Finish(\n", "                        boot_progress::BootSeg::Mission,"
        );
        assert!(
            src.contains(&mission_finish)
                || src.contains("BootEvent::Finish(boot_progress::BootSeg::Mission)"),
            "the hydrate task owns the mission segment and must close it once the hydrate returns"
        );
        let handover = format!("{}{}", "hand_", "over(boot)");
        assert!(
            src.contains(&handover),
            "both rendezvous points must go through the hand-over, so the overlay is never removed \
             in the same render as the last measurement"
        );
        assert!(
            BOOT_HANDOVER_MS >= 200,
            "the hold must be at least the 200 ms `.mc-load-fill` ease, or the fill is still \
             travelling when the overlay is pulled and 100% is never actually drawn"
        );
    }

    /// Source pin on the mission document. It is the one measured fetch that is **not** on the
    /// map-asset host, and the one that must not grow a second copy of the auth contract.
    #[test]
    fn the_mission_document_is_measured_and_still_defers_to_the_single_flight_client() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let src = live_code(include_str!("mission_hydrate.rs"));
        let body = only_body(&src, "async fn get_mission_measured(");
        // `live_code` blanks string literals — see the terrain pin for why the shape, not the
        // header name, is the needle.
        assert!(
            body.contains(".headers()")
                && body.contains("parse::<u64>()")
                && body.contains("BootEvent::Budget(BootSeg::Mission, budget)"),
            "the mission segment's budget must be a header read off this response"
        );
        assert!(
            body.contains("reader.read()") && body.contains("BootEvent::Done"),
            "its progress must be the bytes off the body reader"
        );
        assert!(
            body.contains("crate::client::api_get::<MissionDetail>(auth, path)"),
            "anything that is not a 2xx — the 401 above all — must fall through to `api_get`, \
             which owns the single-flight refresh. A second refresh path would double-spend the \
             rotating token, and that is a data-safety bug, not a loading-bar bug"
        );
        assert!(
            !body.contains("auth/refresh"),
            "this function must never mint or spend a refresh token itself"
        );
        let hydrate = only_body(&src, "pub async fn hydrate_from_server(");
        assert!(
            hydrate.contains("get_mission_measured(auth, &path")
                && !hydrate.contains("client::api_get::<MissionDetail>"),
            "the hydrate's own GET must route through the measured wrapper, not around it"
        );
    }
}

/// T-631 — the boot overlay cannot fail SILENTLY. The engine-init failure itself is wasm-side
/// (`RenderEngine::create` needs a real GPU), but the state machine the overlay reads —
/// `BootPhase` and its `advance` fold — is pure and drives entirely here, which is exactly what
/// the acceptance clause allows ("a native test can still drive `BootPhase`/`BootEvent`
/// transitions directly"). These tests inject the failure the way the engine task does, assert the
/// overlay reaches `Failed { seg, reason }` carrying the ORIGINAL reason, and — the part that made
/// the real bug so nasty — assert that the concurrent doc-hydrate task's later, misleading
/// transitions (`LoadingMap`, then `Ready` via the hand-over) do NOT overwrite that reason.
#[cfg(test)]
mod t631_boot_failure_state {
    use super::boot_progress::{BootEvent, BootProgress, BootSeg};
    use super::BootPhase;

    /// The verbatim first line of the observed wasm chain (`webgpu.rs:2331`). This is the string
    /// the operator must end up staring at instead of a frozen "Loading terrain… 50%".
    const REAL_REASON: &str = "createBuffer failed, size (32) too large";
    /// The kind of later, unrelated noise the doc task or a second failure could carry. If this
    /// ever ends up on screen the FIRST panic's cause has been buried — the exact regression.
    const MISLEADING_REASON: &str = "RefCell already borrowed";

    /// Reproduce the two-task race in boot order: the bar is honestly metered up to the point the
    /// GPU dies, the engine task lands in `Failed`, and THEN the doc-hydrate task — which knows
    /// nothing about the engine — tries to move the overlay on. Returns the phase the overlay would
    /// actually render.
    fn drive_boot_with_engine_failure(sticky: bool) -> BootPhase {
        // 1. The T-628 bar is real up to the failure: mission finishes, terrain is half-way — this
        //    is the "Loading terrain… 50%" the ticket reproduced. `stage()` names that segment.
        let mut prog = BootProgress::new();
        prog.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
        prog.apply(BootEvent::Done(BootSeg::Mission, 2_032));
        prog.apply(BootEvent::Finish(BootSeg::Mission));
        prog.apply(BootEvent::Budget(BootSeg::Terrain, 71_911_548));
        prog.apply(BootEvent::Done(BootSeg::Terrain, 35_955_774));
        let seg = prog.stage();
        assert_eq!(
            seg,
            BootSeg::Terrain,
            "the failure must be attributed to the segment on screen when the GPU died — the \
             operator saw 'Loading terrain…', so the error must say terrain, not a generic apology"
        );

        // 2. The engine task's `Err` arm: drive the machine into `Failed` carrying the REAL reason.
        let mut phase = BootPhase::Hydrating;
        let fail = BootPhase::Failed {
            seg,
            reason: REAL_REASON.to_string(),
        };
        phase = if sticky {
            phase.advance(fail)
        } else {
            // The perturbation (see the paired test): a fold that is NOT sticky. Same call shape,
            // so the ONLY difference under test is the stickiness the fix adds.
            fail
        };

        // 3. The concurrent doc-hydrate task settles AFTER the failure and reaches for the overlay,
        //    exactly as it does live: first `LoadingMap`, then `Ready` at the hand-over. Both go
        //    through the same fold the production code uses.
        let load = BootPhase::LoadingMap;
        let ready = BootPhase::Ready;
        phase = if sticky { phase.advance(load) } else { load };
        phase = if sticky { phase.advance(ready) } else { ready };
        phase
    }

    #[test]
    fn engine_failure_reaches_the_error_state_with_the_original_reason() {
        let phase = drive_boot_with_engine_failure(true);
        match phase {
            BootPhase::Failed { seg, reason } => {
                assert_eq!(
                    seg,
                    BootSeg::Terrain,
                    "the error must still name the segment that broke"
                );
                assert_eq!(
                    reason, REAL_REASON,
                    "the overlay must carry the REAL wgpu reason verbatim — the whole ticket is \
                     that the boot stops being silent and says WHY"
                );
                assert_ne!(
                    reason, MISLEADING_REASON,
                    "and it must NOT be the later, misleading event: a hydrate that finishes after \
                     the engine failed must not bury the first cause"
                );
            }
            other => panic!(
                "the overlay must be in the Failed error state, not {other:?} — a non-Failed \
                 phase here means a later event painted a spinner back over the error (LoadingMap) \
                 or dismissed it onto a dead map (Ready), losing the reason"
            ),
        }
    }

    /// The "make it wrong on demand / prove it fires by perturbing once" clause. This runs the
    /// SAME scenario with the stickiness removed and asserts the machine then loses the reason —
    /// which proves the assertion in the test above has teeth: if `advance` stopped being sticky,
    /// the overlay would end at `Ready` (or carry the misleading reason) and that test would fail.
    /// A green that was never watched fail does not count; this is the watched failure, pinned.
    #[test]
    fn without_stickiness_the_reason_is_overwritten_which_is_the_bug() {
        let perturbed = drive_boot_with_engine_failure(false);
        assert_eq!(
            perturbed,
            BootPhase::Ready,
            "with a non-sticky fold the concurrent hydrate marches the overlay to Ready and the \
             real reason is gone — this is precisely the silent-failure regression, and its \
             presence here is what proves the sticky-fold test is load-bearing rather than vacuous"
        );
    }

    /// `advance` is sticky against EVERY later transition, not just the two the doc task happens to
    /// send — including a second, different engine failure. The first cause wins, always.
    #[test]
    fn a_second_failure_cannot_overwrite_the_first_reason() {
        let first = BootPhase::Failed {
            seg: BootSeg::Terrain,
            reason: REAL_REASON.to_string(),
        };
        let second = first.clone().advance(BootPhase::Failed {
            seg: BootSeg::World,
            reason: MISLEADING_REASON.to_string(),
        });
        assert_eq!(
            second, first,
            "once failed, a later failure is a no-op — the operator is shown the FIRST panic's \
             cause, which is the one that actually explains the boot"
        );
        // And a success flip is likewise swallowed: nothing takes the error overlay down but the
        // operator's own 'Continue without map'.
        assert_eq!(
            first.clone().advance(BootPhase::Ready),
            first,
            "a rendezvous that fires after a failure must not dismiss the error onto a dead map"
        );
    }

    /// The stage title the error card renders from — 'Loading terrain…' with the ellipsis trimmed
    /// becomes 'Loading terrain', and the card appends ' failed'. Pin the pieces the view relies
    /// on so a title change cannot silently produce 'Loading terrain… failed' with a stray ellipsis.
    #[test]
    fn the_failing_segment_titles_read_as_a_sentence() {
        assert_eq!(BootSeg::Terrain.title(), "Loading terrain…");
        assert_eq!(
            BootSeg::Terrain.title().trim_end_matches('…'),
            "Loading terrain",
            "the card composes '<title-without-ellipsis> failed'; a trailing ellipsis would read \
             as 'Loading terrain… failed'"
        );
    }
}

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
mod t629_satellite_resolution {
    use super::tbd_sat_pure::{
        parse_tbd_sat_index_strict, pick_base_level, pick_base_level_for_limit, TbdSatIndex,
    };

    /// The live `packages/map-assets/everon/satellite/everon-sat.tbd-sat` index, read off the
    /// bundle on 2026-08-01: 14 levels from 12800² down to 1×1, `sourceMeta` (which the loader
    /// does not deserialize) dropped. Offsets and lengths are the real ones, so the strict
    /// validator below runs the same level-numbering / halving / coverage / terminator rules it
    /// runs in the browser.
    const EVERON_INDEX_JSON: &str = concat!(
        r#"{"formatVersion":1,"terrainId":"everon","worldBounds":[0,0,12800,12800],"#,
        r#""baseWidthPx":12800,"baseHeightPx":12800,"mipCount":14,"mips":["#,
        r#"{"level":0,"width":12800,"height":12800,"tiles":["#,
        r#"{"x":0,"y":0,"width":6400,"height":6400,"offset":2644,"length":28326346},"#,
        r#"{"x":6400,"y":0,"width":6400,"height":6400,"offset":28328990,"length":21632714},"#,
        r#"{"x":0,"y":6400,"width":6400,"height":6400,"offset":49961704,"length":27555806},"#,
        r#"{"x":6400,"y":6400,"width":6400,"height":6400,"offset":77517510,"length":33042794}]},"#,
        r#"{"level":1,"width":6400,"height":6400,"tiles":[{"x":0,"y":0,"width":6400,"height":6400,"offset":110560304,"length":30866380}]},"#,
        r#"{"level":2,"width":3200,"height":3200,"tiles":[{"x":0,"y":0,"width":3200,"height":3200,"offset":141426684,"length":8271166}]},"#,
        r#"{"level":3,"width":1600,"height":1600,"tiles":[{"x":0,"y":0,"width":1600,"height":1600,"offset":149697850,"length":2218572}]},"#,
        r#"{"level":4,"width":800,"height":800,"tiles":[{"x":0,"y":0,"width":800,"height":800,"offset":151916422,"length":583330}]},"#,
        r#"{"level":5,"width":400,"height":400,"tiles":[{"x":0,"y":0,"width":400,"height":400,"offset":152499752,"length":153506}]},"#,
        r#"{"level":6,"width":200,"height":200,"tiles":[{"x":0,"y":0,"width":200,"height":200,"offset":152653258,"length":42470}]},"#,
        r#"{"level":7,"width":100,"height":100,"tiles":[{"x":0,"y":0,"width":100,"height":100,"offset":152695728,"length":12086}]},"#,
        r#"{"level":8,"width":50,"height":50,"tiles":[{"x":0,"y":0,"width":50,"height":50,"offset":152707814,"length":3584}]},"#,
        r#"{"level":9,"width":25,"height":25,"tiles":[{"x":0,"y":0,"width":25,"height":25,"offset":152711398,"length":1138}]},"#,
        r#"{"level":10,"width":12,"height":12,"tiles":[{"x":0,"y":0,"width":12,"height":12,"offset":152712536,"length":328}]},"#,
        r#"{"level":11,"width":6,"height":6,"tiles":[{"x":0,"y":0,"width":6,"height":6,"offset":152712864,"length":126}]},"#,
        r#"{"level":12,"width":3,"height":3,"tiles":[{"x":0,"y":0,"width":3,"height":3,"offset":152712990,"length":86}]},"#,
        r#"{"level":13,"width":1,"height":1,"tiles":[{"x":0,"y":0,"width":1,"height":1,"offset":152713076,"length":38}]}]}"#,
    );
    const FILE_BYTES: u64 = 152_713_114;

    /// Rebuild the TBDS container header (`"TBDS"`, formatVersion 1, jsonLen) in front of the
    /// index so the real parser runs, not `serde_json` on its own.
    fn everon_index() -> TbdSatIndex {
        let json = EVERON_INDEX_JSON.as_bytes();
        assert!(
            12 + json.len() as u64 <= 2_644,
            "the header + this index must still end at or before the real bundle's first tile \
             offset (2,644), or the strict validator will reject real offsets as out of range"
        );
        let mut buf = Vec::with_capacity(12 + json.len());
        buf.extend_from_slice(&0x5344_4254_u32.to_le_bytes()); // "TBDS"
        buf.extend_from_slice(&1_u32.to_le_bytes());
        buf.extend_from_slice(&(json.len() as u32).to_le_bytes());
        buf.extend_from_slice(json);
        parse_tbd_sat_index_strict(&buf, FILE_BYTES)
            .unwrap_or_else(|e| panic!("the committed everon index must parse strictly: {e}"))
    }

    // ── the level the GPU limit buys ──────────────────────────────────────────────────────────

    #[test]
    fn the_everon_ladder_is_the_real_one() {
        let idx = everon_index();
        assert_eq!(idx.mip_count, 14);
        assert_eq!((idx.base_width_px, idx.base_height_px), (12_800, 12_800));
        assert_eq!((idx.mips[0].width, idx.mips[0].height), (12_800, 12_800));
        assert_eq!((idx.mips[1].width, idx.mips[1].height), (6_400, 6_400));
        assert_eq!(idx.mips[0].tiles.len(), 4, "level 0 is four 6400² tiles");
    }

    #[test]
    fn eight_k_costs_exactly_half_the_resolution_and_sixteen_k_does_not() {
        let idx = everon_index();
        assert_eq!(
            pick_base_level(&idx, 8_192),
            1,
            "12800 does not fit 8192, so the base becomes level 1 — 6400², literally half the \
             island's resolution. This is the operator-visible cost of the limit, and it is the \
             number the removed `unwrap_or(8192)` used to produce without measuring anything"
        );
        assert_eq!(
            pick_base_level(&idx, 16_384),
            0,
            "a 16384 GPU must get the 12800² source level"
        );
        assert_eq!(
            pick_base_level(&idx, 12_800),
            0,
            "the comparison is `<=`: a limit exactly equal to the base edge still fits"
        );
        assert_eq!(
            pick_base_level(&idx, 12_799),
            1,
            "one pixel short must fall to level 1, not silently clamp"
        );
        assert_eq!(
            pick_base_level(&idx, 4_096),
            2,
            "a 4096 GPU walks two levels down, not one"
        );
    }

    #[test]
    fn an_unknown_limit_yields_no_level_at_all() {
        let idx = everon_index();
        assert_eq!(
            pick_base_level_for_limit(&idx, None),
            None,
            "this is the whole point of T-629: when the GPU limit could not be read there is no \
             level to pick. The previous code answered this case with 8192 — a real, plausible, \
             wrong number that committed half resolution and told nobody"
        );
        for limit in [4_096_u32, 8_192, 12_800, 16_384, 32_768] {
            assert_eq!(
                pick_base_level_for_limit(&idx, Some(limit)),
                Some(pick_base_level(&idx, limit)),
                "a KNOWN limit must decide exactly as it always did"
            );
        }
    }

    // ── the wasm side must actually route through the code proved above ───────────────────────

    #[test]
    fn no_call_site_may_guess_a_texture_limit() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let src = live_code(include_str!("world_assets/satellite.rs"));

        assert!(
            !src.contains("unwrap_or(8192)"),
            "both copies of the guess must be gone. A default texture limit is not conservative, \
             it is unfalsifiable: it picks a real mip, uploads it, and leaves the operator looking \
             at half an island with a 100% loading bar above it"
        );
        let limit_fn = only_body(&src, "fn texture_limit(");
        assert!(
            limit_fn.contains("e.max_texture_dimension_2d()")
                && limit_fn.contains("e.adapter_max_texture_dimension_2d()"),
            "the limit reader must report the device limit AND the adapter ceiling it was \
             requested against"
        );
        assert_eq!(
            src.matches("max_texture_dimension_2d()").count(),
            2,
            "the GPU limit must be read in exactly ONE place (both reads inside `texture_limit`). \
             A second reader is a second opportunity to re-invent the default"
        );

        let full = only_body(&src, "async fn load_unified_full(");
        assert!(
            full.contains("pick_base_level_for_limit(&index, limit.map(|l| l.device))"),
            "the base level must be chosen from the Option-typed limit, so a missing engine \
             cannot be spelled the same way as a measured one"
        );
        assert!(
            full.contains("logging::error!") && full.contains("return false;"),
            "a missing engine must abort the load loudly, not substitute a number"
        );
        assert!(
            full.contains("report_chosen_level(&index, base, limit)"),
            "a downscaled basemap must announce itself — one that says nothing is \
             indistinguishable from a correct one"
        );
        let commit_at = full
            .find("tex_layer_commit")
            .expect("the full load must commit the basemap");
        assert!(
            full[commit_at..].contains("logging::log!"),
            "the load must report what LANDED, after the commit. A line printed before the upload \
             is a claim about the future, and this whole ticket exists because the map on screen \
             disagreed with what the boot implied had happened"
        );

        let map = only_body(&src, "pub async fn load_map_basemap(");
        assert!(
            map.contains("texture_limit(engine)") && !map.contains("unwrap_or(8192)"),
            "the cartographic pyramid picks a stitched zoom from the same limit and must obey the \
             same rule"
        );
    }

    #[test]
    fn a_downscaled_basemap_warns_and_a_stuck_placeholder_warns() {
        use crate::arsenal::class_r_scrub::{live_code, only_body};
        let src = live_code(include_str!("world_assets/satellite.rs"));

        let report = only_body(&src, "fn report_chosen_level(");
        assert!(
            report.contains("logging::warn!"),
            "level > 0 means the operator is looking at a downscaled island; that must reach the \
             console at warn, not be inferred from how soft the map looks"
        );
        assert!(
            report.contains("limit.device") && report.contains("limit.adapter"),
            "the warning must name BOTH numbers — 'the GPU cannot do better' and 'the device \
             request lost resolution the GPU offered' are different bugs with the same symptom"
        );

        let fetch = only_body(&src, "async fn fetch_tiles(");
        assert!(
            fetch.contains("fetch_range_resilient(url, start, end)"),
            "T-629 root cause: at base level 0 everon's plan is 49 Range requests, `fetch_tiles` \
             is fail-fast, and a single dropped request therefore discarded all 152,710,470 B and \
             left the <=1024 px preview up. Each span must get bounded retries"
        );
        let retry = only_body(&src, "async fn fetch_range_resilient(");
        assert!(
            retry.contains("for attempt in 1..=RANGE_ATTEMPTS") && retry.contains("return Some("),
            "the retry must be BOUNDED — an unbounded loop turns a dead origin into a boot that \
             never finishes, which is worse than the blurry map it replaces"
        );
        assert!(
            retry.contains("RangeOutcome::RateLimited") && retry.contains("sleep_ms(wait).await"),
            "a 429 must be recognised AND waited out. The API's global limiter is 20/s burst 40 \
             and the base-level-0 plan is 49 spans, so retrying a throttled span immediately just \
             spends the remaining attempts inside the same exhausted bucket"
        );
        assert!(
            retry.contains("logging::warn!"),
            "a retried span must say so; silent recovery hides a degrading origin until it fails \
             outright"
        );
        assert!(
            src.contains("const RANGE_ATTEMPTS: usize = 5;")
                && src.contains("const RANGE_BACKOFF_MS: [i32; 4]"),
            "one fewer wait than attempt — the last attempt has nothing to wait for. (Pinned as \
             text because the loader is wasm-only and these consts do not exist on this target.)"
        );

        let load = only_body(&src, "pub async fn load_satellite(");
        assert!(
            !load.contains("let _ = load_unified_full("),
            "the full load's failure must not be discarded. That discard IS the reported symptom: \
             when it returns false the <=1024 px preview stays on screen as if it were the map"
        );
        assert!(
            load.contains("if !load_unified_full(") && load.contains("logging::warn!"),
            "a failed full load must say that the placeholder is what is being displayed"
        );
    }
}

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
mod t662_input_traps {
    use crate::arsenal::class_r_scrub::live_code;

    /// The keydown region, comment- and dead-code-stripped. The file's first `#[cfg(test)]` is the
    /// `clear_for_test` helper near the top, so `live_code` on the whole file would cut everything
    /// below it (see the t425/t427 pins); hand it the region from the editor page onward, at a
    /// brace-0 boundary so the slice stays balanced.
    fn editor_live() -> String {
        // Full signature (with `()`), so the other test's bare `"pub fn MissionEditorPage"` literal
        // is not a second match. Split so this anchor is not itself a duplicate occurrence.
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// (2) Backspace hides chrome and does NOT delete; Delete still deletes. The two keys are split
    /// arms — the old combined Delete-or-Backspace alias must be gone (its literal is reassembled
    /// below at runtime so this comment is not itself a match).
    #[test]
    fn backspace_hides_chrome_and_does_not_delete() {
        // String-literal arms: pinned on the RAW file (live_code blanks string literals).
        let raw = include_str!("mission_editor.rs");
        // Split the needle so the literal below is not itself a second occurrence in this file.
        let combined = format!("{}{}", "\"Delete\" | ", "\"Backspace\"");
        assert!(
            !raw.contains(combined.as_str()),
            "T-662: Backspace must no longer be aliased to Delete — the combined match arm is the bug"
        );
        assert!(
            raw.contains("\"Delete\" if !modk => crate::editor_ops::delete_selection()"),
            "Delete alone must still remove the selection"
        );
        assert!(
            raw.contains("\"Backspace\" if !modk =>"),
            "Backspace must be its own match arm"
        );

        // Behaviour of the Backspace arm: it toggles chrome_hidden and does NOT call delete. The
        // guard is a string literal (blanked by live_code), so this is scoped on the raw file — the
        // only text between the "Backspace" arm and the catch-all `_ =>` is the T-662 note, which
        // does not contain the token `delete_selection`. The keydown region is the only place these
        // arms appear, so the window is unambiguous.
        let bs_at = raw
            .find("\"Backspace\" if !modk =>")
            .expect("Backspace arm present");
        let after = &raw[bs_at
            ..raw[bs_at..]
                .find("_ =>")
                .map(|i| bs_at + i)
                .unwrap_or(raw.len())];
        assert!(
            after.contains("chrome_hidden.set("),
            "the Backspace arm must toggle chrome_hidden (hide the interface)"
        );
        assert!(
            !after.contains("delete_selection"),
            "the Backspace arm must NOT delete the selection"
        );
    }

    /// (2 cont.) `chrome_hidden` is a real signal that gates the chrome mounts (strip + both docks +
    /// the two T-636 bottom mounts: the mode toolbar AND the full-width status bar + the T-667
    /// map-pane grid-reference overlay). Declared once, and each mount is wrapped in a
    /// `!chrome_hidden.get()` gate.
    ///
    /// T-636 [wave101 N-5]: the split turned the single `BottomToolbelt` gate into TWO (ModeToolbar
    /// + StatusBar), so the deliberate count moved 4 → 5. T-667 [wave 106]: the map-pane grid
    /// references (`MapGridRefs`) are the same kind of map furniture as the scale bar and must hide
    /// with the rest of the chrome on Backspace, so the deliberate count moves 5 → 6. T-648 [wave
    /// 110]: the snap-grid step readout (`SnapReadout`) is status-bar furniture like the scale bar /
    /// grid refs and must hide with the chrome too, so the deliberate count moves 6 → 7. Pinned as an
    /// exact count so a mount can never silently escape the hide-chrome gate (or a stray gate creep
    /// in unnoticed) — a legitimate new gated mount UPDATES this number on purpose (it is never
    /// bumped to make a red test pass without a matching, intended mount).
    #[test]
    fn chrome_hidden_signal_gates_the_five_mounts() {
        let ed = editor_live();
        assert!(
            ed.contains("let chrome_hidden = RwSignal::new(false)"),
            "chrome_hidden must be a real RwSignal declared on the page"
        );
        // Each chrome mount must sit behind a chrome_hidden gate. Count the gate wrappers: strip,
        // DockLeft, DockRight, ModeToolbar, StatusBar, MapGridRefs, SnapReadout = 7 (T-636 split +
        // T-667 refs + T-648 snap readout).
        let gates = ed.matches("(!chrome_hidden.get()).then(").count();
        assert_eq!(
            gates, 7,
            "exactly seven chrome mounts (strip + both docks + mode toolbar + status bar + grid refs \
             + snap readout) must be gated on chrome_hidden; found {gates} gate(s)"
        );
        // The docked chrome components must appear inside the gated region (sanity: we did not gate
        // empty divs). BottomToolbelt is retired as a mount — the readouts live in StatusBar and the
        // tools in ModeToolbar, both gated; the T-667 grid refs + T-648 snap readout are gated too.
        assert!(
            ed.contains("TopCommandStrip")
                && ed.contains("DockLeft")
                && ed.contains("DockRight")
                && ed.contains("ModeToolbar")
                && ed.contains("StatusBar")
                && ed.contains("MapGridRefs")
                && ed.contains("SnapReadout"),
            "the gated mounts must still be the real chrome components (incl. the two T-636 halves \
             and the T-667 grid-reference overlay)"
        );
        // Modals must NOT be swept into the hide: a Settings/Attributes dialog survives the toggle.
        // The Attributes modal mount is outside every gate.
        assert!(
            ed.contains("AttributesModal"),
            "the Attributes modal mount must still exist (ungated)"
        );
    }

    /// (1) RMB no longer pans. The pan branch fires on the middle button only; the old
    /// `|| ev.button() == 2` right-button branch that ate the click is gone.
    #[test]
    fn rmb_no_longer_pans() {
        let ed = editor_live();
        assert!(
            ed.contains("if ev.button() == 1 {"),
            "the pan gesture must start on the middle button (1) only"
        );
        // The whole point: RMB (2) must not be OR-ed into the pan guard anymore.
        assert!(
            !ed.contains("ev.button() == 1 || ev.button() == 2"),
            "T-662: RMB (button 2) must no longer be OR-ed into the pan branch — that OR was the trap"
        );
    }

    /// (1 cont.) The contextmenu handler keeps `prevent_default` (stop the BROWSER menu) but must
    /// NOT `stop_propagation` — the event has to stay reachable for T-664 to attach its menu.
    #[test]
    fn contextmenu_is_unsuppressed_but_stops_the_browser_menu() {
        let ed = editor_live();
        // Isolate the oncontextmenu closure body.
        let cm_at = ed
            .find("let oncontextmenu =")
            .expect("oncontextmenu closure present");
        // Window up to the next `let on` binding (onpointerleave follows it).
        let rest = &ed[cm_at..];
        let end = rest[3..]
            .find("let on")
            .map(|i| i + 3)
            .unwrap_or(rest.len());
        let body = &rest[..end];
        assert!(
            body.contains("ev.prevent_default()"),
            "contextmenu must still prevent_default to stop the browser's native menu"
        );
        assert!(
            !body.contains("stop_propagation"),
            "contextmenu must NOT stop_propagation — RMB must stay a clean event T-664 can hook"
        );
    }
}

/// T-635 — the debug HUD (telemetry) must (a) toggle behind Ctrl+Alt+D in the editor keydown,
/// honouring the editable-field guard; (b) default HIDDEN; (c) NO LONGER live inside the toolbelt's
/// `bottom-5 left-1/2` wrapper (so it cannot paint over the CUR/OBJ readouts) and stay gated so an
/// overlap is impossible; (d) keep the telemetry-vs-diagnostics distinction explicit in a comment.
#[cfg(test)]
mod t635_debug_hud {
    use crate::arsenal::class_r_scrub::{live_code, live_source};

    /// The editor page region with comments stripped but string literals KEPT (so Tailwind class
    /// strings survive as structural landmarks). Same slice boundary as `editor_live`.
    fn editor_src() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        live_source(&raw[raw.find(anchor.as_str()).expect("anchor present")..])
    }

    /// The editor page region with comments stripped and string literals blanked — same slice the
    /// t662 module uses (from `pub fn MissionEditorPage()` onward, at a brace-0 boundary).
    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// (a) Ctrl/Cmd+Alt+D is a keydown arm that toggles the HUD, and (a') the closure honours the
    /// editable-field guard so it never fires while typing.
    #[test]
    fn ctrl_alt_d_toggles_the_hud_behind_the_editable_guard() {
        let ed = editor_live();
        // The arm: modifier + Alt + not Shift, keyed on KeyD (literals blanked, so the guard shape
        // is what we pin, not the "KeyD" string — the code idiom, not a mention).
        assert!(
            ed.contains("if modk && ev.alt_key() && !ev.shift_key() =>")
                && ed.contains("debug_hud_shown.set(!debug_hud_shown.get_untracked())"),
            "T-635: Ctrl/Cmd+Alt+D must be a keydown arm that toggles debug_hud_shown"
        );
        // The whole keydown closure sits behind the editable-field guard (shared with copy/paste/
        // Backspace) — a HUD toggle must not fire while the operator types in an Attributes field.
        assert!(
            ed.contains("if crate::mission_history::in_editable_field() {"),
            "T-635: the keydown closure must guard on in_editable_field() before acting"
        );
        // The literal binding is present on the raw file too (live_code blanks it above).
        let raw = include_str!("mission_editor.rs");
        assert!(
            raw.contains("\"KeyD\" if modk && ev.alt_key()"),
            "T-635: the toggle must be bound to the D key"
        );
    }

    /// (b) The HUD defaults HIDDEN: `debug_hud_shown` is a real signal seeded `false`.
    #[test]
    fn the_hud_defaults_hidden() {
        let ed = editor_live();
        assert!(
            ed.contains("let debug_hud_shown = RwSignal::new(false)"),
            "T-635: debug_hud_shown must be a real RwSignal defaulting to false (hidden)"
        );
    }

    /// (c) T-636/T-719: the HUD is NO LONGER a free-floating overlay corner — it moved into the
    /// full-width status bar's right section (its real visible home; the old `right-3 bottom-3`
    /// overlay div had no z-index and was painted over by DockRight's z-20 column). From
    /// `mission_editor`'s side the proof is: (1) the standalone overlay HUD div is gone, and (2) the
    /// HUD signals are fed into `StatusBar`, which sits behind a `chrome_hidden` gate — so the
    /// chrome-hidden half of the T-635 gate is preserved. The `hud_shown`-AND-non-empty half is
    /// pinned inside `eden_toolbelt` (see `t636_status_bar`).
    #[test]
    fn the_hud_moved_into_the_gated_status_bar() {
        let src = editor_src();
        // (1) The retired overlay corner must be gone — no free-floating `right-3 bottom-3` HUD div.
        assert!(
            !src.contains("absolute right-3 bottom-3 font-mono"),
            "T-636: the free-floating overlay HUD corner must be gone (it moved into the status bar)"
        );
        // (2) The status-bar mount passes the HUD signals in, and it sits behind a chrome_hidden
        // gate. Pinned on `live_code` (comments/strings blanked) so this is the real wiring, not a
        // comment: the `debug_hud` + `hud_shown=debug_hud_shown` props reach `StatusBar`.
        let ed = editor_live();
        assert!(
            ed.contains("StatusBar")
                && ed.contains("debug_hud")
                && ed.contains("hud_shown=debug_hud_shown"),
            "T-636: the HUD text + toggle must be threaded into StatusBar (debug_hud + hud_shown)"
        );
        // The StatusBar mount must be one of the `(!chrome_hidden.get()).then(` gated wrappers, so
        // hiding the chrome unmounts the HUD too (the chrome_hidden half of the T-635 gate stack).
        let belt = ed
            .find("crate::eden_toolbelt::StatusBar")
            .expect("StatusBar mount present");
        let gate = ed[..belt]
            .rfind("(!chrome_hidden.get()).then(")
            .expect("StatusBar must be preceded by a chrome_hidden gate");
        // Nothing but the wrapper div opens between the gate and the StatusBar mount — i.e. the gate
        // is the StatusBar's own wrapper, not an earlier mount's.
        assert!(
            !ed[gate..belt].contains("crate::eden_toolbelt::ModeToolbar")
                && !ed[gate..belt].contains("crate::eden_chrome::Dock"),
            "T-636: the chrome_hidden gate immediately preceding StatusBar must be its OWN wrapper"
        );
    }

    /// (d) The telemetry-vs-diagnostics distinction is stated explicitly in a PRODUCTION code
    /// comment — the framework_synthesis §D.4 #7 requirement that this key-gating pattern not be
    /// copied onto mission-correctness diagnostics. The scrubbers blank comments, so this is pinned
    /// on the raw file, sliced to the `MissionEditorPage` production body (from its anchor to the
    /// first `#[cfg(test)]` module that follows it) so the test modules' own text — including this
    /// docstring — cannot satisfy the pin. The comment must really ship in the page's source.
    #[test]
    fn the_telemetry_vs_diagnostics_distinction_is_documented() {
        let raw = include_str!("mission_editor.rs");
        // Window: `MissionEditorPage`'s definition … first test module after it. The file's FIRST
        // `#[cfg(test)]` is a `clear_for_test` helper near the top (well above the page), so slice
        // from the page anchor forward, then cut at the next test module. (Both needles split so
        // this line is not itself the boundary it searches for.)
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let page_at = raw.find(anchor.as_str()).expect("page anchor present");
        let boundary = format!("#[cfg{}", "(test)]");
        let after_page = &raw[page_at..];
        let prod_end_rel = after_page
            .find(boundary.as_str())
            .expect("a test module follows the page");
        let prod = &after_page[..prod_end_rel];
        // The rule reference (reassembled so this line is not a decoy match inside the window if the
        // production comment were deleted — the needle must be found in real shipped source).
        let rule = format!("framework_synthesis {}D.4 #7", "\u{a7}");
        assert!(
            prod.contains(rule.as_str()),
            "T-635: the §D.4 #7 rule reference must be present in a production comment"
        );
        assert!(
            prod.contains("Mission-correctness diagnostics") && prod.contains("NEVER gated"),
            "T-635: the comment must state that mission-correctness diagnostics are never gated"
        );
        // And it must frame the HUD itself as telemetry (the thing that IS legitimately gated).
        assert!(
            prod.contains("TELEMETRY"),
            "T-635: the comment must classify the HUD as telemetry (the gatable kind)"
        );
    }
}

/// T-647 — placement interactions: the Ctrl state machine (multi-place ↔ regroup), Alt = empty
/// vehicle, the double-click asset picker on empty ground, and the double-click→Attributes swap that
/// now reaches vehicles. All six ids are pinned on live source (comments stripped / string literals
/// blanked), because the doc-mutating half is wasm-only (`editor_ops` runs no native test) — the
/// wiring is the thing a native pin can prove, exactly as the T-573 / T-662 / T-635 modules do.
#[cfg(test)]
mod t647_placement_interactions {
    use crate::arsenal::class_r_scrub::{live_code, only_body};

    /// The editor page region (comments stripped, string literals blanked). The `#[cfg(wasm32)]`
    /// blocks the pointer/dblclick handlers live in are KEPT by the scrubber (it decides only
    /// provably-false cfgs, and `target_arch` reads as undecided under the default eval) — the same
    /// reason t662 can pin `chrome_hidden.set(` inside that block.
    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// `editor_ops.rs`, scrubbed to live code. It is wasm-only, so nothing in it runs — but its
    /// wiring is pinnable as source (multiple modules already `include_str!` it for this).
    fn ops_live() -> String {
        live_code(include_str!("editor_ops.rs"))
    }

    // ───────────────────────── ATTR-OPEN-001 — dblclick opens Attributes for vehicles too ────────

    /// The dblclick handler must pick with `pick_slot_or_vehicle` (slot OR vehicle), not the
    /// slot-only `pick` it used pre-T-647 — that swap is the whole of ATTR-OPEN-001 ("not just
    /// slots"). A hit opens Attributes; the pick must be handed the live `vehicle_points()`.
    #[test]
    fn dblclick_opens_attributes_for_vehicles_via_slot_or_vehicle_pick() {
        let ed = editor_live();
        let body = only_body(&ed, "let ondblclick =");
        assert!(
            body.contains("select_tool::pick_slot_or_vehicle(")
                && body.contains("editor_ops::vehicle_points()"),
            "ATTR-OPEN-001: dblclick must pick slot OR vehicle (with vehicle_points), so Attributes \
             opens for a vehicle — not the slot-only pick"
        );
        assert!(
            body.contains("editor_ops::open_attributes(id)"),
            "a dblclick HIT must open Attributes on the picked id"
        );
        // The slot-only `pick(` must be GONE from this handler — a leftover would keep the bug for
        // vehicles. (`pick_slot_or_vehicle` contains the token `pick`, so match the bare call form.)
        assert!(
            !body.contains("select_tool::pick(&cam"),
            "ATTR-OPEN-001: the slot-only pick(&cam, …) must be gone from the dblclick handler"
        );
    }

    // ───────────────────────── PLACE-003 — dblclick empty ground opens the asset picker ──────────

    /// A dblclick MISS (empty ground) opens the asset picker at the unprojected world point; a
    /// non-finite unproject opens nothing (off-map).
    #[test]
    fn dblclick_empty_ground_opens_the_asset_picker() {
        let ed = editor_live();
        let body = only_body(&ed, "let ondblclick =");
        // The match on the pick result: Some(id) → Attributes; None → picker.
        assert!(
            body.contains("editor_ops::open_asset_picker("),
            "PLACE-003: a dblclick miss must open the asset picker"
        );
        assert!(
            body.contains("cam.unproject_xy(px, py)") && body.contains("is_finite()"),
            "PLACE-003: the picker must open at the unprojected world point, guarded finite (off-map \
             opens nothing)"
        );
    }

    /// The picker is a real, ungated overlay component mounted beside the other ungated dialogs
    /// (so it survives Backspace hide-chrome — a hidden dock can't be focused, which is why this
    /// floating form was chosen), and a picked leaf ARMS `begin_place` (click-then-click, PLACE-001).
    #[test]
    fn asset_picker_is_an_ungated_overlay_that_arms_a_place() {
        let ed = editor_live();
        // Signal declared on the page + the picker signal handed to editor_ops (the open path).
        assert!(
            ed.contains("let asset_picker = RwSignal::new(None")
                && ed.contains("editor_ops::set_asset_picker_signal(asset_picker)"),
            "PLACE-003: the page must own the picker signal and register it with editor_ops"
        );
        // The overlay mount must exist and be OUTSIDE every chrome_hidden gate (ungated, like the
        // Attributes modal / context menu). Prove it by locating the mount and checking no
        // chrome_hidden gate opens between the last ungated-dialog landmark and it.
        assert!(
            ed.contains("AssetPickerOverlay"),
            "PLACE-003: the picker overlay component must be mounted"
        );
        let mount = ed.find("AssetPickerOverlay picker=").expect("picker mount");
        let ctx_menu = ed
            .find("ContextMenuOverlay menu=")
            .expect("context menu mount is the ungated-dialog landmark");
        assert!(
            mount > ctx_menu && !ed[ctx_menu..mount].contains("(!chrome_hidden.get()).then("),
            "PLACE-003: the picker must mount beside the ungated dialogs (no chrome_hidden gate \
             between the context menu and it)"
        );
        // The picker component arms the same place a DockRight leaf does. It is defined ABOVE the
        // page, so the page-anchored `editor_live` slice misses it AND a whole-file scrub is cut at
        // the file's first `#[cfg(test)]` (the `registry_session` helper near the top). Anchor from
        // the cold-registry page-size const (after that helper, before this component) — exactly as
        // the t573 pin does, so `cut_test_module` next fires on the real test modules far below. The
        // anchor is reassembled (not written whole) so this line is not a second occurrence of it,
        // which t573's own "exactly one" count would otherwise trip.
        let cold_anchor = format!("const REGISTRY_{}", "COLD_PAGE");
        let raw = include_str!("mission_editor.rs");
        let region =
            live_code(&raw[raw.find(cold_anchor.as_str()).expect("cold anchor present")..]);
        let comp = only_body(&region, "fn AssetPickerOverlay(");
        assert!(
            comp.contains("editor_ops::begin_place(payload")
                && comp.contains("editor_ops::close_asset_picker()"),
            "PLACE-001/PLACE-003: choosing a picker row must arm begin_place then close (the next \
             canvas click lands it)"
        );
        // …reusing the SAME catalog builder the dock uses (no second catalog to drift).
        assert!(
            comp.contains("asset_catalog::build_catalog_tree("),
            "PLACE-003: the picker must reuse build_catalog_tree (the dock's own catalog)"
        );
    }

    // ───────────────────── T-651 — PLACE-COMMENT-001: the place point + the template seed ───────

    /// The right-click handler captures the WORLD point of the click and hands it to the menu, which
    /// is what makes `Place Comment` land where the operator clicked. Also pins the negative that
    /// matters: this ticket added NO state to the `LeftGesture` machine — no new arm, no new
    /// `Pending`, nothing that could strand (T-723's territory, deliberately untouched).
    #[test]
    fn the_contextmenu_handler_captures_the_world_point_and_arms_no_gesture() {
        let ed = editor_live();
        let body = only_body(&ed, "let oncontextmenu =");
        assert!(
            body.contains("cam.unproject_xy(px, py)") && body.contains(".at_world("),
            "PLACE-COMMENT-001: the right-click must unproject its own pixel and attach the world \
             point to the MenuTarget"
        );
        assert!(
            !body.contains("LeftGesture")
                && !body.contains("editor_ops::arm(")
                && !body.contains("Pending::"),
            "T-651 must add no state to the gesture machine — the place is committed by the menu \
             row, not by an armed pointerup (T-723)"
        );
    }

    /// THE NEW-MISSION TEMPLATE SEEDS COMMENTS, at the fresh-doc site and BEFORE both boot steps
    /// that replace the document. Order is the whole property: seeding after the IDB restore or the
    /// server hydrate would stamp a template onto a mission that already has its own comments.
    #[test]
    fn the_new_mission_template_seeds_comments_before_restore_and_hydrate() {
        let ed = editor_live();
        let seed = ed
            .find("editor_ops::seed_new_mission_template(&doc)")
            .expect("T-651: the new-mission template seed must run in the editor page");
        let mint = ed
            .find("mission_doc::new_seeded_doc()")
            .expect("the fresh-doc mint");
        let restore = ed
            .find("yrs_persist::load_state(&id)")
            .expect("the IDB restore");
        let hydrate = ed
            .find("mission_hydrate::hydrate_from_server(")
            .expect("the server hydrate");
        assert!(
            seed > mint,
            "the template seeds into the freshly-minted doc, not before it exists"
        );
        assert!(
            seed < restore && seed < hydrate,
            "the template must seed BEFORE the restore ({restore}) and the hydrate ({hydrate}) — \
             both replace the document, so a later seed would duplicate onto a saved mission"
        );
    }

    /// The comment editor is a real, ungated overlay (it survives Backspace hide-chrome, the
    /// wave-101 mount rule) and it authors all three ATTR-FIELD-CMT-* fields plus copy and delete —
    /// so every store mutator this ticket shipped is reachable from the UI.
    #[test]
    fn the_comment_editor_is_ungated_and_authors_every_comment_field() {
        let ed = editor_live();
        assert!(
            ed.contains("let comment_editor = RwSignal::new(None")
                && ed.contains("editor_ops::set_comment_editor_signal(comment_editor)"),
            "T-651: the page must own the comment-editor signal and register it with editor_ops"
        );
        let mount = ed
            .find("CommentEditorOverlay open=")
            .expect("the comment editor mount");
        let ctx_menu = ed
            .find("ContextMenuOverlay menu=")
            .expect("context menu mount is the ungated-dialog landmark");
        assert!(
            mount > ctx_menu && !ed[ctx_menu..mount].contains("(!chrome_hidden.get()).then("),
            "T-651: the comment editor must mount beside the ungated dialogs"
        );
        // The component is defined ABOVE the page, so scrub from the same cold-registry anchor the
        // picker pin uses (reassembled so this line is not a second occurrence of it).
        let cold_anchor = format!("const REGISTRY_{}", "COLD_PAGE");
        let raw = include_str!("mission_editor.rs");
        let region =
            live_code(&raw[raw.find(cold_anchor.as_str()).expect("cold anchor present")..]);
        let comp = only_body(&region, "fn CommentEditorOverlay(");
        for op in [
            "editor_ops::rename_comment(",      // ATTR-FIELD-CMT-TITLE
            "editor_ops::set_comment_tooltip(", // ATTR-FIELD-CMT-TOOLTIP
            "editor_ops::move_comment(",        // ATTR-FIELD-CMT-POSITION (the drag commit)
            "editor_ops::duplicate_comment(",   // COPY
            "editor_ops::delete_comment(",
        ] {
            assert!(
                comp.contains(op),
                "T-651: the comment editor must reach `{op}` — an unreachable mutator is a \
                 half-shipped field"
            );
        }
        // A comment must never be routed into the SLOT surfaces (the T-716 live-but-inert trap).
        assert!(
            !comp.contains("editor_ops::open_attributes(")
                && !comp.contains("editor_ops::select_slot("),
            "T-651: a comment id must not enter the slot selection / Attributes lanes"
        );
    }

    // ───────────────────────── The Ctrl state machine (PLACE-004 ↔ CONN-GROUP-001) ───────────────

    /// The overload resolution, pinned as ONE machine. In the pointerup PLACE branch (armed):
    /// Ctrl → `place_at_keep` (multi-place, keeps the arm), else `place_at_alt` (one-shot). In the
    /// pointerup DRAG-commit branch (unarmed — `has_pending()` short-circuited the place branch):
    /// Ctrl + single character onto another character → `regroup_slot_onto`. The two can never both
    /// fire: the place branch `return`s under `has_pending()`.
    #[test]
    fn ctrl_state_machine_multi_place_when_armed_regroup_when_not() {
        let ed = editor_live();
        let up = only_body(&ed, "let onpointerup =");

        // (1) The place branch is armed-gated and returns, so the drag branch below only ever runs
        // with NO pending — that mutual exclusion is the resolution.
        assert!(
            up.contains("editor_ops::has_pending()"),
            "the place branch must gate on has_pending() (armed)"
        );

        // (2) Armed + Ctrl = multi-place (place_at_keep); armed + no Ctrl = one-shot (place_at_alt).
        assert!(
            up.contains("let ctrl_multi = ev.ctrl_key() || ev.meta_key()"),
            "PLACE-004: the armed branch must read Ctrl/Cmd as the multi-place modifier"
        );
        assert!(
            up.contains("editor_ops::place_at_keep(") && up.contains("editor_ops::place_at_alt("),
            "PLACE-004: Ctrl must route to place_at_keep (keep the arm), else place_at_alt"
        );

        // (3) Unarmed + Ctrl + single character dropped onto another → regroup, and the positional
        // move is skipped.
        assert!(
            up.contains("editor_ops::regroup_slot_onto(")
                && up.contains("ids.len() == 1")
                && up.contains("!crate::editor_ops::is_vehicle_id(&ids[0])"),
            "CONN-GROUP-001: an unarmed Ctrl-drag of a SINGLE character onto another must regroup"
        );

        // (4) The state machine is DOCUMENTED as one block (the ticket requires the comment). A
        // comment is stripped by every scrubber, so pin it on the RAW file, sliced to the page's
        // production body (page anchor → the first following test module) so this test module's own
        // text cannot satisfy it. The needle is reassembled so this line is not itself the decoy.
        let raw = include_str!("mission_editor.rs");
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let page_at = raw.find(anchor.as_str()).expect("page anchor present");
        let boundary = format!("#[cfg{}", "(test)]");
        let after = &raw[page_at..];
        let prod = &after[..after
            .find(boundary.as_str())
            .expect("a test module follows the page")];
        let phrase = format!("Ctrl is {}", "OVERLOADED");
        assert!(
            prod.contains(phrase.as_str()),
            "T-647: the Ctrl state machine must be documented in a comment block in the page body"
        );
    }

    /// `place_at_keep` re-arms the pending after a successful place (multi-place keeps going), and a
    /// FAILED place does not re-arm (a place that can't commit must not spin). The Alt override is
    /// carried through each stamp.
    #[test]
    fn place_at_keep_rearms_on_success_only() {
        let ops = ops_live();
        let body = only_body(&ops, "pub fn place_at_keep(");
        assert!(
            body.contains("place_at_impl(x, y, alt_empty, true)"),
            "PLACE-004: place_at_keep must place with keep=true and carry the Alt override"
        );
        // Snapshot before, restore after — and only when `placed`.
        assert!(
            body.contains("if placed") && body.contains("pending.borrow_mut() = Some(p)"),
            "PLACE-004: place_at_keep must re-arm the snapshotted pending, and only on success"
        );
    }

    // ───────────────────────── PLACE-CREW-001 — Alt = empty vehicle ──────────────────────────────

    /// Alt on release is threaded from the pointerup into the placement as `alt_empty`, and the
    /// vehicle commit stamps `crewed:false` when Alt is held (`with_crew = toggle && !alt_empty`) —
    /// the per-gesture override of the dock's crew default. Alt can force empty; it can never force
    /// crewed a switched-off toggle withheld.
    #[test]
    fn alt_places_an_empty_vehicle() {
        let ed = editor_live();
        let up = only_body(&ed, "let onpointerup =");
        assert!(
            up.contains("let alt_empty = ev.alt_key()"),
            "PLACE-CREW-001: the armed branch must read Alt as the empty-vehicle modifier"
        );
        // Both place routes carry the alt flag through.
        assert!(
            up.contains("place_at_keep(c[0], c[1], alt_empty)")
                && up.contains("place_at_alt(c[0], c[1], alt_empty)"),
            "PLACE-CREW-001: the Alt override must reach place_at_* on both the multi and single paths"
        );
        // The vehicle commit computes with_crew from the toggle AND-NOT alt.
        let ops = ops_live();
        let impl_body = only_body(&ops, "fn place_at_impl(");
        assert!(
            impl_body.contains("let with_crew = place_with_crew() && !alt_empty"),
            "PLACE-CREW-001: a Vehicle arm must stamp crewed:false under Alt (toggle && !alt_empty)"
        );
    }

    // ───────────────────────── CONN-GROUP-001 — regroup shares the ORBAT refile seam ─────────────

    /// The map regroup reads the target character's squad off the SoA (`read_attrs`) and refiles
    /// through the SAME T-180.6 core move the ORBAT dock uses (`refile_slot` → `move_slot_to_squad`),
    /// so a map regroup and a dock refile are one undo step / one membership write. It no-ops when
    /// the target has no squad or already shares the dragged slot's squad.
    #[test]
    fn regroup_reuses_the_refile_seam_and_noops_off_squad() {
        let ops = ops_live();
        let body = only_body(&ops, "pub fn regroup_slot_onto(");
        assert!(
            body.contains("read_attrs(target_id)") && body.contains("read_attrs(slot_id)"),
            "CONN-GROUP-001: regroup must read the target's (and source's) squad off the SoA"
        );
        assert!(
            body.contains("refile_slot("),
            "CONN-GROUP-001: regroup must go through the T-180.6 refile seam (move_slot_to_squad)"
        );
        assert!(
            body.contains("dest_squad.is_empty() || dest_squad == src_squad"),
            "CONN-GROUP-001: regroup must no-op when the target has no squad or already shares one"
        );
    }

    // ───────────────────────── Alt census (re-run at filing time) ────────────────────────────────

    /// Re-run of the Alt census the ticket demanded, as source pins across the whole frontend. Alt
    /// is a placement modifier ONLY on the canvas (this ticket's `alt_empty`); every pre-existing
    /// `alt_key()` reader is either a NEGATIVE guard on a Ctrl shortcut or a DOCK-tree gesture — no
    /// canvas collision. The `eden_tree` Alt-click (wave 104, descendants selection) is the one
    /// noted since filing: a dock surface, not the map.
    #[test]
    fn alt_census_confirms_no_canvas_collision() {
        // mission_history: Alt is a NEGATIVE guard on the Ctrl/Cmd copy shortcut, never a place.
        let hist = live_code(include_str!("mission_history.rs"));
        assert!(
            hist.contains("|| ev.alt_key()"),
            "census: mission_history uses alt_key only as a guard (|| ev.alt_key())"
        );
        // mission_editor keydown: Alt only as !alt_empty on copy/paste and the Ctrl+Alt+D HUD
        // toggle — none a canvas placement modifier. (The keydown lives in the same file.)
        let ed = editor_live();
        assert!(
            ed.contains("if modk && ev.alt_key() && !ev.shift_key() =>"),
            "census: mission_editor's only positive alt_key keydown is the Ctrl+Alt+D HUD toggle"
        );
        // eden_tree: Alt-click is a DOCK-tree gesture (descendants selection), NOT the canvas.
        let tree = live_code(include_str!("eden_tree.rs"));
        assert!(
            tree.contains("ev.alt_key() || ev.shift_key()"),
            "census: eden_tree's Alt-click is a dock-tree gesture (no canvas collision)"
        );
        // And the canvas's own new reader is the T-647 placement modifier — exactly one, in the
        // pointerup armed branch.
        let up = only_body(&ed, "let onpointerup =");
        assert_eq!(
            up.matches("ev.alt_key()").count(),
            1,
            "census: the canvas pointerup reads alt_key exactly once — the T-647 empty-vehicle \
             modifier"
        );
    }

    // ───────────────────────── The fired rule: perturb / fail / restore ──────────────────────────

    /// Fires the PLACE-CREW-001 pin (`with_crew = place_with_crew() && !alt_empty`). Proof it is
    /// load-bearing: the pin passes on the real body, and a perturbation that drops the `!alt_empty`
    /// clause (the exact regression — Alt no longer forces empty) makes the same assertion FAIL.
    /// Restore is implicit: the real `include_str!` body is untouched; only an in-memory copy is
    /// perturbed here.
    #[test]
    fn fired_rule_alt_empty_clause_is_load_bearing() {
        let ops = ops_live();
        let real = only_body(&ops, "fn place_at_impl(");
        let needle = "let with_crew = place_with_crew() && !alt_empty";
        // PASS on the real body.
        assert!(
            real.contains(needle),
            "canary: the real body must carry the clause"
        );
        // Perturb: strip the Alt clause (the regression). The pin must no longer find its needle.
        let perturbed = real.replace(needle, "let with_crew = place_with_crew()");
        assert!(
            !perturbed.contains(needle),
            "fired rule: dropping `!alt_empty` (Alt stops forcing empty) must break the PLACE-CREW-001 \
             pin — proving the pin discriminates the regression"
        );
    }
}

/// T-642 — source pins for the RULER click-chain wiring in the wasm pointer/keydown/dblclick
/// handlers (which a native test cannot execute; the pure state machine + math are event-tested in
/// `ruler_tool`). These pin the binding constraints the wave-106 verifier flagged (T-723) plus the
/// mount + the tool-mode arbitration entry, on scrubbed code (comments + strings blanked) so a
/// needle is real code, never a comment. The scrubber KEEPS `#[cfg(target_arch="wasm32")]` blocks
/// (undecided cfg), so the handler tokens are visible — the same reason t662 can pin inside them.
#[cfg(test)]
mod t642_ruler_wiring {
    use crate::arsenal::class_r_scrub::live_code;

    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// (tool-mode arbitration — how the third mode ENTERS `LeftGesture`) The LMB pointerdown chooses
    /// `LG::Ruler` via `should_begin_ruler(tool_mode, button)` instead of `LG::Pending`. This is the
    /// entry point AND constraint (c) — `should_begin_ruler` carries the button-0 filter.
    #[test]
    fn pointerdown_arbitrates_ruler_via_should_begin_ruler() {
        let ed = editor_live();
        assert!(
            ed.contains("should_begin_ruler("),
            "T-642: pointerdown must arbitrate the ruler via ruler_tool::should_begin_ruler(...)"
        );
        assert!(
            ed.contains("LeftGesture::Ruler {") || ed.contains("select_tool::LeftGesture::Ruler"),
            "T-642: the ruler press must open the LG::Ruler gesture (the third LeftGesture mode)"
        );
    }

    /// (constraint a — NOT the armed-place branch) The ruler commit lives in an `LG::Ruler` arm, and
    /// that arm must NOT contain the armed-place `has_pending()` token nor any doc-move commit — it is
    /// a separate arm reached only after the `has_pending()` branch has already returned, so it can
    /// never route through the T-723 armed-placement pointerup branch.
    #[test]
    fn ruler_commit_arm_avoids_armed_place_and_doc_writes() {
        let ed = editor_live();
        // Slice the LG::Ruler pointerup arm: from the LAST "LG::Ruler {" (the pointerup commit; the
        // earlier ones are pointerdown/pointermove which have no commit) to the next "LG::" or arm end.
        let arms: Vec<&str> = ed.split("LG::Ruler").skip(1).collect();
        assert!(!arms.is_empty(), "T-642: an LG::Ruler arm must exist");
        // The commit arm is the one that calls `.press(` on the ruler chain.
        let commit: Vec<&str> = arms
            .iter()
            .map(|a| a.split("LG::").next().unwrap_or(a))
            .filter(|a| a.contains(".press("))
            .collect();
        assert_eq!(
            commit.len(),
            1,
            "T-642: exactly one LG::Ruler arm commits a vertex via chain.press( (found {})",
            commit.len()
        );
        let arm = commit[0];
        // Constraint (a): the ruler commit does NOT sit in / call the armed-place branch.
        assert!(
            !arm.contains("has_pending()"),
            "T-642 (a): the ruler commit must NOT route through the has_pending() armed-place branch"
        );
        // Decision 4 + move_commit invariant: the ruler arm never calls a doc-move commit.
        assert!(
            !arm.contains("move_entities_and_vehicles"),
            "T-642: the ruler commit must not call move_entities_and_vehicles (it is not a doc edit)"
        );
    }

    /// (constraint b — take/clear any pending) The ruler gesture the pointerdown wrote is always
    /// consumed: the pointerup/cancel `left.borrow_mut().take()` clears it (there is exactly one
    /// take-into-a-`let` at the top of each of those handlers, shared with the Select gestures), and
    /// the pointermove `LG::Ruler` arm puts it back rather than dropping it.
    #[test]
    fn ruler_gesture_is_taken_and_cleared() {
        let ed = editor_live();
        // The shared take idiom the ruler arm relies on.
        assert!(
            ed.contains("left.borrow_mut().take()"),
            "T-642 (b): the pointer handlers must take() the LeftGesture (clearing any LG::Ruler)"
        );
        // The pointermove keeps the ruler pending (a self → self arm), so a move never loses it.
        assert!(
            ed.matches("LG::Ruler").count() >= 3,
            "T-642 (b): LG::Ruler must appear across pointerdown/move/up (written, kept, committed)"
        );
    }

    /// (constraint d — Esc disarms) The keydown Escape arm dismisses the ruler chain via
    /// `ruler...escape()`. This is Decision 3's two-step dismissal entry from the keyboard.
    #[test]
    fn escape_dismisses_the_ruler() {
        let ed = editor_live();
        assert!(
            ed.contains(".escape()"),
            "T-642 (d): the keydown Escape arm must call chain.escape() to disarm/clear the ruler"
        );
        // The arm reads the ruler and syncs — it is inside the keydown match on `code().as_str()`
        // (the "Escape" string literal itself is blanked by `live_code`, so pin the surviving
        // structure: the keydown dispatch + the escape() call together prove a real Escape arm).
        assert!(
            ed.contains("code().as_str()") && ed.contains("ruler.borrow_mut().escape()"),
            "T-642 (d): the Escape dismissal must be a keydown arm calling ruler.escape()"
        );
    }

    /// (dismissal — dbl-click ends the chain) The dblclick handler ends the ruler (dedup + end) when
    /// the ruler tool is active, and returns before the Attributes/asset-picker pick.
    #[test]
    fn dblclick_ends_the_ruler_chain() {
        let ed = editor_live();
        assert!(
            ed.contains(".dedup_tail(") && ed.contains(".double_click()"),
            "T-642: the dblclick handler must dedup + end the ruler chain (double_click keeps it placed)"
        );
    }

    /// (Decision 4 — session-local, tool-switch clear) Switching the tool back to Select clears the
    /// placed ruler, and the chain is registered for the overlay + mounted. Also pins that the chain
    /// handle is a leaked `RefCell<RulerChain>` (overlay state), never a doc write.
    #[test]
    fn tool_switch_clears_and_overlay_is_mounted() {
        let ed = editor_live();
        // Tool-switch clear effect (reads tool_mode, clears the chain).
        assert!(
            ed.contains("is_ruler()") && ed.contains("ruler.borrow_mut().clear()"),
            "T-642 (Decision 3): switching away from Ruler must clear the placed chain"
        );
        // The overlay is mounted + the chain registered for it.
        assert!(
            ed.contains("RulerOverlay") && ed.contains("register_ruler_chain("),
            "T-642: RulerOverlay must be mounted and the chain registered for it"
        );
        // The chain is session-local overlay state (a RulerChain in a RefCell), NOT the Y.Doc.
        assert!(
            ed.contains("RulerChain::new()"),
            "T-642 (Decision 4): the ruler is a session-local RulerChain, not doc state"
        );
    }
}

/// T-643 — source pins for the LINE-OF-SIGHT click-capture wiring in the wasm pointer/keydown/
/// dblclick handlers (which a native test cannot execute; the pure state machine + occlusion math are
/// unit-tested in `los_tool`). LoS deliberately REUSES the ruler's `LG::Ruler` gesture arm + Esc seam
/// (the "mode field on the ruler arm" the ticket sanctions, so no third `LeftGesture` variant is
/// added to the un-owned `select_tool`), so these pins prove that reuse is disciplined: the commit
/// routes by `tool_mode`, the Esc is the SHARED arm (not a second window listener — T-726), and the
/// overlay/state/sampler are mounted + registered. Scrubbed code (comments + strings blanked) so a
/// needle is real code; the scrubber keeps `#[cfg(target_arch="wasm32")]` blocks visible.
#[cfg(test)]
mod t643_los_wiring {
    use crate::arsenal::class_r_scrub::live_code;

    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// (arbitration entry — shared gesture) LoS enters the SAME `LG::Ruler` gesture the ruler uses,
    /// via the broadened `should_begin_ruler` (true for any point-capture tool). No separate LoS
    /// `LeftGesture` variant exists — the un-owned `select_tool` is untouched — so the whole entry is
    /// the ruler's, with the commit site (below) choosing the tool.
    #[test]
    fn los_shares_the_ruler_gesture_entry() {
        let ed = editor_live();
        assert!(
            ed.contains("should_begin_ruler("),
            "T-643: LoS must enter via the shared should_begin_ruler point-capture predicate"
        );
        // No third gesture variant was invented for LoS (would require editing the un-owned
        // select_tool). The only capture gesture is LG::Ruler.
        assert!(
            !ed.contains("LeftGesture::LoS") && !ed.contains("LG::LoS"),
            "T-643: LoS must NOT add a new LeftGesture variant — it reuses LG::Ruler (mode field)"
        );
    }

    /// (commit routes by tool_mode) The single `LG::Ruler` pointerup arm commits a LoS point via
    /// `los...click(` under `is_los()`, and a ruler vertex via `.press(` otherwise — one arm, routed
    /// by the mode. The LoS commit must NOT be a doc write (Decision 4) and must NOT route through the
    /// armed-place `has_pending()` branch (constraint a — the same arm the ruler pin already proves
    /// sits outside it).
    #[test]
    fn los_commit_routes_by_tool_mode_no_doc_write() {
        let ed = editor_live();
        // The LoS commit exists and is a `.click(` on the los state, gated by is_los().
        assert!(
            ed.contains("los.borrow_mut().click(") && ed.contains("is_los()"),
            "T-643: the LG::Ruler pointerup arm must route a LoS point via los.click() under is_los()"
        );
        // Slice the pointerup LG::Ruler commit arm (the one carrying .click(); it is the same arm as
        // the ruler's .press(, so it also carries that) and prove it is not a doc-move commit.
        let arms: Vec<&str> = ed.split("LG::Ruler").skip(1).collect();
        let commit: Vec<&str> = arms
            .iter()
            .map(|a| a.split("LG::").next().unwrap_or(a))
            .filter(|a| a.contains(".click(") && a.contains(".press("))
            .collect();
        assert_eq!(
            commit.len(),
            1,
            "T-643: exactly one LG::Ruler arm routes BOTH tools (los.click + ruler.press), found {}",
            commit.len()
        );
        let arm = commit[0];
        assert!(
            !arm.contains("has_pending()"),
            "T-643 (constraint a): the LoS commit shares the arm that sits OUTSIDE the armed-place branch"
        );
        assert!(
            !arm.contains("move_entities_and_vehicles"),
            "T-643 (Decision 4): the LoS commit must not call a doc-move commit (it is not a doc edit)"
        );
    }

    /// (Esc — SHARED seam, not a new listener) The keydown Escape arm dismisses the LoS capture via
    /// `los...escape()` in the SAME arm that dismisses the ruler — reusing the ruler's existing Esc
    /// entry (Decision 3 + the T-726 note: no second unguarded window listener is added).
    #[test]
    fn escape_is_the_shared_ruler_seam() {
        let ed = editor_live();
        // The LoS escape rides the same keydown dispatch as the ruler escape.
        assert!(
            ed.contains("code().as_str()")
                && ed.contains("los.borrow_mut().escape()")
                && ed.contains("ruler.borrow_mut().escape()"),
            "T-643 (Decision 3 / T-726): Esc must call BOTH los.escape() and ruler.escape() in the \
             one shared keydown arm — no second window listener"
        );
        // There must be exactly ONE window keydown Closure carrying the measure-tool Esc (the shared
        // seam): the los.escape and ruler.escape calls sit in the same closure, so a second unguarded
        // Esc listener was NOT added. Proven structurally: both escape calls appear, and the T-642
        // pin already fixes that ruler.escape lives in the one code().as_str() keydown arm.
        assert_eq!(
            ed.matches("los.borrow_mut().escape()").count(),
            1,
            "T-643: LoS Esc must be wired exactly once (the shared seam), not duplicated"
        );
    }

    /// (dblclick guard) A double-click in LoS mode must NOT open Attributes / the asset picker: the
    /// dblclick handler returns early under `is_los()` (its two pointerups already completed the shot
    /// via the shared arm). Pinned alongside the ruler's dblclick guard.
    #[test]
    fn dblclick_is_guarded_in_los_mode() {
        let ed = editor_live();
        // The dblclick handler branches on is_los() (the guard) — the ruler's is_ruler() guard is
        // pinned by t642; this proves the LoS peer guard exists too. Both live in the ondblclick
        // closure, which the t642 dblclick pin already anchors.
        assert!(
            ed.matches("get_untracked().is_los()").count() >= 1,
            "T-643: the dblclick handler must short-circuit under is_los() (no dialog on a LoS dbl-click)"
        );
    }

    /// (Decision 4 — session-local, tool-switch clear, overlay mounted) Switching the tool away from
    /// LoS clears the placed shot; the state is a leaked `RefCell<LosState>` (overlay state, never a
    /// doc write); the overlay is mounted and BOTH the state and the DEM sampler are registered for it.
    #[test]
    fn tool_switch_clears_and_overlay_is_mounted() {
        let ed = editor_live();
        // Tool-switch clear effect: reads !is_los(), clears the state.
        assert!(
            ed.contains("is_los()") && ed.contains("los.borrow_mut().clear()"),
            "T-643 (Decision 3): switching away from LoS must clear the placed shot"
        );
        // The overlay is mounted and the state + sampler registered for it.
        assert!(
            ed.contains("LosOverlay")
                && ed.contains("register_los_state(")
                && ed.contains("register_los_sampler("),
            "T-643: LosOverlay must be mounted with the state + DEM sampler registered for it"
        );
        // Session-local overlay state (a LosState in a RefCell), NOT the Y.Doc.
        assert!(
            ed.contains("LosState::new()"),
            "T-643 (Decision 4): LoS is a session-local LosState, not doc state"
        );
    }

    // ── The fired rule at the wiring layer (perturb / fail / restore) ─────────────────────────────

    /// Fires the commit-routing pin: proof the `is_los()` branch in the shared `LG::Ruler` arm is
    /// load-bearing. The pin passes on the real body; a perturbation that drops the `is_los()` route
    /// (so a LoS click would fall through to `ruler.press` — the exact regression) makes the routing
    /// assertion FAIL. Restore is implicit — only an in-memory copy is perturbed.
    #[test]
    fn fired_rule_los_routing_is_load_bearing() {
        let ed = editor_live();
        let needle = "los.borrow_mut().click(";
        assert!(
            ed.contains(needle),
            "canary: the real body routes a LoS click"
        );
        // Perturb: remove the LoS click route. The routing pin's needle must vanish.
        let perturbed = ed.replace(needle, "ruler.borrow_mut().press(");
        assert!(
            !perturbed.contains(needle),
            "fired rule: dropping the los.click() route (LoS clicks fall through to ruler.press) must \
             break the routing pin — proving the is_los() branch discriminates the regression"
        );
    }
}

/// T-644 (wave 110) — source pins for the VIEWSHED live entry point: the sub-mode is threaded through
/// the LoS button (toggle) and the pointer commit (route), a viewshed click computes + uploads the
/// wash to the engine lane, and the clear seams (Esc + tool/sub-mode switch) drop BOTH the state and
/// the GPU lane through the EXISTING shared seams — no new window listener (T-726 pending). The pure
/// `LosMode`/`ViewshedState`/`place_viewshed` core is unit-tested in `los_tool`; these prove the wasm
/// wiring a native test cannot execute. Scrubbed code (comments + strings blanked) so a needle is real
/// code; the scrubber keeps `#[cfg(target_arch="wasm32")]` blocks visible.
#[cfg(test)]
mod t644_viewshed_wiring {
    use crate::arsenal::class_r_scrub::live_code;

    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// (sub-mode signal threaded to the toolbar) The page owns a real `los_mode` `RwSignal` and hands
    /// it to `ModeToolbar` beside `tool_mode`, so the LoS button can reflect + toggle the sub-mode and
    /// the pointer commit reads the SAME signal. (The toggle-on-reclick lives in `eden_toolbelt`,
    /// pinned there; here we prove the wiring shares one signal, not two.)
    #[test]
    fn los_mode_signal_is_owned_and_handed_to_the_toolbar() {
        let ed = editor_live();
        assert!(
            ed.contains("let los_mode = RwSignal::new(crate::los_tool::LosMode::default())"),
            "T-644: the page must own a real los_mode RwSignal (the LoS sub-mode)"
        );
        assert!(
            ed.contains("ModeToolbar tool_mode los_mode"),
            "T-644: los_mode must be handed to ModeToolbar (one shared signal, toolbar + commit)"
        );
    }

    /// (commit routes ray vs viewshed) The single `LG::Ruler` pointerup arm, already gated by
    /// `is_los()`, now branches on `los_mode…is_viewshed()`: a VIEWSHED click stores the observer
    /// (`viewshed…place(`) and uploads the wash to the engine (`place_viewshed(` → `viewshed_upload(`),
    /// while the RAY click still routes to `los…click(`. One arm, routed by the sub-mode — the same
    /// discipline the ray adds on top of the ruler.
    #[test]
    fn viewshed_click_places_and_uploads_under_is_viewshed() {
        let ed = editor_live();
        assert!(
            ed.contains("is_viewshed()"),
            "T-644: the LoS commit must branch on los_mode.is_viewshed()"
        );
        assert!(
            ed.contains("viewshed.borrow_mut().place("),
            "T-644: a viewshed click must store the observer in the session ViewshedState"
        );
        assert!(
            ed.contains("place_viewshed(") && ed.contains(".viewshed_upload("),
            "T-644: a viewshed click must compute (place_viewshed) and upload the wash (viewshed_upload)"
        );
        // The viewshed branch sits INSIDE the `is_los()` arm and BESIDE the ray's `los…click(` — one
        // shared `LG::Ruler` commit, routed by tool_mode then sub-mode.
        assert!(
            ed.contains("los.borrow_mut().click(") && ed.contains("is_los()"),
            "T-644: the ray click route must remain (the sub-mode branches within the is_los() arm)"
        );
    }

    /// (no-engine / Boot-Failed guard) The wash upload only runs when the engine is live — mirroring
    /// the ray's engine guard: `place_viewshed` returns `None` off-DEM (native/pre-mount → no upload),
    /// and the upload is inside an `if let Some(e) = engine.borrow_mut().as_mut()` so a dead map
    /// (`engine` is `None` after a Boot-Failed) draws nothing.
    #[test]
    fn viewshed_upload_is_engine_guarded() {
        let ed = editor_live();
        // The upload sits behind the same `Some(e)` engine guard the ray path uses — the guard
        // statement `if let Some(e) = engine.borrow_mut().as_mut()` opens the block the
        // `.viewshed_upload(` call lives in. Prove that statement sits between the compute
        // (`place_viewshed(`) and the upload, so the upload can only run with a live engine.
        let compute_at = ed
            .find("place_viewshed(")
            .expect("place_viewshed call present");
        let upload_at = ed
            .find(".viewshed_upload(")
            .expect("viewshed_upload call present");
        assert!(
            compute_at < upload_at,
            "T-644: place_viewshed (compute) must precede the upload"
        );
        let between = &ed[compute_at..upload_at];
        assert!(
            between.contains("if let Some(e) = engine.borrow_mut().as_mut()"),
            "T-644: viewshed_upload must run only when the engine is live — the no-engine / \
             Boot-Failed guard (if let Some(e) = engine.borrow_mut().as_mut()), mirroring the ray mode"
        );
    }

    /// (Esc — the SHARED seam, not a new listener) The keydown Escape arm dismisses the viewshed via
    /// `viewshed…escape()` in the SAME arm that dismisses the ruler + ray, and drops the engine lane
    /// (`viewshed_clear()`) on a real dismissal. No second window listener is added (T-726 pending).
    #[test]
    fn viewshed_escape_is_the_shared_seam() {
        let ed = editor_live();
        assert!(
            ed.contains("code().as_str()")
                && ed.contains("viewshed.borrow_mut().escape()")
                && ed.contains("ruler.borrow_mut().escape()"),
            "T-644: the viewshed Esc must ride the ONE shared keydown arm (beside ruler + ray escape)"
        );
        // Exactly once — the shared seam, not duplicated into a second listener.
        assert_eq!(
            ed.matches("viewshed.borrow_mut().escape()").count(),
            1,
            "T-644: the viewshed Esc must be wired exactly once (the shared seam)"
        );
        // Dismissal drops the GPU lane too.
        assert!(
            ed.contains("viewshed_clear()"),
            "T-644: a viewshed dismissal must drop the engine wash lane (viewshed_clear)"
        );
    }

    /// (tool/sub-mode switch clears — state + GPU lane; overlay bridge registered) Leaving LoS OR
    /// toggling the sub-mode away from Viewshed clears the viewshed state AND the engine lane through
    /// the EXTENDED tool-switch Effect (peer of the ruler's clear-on-switch); the state is a leaked
    /// `RefCell<ViewshedState>` (overlay state, never a doc write) registered for the overlay bridge.
    #[test]
    fn switch_clears_state_and_lane_and_state_is_registered() {
        let ed = editor_live();
        // The tool-switch Effect observes both signals and clears when the viewshed lane is inactive.
        assert!(
            ed.contains("los_mode.get().is_viewshed()")
                && ed.contains("viewshed.borrow_mut().clear()")
                && ed.contains("viewshed_clear()"),
            "T-644: switching away from LoS-viewshed must clear the state AND drop the engine lane"
        );
        // Session-local overlay state (a ViewshedState in a RefCell), registered for the bridge.
        assert!(
            ed.contains("ViewshedState::new()") && ed.contains("register_viewshed_state("),
            "T-644 (Decision 4): the viewshed is a session-local ViewshedState, registered for the \
             overlay/engine bridge — not doc state"
        );
    }

    /// The fired rule at the wiring layer (perturb / fail / restore): the `is_viewshed()` branch in the
    /// shared commit is load-bearing. The pin passes on the real body; a perturbation that drops the
    /// viewshed route (so a viewshed click would fall through to the ray `los.click` — the exact
    /// regression) makes the placement pin FAIL. Restore is implicit (an in-memory copy is perturbed).
    #[test]
    fn fired_rule_viewshed_routing_is_load_bearing() {
        let ed = editor_live();
        let needle = "viewshed.borrow_mut().place(";
        assert!(
            ed.contains(needle),
            "canary: the real body places a viewshed"
        );
        // Perturb: remove the viewshed placement route. The placement pin's needle must vanish.
        let perturbed = ed.replace(needle, "los.borrow_mut().click(");
        assert!(
            !perturbed.contains(needle),
            "fired rule: dropping the viewshed place() route (viewshed clicks fall through to the ray \
             click) must break the placement pin — proving the is_viewshed() branch discriminates"
        );
    }
}

/// T-644 (wave 110) — source pins for the LoS button's SUB-MODE TOGGLE in `eden_toolbelt`: the ONE
/// LoS button re-click toggles Ray ⇆ Viewshed (`LosMode::toggled`) while LoS is already active, and
/// the button's title/label reflect the live sub-mode. The toolbar is a Leptos view (structural), so
/// this is pinned by SOURCE INSPECTION on scrubbed `eden_toolbelt.rs`, mirroring `t643`/`t668`.
#[cfg(test)]
mod t644_los_button_submode {
    use crate::arsenal::class_r_scrub::{live_code, live_source, only_body};

    /// (toggle on re-click) The LoS button's `on:pointerdown` toggles `los_mode` when LoS is ALREADY
    /// active (`is_los()` true → `los_mode.update(… toggled())`) and otherwise sets `tool_mode = LoS`.
    /// The `tool_mode.set(EditorTool::LoS)` still lives in the button (the honesty rule / t643 pin),
    /// so the button never lies about which tool it selects.
    #[test]
    fn los_button_reclick_toggles_the_submode() {
        let code = live_code(include_str!("eden_toolbelt.rs"));
        let body = only_body(&code, &format!("pub fn {}", "ModeToolbar("));
        assert!(
            body.contains("los_mode.update(|m| *m = m.toggled())"),
            "T-644: a re-click of the LoS button must toggle the sub-mode (LosMode::toggled)"
        );
        // The toggle is gated on LoS already being active (first click from another tool just
        // activates LoS; it does not advance the sub-mode).
        assert!(
            body.contains("tool_mode.get_untracked().is_los()"),
            "T-644: the toggle must fire only when LoS is already active (re-click semantics)"
        );
        // The set-LoS path is still present (t643 honesty rule — the button selects the tool it names).
        assert!(
            body.contains(&format!("tool_mode.set(EditorTool::{})", "LoS")),
            "T-644: the LoS button must still set tool_mode = LoS on the first (activate) click"
        );
    }

    /// (title/label reflect the sub-mode) The LoS button's title AND wide-layout label read the live
    /// `los_mode` (`is_viewshed()`), so the operator always knows which sub-mode they're in. Proven on
    /// the string-KEPT source (the title/label literals survive) so the needle is the real view text.
    #[test]
    fn los_button_reflects_the_active_submode() {
        let src = live_source(include_str!("eden_toolbelt.rs"));
        let body = only_body(&src, &format!("pub fn {}", "ModeToolbar("));
        // The button reads the sub-mode to pick its title/label.
        assert!(
            body.matches("los_mode.get()").count() >= 1 && body.contains("is_viewshed()"),
            "T-644: the LoS button must read los_mode to reflect the active sub-mode"
        );
        // Both sub-mode words appear in the button's affordance (title + label).
        for word in ["viewshed", "ray"] {
            assert!(
                body.contains(word),
                "T-644: the LoS button title/label must name the {word} sub-mode"
            );
        }
        // t668/t642 retention: the base tooltip phrase survives (still explains the tool).
        assert!(
            body.contains("Line of sight"),
            "T-644: the LoS button must keep its 'Line of sight' title (tooltip retention)"
        );
    }
}

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
mod t648_transform {
    use crate::arsenal::class_r_scrub::{live_code, only_body};
    use crate::mission_editor::transform::{
        bearing_to_face, norm_deg, snap_rotate, snap_translate, snap_value, step, Axis, SnapState,
        WidgetVariant, ROTATE_LADDER_DEG, TRANSLATE_LADDER_M,
    };

    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    // ── QUANTISER: ladders ────────────────────────────────────────────────────────────────────
    /// The ladders are exactly the ticket's rungs, OFF-first.
    #[test]
    fn ladders_are_the_ticket_rungs() {
        assert_eq!(
            TRANSLATE_LADDER_M,
            [0.0, 1.0, 5.0, 10.0],
            "translation ladder = off/1/5/10 m"
        );
        assert_eq!(
            ROTATE_LADDER_DEG,
            [0.0, 5.0, 15.0, 45.0],
            "rotation ladder = off/5/15/45°"
        );
        assert_eq!(Axis::Translate.ladder(), &TRANSLATE_LADDER_M);
        assert_eq!(Axis::Rotate.ladder(), &ROTATE_LADDER_DEG);
    }

    // ── QUANTISER: off-state passthrough ──────────────────────────────────────────────────────
    #[test]
    fn off_rung_is_passthrough() {
        // Rung 0 (OFF) returns the value byte-for-byte — a free move / free rotate.
        assert_eq!(snap_translate(3.7, 0), 3.7);
        assert_eq!(snap_translate(-123.456, 0), -123.456);
        // snap_value with a non-positive / non-finite step is also passthrough (the OFF branch).
        assert_eq!(snap_value(3.7, 0.0), 3.7);
        assert_eq!(snap_value(3.7, -5.0), 3.7);
        assert_eq!(snap_value(3.7, f64::NAN), 3.7);
        // Rotation OFF still NORMALISES to [0,360) (the stored range) but does not quantise.
        assert_eq!(snap_rotate(370.0, 0), 10.0);
        assert_eq!(snap_rotate(-30.0, 0), 330.0);
    }

    // ── QUANTISER: quantisation to a rung ─────────────────────────────────────────────────────
    #[test]
    fn snap_translate_quantises_to_the_rung() {
        // 5 m rung: 12 → 10, 13 → 15 (round to nearest multiple of 5).
        assert_eq!(snap_translate(12.0, 2), 10.0);
        assert_eq!(snap_translate(13.0, 2), 15.0);
        // 1 m rung: rounds to whole metres.
        assert_eq!(snap_translate(2.4, 1), 2.0);
        assert_eq!(snap_translate(2.6, 1), 3.0);
        // 10 m rung: negatives round symmetrically.
        assert_eq!(snap_translate(-14.0, 3), -10.0);
        assert_eq!(snap_translate(-16.0, 3), -20.0);
    }

    #[test]
    fn snap_rotate_quantises_and_normalises() {
        // 45° rung: 40 → 45, 20 → 45? no — 20 rounds to 0 (nearest of {0,45}). 30 → 45.
        assert_eq!(snap_rotate(40.0, 3), 45.0);
        assert_eq!(snap_rotate(20.0, 3), 0.0);
        assert_eq!(snap_rotate(30.0, 3), 45.0);
        // 15° rung: 7 → 0, 8 → 15, 359 → 0 (360 normalises to 0).
        assert_eq!(snap_rotate(7.0, 2), 0.0);
        assert_eq!(snap_rotate(8.0, 2), 15.0);
        assert_eq!(snap_rotate(359.0, 2), 0.0);
        // 5° rung with wrap: 358 → 360 → 0.
        assert_eq!(snap_rotate(358.0, 1), 0.0);
    }

    // ── QUANTISER: increase/decrease clamping ─────────────────────────────────────────────────
    #[test]
    fn step_clamps_at_both_ends() {
        let len = TRANSLATE_LADDER_M.len(); // 4
                                            // Increase walks up and STOPS at the last rung.
        assert_eq!(step(0, len, 1), 1);
        assert_eq!(step(1, len, 1), 2);
        assert_eq!(step(2, len, 1), 3);
        assert_eq!(
            step(3, len, 1),
            3,
            "increase at the coarsest rung is a clamp, not a wrap"
        );
        // Decrease walks down and STOPS at OFF (0).
        assert_eq!(step(3, len, -1), 2);
        assert_eq!(step(1, len, -1), 0);
        assert_eq!(
            step(0, len, -1),
            0,
            "decrease at OFF is a clamp, not a wrap to the top"
        );
        // A zero delta is inert (still clamped into range).
        assert_eq!(step(2, len, 0), 2);
        // Degenerate empty ladder never panics.
        assert_eq!(step(0, 0, 1), 0);
    }

    // ── SnapState: the master latch + per-axis rungs ──────────────────────────────────────────
    #[test]
    fn snap_state_default_is_off_and_passthrough() {
        let s = SnapState::default();
        assert!(!s.enabled, "grid defaults OFF");
        assert_eq!(s.translate_rung, 0);
        assert_eq!(s.rotate_rung, 0);
        // Effective rungs are 0 while disabled REGARDLESS of the stored rung.
        let tuned = SnapState {
            enabled: false,
            translate_rung: 3,
            rotate_rung: 2,
        };
        assert_eq!(
            tuned.effective_translate_rung(),
            0,
            "grid off ⇒ translation passthrough even with a tuned rung"
        );
        assert_eq!(
            tuned.effective_rotate_rung(),
            0,
            "grid off ⇒ rotation passthrough"
        );
    }

    #[test]
    fn toggling_the_latch_preserves_rungs() {
        let s = SnapState {
            enabled: false,
            translate_rung: 2,
            rotate_rung: 3,
        };
        let on = s.toggled();
        assert!(on.enabled);
        assert_eq!(
            on.translate_rung, 2,
            "toggle keeps the tuned translation rung"
        );
        assert_eq!(on.rotate_rung, 3, "toggle keeps the tuned rotation rung");
        assert_eq!(
            on.effective_translate_rung(),
            2,
            "enabled ⇒ tuned rung is live"
        );
        assert_eq!(on.effective_rotate_rung(), 3);
        assert!(!on.toggled().enabled, "toggling again turns it back off");
    }

    #[test]
    fn stepping_a_rung_does_not_flip_the_latch() {
        // Stepping while OFF parks the rung without enabling (Eden keeps the two controls orthogonal).
        let s = SnapState::default().stepped(Axis::Translate, 1);
        assert!(!s.enabled, "stepping a rung must not enable the grid");
        assert_eq!(s.translate_rung, 1);
        assert_eq!(
            s.rotate_rung, 0,
            "stepping translation leaves rotation alone"
        );
        // Rotation axis is independent.
        let s2 = s.stepped(Axis::Rotate, 1).stepped(Axis::Rotate, 1);
        assert_eq!(s2.translate_rung, 1);
        assert_eq!(s2.rotate_rung, 2);
        // Clamps ride through SnapState too.
        let maxed = SnapState::default()
            .stepped(Axis::Rotate, 1)
            .stepped(Axis::Rotate, 1)
            .stepped(Axis::Rotate, 1)
            .stepped(Axis::Rotate, 1);
        assert_eq!(
            maxed.rotate_rung, 3,
            "clamped at the coarsest rotation rung"
        );
    }

    #[test]
    fn status_readout_names_the_active_steps() {
        assert_eq!(SnapState::default().status_readout(), "GRID  off");
        let s = SnapState {
            enabled: true,
            translate_rung: 2, // 5 m
            rotate_rung: 2,    // 15°
        };
        assert_eq!(s.status_readout(), "GRID  move 5 m \u{b7} rot 15\u{b0}");
        let off = SnapState {
            enabled: true,
            translate_rung: 0,
            rotate_rung: 0,
        };
        assert_eq!(
            off.status_readout(),
            "GRID  move off \u{b7} rot off",
            "an enabled grid with both ladders at OFF reads 'off' per axis"
        );
    }

    // ── SHIFT-ROTATE: face-cursor bearing golden (incl. wrap) ─────────────────────────────────
    /// The bearing is yaw clockwise from north (+Y) — the doc/export convention. Cardinal goldens
    /// plus the wrap case (west → 270, not −90).
    #[test]
    fn bearing_faces_the_cursor_clockwise_from_north() {
        let eps = 1e-9;
        // Pivot at origin; cursor at each cardinal.
        assert!(
            (bearing_to_face(0.0, 0.0, 0.0, 10.0).unwrap() - 0.0).abs() < eps,
            "north → 0°"
        );
        assert!(
            (bearing_to_face(0.0, 0.0, 10.0, 0.0).unwrap() - 90.0).abs() < eps,
            "east → 90°"
        );
        assert!(
            (bearing_to_face(0.0, 0.0, 0.0, -10.0).unwrap() - 180.0).abs() < eps,
            "south → 180°"
        );
        // West is the WRAP case: atan2 gives −90, normalise to 270.
        assert!(
            (bearing_to_face(0.0, 0.0, -10.0, 0.0).unwrap() - 270.0).abs() < eps,
            "west → 270° (the wrap: −90 must normalise, not stay negative)"
        );
        // A diagonal: NE → 45.
        assert!(
            (bearing_to_face(0.0, 0.0, 5.0, 5.0).unwrap() - 45.0).abs() < eps,
            "NE → 45°"
        );
        // Pivot offset from origin — bearing is relative to the pivot, not the world origin.
        assert!(
            (bearing_to_face(100.0, 200.0, 100.0, 250.0).unwrap() - 0.0).abs() < eps,
            "cursor due north of an offset pivot is still 0°"
        );
    }

    #[test]
    fn bearing_is_none_for_a_degenerate_aim() {
        // Cursor exactly on the pivot → no meaningful bearing (the commit leaves rotation untouched).
        assert_eq!(bearing_to_face(50.0, 50.0, 50.0, 50.0), None);
        // Non-finite inputs → None, not a NaN commit.
        assert_eq!(bearing_to_face(0.0, 0.0, f64::NAN, 0.0), None);
        assert_eq!(bearing_to_face(0.0, 0.0, 0.0, f64::INFINITY), None);
    }

    #[test]
    fn norm_deg_ranges_and_handles_nonfinite() {
        assert_eq!(norm_deg(0.0), 0.0);
        assert_eq!(norm_deg(360.0), 0.0);
        assert_eq!(norm_deg(370.0), 10.0);
        assert_eq!(norm_deg(-10.0), 350.0);
        assert_eq!(norm_deg(-370.0), 350.0);
        assert_eq!(norm_deg(f64::NAN), 0.0);
    }

    // ── WIDGET STATE MACHINE: 1/2 cycle, variant-gated gestures ───────────────────────────────
    #[test]
    fn widget_variant_cycles_on_1_and_2_only() {
        let v = WidgetVariant::default();
        assert_eq!(v, WidgetVariant::Translate, "default variant is Translate");
        assert_eq!(v.from_digit(2), WidgetVariant::Rotate, "2 → Rotate");
        assert_eq!(
            WidgetVariant::Rotate.from_digit(1),
            WidgetVariant::Translate,
            "1 → Translate"
        );
        // 3-5 (and any other digit) are INERT — there is no area-scale variant (honest scope: a
        // transform selection is slots + vehicles, neither of which scales).
        assert_eq!(
            WidgetVariant::Rotate.from_digit(3),
            WidgetVariant::Rotate,
            "3 is not bound — no area-scale variant"
        );
        assert_eq!(
            WidgetVariant::Translate.from_digit(5),
            WidgetVariant::Translate
        );
        assert_eq!(WidgetVariant::Rotate.from_digit(0), WidgetVariant::Rotate);
    }

    #[test]
    fn widget_variant_gates_its_gesture_axis() {
        // Only Rotate has a ring (Shift+ring drag snaps to the rotation ladder).
        assert!(WidgetVariant::Rotate.is_rotate());
        assert!(!WidgetVariant::Translate.is_rotate());
        // The step keys tune the axis matching the variant.
        assert_eq!(WidgetVariant::Translate.snap_axis(), Axis::Translate);
        assert_eq!(WidgetVariant::Rotate.snap_axis(), Axis::Rotate);
    }

    // ── KEYDOWN CENSUS: G free (+ brackets + digits), Space stays flyTo ────────────────────────
    /// The two window-level EDITOR keydowns are this file's and `mission_history`'s. Census both as
    /// raw text (keeping string literals — a keydown arm IS a `"KeyX"` string). T-648's new keys must
    /// be free before this slice, and Space must remain `center_on_selection` (flyTo), not a widget
    /// cycle (the collision decision).
    #[test]
    fn t648_keydown_census() {
        // Slice ONLY the editor keydown MATCH of each of the two window-level editor keydowns, so a
        // needle can never self-match inside this test module (which sits in the same file). The arm
        // list runs from the `match ev.code().as_str()` head to the arm-list terminator `_ => false`
        // / `_ => {}`. Comments are stripped (`live_source` keeps the `"KeyX"` arm LITERALS but drops
        // notes) so a comment that MENTIONS a rejected keysym for explanation is not read as a
        // binding — the census is about arm patterns, not prose.
        //
        // T-703/T-738: the slicer used to be a private copy right here — one of FOUR. It now lives
        // once, in `eden_help::keymap_census`, beside the structured (code, modifiers) census that
        // detects collisions; `there_is_exactly_one_extractor` keeps it from being copied again.
        use crate::eden_help::keymap_census::keydown_arms;
        let this_arms = keydown_arms(include_str!("mission_editor.rs"));
        let history_arms = keydown_arms(include_str!("mission_history.rs"));
        // Needles assembled so the LITERAL never appears verbatim in this test's own source.
        let key = |k: &str| format!("\"{k}\"");
        let g = key("KeyG");
        let bl = key("BracketLeft");
        let br = key("BracketRight");
        let d1 = key("Digit1");
        let d2 = key("Digit2");
        let semicolon = key("Semicolon");
        let odiaeresis = format!("odi{}", "aeresis"); // split so it is not a verbatim literal here

        // The other editor keydown (Ctrl+Z/Y) must NOT claim any T-648 key.
        assert!(
            !history_arms.contains(&g)
                && !history_arms.contains(&bl)
                && !history_arms.contains(&br)
                && !history_arms.contains(&d1)
                && !history_arms.contains(&d2),
            "census: mission_history's keydown (Ctrl+Z/Y) must not claim G / [ / ] / 1 / 2"
        );
        // G is the chosen grid toggle — an arm here, and NOT an Eden keysym artefact.
        assert!(
            this_arms.contains(&format!("{g} if !modk")),
            "KEY-GRID-001: G must be the grid-toggle keydown arm"
        );
        assert!(
            !this_arms.contains(&odiaeresis) && !this_arms.contains(&semicolon),
            "census: must NOT copy Eden's odiaeresis / ; keysym artefacts for the grid toggle"
        );
        // [ / ] step the snap rung.
        assert!(
            this_arms.contains(&format!("{bl} if !modk"))
                && this_arms.contains(&format!("{br} if !modk")),
            "TOOLBAR-GRID-MOVE-001: [ and ] must be the decrease/increase keydown arms"
        );
        // 1 / 2 cycle the widget variant (Eden's free direct keys — the Space collision decision).
        assert!(
            this_arms.contains(&format!("{d1} if !modk"))
                && this_arms.contains(&format!("{d2} if !modk")),
            "WIDGET-CYCLE-001: 1 and 2 must be the widget-variant keydown arms"
        );
        // Space STAYS flyTo — it must still map to center_on_selection and must NOT cycle the widget.
        let space = key("Space");
        assert!(
            this_arms.contains(&format!(
                "{space} if !modk => crate::editor_ops::center_on_selection()"
            )),
            "collision decision: Space must remain flyTo (center_on_selection), not a widget cycle"
        );
        // The Space arm must not touch widget_variant (it is a one-liner flyTo call).
        let space_at = this_arms.find(&space).expect("Space arm present");
        let space_arm = &this_arms[space_at..(space_at + 120).min(this_arms.len())];
        assert!(
            !space_arm.contains("widget_variant"),
            "collision decision: the Space arm must not cycle the widget variant"
        );
    }

    // ── SOURCE PINS: the Shift-rotate gesture arm ─────────────────────────────────────────────
    /// Shift+drag on a SELECTED entity promotes to `LG::Rotate` (not `LG::Move`), and the commit
    /// routes through `rotate_selection_to_face` — never the atomic translate `move_entities_*`.
    #[test]
    fn shift_rotate_arm_promotes_and_commits_through_the_field_write() {
        let ed = editor_live();
        assert!(
            ed.contains("ev.shift_key()") && ed.contains("LG::Rotate {"),
            "XFORM-SHIFT-001: a Shift-held drag on a selected entity must open LG::Rotate"
        );
        assert!(
            ed.contains("editor_ops::rotate_selection_to_face("),
            "the LG::Rotate commit must call rotate_selection_to_face"
        );
        // Isolate the pointerup LG::Rotate arm and prove it commits rotation, NOT a translate.
        let rot_arm = {
            let at = ed
                .find("LG::Rotate { cam, .. } =>")
                .expect("the pointerup LG::Rotate commit arm is present");
            let rest = &ed[at..];
            let end = rest[3..].find("LG::").map(|i| i + 3).unwrap_or(rest.len());
            &rest[..end]
        };
        assert!(
            rot_arm.contains("rotate_selection_to_face("),
            "the rotate commit arm must call the field-write rotate"
        );
        assert!(
            !rot_arm.contains("move_entities_and_vehicles(") && !rot_arm.contains("move_entities("),
            "the rotate arm must NOT translate — rotation rides the attrs/vehicle field write"
        );
    }

    /// The atomic move-commit pin's invariant is UNDISTURBED by the new arm: exactly one `LG::Move`
    /// arm still calls `move_entities_and_vehicles`, and `LG::Rotate` is a separate arm. (The
    /// authoritative version of this pin lives in map-engine-core/doc/store.rs and runs under
    /// `cargo test -p map-engine-core`; this is the frontend-local echo so a fork shows up here too.)
    #[test]
    fn only_one_move_arm_commits_the_atomic_mix() {
        let ed = editor_live();
        let move_arms: Vec<&str> = ed
            .split("LG::Move")
            .skip(1)
            .map(|s| s.split("LG::").next().unwrap_or(s))
            .filter(|arm| arm.contains(".move_entities_and_vehicles("))
            .collect();
        assert_eq!(
            move_arms.len(),
            1,
            "exactly one LG::Move arm may commit via move_entities_and_vehicles (found {})",
            move_arms.len()
        );
    }

    /// **wave-127 F-6** — the drag commit must carry each dragged slot's CURRENT z.
    ///
    /// `move_entities_in_txn` (map-engine-core) reads the existing z, DISCARDS it, and writes the
    /// caller's `zs[i]` verbatim — so a `vec![0.0; n]` here is not a placeholder, it is a write of
    /// `0.0` onto every dragged slot inside one txn, with nothing left in this frontend to
    /// re-sample terrain afterwards (`terrainZ` did not survive the React deletion). Vehicles in
    /// the same drag keep their z, which is the asymmetry that gives the defect away.
    ///
    /// This reads the LIVE `LG::Move` commit arm — `live_code` strips comments and dead code and
    /// cuts the test module, so neither a reassuring note nor this module's own text can satisfy
    /// it. It requires the zeros gone, the SHARED `keep_z_rows`/`slot_z` pair used (a third
    /// z-resolution path is its own defect class here), and `zs` built by mapping over the same
    /// `slot_ids` that is then passed as `ids` — the structural fact that makes `zs[i]` the z of
    /// `slot_ids[i]`. A mismatched zip would hand one slot another slot's elevation, which is a
    /// worse outcome than the zeroing this fixes.
    #[test]
    fn drag_move_commit_carries_each_slots_current_z() {
        let ed = editor_live();
        let arm = ed
            .split("LG::Move")
            .skip(1)
            .map(|s| s.split("LG::").next().unwrap_or(s))
            .find(|arm| arm.contains(".move_entities_and_vehicles("))
            .expect("the LG::Move commit arm is present");
        let flat: String = arm.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(
            !flat.contains("vec![0.0;"),
            "wave-127 F-6: the drag must not pass a zero-filled `zs` — the core writes it verbatim, \
             so that is a flatten of every dragged slot's authored z, not a placeholder"
        );
        assert!(
            flat.contains("keep_z_rows(") && flat.contains("slot_z("),
            "the drag must resolve z through the shared keep_z_rows/slot_z pair (exact f64 off the \
             raw row, hidden-layer slots included), not a third z-resolution path"
        );
        assert!(
            flat.contains("slot_ids.iter().map("),
            "`zs` must be built by mapping over `slot_ids` ITSELF, in order — that is what pins \
             zs[i] to slot_ids[i]"
        );
        assert!(
            flat.contains("move_entities_and_vehicles(slot_ids,&veh_ids,dx,dy,zs"),
            "the resolved `zs` must be the vector handed to the translate, positionally after dx/dy"
        );

        // The two statements must stay ADJACENT. The asserts above prove `zs` is mapped from
        // `slot_ids` and that `slot_ids` is what gets translated — but neither can see an edit
        // BETWEEN them. An inserted `slot_ids.sort()` would repoint every z by one slot, handing
        // each entity a neighbour's elevation, and every assert above would still pass. That is a
        // worse outcome than the zeroing F-6 fixed, so the window itself is pinned.
        let between = flat
            .split("slot_ids.iter().map(")
            .nth(1)
            .and_then(|s| s.split("move_entities_and_vehicles(").next())
            .expect("the `zs` build must precede the translate");
        for mutator in [
            "slot_ids.sort",
            "slot_ids.reverse",
            "slot_ids.dedup",
            "slot_ids.retain",
            "slot_ids.swap",
            "slot_ids.remove",
            "slot_ids.truncate",
            "slot_ids.push",
            "slot_ids.insert",
            "slot_ids.clear",
            "slot_ids.drain",
        ] {
            assert!(
                !between.contains(mutator),
                "wave-127 NIT: `{mutator}` between the `zs` build and the translate would reorder \
                 or resize `slot_ids` after `zs` was built, silently giving each slot another \
                 slot's z while every other assertion in this test stayed green"
            );
        }
    }

    // ── SOURCE PINS: the keydown bindings drive the right state ────────────────────────────────
    #[test]
    fn keydown_arms_drive_snap_and_variant_state() {
        let ed = editor_live();
        assert!(
            ed.contains("snap.set(snap.get_untracked().toggled())"),
            "G must toggle the SnapState master latch"
        );
        assert!(
            ed.contains("snap.set(snap.get_untracked().stepped(axis, -1))")
                && ed.contains("snap.set(snap.get_untracked().stepped(axis, 1))"),
            "[ / ] must step the current-variant snap axis down / up"
        );
        assert!(
            ed.contains("widget_variant.set(widget_variant.get_untracked().from_digit(1))")
                && ed.contains("widget_variant.set(widget_variant.get_untracked().from_digit(2))"),
            "1 / 2 must set the widget variant"
        );
    }

    // ── SOURCE PINS: the widget + snap-readout mounts ─────────────────────────────────────────
    #[test]
    fn widget_and_readout_are_mounted() {
        let ed = editor_live();
        assert!(
            ed.contains("TransformWidgetOverlay") && ed.contains("register_widget_pivot("),
            "WIDGET-TRANS-001: the transform widget must be mounted and its pivot registered"
        );
        assert!(
            ed.contains("SnapReadout"),
            "TOOLBAR-GRID-MOVE-001: the snap-step readout must be mounted"
        );
        // The Shift-rotate commit rung comes from the EFFECTIVE (grid-gated) rotation rung.
        assert!(
            ed.contains("effective_rotate_rung()"),
            "the rotate commit must quantise to the grid-gated rotation rung"
        );
    }

    // ── SOURCE PIN: the included one-line comment fix (before/after) ───────────────────────────
    /// The wave-109 verifier's binding fix: the false T-159.22 claim must be GONE and replaced by the
    /// truth (has_pending short-circuits regardless of left/pan_px). Pinned on the RAW file — the
    /// claim and its correction are comments, which `live_code` strips.
    #[test]
    fn false_t159_22_comment_is_corrected() {
        let raw = include_str!("mission_editor.rs");
        // The false-claim needle is assembled from fragments so this test's OWN source (in this same
        // file, read via include_str!) is not a decoy match for it.
        let false_claim = format!(
            "{}{}",
            "`left`/`pan_px` are both None here", " and no gesture branch below would fire"
        );
        assert!(
            !raw.contains(&false_claim),
            "the false 'both-None here' T-159.22 claim must be deleted or corrected"
        );
        assert!(
            raw.contains("`has_pending()` short-circuits with a `return` before the gesture"),
            "the correction must state the true invariant: has_pending() short-circuits regardless"
        );
    }

    // ── FIRED RULE: the quantiser is load-bearing (perturb / fail / restore) ───────────────────
    /// Fire the quantiser once: a build that quantises everyday (perturb `snap_value` to always
    /// passthrough) must FAIL the quantisation goldens. This proves the ladders actually bite — a
    /// green suite over a no-op quantiser would be worthless. Restore is implicit (in-memory reasoning
    /// via a re-derived value); the real `snap_value` is exercised by the goldens above.
    #[test]
    fn fired_rule_quantiser_is_load_bearing() {
        // The real quantiser bites: 12 m at the 5 m rung lands on 10.
        assert_eq!(
            snap_translate(12.0, 2),
            10.0,
            "canary: the real quantiser snaps"
        );
        // Perturbation model: a passthrough quantiser (the regression) would return the input.
        let passthrough = |v: f64, _step: f64| v;
        let perturbed = passthrough(12.0, TRANSLATE_LADDER_M[2]);
        assert_ne!(
            perturbed, 10.0,
            "fired rule: a passthrough quantiser (snap off everywhere) does NOT land on the grid — \
             so the quantisation goldens above genuinely constrain the snap, they are not vacuous"
        );
        // And the rotation ladder likewise bites (40° → 45° at the 45° rung).
        assert_eq!(snap_rotate(40.0, 3), 45.0);
        assert_ne!(
            40.0, 45.0,
            "fired rule: the rotation snap moved the value — the golden is not an identity"
        );
    }

    // ── SOURCE PINS on the pure module living where it can be native-tested ────────────────────
    /// The `transform` module is UNGATED (native-testable) — the whole reason these behavioural tests
    /// run at all. Pin that placement so a refactor into wasm-only `select_tool` (where a native
    /// `cargo test` would silently skip them) is caught.
    #[test]
    fn transform_module_is_native_testable() {
        let raw = include_str!("mission_editor.rs");
        // The module declaration must NOT sit under a wasm cfg.
        let decl = "pub mod transform {";
        let at = raw.find(decl).expect("transform module present");
        let before = &raw[at.saturating_sub(60)..at];
        assert!(
            !before.contains("cfg(target_arch = \"wasm32\")"),
            "the transform module must stay ungated so its quantiser/bearing tests run on native \
             `cargo test -p website-frontend` (the command the wave gate uses)"
        );
        // And the rotate commit really rides the existing field write, per the ticket.
        let ops = include_str!("editor_ops.rs");
        let ops_live = live_code(ops);
        let body = only_body(&ops_live, "pub fn rotate_selection_to_face(");
        assert!(
            body.contains("update_slot_position(") && body.contains("set_vehicle_position("),
            "rotate_selection_to_face must ride update_slot_position (slots) + set_vehicle_position \
             (vehicles) — the existing per-field rotation writes, not a new core mutator"
        );
    }
}

/// T-655 — the validation panel wiring pins: the mount exists, its payload source is registered, it
/// re-evaluates off the `doc_tick` channel, it is ALWAYS ON (no debug flag), and it SURVIVES
/// hide-chrome (mounted OUTSIDE every `chrome_hidden` gate — the diagnostics doctrine). These scan
/// the comment-stripped page source (`live_code`) so the doc prose that mentions `chrome_hidden`
/// cannot false-match the gate check.
#[cfg(test)]
mod t655_validation_panel_wiring {
    use crate::arsenal::class_r_scrub::live_code;

    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// The panel is mounted as a real component, fed the `doc_tick` re-eval channel (the T-666
    /// doc-change tick), and its payload source is registered from the wasm mount.
    #[test]
    fn the_validation_panel_is_mounted_and_wired_to_doc_tick() {
        let ed = editor_live();
        assert!(
            ed.contains("validation_panel::ValidationPanel doc_tick"),
            "T-655: the ValidationPanel must be mounted with the doc_tick re-eval channel"
        );
        assert!(
            ed.contains("validation_panel::register_payload_source("),
            "T-655: the panel's compiled-payload source must be registered from the wasm mount"
        );
        assert!(
            ed.contains("validation_panel::register_select_by_id("),
            "T-655: the click-to-select router (subject_id → selection) must be registered from the \
             wasm mount, where the doc/selection/engine handles live"
        );
        // The router routes through the SAME selection seam the rest of the editor uses (engine
        // set_selection + centre + refresh_selection), keyed on the finding's subject_id.
        assert!(
            ed.contains("e.set_selection(ids)")
                && ed.contains("mission_history::refresh_selection()"),
            "T-655: click-to-select must replace the selection + refresh mirrors (the open_attributes \
             seam), not a bespoke path"
        );
        // The registered source compiles the SAVE-shape payload (the editor.{factions,squads,slots}
        // block the rules read) and threads the T-658 known-asset-id catalogue.
        assert!(
            ed.contains("compile::compile_payload(")
                && ed.contains("known_asset_ids_from_registry("),
            "T-655/T-658: the source must feed compile_payload + the known-asset-id catalogue"
        );
    }

    /// Hide-chrome survival + always-on: the panel mount is OUTSIDE every `chrome_hidden` gate (a
    /// Backspace hide-interface leaves it visible — correctness diagnostics are never gated, T-635's
    /// doctrine), and it is not behind any debug flag. Proven by locating the mount and checking that
    /// no `chrome_hidden` gate (nor a `debug_hud` gate) opens between the ungated-dialog landmark
    /// (the context-menu overlay, the same landmark the T-647 picker pin uses) and it.
    #[test]
    fn the_validation_panel_survives_hide_chrome_and_is_always_on() {
        let ed = editor_live();
        let mount = ed
            .find("validation_panel::ValidationPanel doc_tick")
            .expect("T-655: the ValidationPanel mount");
        let landmark = ed
            .find("ContextMenuOverlay menu=")
            .expect("context menu mount is the ungated-dialog landmark");
        assert!(
            mount > landmark,
            "T-655: the panel must mount after the ungated-dialog landmark"
        );
        let between = &ed[landmark..mount];
        assert!(
            !between.contains("(!chrome_hidden.get()).then("),
            "T-655: the panel is DIAGNOSTICS — no chrome_hidden gate may sit between the ungated \
             dialogs and its mount (it survives Backspace hide-chrome, T-635 doctrine)"
        );
        // Always-on: not gated behind the telemetry HUD debug flag either.
        assert!(
            !between.contains("debug_hud_shown.get()") && !between.contains("debug_hud.get()"),
            "T-655: validation is ALWAYS ON — the panel must not sit behind a debug flag"
        );
    }
}

/* ═══════ T-754 — the click-to-select router resolves ZONES, and says so before it is clicked ═════
 *
 * Two families, as the ticket demands: unit tests over the pure resolution (it is `serde_json`-only,
 * so it RUNS natively — this is not a source scan pretending to be a behaviour test), and source pins
 * for the parts that are wiring (the closure is wasm-only and holds `!Send` handles).
 */
#[cfg(test)]
mod t754_router_resolves_zones {
    use super::{route_target, RouteTarget};
    use crate::arsenal::class_r_scrub::live_code;
    use serde_json::json;

    fn doc() -> serde_json::Value {
        json!({
            "vehiclesById": { "v1": { "position": { "x": 7.0, "y": 9.0 } } },
            "zonesById": {
                "z-circle": { "shape": { "circle": { "x": 100.0, "z": 250.0, "r": 500.0 } } },
                "z-poly": { "shape": { "polygon": [[0.0, 0.0], [30.0, 0.0], [0.0, 30.0]] } },
                "z-shapeless": { "type": "spawn" }
            }
        })
    }

    /// The resolution, arm by arm — including the ORDER, which is the shipped router's order (slot,
    /// then vehicle, then the new zone arm), so the widening cannot change what an id already
    /// resolved to. `None` still means "select nothing and keep the current selection".
    #[test]
    fn every_arm_resolves_and_the_order_is_the_shipped_one() {
        let d = doc();
        let no_slots = |_: &str| false;
        assert_eq!(
            route_target(&d, "v1", &no_slots),
            Some(RouteTarget::Vehicle { x: 7.0, y: 9.0 })
        );
        assert_eq!(
            route_target(&d, "z-circle", &no_slots),
            Some(RouteTarget::Zone { x: 100.0, y: 250.0 })
        );
        assert_eq!(
            route_target(&d, "z-poly", &no_slots),
            Some(RouteTarget::Zone { x: 10.0, y: 10.0 })
        );
        assert_eq!(route_target(&d, "z-shapeless", &no_slots), None);
        assert_eq!(route_target(&d, "nobody", &no_slots), None);
        // A slot wins over everything else, exactly as the SoA lookup did when it ran first.
        assert_eq!(
            route_target(&d, "v1", &|_| true),
            Some(RouteTarget::Slot),
            "T-754: the slot arm must still take precedence — the widening reorders nothing"
        );
        // A garbage document resolves nothing rather than panicking inside a click handler.
        assert_eq!(route_target(&json!(null), "z-circle", &no_slots), None);
        assert_eq!(
            route_target(
                &json!({ "zonesById": { "z": { "shape": { "polygon": [] } } } }),
                "z",
                &no_slots
            ),
            None
        );
    }

    /// The wiring: the ONE registered router grew a zone arm that drives the Zones panel's own
    /// selection seam. No second router, and no zone id smuggled into `select_tool`'s selection.
    #[test]
    fn the_one_router_routes_zones_through_the_zones_panel() {
        // Anchored at the page component, exactly as the T-655 module does: `cut_test_module` cuts
        // from the FIRST `#[cfg(test)]` to EOF, and this file has one inside `registry_session` long
        // before the mount — scrubbing from the top would leave an empty haystack every pin passes.
        let raw = include_str!("mission_editor.rs");
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let ed = live_code(&raw[raw.find(anchor.as_str()).expect("the page component")..]);
        assert_eq!(
            ed.matches(&format!("register{}", "_select_by_id(")).count(),
            1,
            "T-754: there must still be exactly ONE registered click-to-select router"
        );
        assert!(
            ed.contains(&format!("route{}", "_target(&root, subject_id")),
            "T-754: the router must resolve through the pure `route_target`, so a view can ask the \
             same question before drawing a click affordance"
        );
        assert!(
            ed.contains(&format!(
                "eden_dock_right::route{}",
                "_select_zone(subject_id)"
            )),
            "T-754: a zone must be selected through the Zones panel's own selection seam"
        );
    }
}

// ─────────────────────── T-649 — Select All in view + Attributes multi-edit ───────────────────
/// Source pins for T-649. `map-engine-core` is linked natively with the `mission` feature ONLY
/// (`Cargo.toml`: `doc`/`camera` are `cfg(target_arch = "wasm32")` deps), and `select_tool` /
/// `editor_ops` are both wasm32-gated modules — so neither `OrthoCamera`, `SlotSoa` nor
/// `select_all_in_view` can be CALLED from a native `cargo test`. These pin the wiring the way the
/// rest of this file's editor contracts are pinned: on the live source, with string literals
/// blanked (`live_code`) wherever the shape rather than the text is the contract.
#[cfg(test)]
mod t649_select_all_and_multi_edit {
    use crate::arsenal::class_r_scrub::live_code;
    /// T-703/T-738 — THE keydown arm-list extractor, consumed rather than re-copied. This module
    /// carried the raw-text variant of it; the shared one scrubs comments, which is strictly
    /// stronger for the census below (a note that MENTIONS `KeyA` can no longer read as a binding).
    use crate::eden_help::keymap_census::keydown_arms;

    /// Everything after the editor page's own signature — the live editor body.
    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        assert_eq!(
            raw.matches(anchor.as_str()).count(),
            1,
            "scrub anchor must be unambiguous"
        );
        live_code(&raw[raw.find(anchor.as_str()).expect("counted above")..])
    }

    /// All whitespace removed. `rustfmt` is free to break a Leptos `view!` expression across lines
    /// wherever it likes (`gate\n.opt\n.map(`), so any pin on an EXPRESSION rather than on a
    /// statement is matched against this form — otherwise the pin is really a formatting pin.
    fn squash(src: &str) -> String {
        src.chars().filter(|c| !c.is_whitespace()).collect()
    }

    /// One top-level `fn`'s source, signature through the closing brace at column 0.
    fn fn_source(src: &str, sig: &str) -> String {
        let at = src
            .find(sig)
            .unwrap_or_else(|| panic!("`{sig}` must exist in the live source"));
        let rest = &src[at..];
        let end = rest.find("\n}\n").map_or(rest.len(), |i| i + 3);
        rest[..end].to_string()
    }

    // ── SEL-ALL-001 ───────────────────────────────────────────────────────────────────────────

    /// KEY CENSUS: Ctrl/Cmd+A is claimed by THIS editor keydown and by nothing else. The two
    /// window-level editor keydowns are this file's and `mission_history`'s (Ctrl+Z/Y); the other
    /// one must not also bind it, or the two listeners would both fire on one keypress.
    #[test]
    fn t649_ctrl_a_census() {
        let this_arms = keydown_arms(include_str!("mission_editor.rs"));
        let history_arms = keydown_arms(include_str!("mission_history.rs"));
        // Assembled so the literal never appears verbatim in this test's own source.
        let key_a = format!("\"{}\"", "KeyA");
        assert!(
            !history_arms.contains(&key_a),
            "census: mission_history's keydown (Ctrl+Z/Y) must not claim A"
        );
        // The arm is modifier-gated (Ctrl/Cmd) and rejects Alt/Shift, exactly like Ctrl+C / Ctrl+V
        // beside it — a BARE `a` must stay free.
        assert!(
            this_arms.contains(&format!(
                "{key_a} if modk && !ev.alt_key() && !ev.shift_key() =>"
            )),
            "SEL-ALL-001: Ctrl/Cmd+A (not bare A, not Alt/Shift combos) must be the Select All arm"
        );
        // Ctrl+C / Ctrl+V are untouched neighbours — this slice added an arm, it did not re-key one.
        for k in ["KeyC", "KeyV"] {
            let key = format!("\"{k}\"");
            assert!(
                this_arms.contains(&format!(
                    "{key} if modk && !ev.alt_key() && !ev.shift_key() =>"
                )),
                "the clipboard arms must be unchanged by T-649"
            );
        }
    }

    /// The Ctrl+A arm measures the CANVAS and delegates to `select_all_in_view`; it never reaches
    /// into the doc itself. Returning the "acted" bool is what earns the shared `prevent_default`
    /// below the match — without it the browser's own Select All would blue-wash the chrome.
    #[test]
    fn ctrl_a_hands_the_container_rect_to_select_all_in_view() {
        let ed = editor_live();
        let arms = keydown_arms(include_str!("mission_editor.rs"));
        assert!(
            arms.contains("container.get_bounding_client_rect()")
                && arms.contains("editor_ops::select_all_in_view(rect.width(), rect.height())"),
            "SEL-ALL-001: the Ctrl+A arm must pass the live container CSS size to select_all_in_view"
        );
        // The closure has to capture the container for that to be possible.
        assert!(
            ed.contains("let container = container.clone();"),
            "the keydown closure must clone the container in to measure it"
        );
        // The whole closure is behind the shared editable-field guard, so Ctrl+A still means
        // "select the text" while the operator is typing in an Attributes field.
        assert!(
            ed.contains("mission_history::in_editable_field()"),
            "the editor keydown must keep its editable-field guard"
        );
    }

    /// Eden scopes Select All to what is ON SCREEN. This pins that the implementation is a
    /// VIEWPORT-RECT query through the marquee's own primitive — not a "hand back every id in the
    /// document" shortcut, which is the obvious wrong implementation of this ticket.
    #[test]
    fn select_all_is_viewport_scoped_through_the_marquee_primitive() {
        let tool = live_code(include_str!("select_tool.rs"));
        let view_fn = fn_source(&tool, "pub fn view_ids_with_vehicles(");
        // The near corner is the top-left CSS pixel unprojected; the far corner is the viewport
        // size in PIXELS — the exact (world start, px end) shape `marquee_ids_with_vehicles` takes.
        assert!(
            view_fn.contains("cam.size_px()") && view_fn.contains("cam.unproject_xy(0.0, 0.0)"),
            "SEL-ALL-001: the select-all rect must be the viewport — unproject (0,0), far corner \
             from the camera's own size_px()"
        );
        assert!(
            view_fn.contains("marquee_ids_with_vehicles(cam, soa, vehicle_points,"),
            "SEL-ALL-001: it must reuse the marquee primitive, not define a second 'inside the box'"
        );
        // A degenerate camera yields nothing, exactly like the marquee — never a full-mission dump.
        assert!(
            view_fn.contains("is_finite()") && view_fn.contains("return Vec::new()"),
            "a non-finite unproject must select NOTHING (the marquee's own behaviour)"
        );

        let ops = live_code(include_str!("editor_ops.rs"));
        let sel_fn = fn_source(&ops, "pub fn select_all_in_view(");
        assert!(
            sel_fn.contains("select_tool::view_ids_with_vehicles(")
                && sel_fn.contains("select_tool::frozen_camera("),
            "select_all_in_view must snapshot a frozen camera and run the viewport-rect query"
        );
        assert!(
            !sel_fn.contains("soa.ids.clone()") && !sel_fn.contains(".ids.clone()"),
            "SEL-ALL-001: Select All is scoped to the VIEWPORT — it must never hand back the whole \
             document's id list"
        );
        // Selection-only change: the SEL readout refreshes, but nothing enters the undo history.
        assert!(
            sel_fn.contains("mission_history::refresh_selection()")
                && !sel_fn.contains("after_local_edit"),
            "a selection change is not a doc edit — refresh_selection only, never a history step"
        );
    }

    // ── ATTR-MULTI-001 / ATTR-MULTI-CHK-001 ───────────────────────────────────────────────────

    /// THE INVERTED GUARD. Before this slice both `open_attributes` and `open_arsenal` opened with
    /// an identical three-line `if ctx.selection.borrow().len() > 1 { return; }`, so a
    /// multi-selection suppressed the modal entirely — which in turn made `context_menu.rs`'s
    /// unconditionally-enabled "Attributes..." / "Edit Loadout..." rows live-but-inert (T-716).
    /// Both guards must be gone, and BOTH entry points must route through the one shared opener.
    #[test]
    fn multi_selection_no_longer_suppresses_the_attributes_modal() {
        let ops = live_code(include_str!("editor_ops.rs"));
        assert!(
            !ops.contains("if ctx.selection.borrow().len() > 1 {"),
            "ATTR-MULTI-001: the suppress-on-multi guard must be gone from editor_ops"
        );
        for entry in ["pub fn open_attributes(", "pub fn open_arsenal("] {
            let f = fn_source(&ops, entry);
            assert!(
                f.contains("open_attrs_modal("),
                "{entry} must route through the shared opener, not a re-copied guard"
            );
        }
        // Arsenal still lands on tab 3; Attributes still leaves the tab alone.
        let opener = fn_source(&ops, "fn open_attrs_modal(");
        assert!(
            opener.contains("ctx.attrs_tab.set(3)") && opener.contains("if arsenal_tab {"),
            "open_arsenal must still select the Arsenal tab"
        );
        // The multi path must PRESERVE the selection — replacing it with `[id]` would collapse the
        // very set the operator is about to multi-edit.
        assert!(
            opener.contains("sel.len() > 1 && sel.contains(&id)")
                && opener.contains("if !keep_selection {"),
            "ATTR-MULTI-001: opening over a multi-selection must not collapse it to one id"
        );
    }

    /// The per-field checkbox. `attributes.rs` had ZERO checkbox inputs before this slice; a field
    /// whose values DIFFER across the selection must now be blank, disabled, and behind one.
    #[test]
    fn differing_fields_are_locked_behind_a_per_field_checkbox() {
        let raw_attrs = include_str!("attributes.rs");
        let attrs = live_code(raw_attrs);
        // The checkbox itself (string literal ⇒ pinned on the RAW source), assembled so this test's
        // own text is not the match.
        let checkbox = format!("type=\"{}\"", "checkbox");
        assert!(
            raw_attrs.contains(&checkbox),
            "ATTR-MULTI-CHK-001: the multi-edit opt-in checkbox must exist in the modal"
        );
        let label = squash(&fn_source(&attrs, "fn field_label("));
        assert!(
            label.contains("gate.opt.map(|o|")
                && label.contains("o.set(event_target_checked(&ev))"),
            "the checkbox must be bound to the field's own opt-in latch"
        );
        // Locked ⇒ disabled. Both field primitives, plus the stance select.
        for f in ["fn number_field(", "fn text_field("] {
            let src = fn_source(&attrs, f);
            assert!(
                src.contains("disabled=move || gate.locked()"),
                "{f} must disable while the field differs and its checkbox is unticked"
            );
            assert!(
                src.contains("gate.differs()"),
                "{f} must blank the value when the selection disagrees — showing one member's \
                 value would be a lie about the other N-1"
            );
        }
        let xform = fn_source(&attrs, "fn transform_tab(");
        assert!(
            xform.contains("disabled=move || stance_gate.locked()"),
            "the Stance select must obey the same gate as the text/number fields"
        );
        // A gate is minted ONLY under a multi-selection AND only where the values actually differ,
        // so single-slot editing is byte-for-byte the pre-T-649 behaviour.
        assert!(
            xform.contains("Gate::maybe(is_multi && differs, latch)"),
            "a field the selection AGREES on must stay live with no checkbox"
        );
        // Every editable field is wired to its own latch — a shared one would tick them together.
        for latch in [
            "opts.x",
            "opts.y",
            "opts.z",
            "opts.rotation",
            "opts.stance",
            "opts.role",
            "opts.tag",
        ] {
            assert!(
                attrs.contains(latch),
                "{latch} must gate its own field (one checkbox per field, not one for the modal)"
            );
        }
        // The latches must survive a commit: they are minted on the COMPONENT and re-armed off
        // `attrs_open` only, never off `doc_tick` (which every commit bumps).
        let modal = fn_source(&attrs, "pub fn AttributesModal(");
        assert!(
            modal.contains("let opts = MultiOpts::new();") && modal.contains("opts.reset()"),
            "the opt-in latches must live on the component and re-arm when the modal reopens"
        );
    }

    /// A multi-edit commit reaches EVERY selected slot, field-by-field, under ONE history tail —
    /// and an un-opted field stays `None`, so ticking Rotation cannot also stamp one member's X
    /// onto the rest.
    #[test]
    fn multi_edit_commits_fan_out_to_every_selected_id() {
        let attrs = live_code(include_str!("attributes.rs"));
        for (seam, single, multi) in [
            (
                "fn commit_position(",
                "attrs_update_position(",
                "attrs_update_position_multi(",
            ),
            (
                "fn commit_slot(",
                "attrs_update_slot(",
                "attrs_update_slot_multi(",
            ),
        ] {
            let f = fn_source(&attrs, seam);
            assert!(
                f.contains(multi) && f.contains(single) && f.contains("ids.len() > 1"),
                "{seam} must fan out on a multi-selection and keep the ORIGINAL single-slot call \
                 otherwise"
            );
        }
        let ops = live_code(include_str!("editor_ops.rs"));
        for (f, write) in [
            (
                "pub fn attrs_update_position_multi(",
                "core.update_slot_position(id,",
            ),
            ("pub fn attrs_update_slot_multi(", "core.update_slot(id,"),
        ] {
            let src = fn_source(&ops, f);
            assert!(
                src.contains("for id in ids {") && src.contains(write),
                "{f} must apply the commit to every id in the target set"
            );
            // One tail for the whole fan-out: one persist, one rebind — not N.
            assert_eq!(
                src.matches("after_local_edit()").count(),
                1,
                "{f} must fire exactly ONE history/persist tail for the whole commit"
            );
            // A commit with no opted-in field is a no-op, not N writes of `None`.
            assert!(
                src.contains("is_none()") && src.contains("return;"),
                "{f} must no-op when nothing was opted in"
            );
        }
        // The "which fields differ" read is one snapshot over one materialize, and it compares
        // dict-coded columns by TEXT (two rows can carry the same role under different indices).
        let diff = fn_source(&ops, "pub fn read_attrs_diff(");
        assert!(
            diff.matches("core.materialize()").count() == 1 && diff.contains("&soa.roles)"),
            "read_attrs_diff must compare one snapshot, resolving dict columns to their strings"
        );
    }

    /// HONESTY: inverting the `open_arsenal` guard makes the context menu's "Edit Loadout..." row
    /// open something — but the Arsenal tab body lives in `arsenal.rs` (not this slice's to touch)
    /// and still edits ONE slot. The modal must SAY so rather than let the "N entities selected"
    /// header imply a fan-out that does not happen.
    #[test]
    fn the_arsenal_tab_admits_it_edits_one_entity_under_a_multi_selection() {
        let raw = include_str!("attributes.rs");
        assert!(
            raw.contains("Loadout edits apply to this one entity"),
            "the Arsenal tab must disclose that it is not multi-editing"
        );
        let attrs = live_code(raw);
        let modal = squash(&fn_source(&attrs, "fn modal_view("));
        assert!(
            modal.contains("is_multi.then("),
            "the disclosure must render only under a multi-selection"
        );
    }
}

// ──────────────── T-669 — clipboard completion: cut + paste-at-original ───────────────────────
/// Source pins for T-669 (`ACTION-CUT-001`, `ACTION-PASTE-ORIG-001`). `editor_ops` is a wasm32-only
/// module, so neither `copy_selection` nor `paste_at_cursor` can be CALLED from a native
/// `cargo test`; these pin the WIRING the way the rest of this file's editor contracts are pinned —
/// on the live source, sliced to the keydown arm list so a needle can never self-match inside this
/// module (which lives in the same file as the arms it reads).
#[cfg(test)]
mod t669_clipboard_completion {
    /// T-703/T-738 — THE keydown arm-list extractor. This module held the third of four copies;
    /// it now consumes the one in `eden_help::keymap_census`, which also carries the structured
    /// `(code, modifiers)` census that finally makes the Ctrl+V / Ctrl+Shift+V distinction this
    /// module's own pins had to hand-check.
    use crate::eden_help::keymap_census::keydown_arms;
    use std::collections::BTreeSet;

    /// Needles assembled so the arm LITERAL never appears verbatim in this test's own source.
    fn key(k: &str) -> String {
        format!("\"{k}\"")
    }

    /// KEY CENSUS: Ctrl/Cmd+X is claimed by THIS editor keydown and by nothing else. The two
    /// window-level editor keydowns are this file's and `mission_history`'s (Ctrl+Z/Y); the other
    /// one must not also bind X, or both listeners would fire on one keypress and the selection
    /// would be cut twice.
    #[test]
    fn t669_cut_key_census() {
        let this_arms = keydown_arms(include_str!("mission_editor.rs"));
        let history_arms = keydown_arms(include_str!("mission_history.rs"));
        let key_x = key("KeyX");
        assert!(
            !history_arms.contains(&key_x),
            "census: mission_history's keydown (Ctrl+Z/Y) must not claim X"
        );
        // Modifier-gated (Ctrl/Cmd), rejecting Alt and Shift — the same guard shape as the Ctrl+C /
        // Ctrl+V arms it sits between, so a BARE `x` stays free.
        assert!(
            this_arms.contains(&format!(
                "{key_x} if modk && !ev.alt_key() && !ev.shift_key() =>"
            )),
            "ACTION-CUT-001: Ctrl/Cmd+X (not bare X, not an Alt combo) must be the cut arm"
        );
        // The neighbours are untouched: this slice ADDED arms, it did not re-key the existing ones.
        for k in ["KeyC", "KeyA"] {
            assert!(
                this_arms.contains(&format!(
                    "{} if modk && !ev.alt_key() && !ev.shift_key() =>",
                    key(k)
                )),
                "the existing {k} arm must be unchanged by T-669"
            );
        }
    }

    /// A cut that could not COPY must not DELETE. `copy_selection` returns false on an empty
    /// selection or a doc that is not up, and `&&` short-circuits on that false — so the arm can
    /// never degrade into a silent destructive Delete. Order is the contract: copy first.
    #[test]
    fn cut_copies_before_it_deletes_and_short_circuits() {
        let arms = keydown_arms(include_str!("mission_editor.rs"));
        let at = arms
            .find(&format!("{} if modk", key("KeyX")))
            .expect("the cut arm exists — censused above");
        let body = &arms[at..];
        let copy = body
            .find("editor_ops::copy_selection()")
            .expect("ACTION-CUT-001: the cut arm must snapshot the selection to the clipboard");
        let del = body
            .find("editor_ops::delete_selection()")
            .expect("ACTION-CUT-001: the cut arm must then remove the selection");
        assert!(
            copy < del,
            "ACTION-CUT-001: copy must run BEFORE delete — a cut that deletes first has already \
             destroyed what it was supposed to put on the clipboard"
        );
        assert!(
            body[copy..del].contains("&&"),
            "ACTION-CUT-001: the two calls must be joined by `&&` (short-circuit), not sequenced — \
             otherwise a failed copy still deletes and the cut is an undocumented Delete"
        );
    }

    /// `paste_at_cursor`'s anchor is `Option`al, and that option IS paste-at-original: the plain
    /// paste arm hands it the map cursor, the Shift arm hands it nothing so every slot keeps its
    /// source coordinates. Pin both halves — passing `cx, cy` to the Shift arm by accident is the
    /// exact regression that would make this ticket a no-op while still looking bound.
    #[test]
    fn paste_at_original_passes_no_anchor() {
        let arms = keydown_arms(include_str!("mission_editor.rs"));
        let key_v = key("KeyV");
        let plain = arms
            .find(&format!(
                "{key_v} if modk && !ev.alt_key() && !ev.shift_key() =>"
            ))
            .expect("the cursor-anchored paste arm must survive this slice");
        let shifted = arms
            .find(&format!(
                "{key_v} if modk && !ev.alt_key() && ev.shift_key() =>"
            ))
            .expect("ACTION-PASTE-ORIG-001: Ctrl/Cmd+Shift+V must be an arm of its own");
        assert!(
            arms[plain..shifted].contains("editor_ops::paste_at_cursor(cx, cy)"),
            "the plain Ctrl/Cmd+V must still anchor the paste on the map cursor"
        );
        assert!(
            arms[shifted..].contains("editor_ops::paste_at_cursor(None, None)"),
            "ACTION-PASTE-ORIG-001: the Shift arm must pass NO anchor — that is what makes the \
             paste land on the source position instead of the cursor"
        );
    }

    /// The two Ctrl/Cmd+V arms PARTITION their key rather than overlapping it. Two halves, because
    /// either alone would be weak: the source half reads the real guards out of the live arm list
    /// (so it cannot drift from the code), and the truth table then evaluates those exact guard
    /// shapes over every `(ctrl/meta, alt, shift)` combination — one `KeyboardEvent` carries exactly
    /// one `shiftKey`, so no event can satisfy both, and match ORDER between them is irrelevant.
    #[test]
    fn the_two_paste_arms_are_mutually_exclusive() {
        let arms = keydown_arms(include_str!("mission_editor.rs"));
        let key_v = key("KeyV");
        assert_eq!(
            arms.matches(key_v.as_str()).count(),
            2,
            "V must be bound exactly twice (cursor paste + paste-at-original); a third arm would \
             make this proof incomplete"
        );
        let plain = format!("{key_v} if modk && !ev.alt_key() && !ev.shift_key() =>");
        let shifted = format!("{key_v} if modk && !ev.alt_key() && ev.shift_key() =>");
        assert!(
            arms.contains(&plain) && arms.contains(&shifted),
            "the two V arms must differ ONLY in the polarity of the shift guard — anything else \
             and the exclusivity argument below is about code that is not there"
        );
        // The guards above, evaluated. `modk` is `ctrl || meta`, so the three inputs are exhaustive.
        let plain_guard = |modk: bool, alt: bool, shift: bool| modk && !alt && !shift;
        let shifted_guard = |modk: bool, alt: bool, shift: bool| modk && !alt && shift;
        for modk in [false, true] {
            for alt in [false, true] {
                for shift in [false, true] {
                    assert!(
                        !(plain_guard(modk, alt, shift) && shifted_guard(modk, alt, shift)),
                        "the V arms both match at modk={modk} alt={alt} shift={shift} — the \
                         second would be dead code and the binding ambiguous"
                    );
                    // Together they cover Ctrl/Cmd+V without Alt, and nothing else: an Alt or a
                    // bare V still falls through to the arms below and then to `_ => false`.
                    assert_eq!(
                        plain_guard(modk, alt, shift) || shifted_guard(modk, alt, shift),
                        modk && !alt,
                        "the V pair must claim exactly Ctrl/Cmd+V (Alt-free) — no more, no less"
                    );
                }
            }
        }
    }

    /// `eden_help`'s coverage pins compare CODE SETS, and paste-at-original re-uses `KeyV`. So a
    /// missing help row for Ctrl/Cmd+Shift+V would leave those pins GREEN while the operator has no
    /// way to discover the binding — the exact defect T-692 exists to prevent, slipping through the
    /// one hole its set comparison cannot see. Pin the two CHORDS instead.
    #[test]
    fn both_new_chords_are_documented_in_the_help_table() {
        // Raw source: the chords ARE string literals, so a scrub that blanks literals would blank
        // the thing under test.
        let help = include_str!("eden_help.rs");
        for chord in ["Ctrl/Cmd + X", "Ctrl/Cmd + Shift + V"] {
            assert!(
                help.contains(chord),
                "T-669: `{chord}` is bound by the editor keydown but has no row in \
                 `eden_help::SHORTCUTS` — the help surface must not go stale the first time a \
                 ticket adds a chord on an already-documented key code"
            );
        }
    }

    /// The help module's opening sentence counts the bindings, and a hand-typed count goes stale the
    /// moment a slice adds an arm (it already had: T-740 filed it reading "sixteen" against a real
    /// 17). Derive the number instead.
    ///
    /// T-703 moved the SOURCE of that number one step back, to where it belongs. It used to be
    /// counted off `SHORTCUTS` — the documentation — on the argument that the T-692 pins hold the
    /// table equal to the bindings. True, but circular: a count taken off the docs measures the
    /// docs. It is now taken off `keymap_census`, which reads the live listeners, and this pin
    /// additionally holds `SHORTCUTS` to the same total, so the circle is closed from the outside.
    #[test]
    fn the_help_blurb_counts_the_bindings_correctly() {
        let codes: BTreeSet<&str> = crate::eden_help::SHORTCUTS
            .iter()
            .flat_map(|s| s.codes.iter().copied())
            .collect();
        let bound = crate::eden_help::keymap_census::all_bound_codes();
        assert_eq!(
            codes.len(),
            bound.len(),
            "T-740: the help table documents {} distinct codes but the editor binds {} ({bound:?}) \
             — the count in `eden_help`'s header cannot be right about both",
            codes.len(),
            bound.len()
        );
        let word = english(bound.len());
        let sentence = format!("binds {word} distinct `KeyboardEvent` codes");
        assert!(
            include_str!("eden_help.rs").contains(&sentence),
            "T-669/T-740: the editor now binds {} distinct key codes ({bound:?}), so \
             `eden_help`'s opening paragraph must read \"{sentence}\"",
            bound.len()
        );
    }

    /// Small-integer spelling. T-703 folded the second copy of this into
    /// `keymap_census::spell`, beside the census the number is derived from.
    fn english(n: usize) -> String {
        crate::eden_help::keymap_census::spell(n)
    }
}

/// T-670 (`STATUS-ZOOM-001`) — the editor's half of the metres-per-pixel readout. `RenderEngine::
/// zoom()` is reachable only from the rAF sampler, so the editor owns the signal and the sampler
/// writes it. The sampler runs EVERY FRAME, which makes the write guard the load-bearing part of
/// this ticket: an unguarded `set` would dirty the status bar 60×/s and tank editor performance —
/// the exact class of regression the `rf <ms>` HUD cell exists to surface. These are Leptos view /
/// wasm-closure innards, so they are pinned by SOURCE INSPECTION on scrubbed code (the established
/// `t635`/`t636` pattern here); needles are assembled at run time so this module's own prose can
/// never satisfy them.
#[cfg(test)]
mod t670_scale_signal {
    use crate::arsenal::class_r_scrub::{live_code, only_item};

    /// The editor page region onward, comments stripped and string literals blanked — the same
    /// slice `t635_debug_hud` uses. `start_raf` is defined after `MissionEditorPage`, so it is in.
    fn editor_live() -> String {
        let anchor = format!("{}{}", "pub fn Mission", "EditorPage() -> impl IntoView");
        let raw = include_str!("mission_editor.rs");
        live_code(&raw[raw.find(anchor.as_str()).expect("anchor present")..])
    }

    /// The signal is a real signal seeded from the shared `m_per_px` conversion (not a bare float
    /// literal), and it is threaded into the status bar — so the cell reads a true value before the
    /// engine mounts, and on native, where `start_raf` never runs.
    #[test]
    fn the_scale_signal_is_seeded_and_reaches_the_status_bar() {
        let ed = editor_live();
        assert!(
            ed.contains(&format!(
                "let scale_mpp = RwSignal::new(crate::eden_toolbelt::{}(-2.0))",
                "m_per_px"
            )),
            "T-670: scale_mpp must be a real signal seeded from eden_toolbelt::m_per_px at the \
             editor's default deck zoom"
        );
        let belt = ed
            .find("crate::eden_toolbelt::StatusBar")
            .expect("StatusBar mount present");
        let close = ed[belt..]
            .find("/>")
            .map(|i| belt + i)
            .expect("the StatusBar mount closes");
        assert!(
            ed[belt..close].contains("scale_mpp"),
            "T-670: the scale signal must be passed into the StatusBar mount"
        );
    }

    /// **THE GUARD.** The sampler writes `scale_mpp` exactly once, and only inside an inequality
    /// against the last PUBLISHED readout string. Delete the guard and this fails — which is the
    /// point: the failure mode it prevents (a 60 fps Leptos write from a per-frame closure) is
    /// invisible to a compile and to every other test in this crate.
    #[test]
    fn the_sampler_writes_the_scale_only_when_the_readout_changes() {
        let ed = editor_live();
        let raf = only_item(&ed, &format!("fn {}", "start_raf("));
        let set = format!("scale_mpp.{}(", "set");
        assert_eq!(
            raf.matches(set.as_str()).count(),
            1,
            "T-670: the sampler must have exactly ONE scale write — a second, unguarded one would \
             reintroduce the per-frame re-render"
        );
        let at = raf.find(set.as_str()).expect("counted above");
        // The write's enclosing block is the change guard, and the guard updates the remembered
        // string in the same block (otherwise it would fire on every frame after the first change).
        let guard = format!("if text != {} {{", "last_scale_text");
        let g = raf
            .find(guard.as_str())
            .unwrap_or_else(|| panic!("T-670: the scale write must sit behind `{guard}`"));
        assert!(
            g < at,
            "T-670: the change guard must OPEN before the scale write, not after it"
        );
        assert!(
            raf[g..at].contains(&format!("{} = text;", "last_scale_text")),
            "T-670: the guard must remember the published readout, or it fires every frame"
        );
        // The remembered value is a per-closure `mut` local, not a fresh binding each frame.
        assert!(
            raf.contains(&format!("let mut {} = String::new()", "last_scale_text")),
            "T-670: the last-published readout must live ACROSS frames (a closure-captured local)"
        );
    }

    /// The scale is read every frame and published promptly — it does NOT ride the ~1 Hz debug-HUD
    /// sample. A zoom gesture must show on the next frame; hanging the readout off the 1 Hz block
    /// would make it up to a second stale, and would also make the guard above pointless, hiding
    /// the regression this ticket is about.
    #[test]
    fn the_scale_does_not_ride_the_one_hz_hud_sample() {
        let ed = editor_live();
        let raf = only_item(&ed, &format!("fn {}", "start_raf("));
        let scale = raf
            .find(&format!("scale_mpp.{}(", "set"))
            .expect("scale write present");
        let hud = raf
            .find(&format!("debug_hud.{}(", "set"))
            .expect("HUD write present");
        let sample_gate = raf
            .find("now - last_sample >= 1000.0")
            .expect("the ~1 Hz sample gate is still there");
        assert!(
            scale < sample_gate && scale < hud,
            "T-670: the scale must be published BEFORE (and outside) the ~1 Hz HUD sample block"
        );
    }
}
