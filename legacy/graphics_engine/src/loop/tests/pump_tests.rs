//! Role: pump tests.
//! Position: `loop/tests` in the graphics engine.
//! Signals & state: a spy target that records the order of the calls the pump makes.
//! Invariants: these pin the frame contract the three call sites (the editor viewport and the
//! two debug pages) each used to re-implement — render before poll, a contended target skipped
//! rather than panicked, disposal ending the loop exactly once.

use crate::r#loop::pump::FrameTarget;
use crate::r#loop::pump::RafPump;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// Records every call it receives, in order. `poll_device` takes `&self` (the engine's `poll`
/// does), so the log is behind a cell.
#[derive(Default)]
struct Spy {
    log: RefCell<Vec<&'static str>>,
}

impl FrameTarget for Spy {
    fn render_frame(&mut self) {
        self.log.borrow_mut().push("render");
    }

    fn poll_device(&self) {
        self.log.borrow_mut().push("poll");
    }
}

fn spy() -> (Rc<RefCell<Option<Spy>>>, Arc<AtomicBool>) {
    (
        Rc::new(RefCell::new(Some(Spy::default()))),
        Arc::new(AtomicBool::new(false)),
    )
}

fn log_of(target: &Rc<RefCell<Option<Spy>>>) -> Vec<&'static str> {
    target
        .borrow()
        .as_ref()
        .expect("target")
        .log
        .borrow()
        .clone()
}

#[test]
fn a_frame_renders_then_polls() {
    let (target, disposed) = spy();
    let mut pump = RafPump::new(target.clone(), disposed);
    assert!(pump.tick());
    assert_eq!(log_of(&target), vec!["render", "poll"]);
    assert_eq!(pump.frames(), 1);
}

#[test]
fn a_contended_target_skips_the_frame_and_keeps_the_loop_alive() {
    let (target, disposed) = spy();
    let mut pump = RafPump::new(target.clone(), disposed);
    let held = target.borrow_mut(); // someone else is mid-mutation
    assert!(pump.tick(), "T-631: contention must not stop the loop");
    assert_eq!(pump.frames(), 0, "a skipped frame is not a frame");
    drop(held);
    assert!(pump.tick());
    assert_eq!(log_of(&target), vec!["render", "poll"]);
}

#[test]
fn a_target_that_has_not_booted_yet_is_skipped_without_stopping() {
    let target: Rc<RefCell<Option<Spy>>> = Rc::new(RefCell::new(None));
    let mut pump = RafPump::new(target, Arc::new(AtomicBool::new(false)));
    assert!(pump.tick());
    assert_eq!(pump.frames(), 0);
}

#[test]
fn disposal_ends_the_loop_and_renders_nothing() {
    let (target, disposed) = spy();
    let mut pump = RafPump::new(target.clone(), disposed.clone());
    assert!(pump.tick());
    disposed.store(true, Ordering::Relaxed);
    assert!(!pump.tick(), "a disposed pump must tell the caller to stop");
    assert_eq!(log_of(&target), vec!["render", "poll"], "no second frame");
    assert_eq!(pump.frames(), 1);
}

#[test]
fn the_hook_runs_after_the_frame_and_sees_the_running_count() {
    let (target, disposed) = spy();
    let counts: Rc<RefCell<Vec<u32>>> = Rc::new(RefCell::new(Vec::new()));
    let seen = counts.clone();
    let mut pump = RafPump::new(target.clone(), disposed).after_frame(move |t: &mut Spy, n| {
        t.log.borrow_mut().push("hook");
        seen.borrow_mut().push(n);
    });
    for _ in 0..3 {
        assert!(pump.tick());
    }
    assert_eq!(*counts.borrow(), vec![1, 2, 3]);
    assert_eq!(
        log_of(&target),
        vec![
            "render", "poll", "hook", "render", "poll", "hook", "render", "poll", "hook"
        ],
        "the hook runs last, inside the same borrow"
    );
}

#[test]
fn a_skipped_frame_does_not_call_the_hook() {
    let (target, disposed) = spy();
    let calls = Rc::new(RefCell::new(0u32));
    let seen = calls.clone();
    let mut pump = RafPump::new(target.clone(), disposed).after_frame(move |_: &mut Spy, _| {
        *seen.borrow_mut() += 1;
    });
    let held = target.borrow_mut();
    assert!(pump.tick());
    drop(held);
    assert_eq!(*calls.borrow(), 0);
    assert!(pump.tick());
    assert_eq!(*calls.borrow(), 1);
}
