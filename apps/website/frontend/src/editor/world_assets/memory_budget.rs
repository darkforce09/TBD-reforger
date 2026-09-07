//! T-938.6 — the wasm memory budget: one ledger every world-asset loader answers to.
//!
//! **The defect this exists for.** Before this module there was no budget of any kind. Each loader
//! sized its own allocation from the asset and committed to it: the terrain lane holds the 71.9 MB
//! DEM PNG and the 163.8 MB `Vec<f32>` it decodes into at the same instant, the hillshade adds
//! another 163.8 MB of RGBA, and `satellite::load_unified_full` decodes **every** tile it is going
//! to upload before it takes the engine borrow — 655,360,000 B of RGBA for everon's 12800² level 0
//! alone, ~874 MB for the whole chain below it. Nothing compared any of those figures with anything
//! else, and on wasm32 an allocation that cannot be served is not an error a caller can handle:
//! Rust's allocation-failure handler calls `abort()`, the instance traps on `unreachable`, and the
//! editor is gone. The only signal was the tab dying.
//!
//! **What replaces it.** A single `thread_local` [`Ledger`]: a byte budget (default
//! [`DEFAULT_BUDGET_MB`], overridable per boot with `?memBudgetMb=NNN` or `window.__memBudgetMb`),
//! per-asset held/peak bytes, and [`Ledger::decide`] — the one place that answers *"can this
//! allocation be served?"* with [`Decision`] rather than with a trap. `satellite.rs` asks before it
//! commits to a mip level and walks its floor down until the answer is `Ok`; a raised floor is a
//! softer basemap, which is a thing an operator can see and reason about, and it is strictly better
//! than a dead instance.
//!
//! **Two currencies, deliberately not mixed.** `held`/`peak` are *declared* bytes — figures a
//! loader computed from the asset it is about to allocate, and the only ones the budget arithmetic
//! ever reads. `growth` is *measured* wasm linear memory ([`heap_bytes`]) claimed across a phase,
//! attributed to whichever asset ran in it. Growth is the only handle on loaders this slice does
//! not own (`world_host`, `forest_mass`, `labels`, `water`), but it is a **lower bound**: the wasm
//! heap never shrinks, so a phase that reuses pages an earlier phase freed measures zero. Adding
//! the two would double-count every asset that has both, so nothing here does.
//!
//! Everything above [`heap_bytes`] is pure arithmetic with no `web_sys` in it, so it runs on the
//! host test runner through `canvas::viewport`'s `memory_budget_pure` mount — `world_assets` is
//! `#![cfg(target_arch = "wasm32")]`, so a test that lived only here would never be compiled by
//! `cargo test -p website-frontend` and would report green over code it had not examined.

use std::cell::RefCell;

/// Bytes in a mebibyte — the unit the budget, the query param and the HUD are all spelled in.
pub const MIB: u64 = 1_048_576;

/// The default ceiling, in MiB.
///
/// Not a measurement of any particular device: it is the figure the T-938 audit fixed as the point
/// past which the editor should give up resolution rather than gamble on a `grow`. everon's whole
/// declared load (terrain ~320 MB + satellite level 0 down ~979 MB) sits just under it, so the
/// default changes nothing that works today and only bites the case that was already fatal.
pub const DEFAULT_BUDGET_MB: u64 = 1536;

/// The world assets the ledger keeps separate.
///
/// One flat enum rather than a map: the set is closed (it is the list of things `bootstrap` loads),
/// the array indexing below is total, and a new asset is a compile error at every match rather than
/// a silently absent row in the HUD.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Asset {
    /// The DEM: the encoded body and the `Vec<f32>` of metres it decodes into.
    Dem,
    /// The hillshade RGBA raster built from the DEM, alive until it is uploaded.
    Hillshade,
    /// The unified satellite basemap: decoded RGBA per uploaded tile plus the bodies it decodes.
    Satellite,
    /// World chunks — roads, buildings, landcover residency.
    World,
    /// The 8 m forest-mass density bins.
    Forest,
    /// Town / road / height text labels.
    Labels,
    /// The bathymetry mask and water vectors.
    Water,
}

