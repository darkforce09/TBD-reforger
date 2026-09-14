//! One dismiss key, one overlay: the registry the dialogs and sheets share.
//!
//! **Role:** tracks every open overlay, decides which one is on top, and answers both the paint
//! order and the dismiss-key question with the same answer.
//! **Position:** underneath the dialog and sheet primitives; nothing renders here.
//! **Signals & state:** thread-local registries of overlays, transient closers, the open-order
//! clock, the installed key hooks and the polling handle. Openness is read live from each overlay's
//! own predicate rather than pushed on open.
//! **Invariants:** each overlay installs its own window-level key listener, and every listener sees
//! every key press, so "am I open?" alone is not a sufficient guard — a confirmation stacked over an
//! edit form would close both and lose the edits. Topmost means last-opened, which is also what
//! decides the paint order, so the surface a person can see is the surface the key closes.
//!
//! Nothing is pushed on open or popped on close. Openness is read at the moment the key arrives, so
//! an overlay that closes from the middle of the stack simply stops being a candidate, and one that
//! reopens becomes a candidate again as the new top. Removal is by id rather than by popping, so
//! components torn down in any order leave a consistent registry, and both the listener and the
//! registration are released in the same cleanup so neither can outlive its component.
//!
//! Transient surfaces — small popovers rather than modal overlays — are not stack entries. They
//! register a closer instead, and any closed-to-open edge fires them, so a surface opened from the
//! canvas cannot sit under a popover that is still visible. A capture-phase sentinel records when a
//! registered overlay owned a key press, so a transient yields even when a peer listener has already
//! closed that overlay within the same event and the live read would otherwise say nothing is open.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// One registered overlay.
///
/// Its open state is read live rather than pushed, and its stamp is the moment it was last observed
/// going from closed to open — zero while it reports closed. Registration order and open order are
/// different questions: both the paint order and the dismiss order follow open order, and registration
/// order is only how the list is laid out for stable lookup.
struct Entry {
    id: u64,
    is_open: Rc<dyn Fn() -> bool>,
    open_seq: Cell<u64>,
}

thread_local! {
    static REGISTRY: RefCell<Vec<Entry>> = const { RefCell::new(Vec::new()) };
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
    static OPEN_CLOCK: Cell<u64> = const { Cell::new(0) };
    static TRANSIENT_CLOSERS: RefCell<Vec<(u64, Rc<dyn Fn()>)>> =
        const { RefCell::new(Vec::new()) };
    static NEXT_CLOSER_ID: Cell<u64> = const { Cell::new(1) };
    static ESCAPE_CONSUMED: Cell<bool> = const { Cell::new(false) };
    static ESCAPE_TOP_ID: Cell<Option<u64>> = const { Cell::new(None) };
    #[cfg(target_arch = "wasm32")]
    static ESCAPE_HOOKS_ARMED: Cell<bool> = const { Cell::new(false) };
    #[cfg(target_arch = "wasm32")]
    static OPEN_EDGE_PUMP: RefCell<Option<OpenEdgePump>> = const { RefCell::new(None) };
}

/// Keeps the polling handle alive while any overlay is registered.
#[cfg(target_arch = "wasm32")]
struct OpenEdgePump {
    handle: i32,
    _closure: wasm_bindgen::closure::Closure<dyn FnMut()>,
}

/// Register an overlay and return the id that identifies it for the rest of its life.
pub(crate) fn register(is_open: impl Fn() -> bool + 'static) -> u64 {
    ensure_escape_hooks();
    let id = NEXT_ID.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    });
    REGISTRY.with_borrow_mut(|r| {
        r.push(Entry {
            id,
            is_open: Rc::new(is_open),
            open_seq: Cell::new(0),
        });
    });
    id
}

/// Drop an overlay's registration.
///
/// Removes by id, so unmount order does not matter, and does nothing at all for an id that is already
/// gone — a double cleanup must neither panic nor evict a stranger.
pub(crate) fn unregister(id: u64) {
    REGISTRY.with_borrow_mut(|r| r.retain(|e| e.id != id));
}

/// Register a closer for a transient surface, and return the id to drop it by.
#[allow(dead_code)]
pub(crate) fn register_transient_closer(close: impl Fn() + 'static) -> u64 {
    ensure_escape_hooks();
    let id = NEXT_CLOSER_ID.with(|n| {
        let id = n.get();
        n.set(id + 1);
        id
    });
    TRANSIENT_CLOSERS.with_borrow_mut(|c| c.push((id, Rc::new(close))));
    #[cfg(target_arch = "wasm32")]
    ensure_open_edge_pump();
    id
}

