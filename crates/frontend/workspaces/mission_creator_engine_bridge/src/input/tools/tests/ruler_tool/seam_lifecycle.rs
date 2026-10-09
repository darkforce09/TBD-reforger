//! Seam registration lifetime: an owner's cleanup unregisters its seam, and a superseded
//! owner's cleanup leaves the newer registration alone.

use super::register_ruler_chain;
use crate::input::tools::los_tool::{
    register_los_sampler, register_los_state, register_viewshed_state,
};
use leptos::prelude::*;
use map_editing_tools::line_of_sight::capture::{LosState, ViewshedState};
use map_editing_tools::line_of_sight::host_registry::{
    read_registered_sampler, read_registered_state, read_registered_viewshed,
};
use map_editing_tools::ruler::{RulerChain, RulerPoint, read_registered_chain};
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Every tag that ANSWERED a seam's question, in call order. "Did anything actually happen"
    /// is answered by WHICH registration answered, not only by the seam's boolean — a seam that
    /// reports failure while still reading a dead handle has not been fixed.
    static ANSWERED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn note(tag: &str) {
    ANSWERED.with(|l| l.borrow_mut().push(tag.to_string()));
}
fn answered() -> Vec<String> {
    ANSWERED.with(|l| l.borrow().clone())
}
fn forget_answers() {
    ANSWERED.with(|l| l.borrow_mut().clear());
}

/// The world X a given mount registers. Both are NON-ZERO and distinct, so "the `Default` empty
/// state answered" (the honest failure) can never be mistaken for "a registration answered", and
/// the two mounts can never be mistaken for each other.
fn mark(tag: &str) -> f64 {
    match tag {
        "A" => 1000.0,
        "B" => 2000.0,
        other => unreachable!("unknown mount tag {other}"),
    }
}

/// Decode a read-back world X into the mount that registered it, and record that it ANSWERED.
fn note_mark(x: f64) {
    let tag = ["A", "B"]
        .into_iter()
        .find(|t| (mark(t) - x).abs() < f64::EPSILON)
        .unwrap_or("<unregistered value>");
    note(tag);
}

/// One seam, reduced to the two operations its lifecycle turns on.
struct Seam {
    /// The thread_local's name, so a failure names the seam rather than a row index.
    name: &'static str,
    /// Register a `tag`-marked value under the CURRENT reactive owner.
    install: fn(&'static str),
    /// Ask the seam its OWN question. `true` = a live registration answered (and `note`d itself).
    ask: fn() -> bool,
}

/// Every natively-compiled seam in this cluster. A new one belongs here.
fn seams() -> [Seam; 4] {
    [
        Seam {
            name: "RULER_CHAIN",
            install: |tag| {
                register_ruler_chain(Rc::new(RefCell::new(RulerChain {
                    points: vec![RulerPoint::new(mark(tag), 0.0, None)],
                    ..RulerChain::default()
                })));
            },
            ask: || match read_registered_chain().points.first() {
                Some(p) => {
                    note_mark(p.x);
                    true
                }
                None => false,
            },
        },
        Seam {
            name: "LOS_STATE",
            install: |tag| {
                register_los_state(Rc::new(RefCell::new(LosState {
                    pending_obs: Some((mark(tag), 0.0, None)),
                    ..LosState::default()
                })));
            },
            ask: || match read_registered_state().pending_obs {
                Some((x, _, _)) => {
                    note_mark(x);
                    true
                }
                None => false,
            },
        },
        Seam {
            name: "LOS_SAMPLER",
            // The one seam that is a real closure: it notes its own tag when CALLED, so the unmount case
            // can distinguish "the seam reported failure" from "the stale closure still ran".
            install: |tag| {
                register_los_sampler(Rc::new(move |_x, _y| {
                    note(tag);
                    Some(mark(tag))
                }));
            },
            ask: || match read_registered_sampler() {
                Some(f) => f(0.0, 0.0).is_some(),
                None => false,
            },
        },
        Seam {
            name: "VIEWSHED_STATE",
            install: |tag| {
                register_viewshed_state(Rc::new(RefCell::new(ViewshedState {
                    observer: Some((mark(tag), 0.0, None)),
                    ..ViewshedState::default()
                })));
            },
            ask: || match read_registered_viewshed().observer {
                Some((x, _, _)) => {
                    note_mark(x);
                    true
                }
                None => false,
            },
        },
    ]
}

/// Install then unmount. The seam must report failure AND not read the dead handle.
#[test]
fn a_seam_is_unregistered_when_its_owner_is_cleaned_up() {
    let root = Owner::new();
    for seam in seams() {
        let mounted = root.child();
        mounted.with(|| (seam.install)("A"));

        forget_answers();
        assert!(
            (seam.ask)(),
            "{} precondition: while mounted the seam really does answer",
            seam.name
        );
        assert_eq!(
            answered(),
            vec!["A".to_string()],
            "{} precondition: the LIVE registration is the one that answered",
            seam.name
        );

        mounted.cleanup();

        forget_answers();
        assert!(
            !(seam.ask)(),
            "{}: the installing owner is gone, so the seam must report FAILURE rather \
                 than success over state whose every write is a disposed no-op",
            seam.name
        );
        assert!(
            answered().is_empty(),
            "{}: the stale registration must not be read at all after unmount — got {:?}",
            seam.name,
            answered()
        );
    }
}

/// The identity guard. A remount installs its NEWER value before the old owner's
/// cleanup runs. The losing cleanup must recognise it is no longer the live registration and
/// leave the new one alone — otherwise the fix for a stale seam becomes a fresh way to kill a
/// live one, and the click is dead again. This is the case an unconditional unregister fails.
#[test]
fn an_older_owners_cleanup_does_not_clobber_a_newer_registration() {
    let root = Owner::new();
    for seam in seams() {
        // Siblings, not parent/child: two successive mounts under the page owner. A child would
        // be cleaned up BY the parent and would prove nothing about the guard.
        let old = root.child();
        let new = root.child();
        old.with(|| (seam.install)("A"));
        new.with(|| (seam.install)("B"));

        old.cleanup();

        forget_answers();
        assert!(
            (seam.ask)(),
            "{}: the NEW mount is live — the superseded owner's cleanup must not \
                 unregister it",
            seam.name
        );
        assert_eq!(
            answered(),
            vec!["B".to_string()],
            "{}: the surviving registration must be the NEWER one, not a leftover that \
                 merely happens to answer",
            seam.name
        );

        new.cleanup();

        forget_answers();
        assert!(
            !(seam.ask)(),
            "{}: the live mount's OWN cleanup does clear it — the guard skips losers, \
                 not everyone",
            seam.name
        );
    }
}
