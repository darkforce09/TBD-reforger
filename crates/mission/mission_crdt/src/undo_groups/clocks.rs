//! The undo clocks, window and cap.
//!
//! **Role:** [`GESTURE_WINDOW_MS`] and [`MAX_UNDO_GROUPS`], the [`GroupingClock`] that turns a
//! host `time_source::Clock` into the `yrs` undo timestamp, [`undo_options`] and the cap math
//! [`hidden_prefix_after`].
//! **Position:** private to [`crate::undo_groups`], which re-exports every public item.
//! **Signals & state:** a [`GroupingClock`] owns an atomic depth and anchor, shared through `Arc`
//! by the document and its undo manager.
//! **Invariants:** every reading of a [`GroupingClock`] is at least 1 ms, because an anchor of 0
//! means no open group; while a group is open every reading is the anchor taken when it opened.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use time_source::{Clock, PlatformClock};
use yrs::Origin;
use yrs::sync::Timestamp;
use yrs::undo::Options as UndoOptions;

/// The capture window in milliseconds: tracked transactions closer together than this merge into
/// one undo step.
pub const GESTURE_WINDOW_MS: u64 = 300;

/// Maximum undo *groups* retained. The oldest whole group is dropped when a new one would exceed this; a group is never split.
pub const MAX_UNDO_GROUPS: usize = 200;

/// The wall clock of the compilation target (`Date.now()` in a browser, the system clock
/// natively): the inner clock a document uses when its host injects none.
#[must_use]
pub fn platform_clock() -> Arc<dyn Clock> {
    Arc::new(PlatformClock)
}

/// The undo options of a mission document: the [`GESTURE_WINDOW_MS`] capture window, the tracked
/// origins, and `clock` as the timestamp source.
#[must_use]
pub fn undo_options(
    clock: Arc<GroupingClock>,
    tracked_origins: HashSet<Origin>,
) -> UndoOptions<()> {
    UndoOptions {
        capture_timeout_millis: GESTURE_WINDOW_MS,
        tracked_origins,
        capture_transaction: None,
        timestamp: clock,
        init_undo_stack: Vec::new(),
        init_redo_stack: Vec::new(),
    }
}

/// Wraps a host clock as the undo timestamp, freezing `now()` while an explicit group is open so batches win over [`GESTURE_WINDOW_MS`].
pub struct GroupingClock {
    inner: Arc<dyn Clock>,
    depth: AtomicU32,
    anchor: AtomicU64,
}

impl GroupingClock {
    /// A grouping clock over `inner`, with no group open.
    #[must_use]
    pub fn wrap(inner: Arc<dyn Clock>) -> Arc<Self> {
        Arc::new(Self {
            inner,
            depth: AtomicU32::new(0),
            anchor: AtomicU64::new(0),
        })
    }

    /// Opens a group; the outermost call anchors every reading at the inner clock's time.
    pub fn begin_group(&self) {
        let prev = self.depth.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            self.anchor.store(self.floored_now(), Ordering::SeqCst);
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

    fn floored_now(&self) -> Timestamp {
        self.inner.now_unix_ms().max(1)
    }
}

impl yrs::sync::Clock for GroupingClock {
    fn now(&self) -> Timestamp {
        if self.depth.load(Ordering::SeqCst) > 0 {
            let a = self.anchor.load(Ordering::SeqCst);
            if a > 0 {
                return a;
            }
        }
        self.floored_now()
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
