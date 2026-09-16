//! Role: progress.
//! Position: `streaming/bridge` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Concurrent HTTP Range requests in flight against the map-asset host.
pub const SAT_FETCH_CONCURRENCY: usize = 4;

/// Canonical sat chunk bytes value.
pub const SAT_CHUNK_BYTES: u64 = 4 * 1024 * 1024;

/// Split one tile's byte extent into consecutive `fetch_range` arguments (**inclusive** ends, which is what the `Range: bytes=a-b` header wants).
#[must_use]
pub fn split_range(offset: u64, length: u64, chunk: u64) -> Vec<(u64, u64)> {
    if length == 0 {
        return Vec::new();
    }
    let step = chunk.max(1);
    let end = offset + length;
    let mut out = Vec::new();
    let mut at = offset;
    while at < end {
        let stop = at.saturating_add(step).min(end);
        out.push((at, stop - 1));
        at = stop;
    }
    out
}

/// Index-addressed collector for completions that arrive out of order.
pub struct Ordered<T> {
    slots: Vec<Option<T>>,
}

impl<T> Ordered<T> {
    /// New.
    #[must_use]
    pub fn new(n: usize) -> Self {
        Self {
            slots: (0..n).map(|_| None).collect(),
        }
    }

    /// Place `v` at `i`. `false` when `i` is out of range — the caller must treat that as a failed fetch, not skip it, or the run silently loses a chunk.
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

/// `fetch`'s `ReadableStream` hands back whatever came off the socket — typically 16–64 KB — so reporting every chunk would push ~1,100 signal writes through the overlay for the DEM alone. At 512 KB the terrain segment still reports ~140 times (0.7% of itself a step), which is finer than the bar can render, and the boot does ~200 signal writes instead of ~2,000.
pub const STREAM_REPORT_BYTES: u64 = 512 * 1024;

/// Boot seg.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BootSeg {
    /// IDB replay + `GET /api/v1/missions/:id` — byte-metered off that response's `content-length` (the editor's own document can be anything from 700 B to ~142 MB, so it is emphatically not a rounding error on a big mission).
    Mission,

    /// `dem/everon-dem-16bit.png` — byte-metered off its `content-length`.
    Terrain,

    /// Satellite.
    Satellite,

    /// World.
    World,
}

/// Boot event.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BootEvent {
    /// This segment's real byte budget, read from a `content-length` header or summed from the tbd-sat index. Replaces both the segment's unit budget and its pacing weight, so the bar's speed tracks the transfer that is actually going to happen (a 16384-limit GPU fetches 152.7 MB of satellite where an 8192-limit one fetches 42.2 MB).
    Budget(BootSeg, u64),

    /// Files.
    Files(BootSeg, u64),

    /// `n` more units — bytes off the socket, or completed fetches — that have **already** landed.
    Done(BootSeg, u64),

    /// Finish.
    Finish(BootSeg),
}

/// How the reporter reaches the loaders. `Rc` because the satellite fetch holds it inside a future that outlives `bootstrap`'s stack frame.
pub type ProgressFn = std::rc::Rc<dyn Fn(BootEvent)>;
