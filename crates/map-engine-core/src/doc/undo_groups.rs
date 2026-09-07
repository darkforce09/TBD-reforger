//! T-937.2 — undo grouping: gesture window, explicit begin/end groups, depth cap.
//!
//! T-159.22.1 pinned `capture_timeout_millis = 0` + `ZeroClock` so every LOCAL yrs transaction
//! was its own Ctrl+Z (Yjs `captureTimeout: 0` parity). A drag that commits once per pointer
//! tick therefore needed one undo per tick, and the stack was unbounded.
//!
//! This module owns the replacement:
//! - **Window** [`GESTURE_WINDOW_MS`] (300): consecutive LOCAL txns inside the window merge.
//! - **Clock** is real on native and wasm (wasm installs `Date.now` from the SPA), injectable
//!   in tests.
//! - **`begin_group` / `end_group`** freeze the clock so an explicit batch wins over the window.
//! - **Cap** [`MAX_UNDO_GROUPS`] (200): oldest whole group is forgotten, not split.

use std::collections::HashSet;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

use yrs::Origin;
use yrs::sync::{Clock, Timestamp};
use yrs::undo::Options as UndoOptions;

/// Gesture capture window. yrs extends the last stack item when
/// `last_change > 0 && now - last_change < capture_timeout_millis`. Zero disables merging
/// (the T-937.2 perturbation).
pub const GESTURE_WINDOW_MS: u64 = 300;

/// Maximum undo *groups* retained. The oldest whole group is dropped when a new one would
/// exceed this; a group is never split.
pub const MAX_UNDO_GROUPS: usize = 200;

static WASM_NOW: OnceLock<fn() -> Timestamp> = OnceLock::new();

/// Install the wasm time source (`js_sys::Date::now`). Called from the SPA `#[wasm_bindgen(start)]`
/// in `operations/batch.rs` so `MissionDocCore::new` can keep this crate free of `wasm-bindgen`.
pub fn install_wasm_now(f: fn() -> Timestamp) {
    let _ = WASM_NOW.set(f);
}