impl Asset {
    /// Every asset, in ledger order. `ALL[a.index()] == a` for all `a`.
    pub const ALL: [Asset; 7] = [
        Asset::Dem,
        Asset::Hillshade,
        Asset::Satellite,
        Asset::World,
        Asset::Forest,
        Asset::Labels,
        Asset::Water,
    ];

    /// Position in [`Asset::ALL`] and in [`Ledger`]'s entry array.
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Asset::Dem => 0,
            Asset::Hillshade => 1,
            Asset::Satellite => 2,
            Asset::World => 3,
            Asset::Forest => 4,
            Asset::Labels => 5,
            Asset::Water => 6,
        }
    }

    /// Stable key for the HUD and for `window.__t9386`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Asset::Dem => "dem",
            Asset::Hillshade => "hillshade",
            Asset::Satellite => "satellite",
            Asset::World => "world",
            Asset::Forest => "forest",
            Asset::Labels => "labels",
            Asset::Water => "water",
        }
    }
}

/// What the budget says about one prospective allocation.
///
/// The three arms are distinguishable on purpose. `Degrade` means *this* request is too big for
/// what is left but not for the budget itself, so a smaller variant of the same asset is worth
/// asking for — which is exactly the satellite's mip ladder. `Refuse` means the request exceeds
/// the whole budget, so no release by any other asset could ever make it fit and a retry at the
/// same size is pointless. Collapsing them to a bool would leave the caller unable to tell "ask
/// smaller" from "never".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Decision {
    /// Fits alongside everything currently held.
    Ok,
    /// Does not fit what is left, but would fit an empty budget — shrink and ask again.
    Degrade,
    /// Larger than the entire budget — no arrangement of the ledger can hold it.
    Refuse,
}

/// One asset's row.
#[derive(Clone, Copy, Default, Debug)]
pub struct Entry {
    /// Declared bytes currently reserved and not yet released.
    pub held: u64,
    /// The high-water mark of [`Entry::held`].
    pub peak: u64,
    /// Measured wasm linear memory (see [`heap_bytes`]) claimed while this asset was loading.
    /// A lower bound, never added to `held`.
    pub growth: u64,
}

/// The budget itself.
#[derive(Clone, Debug)]
pub struct Ledger {
    budget: u64,
    entries: [Entry; 7],
    total_peak: u64,
    sat_floor: Option<usize>,
    sat_raised: u32,
}

impl Ledger {
    /// A ledger with `budget` bytes and nothing held.
    #[must_use]
    pub fn with_budget(budget: u64) -> Self {
        Self {
            budget,
            entries: [Entry::default(); 7],
            total_peak: 0,
            sat_floor: None,
            sat_raised: 0,
        }
    }

    /// The ceiling, in bytes.
    #[must_use]
    pub fn budget(&self) -> u64 {
        self.budget
    }

    /// Total declared bytes currently held across every asset.
    #[must_use]
    pub fn held_total(&self) -> u64 {
        self.entries.iter().map(|e| e.held).sum()
    }

    /// High-water mark of [`Ledger::held_total`].
    #[must_use]
    pub fn total_peak(&self) -> u64 {
        self.total_peak
    }

    /// One asset's row.
    #[must_use]
    pub fn entry(&self, a: Asset) -> Entry {
        self.entries[a.index()]
    }

    /// Would `bytes` be servable right now? Pure — asks without recording.
    #[must_use]
    pub fn decide(&self, bytes: u64) -> Decision {
        if self.held_total().saturating_add(bytes) <= self.budget {
            Decision::Ok
        } else if bytes <= self.budget {
            Decision::Degrade
        } else {
            Decision::Refuse
        }
    }

    /// Ask for `bytes` on `a`'s behalf; on [`Decision::Ok`] the bytes are recorded as held.
    ///
    /// A refused or degraded request records nothing: the ledger must never claim to be holding
    /// memory that was not allocated, or the next caller is refused on the strength of a fiction.
    pub fn reserve(&mut self, a: Asset, bytes: u64) -> Decision {
        let d = self.decide(bytes);
        if d == Decision::Ok {
            let e = &mut self.entries[a.index()];
            e.held = e.held.saturating_add(bytes);
            e.peak = e.peak.max(e.held);
            self.total_peak = self.total_peak.max(self.held_total());
        }
        d
    }

