use super::armed_place::{
    decide_armed_pointerup, may_promote, open_left_gesture_while_armed, run, step, ArmedUp, Effect,
    Ev, Host, LeftKind,
};

#[test]
fn decide_helpers_match_the_contract() {
    assert_eq!(decide_armed_pointerup(0, true), ArmedUp::Place);
    assert_eq!(decide_armed_pointerup(0, false), ArmedUp::KeepArmed);
    assert_eq!(decide_armed_pointerup(1, true), ArmedUp::FallThroughPan);
    assert_eq!(decide_armed_pointerup(2, true), ArmedUp::Disarm);
    assert_eq!(decide_armed_pointerup(3, true), ArmedUp::Ignore);
    assert!(may_promote(1));
    assert!(!may_promote(0));
    assert!(!open_left_gesture_while_armed(true));
    assert!(open_left_gesture_while_armed(false));
}

/// MAJOR-1 / wave-108 composition: arm on dock, release over chrome → still armed.
#[test]
fn sequence_dock_arm_then_chrome_release_keeps_armed() {
    let (host, effects) = run(
        Host::default(),
        &[
            Ev::Arm,
            Ev::PointerUp {
                button: 0,
                on_canvas: false,
            },
        ],
    );
    assert!(host.armed, "arming click's own release must NOT cancel");
    assert!(
        !effects.contains(&Effect::Place) && !effects.contains(&Effect::Disarm),
        "chrome release is KeepArmed — no place, no disarm; got {effects:?}"
    );
}

/// MAJOR-1 picker path (after on:click arm): genuine second canvas click places.
#[test]
fn sequence_arm_then_canvas_lmb_places() {
    let (host, effects) = run(
        Host::default(),
        &[
            Ev::Arm,
            Ev::PointerDown {
                button: 0,
                on_canvas: true,
            },
            Ev::PointerUp {
                button: 0,
                on_canvas: true,
            },
        ],
    );
    assert!(!host.armed);
    assert!(effects.contains(&Effect::Place), "got {effects:?}");
    assert!(
        host.left.is_none(),
        "armed up must clear left; left={:?}",
        host.left
    );
}

/// MAJOR-2: canvas press while armed must NOT latch Pending; up clears anyway.
#[test]
fn sequence_armed_canvas_press_does_not_latch_pending() {
    let mut host = Host {
        armed: true,
        ..Host::default()
    };
    let effects = step(
        &mut host,
        Ev::PointerDown {
            button: 0,
            on_canvas: true,
        },
    );
    assert!(effects.is_empty());
    assert!(
        host.left.is_none(),
        "must not open Pending while armed; left={:?}",
        host.left
    );
    // Even if a prior bug left a Ruler stranded, armed up clears it.
    host.left = Some(LeftKind::Ruler);
    let effects = step(
        &mut host,
        Ev::PointerUp {
            button: 0,
            on_canvas: true,
        },
    );
    assert!(effects.contains(&Effect::ClearLeft), "got {effects:?}");
    assert!(effects.contains(&Effect::Place), "got {effects:?}");
    assert!(host.left.is_none());
}

/// MAJOR-2 phantom Move: button-less move past threshold must NOT promote.
#[test]
fn sequence_buttonless_move_does_not_promote_stranded_pending() {
    let mut host = Host {
        left: Some(LeftKind::Pending),
        ..Host::default()
    };
    let effects = step(
        &mut host,
        Ev::PointerMove {
            buttons: 0,
            past_threshold: true,
        },
    );
    assert!(effects.contains(&Effect::ClearLeft), "got {effects:?}");
    assert!(!effects.contains(&Effect::PromoteMove));
    assert!(host.left.is_none());
}

/// MAJOR-2: after a real promote, non-LMB pointerup must NOT CommitMove.
#[test]
fn sequence_rmb_does_not_commit_a_move() {
    let mut host = Host {
        left: Some(LeftKind::Move),
        ..Host::default()
    };
    let effects = step(
        &mut host,
        Ev::PointerUp {
            button: 2,
            on_canvas: true,
        },
    );
    assert!(effects.contains(&Effect::ClearLeft), "got {effects:?}");
    assert!(!effects.contains(&Effect::CommitMove));
}

