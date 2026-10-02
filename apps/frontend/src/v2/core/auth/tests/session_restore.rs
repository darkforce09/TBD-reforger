//! Native tests for the cold-start restore every request waits for.

#![cfg(not(target_arch = "wasm32"))]

use super::*;
use futures::executor::{block_on, LocalPool};
use futures::task::LocalSpawnExt;
use std::cell::Cell;
use std::rc::Rc;

fn with_owner(test: impl FnOnce()) {
    let owner = Owner::new();
    owner.with(test);
}

/// Spawn one waiter on `pool`; the returned flag turns true once its wait has returned.
fn spawn_waiter(pool: &LocalPool, restore: SessionRestore) -> Rc<Cell<bool>> {
    let released = Rc::new(Cell::new(false));
    let flag = released.clone();
    pool.spawner()
        .spawn_local(async move {
            restore.wait().await;
            flag.set(true);
        })
        .expect("spawn the waiter");
    released
}

#[test]
fn a_waiting_request_parks_until_the_restore_settles() {
    with_owner(|| {
        let restore = SessionRestore::new(false);
        let mut pool = LocalPool::new();
        let released = spawn_waiter(&pool, restore);
        pool.run_until_stalled();
        assert!(!released.get(), "the request must wait for the restore");
        restore.settle();
        pool.run_until_stalled();
        assert!(released.get(), "settling releases the request");
    });
}

#[test]
fn a_settled_restore_does_not_hold_a_request() {
    with_owner(|| {
        let restore = SessionRestore::new(true);
        block_on(restore.wait());
        assert!(restore.is_settled());
    });
}

#[test]
fn settling_releases_every_waiting_request() {
    with_owner(|| {
        let restore = SessionRestore::new(false);
        let mut pool = LocalPool::new();
        let waiters: Vec<_> = (0..3).map(|_| spawn_waiter(&pool, restore)).collect();
        pool.run_until_stalled();
        assert!(waiters.iter().all(|released| !released.get()));
        restore.settle();
        pool.run_until_stalled();
        assert!(waiters.iter().all(|released| released.get()));
    });
}

#[test]
fn settling_twice_changes_nothing() {
    with_owner(|| {
        let restore = SessionRestore::new(false);
        restore.settle();
        restore.settle();
        assert!(restore.is_settled());
        block_on(restore.wait());
    });
}