    /// Record `bytes` on `a` **unconditionally** — no decision, no refusal.
    ///
    /// For memory that already exists, or that certainly will (a forecast read off the manifest
    /// before the loader runs). The budget's job is to stop a loader *before* it allocates; once
    /// the bytes are real, hiding them from the ledger would only make the next [`Ledger::reserve`]
    /// answer from a fiction — a budget that under-reports is worse than no budget, because it is
    /// believed.
    pub fn hold(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.held = e.held.saturating_add(bytes);
        e.peak = e.peak.max(e.held);
        self.total_peak = self.total_peak.max(self.held_total());
    }

    /// Replace `a`'s held bytes outright — a forecast superseded by what was actually allocated.
    /// The peak still ratchets, so a forecast larger than the truth stays in the record.
    pub fn set_held(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.held = bytes;
        e.peak = e.peak.max(e.held);
        self.total_peak = self.total_peak.max(self.held_total());
    }

    /// Give `bytes` back. Saturating: releasing more than is held is a bookkeeping bug, and
    /// wrapping to `u64::MAX` would turn it into a permanently exhausted budget.
    pub fn release(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.held = e.held.saturating_sub(bytes);
    }

    /// Record measured linear-memory growth against `a` (see [`Entry::growth`]).
    pub fn add_growth(&mut self, a: Asset, bytes: u64) {
        let e = &mut self.entries[a.index()];
        e.growth = e.growth.saturating_add(bytes);
    }

    /// Record the satellite mip level the budget settled on, and how many levels it cost.
    pub fn set_satellite_floor(&mut self, base: usize, raised: u32) {
        self.sat_floor = Some(base);
        self.sat_raised = raised;
    }

    /// The live satellite floor, once one has been chosen.
    #[must_use]
    pub fn satellite_floor(&self) -> Option<usize> {
        self.sat_floor
    }

    /// How many levels the budget raised that floor by.
    #[must_use]
    pub fn satellite_raised(&self) -> u32 {
        self.sat_raised
    }

    /// The debug-HUD tail: reserved bytes against the budget, and the current satellite floor.
    ///
    /// Empty until something is held or a floor is chosen, so the HUD carries no dead cell before
    /// the boot has anything to say. Shaped like `los_world_wasm::hud_suffix` — a leading ` · `
    /// per cell — because `canvas/viewport.rs` concatenates both onto the same line.
    #[must_use]
    pub fn hud_suffix(&self) -> String {
        if self.held_total() == 0 && self.sat_floor.is_none() {
            return String::new();
        }
        let sat = match (self.sat_floor, self.sat_raised) {
            (Some(f), 0) => format!(" · sat L{f}"),
            (Some(f), n) => format!(" · sat L{f} (+{n})"),
            (None, _) => String::new(),
        };
        format!(
            " · mem {}/{}MB{sat}",
            self.held_total() / MIB,
            self.budget / MIB
        )
    }
}

/// One satellite mip level, reduced to the two figures the budget cares about.
///
/// Deliberately not `TbdSatMip`: that type lives in `tbd_sat.rs`, which is wasm-only in the bundle,
/// and depending on it here would put this module's arithmetic out of reach of the host runner —
/// the whole reason the numbers below were never checked by anything.
#[derive(Clone, Copy, Debug)]
pub struct LevelBytes {
    /// Level width in pixels.
    pub width: u32,
    /// Level height in pixels.
    pub height: u32,
    /// Sum of this level's tile `length`s — the compressed bodies fetched before they decode.
    pub compressed: u64,
}

impl LevelBytes {
    /// Decoded RGBA for this level: 4 B per pixel, the figure `tex_layer_write_rgba` is handed.
    #[must_use]
    pub fn rgba(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height) * 4
    }
}

/// Peak wasm-heap bytes `load_unified_full` holds when its base level is `base`.
///
/// Every level from `base` down is fetched, decoded, and held **simultaneously** — the decode loop
/// fills one `Vec<(rel, tile, Decoded)>` before the engine borrow is taken — so the peak is the sum
/// of the whole tail, not the largest level in it. The compressed bodies are counted alongside
/// because they are alive across the same decode: `bodies` is consumed one element at a time while
/// the decoded RGBA accumulates.
///
/// everon, base 0: 873,813,260 B of RGBA + 152,710,470 B of bodies = 978.97 MiB. Base 1:
/// 218,453,260 + 42,152,810 = 248.53 MiB. The 3.9× step between them is the whole reason a floor
/// is worth having.
#[must_use]
pub fn satellite_resident_bytes(levels: &[LevelBytes], base: usize) -> u64 {
    levels
        .iter()
        .skip(base)
        .fold(0u64, |acc, l| acc.saturating_add(l.rgba() + l.compressed))
}

