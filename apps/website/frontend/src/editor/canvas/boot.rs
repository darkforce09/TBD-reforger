//! Role: boot.
//! Position: `editor/canvas` in the frontend editor adapter.
//! Signals & state: host signals, input state, and explicit map-engine `data::store` calls.
//! Invariants: preserve input routing, borrow lifetimes, and post-edit refresh order.

#![allow(dead_code)]

#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// Boot phase.
#[derive(Clone, PartialEq, Debug)]
pub(crate) enum BootPhase {
    /// IDB restore + server hydrate in flight.
    Hydrating,

    /// Doc ready; engine/atlas/world residency still settling.
    LoadingMap,

    /// Doc hydrated + world settled — overlay hidden.
    Ready,

    /// A boot segment failed unrecoverably. Terminal: the overlay shows the error state (the failing segment + the underlying reason), not the bar. `seg` names which [`boot_progress::BootSeg`] broke so the caption reads "Rendering engine failed" rather than a generic apology; `reason` is the loader's own error text.
    Failed {
        seg: boot_progress::BootSeg,
        reason: String,
    },
}

impl BootPhase {
    /// Fold a boot transition, with `Failed` **sticky**.
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
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(crate) const BOOT_HANDOVER_MS: i32 = 220;

/// Boot progress.
pub mod boot_progress {
    /// Re-export `website_map_engine::streaming::bridge::progress::BootEvent`.
    pub use website_map_engine::streaming::bridge::progress::BootEvent;
    /// Re-export `website_map_engine::streaming::bridge::progress::BootSeg`.
    pub use website_map_engine::streaming::bridge::progress::BootSeg;
    /// Re-export `website_map_engine::streaming::bridge::progress::ProgressFn`.
    #[cfg(target_arch = "wasm32")]
    pub use website_map_engine::streaming::bridge::progress::ProgressFn;

    pub trait BootSegView {
        /// Stable ordering used to select the visible loading caption.
        const ALL: [BootSeg; 4];
        /// Index into the overlay's per-segment accumulator.
        #[must_use]
        fn idx(self) -> usize;
        /// User-facing caption for the segment.
        #[must_use]
        fn title(self) -> &'static str;
    }

    impl BootSegView for BootSeg {
        const ALL: [BootSeg; 4] = [
            BootSeg::Mission,
            BootSeg::Terrain,
            BootSeg::Satellite,
            BootSeg::World,
        ];

        fn idx(self) -> usize {
            match self {
                Self::Mission => 0,
                Self::Terrain => 1,
                Self::Satellite => 2,
                Self::World => 3,
            }
        }

        fn title(self) -> &'static str {
            match self {
                Self::Mission => "Loading mission…",
                Self::Terrain => "Loading terrain…",
                Self::Satellite => "Loading satellite…",
                Self::World => "Loading world objects…",
            }
        }
    }

    /// Terrain DEM, measured on the live stack (2026-08-01): `content-length` of `/map-assets/everon/dem/everon-dem-16bit.png`.
    pub const PLANNED_TERRAIN_BYTES: u64 = 71_911_548;

    /// Satellite, measured off the live tbd-sat index: the mips from level 1 down sum to 42,152,810 B, which is what an 8192-limit `maxTextureDimension2D` uploads. A 16384-limit GPU takes level 0 as well (152,710,470 B) — the [`BootEvent::Budget`] the loader sends after reading the index replaces this with whichever of the two it is actually going to fetch, so this constant only paces the first ~2 round trips.
    pub const PLANNED_SATELLITE_BYTES: u64 = 42_152_810;

    /// Canonical planned world bytes value.
    pub const PLANNED_WORLD_BYTES: u64 = 27_000_000;

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

    /// Three properties, all pinned by `t628_boot_progress`:.
    #[derive(Clone, Copy, PartialEq, Debug)]
    pub struct BootProgress {
        segs: [Segment; 4],

        floor: f64,
    }

    impl Default for BootProgress {
        fn default() -> Self {
            Self::new()
        }
    }

    impl BootProgress {
        /// New.
        #[must_use]
        pub const fn new() -> Self {
            Self {
                segs: [
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

        /// The weighted ratio as it stands *right now* — may be lower than [`Self::percent`] after a budget grew. Exposed for the tests that prove the difference between the two is exactly the monotonicity guarantee.
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

            pct.clamp(0.0, 100.0)
        }

        /// What the bar draws: 0..=100, monotonically non-decreasing for the life of the boot.
        #[must_use]
        pub fn percent(&self) -> f64 {
            self.floor
        }

        /// Is complete.
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

        /// The line under the bar: the overall percentage, then what the current stage has actually counted. A stage that has not yet read its own budget shows the percentage alone rather than a denominator nobody measured.
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

    /// `"214 / 834 files"` — the world segment counts completed fetches, so it says so instead of borrowing the byte formatter and implying a byte budget nothing published.
    #[must_use]
    pub fn fmt_files_pair(done: u64, total: u64) -> String {
        format!("{done} / {total} files")
    }

    /// `done / total` as 0..=100. Clamped at the top so a server that returns one byte more than the index promised cannot push the bar past the end of its track, and `0` for a zero total (nothing measured is nothing done, not a division).
    #[must_use]
    pub fn percent(done: u64, total: u64) -> f64 {
        if total == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        let pct = (done as f64 / total as f64) * 100.0;
        pct.clamp(0.0, 100.0)
    }

    /// `"47.3 MB / 152.7 MB"` — both sides in whatever unit `total` warrants, so the pair reads as one measurement instead of switching units under the operator mid-download. Base-10 (MB, not MiB) to match the manifest's `bytes` field and every browser download UI.
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

/// Hand over.
#[cfg(target_arch = "wasm32")]
pub(crate) fn hand_over(boot: RwSignal<BootPhase>) {
    use wasm_bindgen::prelude::Closure;
    use wasm_bindgen::JsCast;

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