/// Drop a transient closer by id.
#[allow(dead_code)]
pub(crate) fn unregister_transient_closer(id: u64) {
    TRANSIENT_CLOSERS.with_borrow_mut(|c| c.retain(|(i, _)| *i != id));
    #[cfg(target_arch = "wasm32")]
    maybe_stop_open_edge_pump();
}

/// Fire every registered transient closer.
///
/// Safe to call re-entrantly from an open edge: the closers are cloned out before any of them runs.
#[allow(dead_code)]
pub(crate) fn close_registered_transients() {
    let closers: Vec<Rc<dyn Fn()>> =
        TRANSIENT_CLOSERS.with_borrow(|c| c.iter().map(|(_, f)| Rc::clone(f)).collect());
    for f in closers {
        f();
    }
}

/// Whether a registered overlay has already claimed the current key press.
#[allow(dead_code)]
pub(crate) fn escape_consumed() -> bool {
    ESCAPE_CONSUMED.get()
}

/// Mark the current key press as claimed by an overlay.
#[allow(dead_code)]
pub(crate) fn mark_escape_consumed() {
    ESCAPE_CONSUMED.set(true);
}

/// Poll for open edges and fire the transient closers they imply.
#[allow(dead_code)]
pub(crate) fn flush_open_edges() {
    let _ = reconcile_open_order();
}

/// Install the window-level key listeners once per browsing context.
fn ensure_escape_hooks() {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if ESCAPE_HOOKS_ARMED.get() {
            return;
        }
        let Some(win) = web_sys::window() else {
            return;
        };
        // Capture-phase: runs before every bubble window listener (Dialog / Sheet / strip).
        // Snapshot "an overlay owns this Esc" before any peer can close and make any_open lie.
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut(_)>::new(
            move |ev: web_sys::KeyboardEvent| {
                if ev.type_() == "keyup" {
                    ESCAPE_TOP_ID.set(None);
                    ESCAPE_CONSUMED.set(false);
                    return;
                }
                if ev.key() == "Escape" {
                    // Snapshot the open-order top BEFORE any bubble listener can close it.
                    let top = open_order_top_id();
                    ESCAPE_TOP_ID.set(top);
                    ESCAPE_CONSUMED.set(top.is_some());
                } else {
                    ESCAPE_TOP_ID.set(None);
                    ESCAPE_CONSUMED.set(false);
                }
            },
        );
        let _ = win.add_event_listener_with_callback_and_bool(
            "keydown",
            closure.as_ref().unchecked_ref(),
            true,
        );
        let _ = win.add_event_listener_with_callback_and_bool(
            "keyup",
            closure.as_ref().unchecked_ref(),
            true,
        );
        // Lives for the page — arm once per wasm session.
        closure.forget();
        ESCAPE_HOOKS_ARMED.set(true);
    }
}

/// Start the poll that watches for open edges, if it is not already running.
#[cfg(target_arch = "wasm32")]
fn ensure_open_edge_pump() {
    use wasm_bindgen::JsCast;
    OPEN_EDGE_PUMP.with(|slot| {
        if slot.borrow().is_some() {
            return;
        }
        let Some(win) = web_sys::window() else {
            return;
        };
        // Roughly one frame: the exclusivity must land with the overlay's own paint, not after a
        // visible flash of a popover still showing underneath it.
        let closure = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(|| {
            flush_open_edges();
        });
        let handle = win
            .set_interval_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                16,
            )
            .unwrap_or(0);
        *slot.borrow_mut() = Some(OpenEdgePump {
            handle,
            _closure: closure,
        });
    });
}

/// Stop the poll once nothing is registered any more.
#[cfg(target_arch = "wasm32")]
fn maybe_stop_open_edge_pump() {
    let empty = TRANSIENT_CLOSERS.with_borrow(|c| c.is_empty());
    if !empty {
        return;
    }
    let pump = OPEN_EDGE_PUMP.with(|slot| slot.borrow_mut().take());
    if let Some(p) = pump {
        if let Some(win) = web_sys::window() {
            win.clear_interval_with_handle(p.handle);
        }
    }
}

/// Whether `id` is the overlay opened most recently that is still open — the one painted on top, and
/// the one a dismiss key must close.
///
/// False when nothing is open, and false for an id that is not registered.
///
/// The predicates are cloned out from under the borrow before any of them runs: they read signals, and
/// a signal read is arbitrary code. Evaluating them while the registry is borrowed would turn a
/// re-entrant registration from inside one into a panic rather than a merely surprising ordering.
pub(crate) fn is_topmost_open(id: u64) -> bool {
    // During an Escape keydown the capture sentinel freezes the open-order top so every
    // bubble listener agrees on a single owner even after that owner closes itself.
    if let Some(top) = ESCAPE_TOP_ID.get() {
        return id == top;
    }
    is_top_by_open_order(id)
}

