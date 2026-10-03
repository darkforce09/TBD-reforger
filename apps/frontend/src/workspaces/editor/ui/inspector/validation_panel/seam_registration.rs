//! Validation panel seam registration.

use super::*;

/// Compares mounted callback registrations by identity.
pub(crate) trait SeamRegistration: Clone + 'static {
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

/// Thread-local storage for an optional mounted callback.
pub(crate) type SeamCell<H> = std::thread::LocalKey<std::cell::RefCell<Option<H>>>;

/// Registers a callback and clears it when its owner unmounts.
pub(crate) fn install_seam<H: SeamRegistration>(cell: &'static SeamCell<H>, hook: H) {
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