/// The outcome of walking the mip ladder for a level the budget will accept.
#[derive(Clone, Debug)]
pub struct FloorWalk {
    /// The level the GPU limit chose — the ceiling this walk started from.
    pub requested: usize,
    /// The level to load from.
    pub base: usize,
    /// [`satellite_resident_bytes`] at that level.
    pub bytes: u64,
    /// Every level rejected on the way down, with what it would have cost and why it was refused.
    pub rejected: Vec<(usize, u64, Decision)>,
}

impl FloorWalk {
    /// How many levels of resolution the budget cost.
    ///
    /// Derived from the floor that actually moved, **not** from `rejected.len()`: in the degenerate
    /// case where no level fits at all, the coarsest level is rejected too and then loaded anyway,
    /// so the rejection count is one higher than the resolution that was lost. Reporting that as a
    /// raise would put a number on the HUD that no level of the ladder corresponds to.
    #[must_use]
    pub fn raised(&self) -> u32 {
        u32::try_from(self.base.saturating_sub(self.requested)).unwrap_or(u32::MAX)
    }
}

/// Walk down from `base` until the ledger accepts a level, raising the floor by one each time.
///
/// Pure: it asks [`Ledger::decide`] and records nothing, so the caller commits the reservation once
/// — otherwise every rejected rung would leave bytes held for a level that was never loaded.
///
/// If no level fits (a budget smaller than a 1×1 mip), the coarsest level is returned with the
/// whole ladder in `rejected`. Returning "no basemap" instead would trade a soft island for a blank
/// one, and the caller already reports a downscaled basemap loudly.
#[must_use]
pub fn floor_for_budget(levels: &[LevelBytes], base: usize, ledger: &Ledger) -> FloorWalk {
    let mut rejected = Vec::new();
    let mut lvl = base;
    while lvl < levels.len() {
        let bytes = satellite_resident_bytes(levels, lvl);
        match ledger.decide(bytes) {
            Decision::Ok => {
                return FloorWalk {
                    requested: base,
                    base: lvl,
                    bytes,
                    rejected,
                }
            }
            d => rejected.push((lvl, bytes, d)),
        }
        lvl += 1;
    }
    let last = levels.len().saturating_sub(1);
    FloorWalk {
        requested: base,
        base: last,
        bytes: satellite_resident_bytes(levels, last),
        rejected,
    }
}

// ── the live, process-wide ledger ────────────────────────────────────────────────────────────

thread_local! {
    /// The one budget. Seeded from [`configured_budget_bytes`] on first touch, which is inside the
    /// editor mount — late enough for `window.location` to carry the boot's query string.
    static LEDGER: RefCell<Ledger> = RefCell::new(Ledger::with_budget(configured_budget_bytes()));
}

/// The budget for this boot, in bytes: `?memBudgetMb=NNN`, else `window.__memBudgetMb`, else
/// [`DEFAULT_BUDGET_MB`].
///
/// The param carries a **value**, not a flag, so it is read through `UrlSearchParams` rather than
/// by `search().contains(…)` — a substring test cannot tell `?memBudgetMb=512` from
/// `?xmemBudgetMb=5120`. A zero or unparseable value falls back to the default rather than
/// installing a budget nothing can fit.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn configured_budget_bytes() -> u64 {
    let from_param = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .and_then(|s| web_sys::UrlSearchParams::new_with_str(&s).ok())
        .and_then(|p| p.get("memBudgetMb"))
        .and_then(|v| v.trim().parse::<u64>().ok());
    let from_global = || {
        web_sys::window()
            .and_then(|w| {
                js_sys::Reflect::get(&w, &wasm_bindgen::JsValue::from_str("__memBudgetMb")).ok()
            })
            .and_then(|v| v.as_f64())
            .filter(|v| *v > 0.0 && v.is_finite())
            .map(|v| v as u64)
    };
    from_param
        .or_else(from_global)
        .filter(|mb| *mb > 0)
        .unwrap_or(DEFAULT_BUDGET_MB)
        .saturating_mul(MIB)
}

