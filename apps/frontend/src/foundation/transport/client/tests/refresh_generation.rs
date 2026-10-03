//! Session-generation boundaries in the actual refresh-and-retry state machine.

use super::{send_with_refresh_for_generation, ApiErr, RefreshResponse, SingleFlight};
use futures::channel::oneshot;
use futures::future::{FutureExt, LocalBoxFuture};
use futures::task::noop_waker_ref;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

#[derive(Default)]
struct Observed {
    sends: Vec<Option<String>>,
    refreshes: usize,
    adopted: Vec<String>,
}

fn pair(access: &str) -> RefreshResponse {
    RefreshResponse {
        access_token: access.into(),
        refresh_token: format!("refresh-{access}"),
        expires_at: "2026-09-23T00:00:00Z".into(),
    }
}

fn ready_refresh(access: &'static str) -> LocalBoxFuture<'static, Option<RefreshResponse>> {
    async move { Some(pair(access)) }.boxed_local()
}

fn blocked_refresh() -> (
    oneshot::Sender<Option<RefreshResponse>>,
    LocalBoxFuture<'static, Option<RefreshResponse>>,
) {
    let (release, ready) = oneshot::channel();
    let refresh = async move { ready.await.expect("release refresh response") }.boxed_local();
    (release, refresh)
}

fn request<'a>(
    single_flight: &'a SingleFlight<Option<RefreshResponse>>,
    generation: u64,
    current_generation: Rc<Cell<u64>>,
    observed: Rc<RefCell<Observed>>,
    refresh: LocalBoxFuture<'static, Option<RefreshResponse>>,
) -> LocalBoxFuture<'a, Result<String, ApiErr>> {
    let sends = observed.clone();
    let refreshes = observed.clone();
    send_with_refresh_for_generation(
        single_flight,
        generation,
        move || current_generation.get() == generation,
        move |token| {
            sends.borrow_mut().sends.push(token.clone());
            async move {
                match token {
                    Some(token) if token != "expired" => Ok(token),
                    _ => Err((401, None)),
                }
            }
            .boxed_local()
        },
        || Some("expired".into()),
        move || {
            refreshes.borrow_mut().refreshes += 1;
            refresh
        },
        move |rotated| {
            observed
                .borrow_mut()
                .adopted
                .push(rotated.access_token.clone())
        },
    )
    .boxed_local()
}

fn poll_once<F: Future + ?Sized>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(noop_waker_ref()))
}

#[test]
fn stale_generation_at_first_401_cannot_join_or_replace_current_flight() {
    let single_flight = SingleFlight::new();
    let current = Rc::new(Cell::new(2));
    let live_observed = Rc::new(RefCell::new(Observed::default()));
    let stale_observed = Rc::new(RefCell::new(Observed::default()));
    let follower_observed = Rc::new(RefCell::new(Observed::default()));
    let (release, refresh) = blocked_refresh();
    let mut live = request(
        &single_flight,
        2,
        current.clone(),
        live_observed.clone(),
        refresh,
    );
    assert!(poll_once(live.as_mut()).is_pending());

    let mut stale = request(
        &single_flight,
        1,
        current.clone(),
        stale_observed.clone(),
        ready_refresh("stale"),
    );
    assert_eq!(poll_once(stale.as_mut()), Poll::Ready(Err((401, None))));
    assert_eq!(stale_observed.borrow().refreshes, 0);
    assert!(stale_observed.borrow().adopted.is_empty());
    assert_eq!(stale_observed.borrow().sends.len(), 1);

    let mut follower = request(
        &single_flight,
        2,
        current,
        follower_observed.clone(),
        ready_refresh("replacement"),
    );
    assert!(poll_once(follower.as_mut()).is_pending());
    assert_eq!(follower_observed.borrow().refreshes, 0);
    release
        .send(Some(pair("current")))
        .unwrap_or_else(|_| panic!("current refresh is alive"));
    assert_eq!(poll_once(live.as_mut()), Poll::Ready(Ok("current".into())));
    assert_eq!(
        poll_once(follower.as_mut()),
        Poll::Ready(Ok("current".into()))
    );
    assert_eq!(live_observed.borrow().refreshes, 1);
}

