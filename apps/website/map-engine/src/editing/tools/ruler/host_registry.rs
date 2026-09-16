//! Role: the host-registered chain handle the drawer reads.
//! Position: `editing/tools/ruler` in the map engine.
//! Signals & state: session-local measurement state; never the authored document.
//! Invariants: a cell that holds nothing reports the empty chain rather than a dead surface's polyline. Installing into it is the host's job, because only the host knows when its surface dies.

use std::cell::RefCell;
use std::rc::Rc;

use super::chain::RulerChain;

thread_local! {
    /// The live ruler chain. `pub` because installing into it belongs to the host: the
    /// identity-guarded unregister that makes a remount safe is a lifecycle question, not a
    /// geometry one.
    pub static RULER_CHAIN: RefCell<Option<Rc<RefCell<RulerChain>>>> = const { RefCell::new(None) };
}

/// A snapshot clone of the registered chain — empty when no host has registered one.
#[must_use]
pub fn read_registered_chain() -> RulerChain {
    RULER_CHAIN.with(|c| {
        c.borrow()
            .as_ref()
            .map(|rc| rc.borrow().clone())
            .unwrap_or_default()
    })
}
