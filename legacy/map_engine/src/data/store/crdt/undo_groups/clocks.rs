//! Role: clocks.
//! Position: `doc/crdt/undo_groups` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Arc;
use super::AtomicU32;
use super::AtomicU64;
use super::Clock;
use super::HashSet;
use super::OnceLock;
use super::Ordering;
use super::Origin;
use super::Timestamp;
use super::UndoOptions;

/// Canonical gesture window ms value.
pub const GESTURE_WINDOW_MS: u64 = 300;

/// Maximum undo *groups* retained. The oldest whole group is dropped when a new one would exceed this; a group is never split.
pub const MAX_UNDO_GROUPS: usize = 200;

/// Canonical wasm now value.
pub(super) static WASM_NOW: OnceLock<fn() -> Timestamp> = OnceLock::new();

/// Install the wasm time source (`js_sys::Date::now`). Called from the SPA `#[wasm_bindgen(start)]` in `operations/batch.rs` so `MissionDocCore::new` can keep this crate free of `wasm-bindgen`.
pub fn install_wasm_now(f: fn() -> Timestamp) {
    let _ = WASM_NOW.set(f);
}

/// Inner clock used when the host does not inject one.
#[must_use]
pub fn default_inner_clock() -> Arc<dyn Clock> {
    #[cfg(test)]
    {
        Arc::new(AutoAdvanceClock::new(GESTURE_WINDOW_MS.saturating_add(1)))
    }
    #[cfg(not(test))]
    {
        Arc::new(RealClock)
    }
}

/// Build `UndoOptions` for [`super::store::MissionDocCore`]: window + injectable timestamp.
#[must_use]
pub fn undo_options(clock: Arc<dyn Clock>, tracked_origins: HashSet<Origin>) -> UndoOptions<()> {
    UndoOptions {
        capture_timeout_millis: GESTURE_WINDOW_MS,
        tracked_origins,
        capture_transaction: None,
        timestamp: clock,
        init_undo_stack: Vec::new(),
        init_redo_stack: Vec::new(),
    }
}

/// Domain representation of real clock.
#[allow(dead_code)]
pub(super) struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> Timestamp {
        real_now()
    }
}

/// Real now using the supplied domain data.
#[allow(dead_code)]
pub(super) fn real_now() -> Timestamp {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        if let Some(f) = WASM_NOW.get() {
            return f();
        }

        1
    }
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as Timestamp)
            .unwrap_or(1)
            .max(1)
    }
}

/// Domain representation of auto advance clock.
#[cfg(test)]
pub(super) struct AutoAdvanceClock {
    /// T.
    pub(super) t: AtomicU64,

    /// Step.
    pub(super) step: u64,
}

#[cfg(test)]
impl AutoAdvanceClock {
    /// New using the supplied domain data.
    pub(super) fn new(step: u64) -> Self {
        Self {
            t: AtomicU64::new(0),
            step: step.max(1),
        }
    }
}

#[cfg(test)]
impl Clock for AutoAdvanceClock {
    fn now(&self) -> Timestamp {
        let step = self.step;
        self.t.fetch_add(step, Ordering::Relaxed) + step
    }
}

/// Test clock: the caller sets the millisecond time.
pub struct ManualClock {
    /// Now.
    pub(super) now: AtomicU64,
}

impl ManualClock {
    /// New using the supplied domain data.
    #[must_use]
    pub fn new(ms: Timestamp) -> Arc<Self> {
        Arc::new(Self {
            now: AtomicU64::new(ms.max(1)),
        })
    }

    /// Set using the supplied domain data.
    pub fn set(&self, ms: Timestamp) {
        self.now.store(ms.max(1), Ordering::Relaxed);
    }

    /// Advance using the supplied domain data.
    pub fn advance(&self, dt: Timestamp) {
        self.now.fetch_add(dt, Ordering::Relaxed);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        self.now.load(Ordering::Relaxed)
    }
}

/// Wraps an inner clock, freezing `now()` while an explicit group is open so batches win over [`GESTURE_WINDOW_MS`].
pub struct GroupingClock {
    /// Inner.
    pub(super) inner: Arc<dyn Clock>,

    /// Depth.
    pub(super) depth: AtomicU32,

    /// Anchor.
    pub(super) anchor: AtomicU64,
}

impl GroupingClock {
    /// Wrap using the supplied domain data.
    #[must_use]
    pub fn wrap(inner: Arc<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            depth: AtomicU32::new(0),
            anchor: AtomicU64::new(0),
        })
    }

    /// Begin group using the supplied domain data.
    pub fn begin_group(&self) {
        let prev = self.depth.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            let t = self.inner.now().max(1);
            self.anchor.store(t, Ordering::SeqCst);
        }
    }

    /// Returns `true` when the outermost group closed (caller should `UndoManager::reset`).
    pub fn end_group(&self) -> bool {
        loop {
            let d = self.depth.load(Ordering::SeqCst);
            if d == 0 {
                return false;
            }
            if self
                .depth
                .compare_exchange(d, d - 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                if d == 1 {
                    self.anchor.store(0, Ordering::SeqCst);
                    return true;
                }
                return false;
            }
        }
    }
}

impl Clock for GroupingClock {
    fn now(&self) -> Timestamp {
        if self.depth.load(Ordering::SeqCst) > 0 {
            let a = self.anchor.load(Ordering::SeqCst);
            if a > 0 {
                return a;
            }
        }
        self.inner.now().max(1)
    }
}

/// How many prefix stack items are forgotten by the depth cap (still physically on the yrs stack, but not undoable). Sticky across undos; grows when a new group would exceed [`MAX_UNDO_GROUPS`].
#[must_use]
pub fn hidden_prefix_after(stack_len: usize, hidden: usize) -> usize {
    let visible = stack_len.saturating_sub(hidden);
    if visible > MAX_UNDO_GROUPS {
        stack_len - MAX_UNDO_GROUPS
    } else {
        hidden.min(stack_len)
    }
}
