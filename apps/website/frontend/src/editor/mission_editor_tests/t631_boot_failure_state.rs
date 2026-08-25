use super::boot_progress::{BootEvent, BootProgress, BootSeg};
use super::BootPhase;

/// The verbatim first line of the observed wasm chain (`webgpu.rs:2331`). This is the string
/// the operator must end up staring at instead of a frozen "Loading terrain… 50%".
const REAL_REASON: &str = "createBuffer failed, size (32) too large";
/// The kind of later, unrelated noise the doc task or a second failure could carry. If this
/// ever ends up on screen the FIRST panic's cause has been buried — the exact regression.
const MISLEADING_REASON: &str = "RefCell already borrowed";

/// Reproduce the two-task race in boot order: the bar is honestly metered up to the point the
/// GPU dies, the engine task lands in `Failed`, and THEN the doc-hydrate task — which knows
/// nothing about the engine — tries to move the overlay on. Returns the phase the overlay would
/// actually render.
fn drive_boot_with_engine_failure(sticky: bool) -> BootPhase {
    // 1. The T-628 bar is real up to the failure: mission finishes, terrain is half-way — this
    //    is the "Loading terrain… 50%" the ticket reproduced. `stage()` names that segment.
    let mut prog = BootProgress::new();
    prog.apply(BootEvent::Budget(BootSeg::Mission, 2_032));
    prog.apply(BootEvent::Done(BootSeg::Mission, 2_032));
    prog.apply(BootEvent::Finish(BootSeg::Mission));
    prog.apply(BootEvent::Budget(BootSeg::Terrain, 71_911_548));
    prog.apply(BootEvent::Done(BootSeg::Terrain, 35_955_774));
    let seg = prog.stage();
    assert_eq!(
        seg,
        BootSeg::Terrain,
        "the failure must be attributed to the segment on screen when the GPU died — the \
         operator saw 'Loading terrain…', so the error must say terrain, not a generic apology"
    );

    // 2. The engine task's `Err` arm: drive the machine into `Failed` carrying the REAL reason.
    let mut phase = BootPhase::Hydrating;
    let fail = BootPhase::Failed {
        seg,
        reason: REAL_REASON.to_string(),
    };
    phase = if sticky {
        phase.advance(fail)
    } else {
        // The perturbation (see the paired test): a fold that is NOT sticky. Same call shape,
        // so the ONLY difference under test is the stickiness the fix adds.
        fail
    };

    // 3. The concurrent doc-hydrate task settles AFTER the failure and reaches for the overlay,
    //    exactly as it does live: first `LoadingMap`, then `Ready` at the hand-over. Both go
    //    through the same fold the production code uses.
    let load = BootPhase::LoadingMap;
    let ready = BootPhase::Ready;
    phase = if sticky { phase.advance(load) } else { load };
    phase = if sticky { phase.advance(ready) } else { ready };
    phase
}

#[test]
fn engine_failure_reaches_the_error_state_with_the_original_reason() {
    let phase = drive_boot_with_engine_failure(true);
    match phase {
        BootPhase::Failed { seg, reason } => {
            assert_eq!(
                seg,
                BootSeg::Terrain,
                "the error must still name the segment that broke"
            );
            assert_eq!(
                reason, REAL_REASON,
                "the overlay must carry the REAL wgpu reason verbatim — the whole ticket is \
                 that the boot stops being silent and says WHY"
            );
            assert_ne!(
                reason, MISLEADING_REASON,
                "and it must NOT be the later, misleading event: a hydrate that finishes after \
                 the engine failed must not bury the first cause"
            );
        }
        other => panic!(
            "the overlay must be in the Failed error state, not {other:?} — a non-Failed \
             phase here means a later event painted a spinner back over the error (LoadingMap) \
             or dismissed it onto a dead map (Ready), losing the reason"
        ),
    }
}

/// The "make it wrong on demand / prove it fires by perturbing once" clause. This runs the
/// SAME scenario with the stickiness removed and asserts the machine then loses the reason —
/// which proves the assertion in the test above has teeth: if `advance` stopped being sticky,
/// the overlay would end at `Ready` (or carry the misleading reason) and that test would fail.
/// A green that was never watched fail does not count; this is the watched failure, pinned.
#[test]
fn without_stickiness_the_reason_is_overwritten_which_is_the_bug() {
    let perturbed = drive_boot_with_engine_failure(false);
    assert_eq!(
        perturbed,
        BootPhase::Ready,
        "with a non-sticky fold the concurrent hydrate marches the overlay to Ready and the \
         real reason is gone — this is precisely the silent-failure regression, and its \
         presence here is what proves the sticky-fold test is load-bearing rather than vacuous"
    );
}

/// `advance` is sticky against EVERY later transition, not just the two the doc task happens to
/// send — including a second, different engine failure. The first cause wins, always.
#[test]
fn a_second_failure_cannot_overwrite_the_first_reason() {
    let first = BootPhase::Failed {
        seg: BootSeg::Terrain,
        reason: REAL_REASON.to_string(),
    };
    let second = first.clone().advance(BootPhase::Failed {
        seg: BootSeg::World,
        reason: MISLEADING_REASON.to_string(),
    });
    assert_eq!(
        second, first,
        "once failed, a later failure is a no-op — the operator is shown the FIRST panic's \
         cause, which is the one that actually explains the boot"
    );
    // And a success flip is likewise swallowed: nothing takes the error overlay down but the
    // operator's own 'Continue without map'.
    assert_eq!(
        first.clone().advance(BootPhase::Ready),
        first,
        "a rendezvous that fires after a failure must not dismiss the error onto a dead map"
    );
}

/// The stage title the error card renders from — 'Loading terrain…' with the ellipsis trimmed
/// becomes 'Loading terrain', and the card appends ' failed'. Pin the pieces the view relies
/// on so a title change cannot silently produce 'Loading terrain… failed' with a stray ellipsis.
#[test]
fn the_failing_segment_titles_read_as_a_sentence() {
    assert_eq!(BootSeg::Terrain.title(), "Loading terrain…");
    assert_eq!(
        BootSeg::Terrain.title().trim_end_matches('…'),
        "Loading terrain",
        "the card composes '<title-without-ellipsis> failed'; a trailing ellipsis would read \
         as 'Loading terrain… failed'"
    );
}