/// Host build: there is no `window` to read an override from, so the default is the only answer.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn configured_budget_bytes() -> u64 {
    DEFAULT_BUDGET_MB * MIB
}

/// Wasm linear memory currently claimed, in bytes.
///
/// `wasm_bindgen::memory()` is the module's own `WebAssembly.Memory`; its buffer length is the
/// figure `memory.grow` moves and the one that stops moving when a `grow` fails. It never shrinks,
/// which is exactly why [`Entry::growth`] is documented as a lower bound.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn heap_bytes() -> u64 {
    use wasm_bindgen::JsCast;
    wasm_bindgen::memory()
        .dyn_into::<js_sys::WebAssembly::Memory>()
        .ok()
        .map(|m| js_sys::ArrayBuffer::from(m.buffer()).byte_length())
        .map_or(0, u64::from)
}

/// Host build: no linear memory to sample, so growth is simply never attributed.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn heap_bytes() -> u64 {
    0
}

/// Run `f` against the live ledger.
pub fn with_ledger<R>(f: impl FnOnce(&mut Ledger) -> R) -> R {
    LEDGER.with(|l| f(&mut l.borrow_mut()))
}

// The gate itself — `reserve(bytes) -> Decision` and its non-recording twin `decide` — is
// `Ledger::reserve` / `Ledger::decide` above, reached through `with_ledger`. There is deliberately
// no free-function wrapper for either: the only caller that must ASK before allocating is the
// satellite, and it asks through [`claim_satellite_floor`], which walks the ladder and commits the
// answer in one borrow. A second, uncommitted entry point would let a caller take a decision and
// then act on a ledger that had moved underneath it.

/// Record `bytes` on the live budget unconditionally. See [`Ledger::hold`].
pub fn hold(a: Asset, bytes: u64) {
    with_ledger(|l| l.hold(a, bytes));
    publish();
}

/// Replace `a`'s held bytes on the live budget. See [`Ledger::set_held`].
pub fn set_held(a: Asset, bytes: u64) {
    with_ledger(|l| l.set_held(a, bytes));
    publish();
}

/// Release `bytes` held by `a`. See [`Ledger::release`].
pub fn release(a: Asset, bytes: u64) {
    with_ledger(|l| l.release(a, bytes));
    publish();
}

/// A linear-memory mark to hand back to [`observe_since`].
#[must_use]
pub fn heap_mark() -> u64 {
    heap_bytes()
}

/// Attribute the linear memory claimed since `mark` to `a`. Accumulates, so a loader that runs in
/// several passes ends up with the total it caused rather than the largest single pass.
pub fn observe_since(a: Asset, mark: u64) {
    let now = heap_bytes();
    with_ledger(|l| l.add_growth(a, now.saturating_sub(mark)));
}

/// Choose the satellite's mip floor under the live budget and reserve what that level will cost.
///
/// One call so the walk and the reservation cannot drift apart: [`floor_for_budget`] is pure and
/// would otherwise leave the caller free to load one level and reserve another.
#[must_use]
pub fn claim_satellite_floor(levels: &[LevelBytes], base: usize) -> FloorWalk {
    let walk = with_ledger(|l| {
        let walk = floor_for_budget(levels, base, l);
        // The chosen level is recorded even when the ledger could not accept it (the no-level-fits
        // tail above): the HUD must show the floor that is actually being loaded.
        l.reserve(Asset::Satellite, walk.bytes);
        l.set_satellite_floor(walk.base, walk.raised());
        walk
    });
    publish();
    walk
}

/// The debug-HUD tail. See [`Ledger::hud_suffix`].
#[must_use]
pub fn hud_suffix() -> String {
    with_ledger(|l| l.hud_suffix())
}

