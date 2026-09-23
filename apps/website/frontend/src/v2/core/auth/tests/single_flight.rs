//! Deterministic waiter cancellation and session-generation interleavings.

use super::SingleFlight;
use futures::channel::oneshot;
use futures::executor::block_on;
use futures::task::noop_waker_ref;
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(noop_waker_ref()))
}

#[test]
fn same_generation_waiters_share_one_operation() {
    let single_flight = SingleFlight::new();
    let starts = Rc::new(Cell::new(0));
    let first_starts = starts.clone();
    let second_starts = starts.clone();
    let (release, ready) = oneshot::channel();
    let mut first = Box::pin(single_flight.run_keyed(7, move || {
        first_starts.set(first_starts.get() + 1);
        async move { ready.await.expect("release shared operation") }
    }));
    let mut second = Box::pin(single_flight.run_keyed(7, move || {
        second_starts.set(second_starts.get() + 1);
        async { 99 }
    }));

    assert!(poll_once(first.as_mut()).is_pending());
    assert!(poll_once(second.as_mut()).is_pending());
    assert_eq!(starts.get(), 1);
    release.send(42).expect("shared operation is alive");
    assert_eq!(poll_once(second.as_mut()), Poll::Ready(42));
    assert_eq!(poll_once(first.as_mut()), Poll::Ready(42));
    assert_eq!(starts.get(), 1);
}

#[test]
fn cancelled_initiator_does_not_cache_completed_follower_result() {
    let single_flight = SingleFlight::new();
    let (release, ready) = oneshot::channel();
    let mut initiator = Box::pin(single_flight.run_keyed(7, || async move {
        ready.await.expect("release shared operation")
    }));
    let mut follower = Box::pin(single_flight.run_keyed(7, || async { 99 }));

    assert!(poll_once(initiator.as_mut()).is_pending());
    assert!(poll_once(follower.as_mut()).is_pending());
    drop(initiator);
    release.send(42).expect("follower retains shared operation");
    assert_eq!(poll_once(follower.as_mut()), Poll::Ready(42));
    assert_eq!(block_on(single_flight.run_keyed(7, || async { 84 })), 84);
}

#[test]
fn older_generation_completion_cannot_clear_newer_pending_flight() {
    let single_flight = SingleFlight::new();
    let (release_old, old_ready) = oneshot::channel();
    let (release_new, new_ready) = oneshot::channel();
    let mut old = Box::pin(single_flight.run_keyed(10, || async move {
        old_ready.await.expect("release old generation")
    }));
    let mut new = Box::pin(single_flight.run_keyed(11, || async move {
        new_ready.await.expect("release new generation")
    }));

    assert!(poll_once(old.as_mut()).is_pending());
    assert!(poll_once(new.as_mut()).is_pending());
    release_old.send(10).expect("old operation is alive");
    assert_eq!(poll_once(old.as_mut()), Poll::Ready(10));
    let mut new_follower = Box::pin(single_flight.run_keyed(11, || async { 99 }));
    assert!(poll_once(new_follower.as_mut()).is_pending());
    release_new.send(11).expect("new operation is independent");
    assert_eq!(poll_once(new_follower.as_mut()), Poll::Ready(11));
    assert_eq!(poll_once(new.as_mut()), Poll::Ready(11));
}

#[test]
fn newer_generation_can_finish_and_restart_before_older_completion() {
    let single_flight = SingleFlight::new();
    let (release_old, old_ready) = oneshot::channel();
    let (release_new, new_ready) = oneshot::channel();
    let mut old = Box::pin(single_flight.run_keyed(10, || async move {
        old_ready.await.expect("release old generation")
    }));
    assert!(poll_once(old.as_mut()).is_pending());
    assert_eq!(block_on(single_flight.run_keyed(11, || async { 11 })), 11);
    let mut restarted = Box::pin(single_flight.run_keyed(11, || async move {
        new_ready.await.expect("release restarted generation")
    }));
    assert!(poll_once(restarted.as_mut()).is_pending());

    release_old.send(10).expect("old operation is alive");
    assert_eq!(poll_once(old.as_mut()), Poll::Ready(10));
    let mut follower = Box::pin(single_flight.run_keyed(11, || async { 99 }));
    assert!(poll_once(follower.as_mut()).is_pending());
    release_new.send(12).expect("restarted operation is alive");
    assert_eq!(poll_once(follower.as_mut()), Poll::Ready(12));
    assert_eq!(poll_once(restarted.as_mut()), Poll::Ready(12));
}

#[test]
fn completed_waiter_cannot_clear_a_replacement_with_the_same_generation() {
    let single_flight = SingleFlight::new();
    let (release_first, first_ready) = oneshot::channel();
    let (release_second, second_ready) = oneshot::channel();
    let mut first = Box::pin(single_flight.run_keyed(7, || async move {
        first_ready.await.expect("release first flight")
    }));
    let mut late_follower = Box::pin(single_flight.run_keyed(7, || async { 99 }));
    assert!(poll_once(first.as_mut()).is_pending());
    assert!(poll_once(late_follower.as_mut()).is_pending());
    release_first.send(1).expect("first operation is alive");
    assert_eq!(poll_once(first.as_mut()), Poll::Ready(1));

    let mut second = Box::pin(single_flight.run_keyed(7, || async move {
        second_ready.await.expect("release second flight")
    }));
    assert!(poll_once(second.as_mut()).is_pending());
    assert_eq!(poll_once(late_follower.as_mut()), Poll::Ready(1));
    let mut second_follower = Box::pin(single_flight.run_keyed(7, || async { 99 }));
    assert!(poll_once(second_follower.as_mut()).is_pending());
    release_second.send(2).expect("second operation is alive");
    assert_eq!(poll_once(second_follower.as_mut()), Poll::Ready(2));
    assert_eq!(poll_once(second.as_mut()), Poll::Ready(2));
}

#[test]
fn unkeyed_calls_share_generation_zero() {
    let single_flight = SingleFlight::new();
    let (release, ready) = oneshot::channel();
    let mut unkeyed = Box::pin(
        single_flight.run(|| async move { ready.await.expect("release unkeyed operation") }),
    );
    let mut keyed = Box::pin(single_flight.run_keyed(0, || async { 99 }));
    assert!(poll_once(unkeyed.as_mut()).is_pending());
    assert!(poll_once(keyed.as_mut()).is_pending());
    release.send(42).expect("unkeyed operation is alive");
    assert_eq!(poll_once(keyed.as_mut()), Poll::Ready(42));
    assert_eq!(poll_once(unkeyed.as_mut()), Poll::Ready(42));
}
