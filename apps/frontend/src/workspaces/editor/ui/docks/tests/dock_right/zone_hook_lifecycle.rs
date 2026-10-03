use super::{install_select_zone, route_select_zone, ZoneSelectHook};
use leptos::prelude::Owner;
use std::{cell::RefCell, rc::Rc};

/// A hook plus the log of every id it is handed — so "did the click actually select something"
/// is answered by what the PANEL saw, not only by the router's boolean.
fn spy() -> (Rc<RefCell<Vec<String>>>, ZoneSelectHook) {
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let sink = Rc::clone(&log);
    let hook: ZoneSelectHook = Rc::new(move |id: &str| sink.borrow_mut().push(id.to_string()));
    (log, hook)
}

/// With nothing ever installed the route reports failure — the baseline the other two are
/// measured against, so a green there cannot be "it was already false".
#[test]
fn a_route_with_no_panel_ever_installed_reports_failure() {
    assert!(
        !route_select_zone("z-nobody"),
        "F2: no panel has ever registered, so the click selected nothing and must say so"
    );
}

/// Unmount unregisters: after the owner is cleaned up the route must return `false`, and the
/// dead hook must not run. Returning `true` here is the lie F2 exists to kill — the router uses
/// that boolean to decide whether the click did anything.
#[test]
fn unmount_unregisters_so_the_route_stops_reporting_success() {
    let (log, hook) = spy();
    let owner = Owner::new();
    owner.with(|| install_select_zone(hook));
    assert!(
        route_select_zone("z-mounted"),
        "F2 precondition: while mounted the panel really does receive the selection"
    );
    owner.cleanup();
    assert!(
        !route_select_zone("z-after-unmount"),
        "F2: the unmounted panel's signals are DISPOSED, so every `set` is a silent no-op — the \
         route must report FAILURE, not `true` over a click that selected nothing"
    );
    assert_eq!(
        log.borrow().as_slice(),
        ["z-mounted".to_string()],
        "F2: the stale hook must not be called at all after unmount"
    );
}

/// The `Rc::ptr_eq` guard: a remount installs its hook BEFORE the old component's cleanup runs
/// (leptos does not guarantee the other interleaving). The OLD cleanup must recognise that it is
/// no longer the live registration and leave the NEW panel's hook alone — otherwise the fix for
/// the stale hook becomes a fresh way to kill a live one.
#[test]
fn an_older_owners_cleanup_does_not_clobber_a_newer_registration() {
    let root = Owner::new();
    // Siblings, not parent/child: two successive `DockRight` instances under the page owner. A
    // child would be cleaned up BY the parent and prove nothing about the guard.
    let old = root.child();
    let new = root.child();
    let (log_old, hook_old) = spy();
    let (log_new, hook_new) = spy();
    old.with(|| install_select_zone(hook_old));
    new.with(|| install_select_zone(hook_new));

    old.cleanup();

    assert!(
        route_select_zone("z-remounted"),
        "F2: the NEW panel is live — the old component's cleanup must not unregister it"
    );
    assert_eq!(
        log_new.borrow().as_slice(),
        ["z-remounted".to_string()],
        "F2: the id must reach the NEW panel, so the surviving registration is the new hook and \
         not a leftover that merely happens to answer `true`"
    );
    assert!(
        log_old.borrow().is_empty(),
        "F2: the superseded hook must never run again"
    );

    new.cleanup();
    assert!(
        !route_select_zone("z-gone"),
        "F2: the live panel's OWN cleanup does clear it — the guard skips losers, not everyone"
    );
}