/// Mirror the ledger onto `window.__t9386` so a headless probe can read the per-asset peaks
/// without the HUD, the debug flag, or a screenshot. Shaped like T-938.2's `window.__t9382`.
#[cfg(target_arch = "wasm32")]
pub fn publish() {
    use wasm_bindgen::JsValue;
    let Some(win) = web_sys::window() else {
        return;
    };
    let obj = js_sys::Object::new();
    let set = |o: &js_sys::Object, k: &str, v: f64| {
        let _ = js_sys::Reflect::set(o, &JsValue::from_str(k), &JsValue::from_f64(v));
    };
    with_ledger(|l| {
        set(&obj, "budget", l.budget() as f64);
        set(&obj, "held", l.held_total() as f64);
        set(&obj, "peak", l.total_peak() as f64);
        set(&obj, "heap", heap_bytes() as f64);
        set(
            &obj,
            "satFloor",
            l.satellite_floor().map_or(-1.0, |f| f as f64),
        );
        set(&obj, "satRaised", f64::from(l.satellite_raised()));
        let assets = js_sys::Object::new();
        for a in Asset::ALL {
            let e = l.entry(a);
            let row = js_sys::Object::new();
            set(&row, "held", e.held as f64);
            set(&row, "peak", e.peak as f64);
            set(&row, "growth", e.growth as f64);
            let _ = js_sys::Reflect::set(&assets, &JsValue::from_str(a.name()), &row);
        }
        let _ = js_sys::Reflect::set(&obj, &JsValue::from_str("assets"), &assets);
    });
    let _ = js_sys::Reflect::set(&win, &JsValue::from_str("__t9386"), &obj);
}

/// Host build: nothing to publish onto.
#[cfg(not(target_arch = "wasm32"))]
pub fn publish() {}

// ── the arithmetic, executed on the host ─────────────────────────────────────────────────────
//
// These are BELOW every production item above, because `class_r_scrub::live_source` cuts a file
// from its first `#[cfg(test)]` to EOF: a test module in the middle would blind every source pin
// to everything after it.

#[cfg(test)]
mod t938_6 {
    use super::*;

    /// everon's real satellite ladder, read off `packages/map-assets/everon/satellite/
    /// everon-sat.tbd-sat` (the same index `t629_satellite_resolution` pins): 14 levels from
    /// 12800² down to 1×1, each level's `compressed` being the sum of its tiles' lengths.
    fn everon() -> Vec<LevelBytes> {
        const DIMS: [u32; 14] = [
            12_800, 6_400, 3_200, 1_600, 800, 400, 200, 100, 50, 25, 12, 6, 3, 1,
        ];
        // Level 0 is four 6400² tiles: 28,326,346 + 21,632,714 + 27,555,806 + 33,042,794.
        const COMPRESSED: [u64; 14] = [
            110_557_660,
            30_866_380,
            8_271_166,
            2_218_572,
            583_330,
            153_506,
            42_470,
            12_086,
            3_584,
            1_138,
            328,
            126,
            86,
            38,
        ];
        DIMS.iter()
            .zip(COMPRESSED)
            .map(|(&d, compressed)| LevelBytes {
                width: d,
                height: d,
                compressed,
            })
            .collect()
    }

    /// 512 MiB, 1024 MiB — the two budgets the cases below turn on.
    const MB_512: u64 = 512 * MIB;
    const MB_1024: u64 = 1024 * MIB;
    /// everon's DEM raster: 6400² metres at 4 B each, held for the whole boot.
    const DEM_METERS: u64 = 6_400 * 6_400 * 4;

    #[test]
    fn the_everon_ladder_costs_what_the_audit_says() {
        let l = everon();
        assert_eq!(
            l[0].rgba(),
            655_360_000,
            "the audit's headline figure: satellite L0 is 655 MB of RGBA on its own"
        );
        assert_eq!(
            satellite_resident_bytes(&l, 0),
            1_026_523_730,
            "a base-0 load holds the WHOLE tail at once — every level is decoded before the \
             engine borrow is taken — so the peak is 873,813,260 B of RGBA plus 152,710,470 B of \
             bodies, not just the largest level"
        );
        assert_eq!(
            satellite_resident_bytes(&l, 1),
            260_606_070,
            "one level down is 218,453,260 + 42,152,810"
        );
        assert_eq!(satellite_resident_bytes(&l, 13), 4 + 38, "the 1x1 tail");
        assert_eq!(
            satellite_resident_bytes(&l, 99),
            0,
            "a base past the ladder costs nothing rather than panicking"
        );
    }

