//! The registration every mounted hook uses: install now, clear at the owner's unmount.
//!
//! **Role:** writes a hook into its thread-local cell and arranges for the owner that installed it
//! to clear it when that owner is cleaned up, comparing registrations by identity.
//! **Position:** part of the editor's state layer; the validation panel's seams, the input tools'
//! seams and the bridge's world-assets host install their hooks through it.
//! **Signals & state:** none of its own; it writes the caller's cell and keeps a stored copy of
//! the hook for the cleanup.
//! **Invariants:** an owner's cleanup clears only the registration that owner installed, so a
//! remount's newer hook survives the old owner's unmount.

use leptos::prelude::*;

/// Compares mounted callback registrations by identity.
pub trait SeamRegistration: Clone + 'static {
    /// True when `live` is this very registration, compared by identity.
    fn is_same_registration(&self, live: &Self) -> bool;
}

impl<T: ?Sized + 'static> SeamRegistration for std::rc::Rc<T> {
    fn is_same_registration(&self, live: &Self) -> bool {
        std::rc::Rc::ptr_eq(self, live)
    }
}

impl<T: 'static, S: 'static> SeamRegistration for RwSignal<T, S> {
    fn is_same_registration(&self, live: &Self) -> bool {
        self == live
    }
}

/// A pair registered together is the same registration only when both halves are.
impl<A: SeamRegistration, B: SeamRegistration> SeamRegistration for (A, B) {
    fn is_same_registration(&self, live: &Self) -> bool {
        self.0.is_same_registration(&live.0) && self.1.is_same_registration(&live.1)
    }
}

/// Thread-local storage for an optional mounted callback.
pub type SeamCell<H> = std::thread::LocalKey<std::cell::RefCell<Option<H>>>;

/// Registers a callback and clears it when its owner unmounts.
pub fn install_seam<H: SeamRegistration>(cell: &'static SeamCell<H>, hook: H) {
    let mine = StoredValue::new_local(hook.clone());
    cell.with(|c| *c.borrow_mut() = Some(hook));
    on_cleanup(move || {
        let _ = mine.try_with_value(|mine| unregister_seam(cell, mine));
    });
}

/// Clears only the callback installed by the matching owner.
pub(crate) fn unregister_seam<H: SeamRegistration>(cell: &'static SeamCell<H>, mine: &H) -> bool {
    let taken = cell.with(|c| {
        let mut slot = c.borrow_mut();
        if slot
            .as_ref()
            .is_some_and(|live| mine.is_same_registration(live))
        {
            slot.take()
        } else {
            None
        }
    });
    taken.is_some()
}