#[test]
fn generation_change_while_refresh_waits_prevents_adoption_and_retry() {
    let single_flight = SingleFlight::new();
    let current = Rc::new(Cell::new(1));
    let observed = Rc::new(RefCell::new(Observed::default()));
    let (release, refresh) = blocked_refresh();
    let mut pending = request(
        &single_flight,
        1,
        current.clone(),
        observed.clone(),
        refresh,
    );
    assert!(poll_once(pending.as_mut()).is_pending());

    current.set(2);
    release
        .send(Some(pair("old-account")))
        .unwrap_or_else(|_| panic!("old refresh is alive"));
    assert_eq!(poll_once(pending.as_mut()), Poll::Ready(Err((401, None))));
    let observed = observed.borrow();
    assert_eq!(observed.refreshes, 1);
    assert_eq!(observed.sends, vec![Some("expired".into())]);
    assert!(observed.adopted.is_empty());
}

#[test]
fn new_generation_does_not_wait_for_cancelled_older_initiator() {
    let single_flight = SingleFlight::new();
    let current = Rc::new(Cell::new(1));
    let follower_observed = Rc::new(RefCell::new(Observed::default()));
    let new_observed = Rc::new(RefCell::new(Observed::default()));
    let (release, refresh) = blocked_refresh();
    let mut initiator = request(
        &single_flight,
        1,
        current.clone(),
        Rc::new(RefCell::new(Observed::default())),
        refresh,
    );
    let mut follower = request(
        &single_flight,
        1,
        current.clone(),
        follower_observed.clone(),
        ready_refresh("unused"),
    );
    assert!(poll_once(initiator.as_mut()).is_pending());
    assert!(poll_once(follower.as_mut()).is_pending());
    drop(initiator);
    current.set(2);

    let mut new = request(
        &single_flight,
        2,
        current,
        new_observed.clone(),
        ready_refresh("new-account"),
    );
    assert_eq!(
        poll_once(new.as_mut()),
        Poll::Ready(Ok("new-account".into()))
    );
    assert_eq!(new_observed.borrow().refreshes, 1);
    assert_eq!(new_observed.borrow().adopted, vec!["new-account"]);
    release
        .send(Some(pair("old-account")))
        .unwrap_or_else(|_| panic!("follower retains old refresh"));
    assert_eq!(poll_once(follower.as_mut()), Poll::Ready(Err((401, None))));
    assert!(follower_observed.borrow().adopted.is_empty());
    assert_eq!(follower_observed.borrow().sends.len(), 1);
}

#[test]
fn cancelled_initiator_completed_result_cannot_supply_new_generation_tokens() {
    let single_flight = SingleFlight::new();
    let current = Rc::new(Cell::new(1));
    let (release, refresh) = blocked_refresh();
    let mut initiator = request(
        &single_flight,
        1,
        current.clone(),
        Rc::new(RefCell::new(Observed::default())),
        refresh,
    );
    let mut follower = request(
        &single_flight,
        1,
        current.clone(),
        Rc::new(RefCell::new(Observed::default())),
        ready_refresh("unused"),
    );
    assert!(poll_once(initiator.as_mut()).is_pending());
    assert!(poll_once(follower.as_mut()).is_pending());
    drop(initiator);
    release
        .send(Some(pair("old-account")))
        .unwrap_or_else(|_| panic!("follower retains old refresh"));
    assert_eq!(
        poll_once(follower.as_mut()),
        Poll::Ready(Ok("old-account".into()))
    );
    current.set(2);

    let observed = Rc::new(RefCell::new(Observed::default()));
    let mut new = request(
        &single_flight,
        2,
        current,
        observed.clone(),
        ready_refresh("new-account"),
    );
    assert_eq!(
        poll_once(new.as_mut()),
        Poll::Ready(Ok("new-account".into()))
    );
    assert_eq!(observed.borrow().refreshes, 1);
    assert_eq!(observed.borrow().adopted, vec!["new-account"]);
}

#[test]
fn current_generation_refreshes_adopts_and_retries_once() {
    let single_flight = SingleFlight::new();
    let observed = Rc::new(RefCell::new(Observed::default()));
    let mut pending = request(
        &single_flight,
        7,
        Rc::new(Cell::new(7)),
        observed.clone(),
        ready_refresh("current"),
    );
    assert_eq!(
        poll_once(pending.as_mut()),
        Poll::Ready(Ok("current".into()))
    );
    let observed = observed.borrow();
    assert_eq!(observed.refreshes, 1);
    assert_eq!(observed.adopted, vec!["current"]);
    assert_eq!(
        observed.sends,
        vec![Some("expired".into()), Some("current".into())]
    );
}
