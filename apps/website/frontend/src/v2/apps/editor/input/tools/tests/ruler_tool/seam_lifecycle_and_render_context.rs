//! Checks seam registration lifetime and render-context source invariants.

use super::register_ruler_chain;
use crate::v2::apps::editor::input::tools::los_tool::{
    register_los_sampler, register_los_state, register_viewshed_state,
};
use leptos::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use website_map_engine::editing::tools::line_of_sight::capture::{LosState, ViewshedState};
use website_map_engine::editing::tools::line_of_sight::host_registry::{
    read_registered_sampler, read_registered_state, read_registered_viewshed,
};
use website_map_engine::editing::tools::ruler::{read_registered_chain, RulerChain, RulerPoint};

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
            // The one seam that is a real closure: it notes its own tag when CALLED, so shape 2
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

/// Shape 1 — never installed. The baseline: without it, a green in shape 2 could just mean the
/// seam never worked in the first place.
#[test]
fn an_uninstalled_seam_reports_honest_failure() {
    let _root = Owner::new();
    for seam in seams() {
        forget_answers();
        assert!(
            !(seam.ask)(),
            "T-778 {}: a seam nothing ever registered must report FAILURE",
            seam.name
        );
        assert!(
            answered().is_empty(),
            "T-778 {}: nothing may answer when nothing is installed — got {:?}",
            seam.name,
            answered()
        );
    }
}

/// Shape 2 — install then unmount. The seam must report failure AND not read the dead handle.
#[test]
fn a_seam_is_unregistered_when_its_owner_is_cleaned_up() {
    let root = Owner::new();
    for seam in seams() {
        let mounted = root.child();
        mounted.with(|| (seam.install)("A"));

        forget_answers();
        assert!(
            (seam.ask)(),
            "T-778 {} precondition: while mounted the seam really does answer",
            seam.name
        );
        assert_eq!(
            answered(),
            vec!["A".to_string()],
            "T-778 {} precondition: the LIVE registration is the one that answered",
            seam.name
        );

        mounted.cleanup();

        forget_answers();
        assert!(
            !(seam.ask)(),
            "T-778 {}: the installing owner is gone, so the seam must report FAILURE rather \
                 than success over state whose every write is a disposed no-op",
            seam.name
        );
        assert!(
            answered().is_empty(),
            "T-778 {}: the stale registration must not be read at all after unmount — got {:?}",
            seam.name,
            answered()
        );
    }
}

/// Shape 3 — the identity guard. A remount installs its NEWER value before the old owner's
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
            "T-778 {}: the NEW mount is live — the superseded owner's cleanup must not \
                 unregister it",
            seam.name
        );
        assert_eq!(
            answered(),
            vec!["B".to_string()],
            "T-778 {}: the surviving registration must be the NEWER one, not a leftover that \
                 merely happens to answer",
            seam.name
        );

        new.cleanup();

        forget_answers();
        assert!(
            !(seam.ask)(),
            "T-778 {}: the live mount's OWN cleanup does clear it — the guard skips losers, \
                 not everyone",
            seam.name
        );
    }
}

/// Class-R — the fifth seam, `world_assets::RENDER_CTX`, which is wasm-only and so invisible to
/// every test above. Pinned over `live_code`, which cuts the test module, the comments AND the
/// string literals: the needles below therefore have to be real calls in the production body,
/// not the prose two lines above them nor a decoy in a string.
///
/// [wave 142 F-3] TIGHTENED. The pin reddened on the honest regression (the pre-T-778 direct
/// write) but GREENED on a decoy: rename a local to `install_seam_later` and write the cell with
/// `RefCell::replace` instead of `borrow_mut`, and both needles were satisfied while the
/// un-unregisterable registration was back. `install_seam` was a bare substring, and the negative
/// named ONE of the several ways to write a `RefCell`. Two changes close it:
///
/// * the positive names the CALL and its ARGUMENT — the seam has to be installed on THIS cell,
///   which no local's name can satisfy;
/// * the negative forbids reaching the cell at all from this body. `install_seam(&RENDER_CTX, …)`
///   passes the cell; it never opens it. So `RENDER_CTX.with` inside `register_render_ctx` means
///   a hand-rolled registration by definition, whatever mutator it then reaches for — and the
///   three write shapes are forbidden by name as well, so the failure message says which one.
///
/// This seam has no behavioural test anywhere (`world_assets` is wasm32-only with no
/// wasm-bindgen-test target), so this pin is its only guarantee. That is exactly the case where
/// the standard Class-R substring ceiling is worth paying to raise.
#[test]
fn the_render_ctx_seam_is_installed() {
    use crate::v2::core::test_support::class_r_scrub::{live_code, only_body, only_item};
    let src = live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/bridge/world_assets.rs"
    )));

    let body = only_body(&src, "pub fn register_render_ctx(");
    // The call AND the cell it installs on — `install_seam_later = ()` does not contain this.
    let install = ["install_seam(&", "RENDER_CTX"].concat();
    assert!(
        body.contains(&install),
        "T-778: register_render_ctx must INSTALL on RENDER_CTX (register + guarded unregister at \
             the owner's cleanup), not write the cell directly; got:\n{body}"
    );
    // `install_seam` takes the cell; it never opens it. Opening it here is a hand-rolled
    // registration whichever mutator follows — `borrow_mut`, `replace`, `take`, `set`.
    let opens_cell = ["RENDER_CTX", ".with"].concat();
    for forbidden in [opens_cell.as_str(), "borrow_mut", ".replace(", ".take("] {
        assert!(
            !body.contains(forbidden),
            "T-778: register_render_ctx must not reach into RENDER_CTX behind install_seam's \
                 back (`{forbidden}`) — a bare write is the un-unregisterable registration this \
                 ticket removes; got:\n{body}"
        );
    }

    // The tuple-valued seam needs its own identity, and it must be Rc IDENTITY on both handles —
    // never a `usize` address (ABA) and never `||`, which would let a half-matching cleanup clear
    // a live remount.
    let ident = only_item(&src, "fn is_same_registration(");
    assert!(
        ident.contains("Rc::ptr_eq") && ident.contains("&&"),
        "T-778: RENDER_CTX identity must be Rc::ptr_eq on BOTH leaked handles; got:\n{ident}"
    );
}