    #[test]
    fn decide_separates_shrink_from_never() {
        let mut led = Ledger::with_budget(1_000);
        assert_eq!(led.decide(1_000), Decision::Ok, "exactly the budget fits");
        assert_eq!(
            led.decide(1_001),
            Decision::Refuse,
            "bigger than the whole budget can never be made to fit, so the caller must not be \
             told to retry"
        );
        assert_eq!(led.reserve(Asset::Dem, 600), Decision::Ok);
        assert_eq!(
            led.decide(500),
            Decision::Degrade,
            "500 does not fit the 400 that is left but would fit an empty budget — that is the \
             `ask smaller` answer, and collapsing it into Refuse is what would leave the \
             satellite with no ladder to walk"
        );
        assert_eq!(led.decide(400), Decision::Ok, "the remainder still fits");
        assert_eq!(led.decide(0), Decision::Ok, "a zero request is free");
    }

    #[test]
    fn reserve_records_only_on_ok() {
        let mut led = Ledger::with_budget(1_000);
        assert_eq!(led.reserve(Asset::Dem, 600), Decision::Ok);
        assert_eq!(led.held_total(), 600);
        assert_eq!(
            led.reserve(Asset::Satellite, 500),
            Decision::Degrade,
            "reserve and decide answer with the same arm"
        );
        assert_eq!(
            led.held_total(),
            600,
            "a degraded request must record NOTHING: bytes the loader never allocated would \
             refuse the next caller on the strength of a fiction"
        );
        assert_eq!(led.reserve(Asset::Satellite, 5_000), Decision::Refuse);
        assert_eq!(led.held_total(), 600);
        assert_eq!(led.entry(Asset::Satellite).held, 0);
        assert_eq!(led.entry(Asset::Satellite).peak, 0);
    }

    #[test]
    fn peaks_are_per_asset_high_water_marks() {
        let mut led = Ledger::with_budget(MB_1024);
        assert_eq!(led.reserve(Asset::Dem, 71_911_548), Decision::Ok);
        assert_eq!(led.reserve(Asset::Dem, DEM_METERS), Decision::Ok);
        let peak = 71_911_548 + DEM_METERS;
        assert_eq!(
            led.entry(Asset::Dem).peak,
            peak,
            "the terrain lane's peak is the encoded body AND the raster it decodes into, alive \
             across the same decode call"
        );
        led.release(Asset::Dem, 71_911_548);
        assert_eq!(led.entry(Asset::Dem).held, DEM_METERS);
        assert_eq!(
            led.entry(Asset::Dem).peak,
            peak,
            "releasing must not lower the peak — the peak is the number the budget was sized for"
        );
        assert_eq!(led.total_peak(), peak);
        led.release(Asset::Dem, u64::MAX);
        assert_eq!(
            led.entry(Asset::Dem).held,
            0,
            "over-release saturates; wrapping to u64::MAX would install a budget nothing fits"
        );
    }

    /// **The floor test.** This is the one the perturbation reddens: remove the `Degrade` arm from
    /// [`Ledger::decide`] and the walk accepts level 0, so `base` stays 0 and `raised()` is 0.
    #[test]
    fn the_floor_rises_one_level_per_degrade() {
        let l = everon();
        let mut led = Ledger::with_budget(MB_1024);
        assert_eq!(led.reserve(Asset::Dem, DEM_METERS), Decision::Ok);

        let walk = floor_for_budget(&l, 0, &led);
        assert_eq!(
            walk.base, 1,
            "level 0 costs 1,026,523,730 B, which fits a 1024 MiB budget on its own but NOT \
             beside the 163,840,000 B DEM raster — so the floor must rise by exactly one"
        );
        assert_eq!(walk.raised(), 1);
        assert_eq!(
            walk.rejected,
            vec![(0, 1_026_523_730, Decision::Degrade)],
            "the rejected rung must name the level, its cost and WHY — a bare floor number \
             cannot tell an operator whether the budget or the GPU took his resolution"
        );
        assert_eq!(walk.bytes, 260_606_070);
        assert_eq!(
            led.decide(walk.bytes),
            Decision::Ok,
            "the level the walk lands on must actually be servable"
        );
    }

