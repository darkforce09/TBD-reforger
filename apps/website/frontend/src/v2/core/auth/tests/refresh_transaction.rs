use super::rotate_with_storage;
use futures::executor::block_on;
use std::cell::{Cell, RefCell};

#[test]
fn refresh_does_not_send_when_the_spent_credential_cannot_be_removed() {
    let requests = Cell::new(0);
    let writes = Cell::new(0);
    let result = block_on(rotate_with_storage(
        || false,
        async {
            requests.set(requests.get() + 1);
            Some("successor")
        },
        |_| {
            writes.set(writes.get() + 1);
            true
        },
    ));
    assert!(result.is_none());
    assert_eq!(requests.get(), 0);
    assert_eq!(writes.get(), 0);
}

#[test]
fn refresh_removes_the_old_credential_before_any_network_effect() {
    let storage = RefCell::new(Some("original"));
    let result = block_on(rotate_with_storage(
        || {
            storage.replace(None);
            true
        },
        async {
            assert!(
                storage.borrow().is_none(),
                "another page cannot replay the in-flight credential"
            );
            Some("successor")
        },
        |token| {
            storage.replace(Some(*token));
            true
        },
    ));
    assert_eq!(result, Some("successor"));
    assert_eq!(*storage.borrow(), Some("successor"));
}

#[test]
fn refresh_response_loss_never_restores_a_possibly_spent_credential() {
    let storage = RefCell::new(Some("original"));
    let server_spent = Cell::new(false);
    let result = block_on(rotate_with_storage(
        || {
            storage.replace(None);
            true
        },
        async {
            server_spent.set(true);
            None::<&str>
        },
        |_| panic!("lost response must not persist a successor"),
    ));
    assert!(server_spent.get());
    assert!(result.is_none());
    assert!(storage.borrow().is_none());
}

#[test]
fn failed_successor_storage_cannot_leave_the_spent_credential_for_another_tab() {
    let storage = RefCell::new(Some("original"));
    let result = block_on(rotate_with_storage(
        || {
            storage.replace(None);
            true
        },
        async { Some("successor") },
        |_| false,
    ));
    assert!(result.is_none());
    assert!(storage.borrow().is_none());
}

#[test]
fn changed_session_discards_the_late_successor_without_restoring_old_credentials() {
    let storage = RefCell::new(Some("original"));
    let result = block_on(rotate_with_storage(
        || {
            storage.replace(None);
            true
        },
        async {
            storage.replace(Some("new-login"));
            Some("old-session-successor")
        },
        |_| false,
    ));
    assert!(result.is_none());
    assert_eq!(*storage.borrow(), Some("new-login"));
}

#[test]
fn refresh_returns_success_only_after_successor_storage_finishes() {
    let recorded = RefCell::new(Vec::new());
    let result = block_on(rotate_with_storage(
        || {
            recorded.borrow_mut().push("consume");
            true
        },
        async {
            recorded.borrow_mut().push("effect");
            Some("successor")
        },
        |_| {
            recorded.borrow_mut().push("persist");
            true
        },
    ));
    if result.is_some() {
        recorded.borrow_mut().push("announce");
    }
    assert_eq!(
        *recorded.borrow(),
        ["consume", "effect", "persist", "announce"]
    );
}

#[test]
fn a_stalled_request_is_aborted_when_its_deadline_expires() {
    let aborted = Cell::new(false);
    let result = block_on(super::with_deadline(
        futures::future::pending::<Option<&str>>(),
        async {},
        || aborted.set(true),
    ));
    assert!(result.is_none());
    assert!(aborted.get());
}

#[test]
fn a_completed_request_cancels_the_deadline_without_aborting() {
    let aborted = Cell::new(false);
    let result = block_on(super::with_deadline(
        async { Some("successor") },
        futures::future::pending(),
        || aborted.set(true),
    ));
    assert_eq!(result, Some("successor"));
    assert!(!aborted.get());
}

#[test]
fn deadline_failure_leaves_the_consumed_credential_absent() {
    let storage = RefCell::new(Some("original"));
    let aborted = Cell::new(false);
    let request =
        super::with_deadline(futures::future::pending::<Option<&str>>(), async {}, || {
            aborted.set(true)
        });
    let result = block_on(rotate_with_storage(
        || {
            storage.replace(None);
            true
        },
        request,
        |_| panic!("deadline cannot persist a successor"),
    ));
    assert!(aborted.get());
    assert!(result.is_none());
    assert!(storage.borrow().is_none());
}
