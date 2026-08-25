//! T-934.12 — the Mission Creator BOOT machine, split out of `mission_editor.rs` (Phase B; audit
//! §4 Phase 1 item 3): the [`BootPhase`] overlay state (T-175 B5, with the T-631 sticky `Failed`
//! terminal), the [`boot_progress`] arithmetic module (T-627/T-628 — segment budgets and the
//! monotonic one-bar fold) and the [`hand_over`] timer that takes the overlay down only after the
//! bar has actually drawn 100%.
//!
//! Bodies are byte-identical to their `mission_editor.rs` originals, and `mission_editor`
//! re-exports every name here, so the page's bare call sites (`hand_over(boot)`,
//! `BootPhase::…`), the `crate::editor::mission_editor::boot_progress::…` paths in
//! `world_assets/*` + `state/hydrate.rs`, and the evacuated pins' `super::…` imports
//! (`t628_boot_progress`, `t631_boot_failure_state`) all keep their exact spelling.
//!
//! The DEM / satellite / world-chunk LOADER tasks this module meters did not move here — they
//! already live in `editor::world_assets` (`fetch` / `satellite` / `world_host` / `forest_mass` /
//! `labels`, plus `bootstrap` in its `mod.rs`), and the two boot `spawn_local` tasks stay inside
//! `MissionEditorPage`, closing over the page's own locals. This file deliberately carries no
//! `#[cfg(test)]`, so `class_r_scrub::live_code` keeps all of it.
// The same gate `mission_editor.rs` carries, for the same reason: `hand_over` is only reached
// from `#[cfg(target_arch = "wasm32")]` boot tasks, and the native build sees several items as
// test-pin-only.
#![allow(dead_code)]

// wasm-gated: only `hand_over` reaches leptos (`RwSignal` + `.update`), and it is wasm-only —
// an ungated glob here is a native unused-import warning.
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

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
pub(crate) enum BootPhase {
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
    pub(crate) fn advance(self, next: BootPhase) -> BootPhase {
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
pub(crate) const BOOT_HANDOVER_MS: i32 = 220;

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

/// T-628 — take the overlay down, [`BOOT_HANDOVER_MS`] after the last segment reported in.
///
/// Called from the two boot tasks' rendezvous, so by the time it runs every segment has already
/// sent `Finish` and the bar reads exactly 100%. The delay is the hand-over, not the work: without
/// it the final report and the overlay's removal are folded into one Leptos render and the operator
/// sees the bar stop short and the screen change under it. A window that has gone away simply skips
/// the timer and the overlay stays — the same thing that already happens if a boot task never
/// returns.
#[cfg(target_arch = "wasm32")]
pub(crate) fn hand_over(boot: RwSignal<BootPhase>) {
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