/// Whether any registered overlay currently reports itself open.
///
/// Consulted by dismiss-key handlers that are not overlays themselves, so that an open overlay owns the
/// key press. Overlays use [`is_topmost_open`] instead, and transient surfaces use
/// [`escape_consumed`], because a peer listener may already have closed the overlay within the same
/// event.
///
/// Same clone-out-before-call discipline as [`is_topmost_open`].
#[allow(dead_code)]
pub(crate) fn any_open() -> bool {
    let preds: Vec<Rc<dyn Fn() -> bool>> =
        REGISTRY.with_borrow(|r| r.iter().map(|e| Rc::clone(&e.is_open)).collect());
    preds.iter().any(|is_open| is_open())
}

/// How many overlays are registered — the leak check the tests use.
///
/// Deliberately not gated to test builds. The source scrubber treats the first test gate in a file as
/// the start of its test half and drops everything after it, so a test-gated helper up here would hide
/// everything below it from every source assertion in this file.
#[allow(dead_code)]
pub(crate) fn depth() -> usize {
    REGISTRY.with_borrow(Vec::len)
}

/// Observe every overlay's open and close edges, keep each stamp current, and return the id and stamp
/// of everything open right now.
///
/// A newly open overlay takes the next tick of the clock; one that has gone closed drops back to zero,
/// so a reopen counts as a fresh open rather than as the oldest one. Because the paint order is read on
/// essentially every render, an open edge is observed within a frame of happening, which is what lets
/// this poll-on-query scheme track real open order without a subscription plumbed through every call
/// site.
///
/// The one thing it cannot see is a close and a reopen collapsed into a single frame with no render
/// between them, since the poll only ever sees the final state. That is not a real interaction: every
/// open is a signal write that schedules a render, and the views that read paint order re-run on it.
///
/// Same re-entrancy discipline as [`is_topmost_open`]: the predicates are cloned out and called with
/// the registry unborrowed, and the stamps are written back afterwards through each entry's own cell.
///
/// Every closed-to-open edge also fires the transient closers, after the stamps are written and the
/// registry is unborrowed, so a surface opened from the canvas clears any open popover.
fn reconcile_open_order() -> Vec<(u64, u64)> {
    let snapshot: Vec<(u64, Rc<dyn Fn() -> bool>, u64)> = REGISTRY.with_borrow(|r| {
        r.iter()
            .map(|e| (e.id, Rc::clone(&e.is_open), e.open_seq.get()))
            .collect()
    });
    // Decide the new stamp for each id with the registry unborrowed.
    let mut updates: Vec<(u64, u64)> = Vec::with_capacity(snapshot.len());
    let mut open_now: Vec<(u64, u64)> = Vec::new();
    let mut saw_open_edge = false;
    for (id, is_open, prev) in snapshot {
        let new_seq = if is_open() {
            if prev == 0 {
                saw_open_edge = true;
                OPEN_CLOCK.with(|c| {
                    let next = c.get() + 1;
                    c.set(next);
                    next
                })
            } else {
                prev // already open — keep the stamp it opened at
            }
        } else {
            0 // closed — clear so the next open is a fresh edge
        };
        updates.push((id, new_seq));
        if new_seq != 0 {
            open_now.push((id, new_seq));
        }
    }
    // Write the stamps back. Removals between snapshot and here are fine: we match by id.
    REGISTRY.with_borrow(|r| {
        for (id, seq) in &updates {
            if let Some(e) = r.iter().find(|e| e.id == *id) {
                e.open_seq.set(*seq);
            }
        }
    });
    if saw_open_edge {
        close_registered_transients();
    }
    open_now
}

/// The id of the overlay opened most recently that is still open.
fn open_order_top_id() -> Option<u64> {
    reconcile_open_order()
        .into_iter()
        .max_by_key(|(_, seq)| *seq)
        .map(|(top, _)| top)
}

/// Whether `id` is that overlay.
pub(crate) fn is_top_by_open_order(id: u64) -> bool {
    open_order_top_id().is_some_and(|top| top == id)
}

/// The stacking class an overlay should carry so that the last-opened surface wins the paint order.
///
/// The top of the open order sits at the modal tier; anything open underneath it drops one tier, which
/// is still above page content. Two tiers rather than a rank per overlay because one sibling surface
/// still carries a hard-coded modal tier of its own, so the surfaces that do consume this have to be
/// able to go below it and let that one win when it is the surface on top.
pub(crate) fn z_class(id: u64) -> &'static str {
    if is_top_by_open_order(id) {
        "z-50"
    } else {
        "z-40"
    }
}
