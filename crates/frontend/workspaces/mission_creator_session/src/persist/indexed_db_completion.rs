//! Awaits IndexedDB requests and transactions from their DOM events.
//!
//! **Role:** the two futures the record store awaits: a request's answer (its raw result on
//! `success`, its DOMException on `error`) and a transaction's commit (written on `complete`,
//! failed on `abort` or `error`).
//! **Position:** between `record_store` and the callback API of the `idb` crate, which the workspace
//! builds without its `futures` feature so that the browser build links no tokio; nothing else in
//! the crate awaits IndexedDB.
//! **Signals & state:** each await owns one `futures` oneshot channel whose sender the event
//! callbacks share; the request or transaction keeps the callbacks alive and outlives the await.
//! **Invariants:** the first event to fire settles the await and later events are ignored; a
//! transaction counts as written only on `complete`, so an aborted commit (a full quota aborts at
//! commit time) is a failure carrying the transaction's DOMException, never a success.

use std::cell::RefCell;
use std::rc::Rc;

use futures::channel::oneshot;
use idb::{Request, Transaction};
use wasm_bindgen::JsValue;
use web_sys::DomException;

/// How a request or transaction ended, as its first event reported it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ending {
    /// `success` on a request, `complete` on a transaction.
    Succeeded,
    /// `error` on a request, `abort` or `error` on a transaction.
    Failed,
}

/// The sender the competing event callbacks share; the first callback to fire takes it.
type SharedSender = Rc<RefCell<Option<oneshot::Sender<Ending>>>>;

/// Send `ending` if no other event has settled the channel yet.
fn settle(sender: &SharedSender, ending: Ending) {
    if let Some(sender) = sender.borrow_mut().take() {
        let _ = sender.send(ending);
    }
}

/// The failure an IndexedDB object reports: its DOMException, or the absence of one.
fn failure(error: Option<DomException>) -> idb::Error {
    error.map_or(idb::Error::DomExceptionNotFound, idb::Error::DomException)
}

/// Wait for `request` to fire `success` or `error` and return its raw result on success.
///
/// The request is borrowed, so it and the callbacks it holds live until this returns. A channel
/// cancelled without an event (unreachable while the request lives) reads as the request's error.
pub(super) async fn request_result<R: Request>(request: &mut R) -> Result<JsValue, idb::Error> {
    let (sender, receiver) = oneshot::channel();
    let on_success: SharedSender = Rc::new(RefCell::new(Some(sender)));
    let on_error = Rc::clone(&on_success);
    request.on_success(move |_| settle(&on_success, Ending::Succeeded));
    request.on_error(move |_| settle(&on_error, Ending::Failed));
    match receiver.await {
        Ok(Ending::Succeeded) => request.result(),
        Ok(Ending::Failed) | Err(oneshot::Canceled) => {
            Err(request.error().map_or_else(|e| e, failure))
        }
    }
}

/// Commit `transaction` and wait until it is written (`complete`) or fails (`abort`, `error`).
pub(super) async fn transaction_committed(transaction: Transaction) -> Result<(), idb::Error> {
    let mut transaction = transaction.commit()?;
    let (sender, receiver) = oneshot::channel();
    let on_complete: SharedSender = Rc::new(RefCell::new(Some(sender)));
    let on_abort = Rc::clone(&on_complete);
    let on_error = Rc::clone(&on_complete);
    transaction.on_complete(move |_| settle(&on_complete, Ending::Succeeded));
    transaction.on_abort(move |_| settle(&on_abort, Ending::Failed));
    transaction.on_error(move |_| settle(&on_error, Ending::Failed));
    match receiver.await {
        Ok(Ending::Succeeded) => Ok(()),
        Ok(Ending::Failed) | Err(oneshot::Canceled) => Err(failure(transaction.error())),
    }
}