/// MAJOR-3: MMB while armed falls through to pan; does not place; clears stranded left.
#[test]
fn sequence_mmb_while_armed_pans_without_placing() {
    let mut host = Host {
        armed: true,
        left: Some(LeftKind::Pending),
        ..Host::default()
    };
    step(
        &mut host,
        Ev::PointerDown {
            button: 1,
            on_canvas: true,
        },
    );
    assert!(host.pan);
    let effects = step(
        &mut host,
        Ev::PointerMove {
            buttons: 4, // middle bit
            past_threshold: true,
        },
    );
    assert!(effects.contains(&Effect::PanDelta));
    let effects = step(
        &mut host,
        Ev::PointerUp {
            button: 1,
            on_canvas: true,
        },
    );
    assert!(host.armed, "MMB must not disarm");
    assert!(!effects.contains(&Effect::Place), "got {effects:?}");
    assert!(effects.contains(&Effect::ClearLeft));
    assert!(!host.pan);
    assert!(host.left.is_none());
}

/// MAJOR-3: RMB while armed disarms (Eden cancel), does not place.
#[test]
fn sequence_rmb_while_armed_disarms() {
    let (host, effects) = run(
        Host {
            armed: true,
            left: Some(LeftKind::Ruler),
            ..Host::default()
        },
        &[Ev::PointerUp {
            button: 2,
            on_canvas: true,
        }],
    );
    assert!(!host.armed);
    assert!(effects.contains(&Effect::Disarm), "got {effects:?}");
    assert!(!effects.contains(&Effect::Place));
    assert!(host.left.is_none(), "RMB armed up clears stranded Ruler");
}

/// MAJOR-3: Esc disarms.
#[test]
fn sequence_escape_disarms_armed_place() {
    let (host, effects) = run(
        Host {
            armed: true,
            ..Host::default()
        },
        &[Ev::Escape],
    );
    assert!(!host.armed);
    assert!(effects.contains(&Effect::Disarm), "got {effects:?}");
}

/// wave-108 MINOR-1 / wave-109 MINOR-5: armed place + measure press strands Ruler;
/// armed up must clear it so a later same-spot RMB cannot commit a phantom vertex/observer.
#[test]
fn sequence_armed_up_clears_stale_ruler_before_later_rmb() {
    let mut host = Host {
        armed: true,
        left: Some(LeftKind::Ruler),
        ..Host::default()
    };
    // place click
    let effects = step(
        &mut host,
        Ev::PointerUp {
            button: 0,
            on_canvas: true,
        },
    );
    assert!(effects.contains(&Effect::ClearLeft));
    assert!(effects.contains(&Effect::Place));
    assert!(host.left.is_none());
    // later RMB at same spot — no Ruler left to commit
    let effects = step(
        &mut host,
        Ev::PointerUp {
            button: 2,
            on_canvas: true,
        },
    );
    assert!(
        !effects.contains(&Effect::CommitRulerVertex),
        "got {effects:?}"
    );
}

/// Ctrl multi-place chain: each canvas up places without leaving left latched.
#[test]
fn sequence_ctrl_multi_place_never_strands_left() {
    // Pure step one-shots the arm on Place; Ctrl-keep is a place_at_keep concern.
    // Model keep by re-Arming after each Place (the host re-arms via place_at_keep).
    let mut host = Host::default();
    for _ in 0..3 {
        step(&mut host, Ev::Arm);
        step(
            &mut host,
            Ev::PointerDown {
                button: 0,
                on_canvas: true,
            },
        );
        assert!(host.left.is_none(), "no Pending while armed");
        let effects = step(
            &mut host,
            Ev::PointerUp {
                button: 0,
                on_canvas: true,
            },
        );
        assert!(effects.contains(&Effect::Place));
        assert!(host.left.is_none());
    }
}