    #[test]
    fn a_tighter_budget_walks_further_down_the_ladder() {
        let l = everon();
        let led = Ledger::with_budget(MB_512);
        let walk = floor_for_budget(&l, 0, &led);
        assert_eq!(
            walk.base, 1,
            "512 MiB is smaller than level 0's 978.97 MiB outright — Refuse, not Degrade — and \
             level 1's 248.53 MiB fits"
        );
        assert_eq!(walk.rejected.len(), 1);
        assert_eq!(walk.rejected[0].2, Decision::Refuse);

        let led = Ledger::with_budget(64 * MIB);
        let walk = floor_for_budget(&l, 0, &led);
        assert_eq!(
            walk.base, 2,
            "62.85 MiB from level 2 down is the first rung under a 64 MiB budget"
        );
        assert_eq!(walk.raised(), 2);
    }

    #[test]
    fn the_walk_never_reaches_past_the_ladder() {
        let l = everon();
        let led = Ledger::with_budget(1);
        let walk = floor_for_budget(&l, 0, &led);
        assert_eq!(
            walk.base,
            l.len() - 1,
            "when nothing fits, the coarsest level is loaded and the whole ladder is reported — \
             a blank basemap is strictly worse than a 1x1 one, and the caller warns either way"
        );
        assert_eq!(
            walk.raised(),
            13,
            "the floor moved 13 levels, even though 14 rungs were rejected — the coarsest was \
             rejected AND loaded. `raised()` must report the resolution that was lost, not the \
             number of questions asked"
        );
        assert_eq!(walk.rejected.len(), 14);
        assert_eq!(
            floor_for_budget(&[], 0, &led).base,
            0,
            "an empty ladder must not underflow"
        );
    }

    #[test]
    fn the_floor_never_rises_above_the_gpu_limit_choice() {
        let l = everon();
        let led = Ledger::with_budget(MB_1024);
        let walk = floor_for_budget(&l, 3, &led);
        assert_eq!(
            walk.base, 3,
            "the budget may only take resolution AWAY from the level the GPU limit chose; it \
             must never hand back a level `pick_base_level_for_limit` already rejected"
        );
        assert!(walk.rejected.is_empty());
    }

    #[test]
    fn the_hud_shows_reserved_against_budget_and_the_floor() {
        let mut led = Ledger::with_budget(MB_1024);
        assert_eq!(
            led.hud_suffix(),
            "",
            "nothing held and no floor chosen is a dead cell, not a readout"
        );
        assert_eq!(led.reserve(Asset::Dem, DEM_METERS), Decision::Ok);
        assert_eq!(led.hud_suffix(), " · mem 156/1024MB");
        led.set_satellite_floor(1, 1);
        assert_eq!(
            led.hud_suffix(),
            " · mem 156/1024MB · sat L1 (+1)",
            "the raise count is on the HUD because `sat L1` alone cannot be told apart from a \
             GPU that only ever offered L1"
        );
        led.set_satellite_floor(0, 0);
        assert_eq!(led.hud_suffix(), " · mem 156/1024MB · sat L0");
    }

    #[test]
    fn growth_is_tracked_beside_the_declared_bytes_and_never_inside_them() {
        let mut led = Ledger::with_budget(MB_1024);
        led.add_growth(Asset::World, 40 * MIB);
        led.add_growth(Asset::World, 8 * MIB);
        assert_eq!(
            led.entry(Asset::World).growth,
            48 * MIB,
            "growth accumulates across passes — the residency drain runs up to twelve times"
        );
        assert_eq!(
            led.held_total(),
            0,
            "measured growth must never enter the budget arithmetic: an asset with both figures \
             would be counted twice and refuse loaders that would have fitted"
        );
        assert_eq!(led.hud_suffix(), "");
    }

    #[test]
    fn the_default_budget_is_the_one_the_spec_names() {
        assert_eq!(DEFAULT_BUDGET_MB, 1536);
        assert_eq!(configured_budget_bytes(), 1536 * MIB);
        assert_eq!(
            heap_bytes(),
            0,
            "there is no wasm linear memory on the host"
        );
    }

    #[test]
    fn every_asset_indexes_its_own_row() {
        for (i, a) in Asset::ALL.iter().enumerate() {
            assert_eq!(a.index(), i);
        }
        let mut led = Ledger::with_budget(MB_1024);
        for a in Asset::ALL {
            assert_eq!(led.reserve(a, 1), Decision::Ok);
        }
        for a in Asset::ALL {
            assert_eq!(led.entry(a).held, 1, "{} must have its own row", a.name());
        }
        assert_eq!(led.held_total(), 7);
    }
}