/// Inner clock used when the host does not inject one.
#[must_use]
pub fn default_inner_clock() -> Arc<dyn Clock> {
    #[cfg(test)]
    {
        // Existing store tests assume one LOCAL txn = one stack item. Auto-advance past the
        // window so those pins stay green; grouping tests inject [`ManualClock`].
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

/// Wall clock. Native: `SystemTime`. Wasm: host `Date.now` via [`install_wasm_now`].
#[allow(dead_code)]
struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> Timestamp {
        real_now()
    }
}

#[allow(dead_code)]
fn real_now() -> Timestamp {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        if let Some(f) = WASM_NOW.get() {
            return f();
        }
        // Start() has not run yet; keep last_change > 0 so a later real clock can still merge.
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

/// Each `now()` jumps `step` ms so sequential test txns fall outside [`GESTURE_WINDOW_MS`].
#[cfg(test)]
struct AutoAdvanceClock {
    t: AtomicU64,
    step: u64,
}

#[cfg(test)]
impl AutoAdvanceClock {
    fn new(step: u64) -> Self {
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
    now: AtomicU64,
}

impl ManualClock {
    #[must_use]
    pub fn new(ms: Timestamp) -> Arc<Self> {
        Arc::new(Self {
            now: AtomicU64::new(ms.max(1)),
        })
    }

    pub fn set(&self, ms: Timestamp) {
        self.now.store(ms.max(1), Ordering::Relaxed);
    }

    pub fn advance(&self, dt: Timestamp) {
        self.now.fetch_add(dt, Ordering::Relaxed);
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        self.now.load(Ordering::Relaxed)
    }
}

/// Wraps an inner clock, freezing `now()` while an explicit group is open so batches win
/// over [`GESTURE_WINDOW_MS`].
pub struct GroupingClock {
    inner: Arc<dyn Clock>,
    depth: AtomicU32,
    anchor: AtomicU64,
}

impl GroupingClock {
    #[must_use]
    pub fn wrap(inner: Arc<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            depth: AtomicU32::new(0),
            anchor: AtomicU64::new(0),
        })
    }

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

/// How many prefix stack items are forgotten by the depth cap (still physically on the yrs
/// stack, but not undoable). Sticky across undos; grows when a new group would exceed
/// [`MAX_UNDO_GROUPS`].
#[must_use]
pub fn hidden_prefix_after(stack_len: usize, hidden: usize) -> usize {
    let visible = stack_len.saturating_sub(hidden);
    if visible > MAX_UNDO_GROUPS {
        stack_len - MAX_UNDO_GROUPS
    } else {
        hidden.min(stack_len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::doc::MissionDocCore;

    fn slot_doc(clock: Arc<dyn Clock>) -> MissionDocCore {
        let doc = MissionDocCore::with_undo_clock(clock);
        doc.set_origin_init(true);
        doc.add_slot(
            "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        doc.set_origin_init(false);
        doc
    }

    fn x_of(doc: &MissionDocCore) -> f32 {
        let soa = doc.materialize();
        let i = soa.ids.iter().position(|id| id == "s0").expect("s0");
        soa.xs[i]
    }

    /// T-937.2 — three position ops inside 50 ms are one undo group.
    #[test]
    fn three_position_ops_within_50ms_are_one_undo_group() {
        let clock = ManualClock::new(1_000);
        let mut doc = slot_doc(clock.clone());
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        clock.advance(25);
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        clock.advance(25);
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        assert_eq!(
            doc.undo_depth(),
            1,
            "T-937.2: three position ops within 50ms are ONE undo group; got {}",
            doc.undo_depth()
        );
        assert!(doc.undo());
        assert!((x_of(&doc) - 100.0).abs() < f32::EPSILON);
        assert!(!doc.can_undo());
    }

    #[test]
    fn edits_separated_by_more_than_the_window_are_two_groups() {
        let clock = ManualClock::new(1_000);
        let doc = slot_doc(clock.clone());
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        clock.advance(GESTURE_WINDOW_MS);
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        assert_eq!(
            doc.undo_depth(),
            2,
            "two edits ≥ {GESTURE_WINDOW_MS} ms apart must not merge"
        );
    }

    #[test]
    fn explicit_group_wins_over_the_window() {
        let clock = ManualClock::new(1_000);
        let mut doc = slot_doc(clock.clone());
        doc.begin_group();
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        clock.advance(GESTURE_WINDOW_MS + 50);
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        doc.end_group();
        assert_eq!(
            doc.undo_depth(),
            1,
            "begin_group/end_group must merge even across the window"
        );
        assert!(doc.undo());
        assert!((x_of(&doc) - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn end_group_splits_from_the_next_gesture() {
        let clock = ManualClock::new(1_000);
        let mut doc = slot_doc(clock.clone());
        doc.begin_group();
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        doc.end_group();
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        assert_eq!(
            doc.undo_depth(),
            2,
            "the op after end_group must be a new stack item"
        );
    }

    #[test]
    fn depth_cap_drops_oldest_of_201_groups() {
        let mut doc = MissionDocCore::new();
        doc.set_origin_init(true);
        doc.add_slot(
            "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
        );
        doc.set_origin_init(false);
        for _ in 0..201 {
            doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
        }
        assert_eq!(
            doc.undo_depth(),
            MAX_UNDO_GROUPS,
            "201 groups must leave {MAX_UNDO_GROUPS} undoable"
        );
        for _ in 0..MAX_UNDO_GROUPS {
            assert!(doc.undo());
        }
        assert!(!doc.can_undo(), "earliest group is gone — not a 201st undo");
        assert!(
            (x_of(&doc) - 110.0).abs() < f32::EPSILON,
            "first move remains applied after 200 undos; got {}",
            x_of(&doc)
        );
    }

    #[test]
    fn hidden_prefix_math_drops_oldest_whole_group() {
        assert_eq!(hidden_prefix_after(200, 0), 0);
        assert_eq!(hidden_prefix_after(201, 0), 1);
        assert_eq!(hidden_prefix_after(205, 1), 5);
        assert_eq!(hidden_prefix_after(199, 5), 5);
    }
}
