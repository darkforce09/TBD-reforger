use super::*;

const W: u64 = DEBOUNCE_MS;

/// A burst of events fires exactly once, one window after the LAST event.
#[test]
fn burst_coalesces_to_one_fire() {
    let mut d = Debouncer::new(W);
    assert_eq!(d.poll(0), None, "idle never fires");
    d.on_event(0);
    d.on_event(100);
    d.on_event(300);
    assert_eq!(d.poll(300 + W - 1), None, "still inside the quiet window");
    assert_eq!(d.due_in(300), Some(W));
    assert_eq!(
        d.poll(300 + W),
        Some(Fire { run_check: true }),
        "fires one window after the last event"
    );
    assert_eq!(d.poll(300 + W + 1), None, "exactly one fire per burst");
    assert_eq!(d.due_in(300 + W + 1), None);
}

/// A new event during the quiet countdown restarts it (debounce, not delay).
#[test]
fn new_event_extends_the_quiet_window() {
    let mut d = Debouncer::new(W);
    d.on_event(0);
    assert_eq!(d.poll(W - 10), None);
    d.on_event(W - 5);
    assert_eq!(d.poll(W + 10), None, "the late event reset the window");
    assert_eq!(d.poll(W - 5 + W), Some(Fire { run_check: true }));
}

/// While suppressed: fires still happen (reload allowed) but never run the
/// check.
#[test]
fn suppressed_fires_reload_only() {
    let mut d = Debouncer::new(W);
    d.set_suppressed(true, 0);
    d.on_event(10);
    assert_eq!(d.poll(10 + W), Some(Fire { run_check: false }));
}

/// The trailing window: events landing within one window of suppression
/// clearing are verb-write residue — reload-only. Events after it are real.
#[test]
fn trailing_window_after_clear_stays_check_silent() {
    let mut d = Debouncer::new(W);
    d.set_suppressed(true, 0);
    d.set_suppressed(false, 1000);
    // Residue inside the trailing window (1000..1600): no check.
    d.on_event(1100);
    assert_eq!(d.poll(1100 + W), Some(Fire { run_check: false }));
    // A real edit after the trailing window: check runs again.
    d.on_event(1700);
    assert_eq!(d.poll(1700 + W), Some(Fire { run_check: true }));
}

/// A burst spanning the trailing edge: ONE fire, and the real (post-window)
/// edit in it wins — the check runs.
#[test]
fn mixed_burst_across_the_trailing_edge_keeps_its_check() {
    let mut d = Debouncer::new(W);
    d.set_suppressed(true, 0);
    d.set_suppressed(false, 1000);
    d.on_event(1100); // residue (trailing until 1600)
    d.on_event(1650); // real edit — same burst
    let fire = d.poll(1650 + W);
    assert_eq!(fire, Some(Fire { run_check: true }));
    assert_eq!(d.poll(1650 + W + 1), None, "still exactly one fire");
}

/// Suppression turning ON between event and fire wins: a verb in flight is
/// an absolute no-check, even for a pre-verb edit.
#[test]
fn suppression_at_fire_time_beats_a_checkworthy_event() {
    let mut d = Debouncer::new(W);
    d.on_event(0);
    d.set_suppressed(true, 100);
    assert_eq!(d.poll(W), Some(Fire { run_check: false }));
}

/// Re-setting suppression cancels a pending trailing window.
#[test]
fn resuppression_cancels_the_trailing_window() {
    let mut d = Debouncer::new(W);
    d.set_suppressed(true, 0);
    d.set_suppressed(false, 1000); // trailing until 1600
    d.set_suppressed(true, 1200);
    d.set_suppressed(false, 1300); // NEW trailing until 1900
    d.on_event(1700); // inside the new window — residue
    assert_eq!(d.poll(1700 + W), Some(Fire { run_check: false }));
    d.on_event(2000); // past it — real
    assert_eq!(d.poll(2000 + W), Some(Fire { run_check: true }));
}

#[test]
fn relevance_filter_matches_the_watched_surfaces_only() {
    let root = Path::new("/repo");
    // Everything under .ai/tickets/, recursively.
    assert!(relevant(root, Path::new("/repo/.ai/tickets/T-915.3.toml")));
    assert!(relevant(root, Path::new("/repo/.ai/tickets/wave.lock")));
    assert!(relevant(
        root,
        Path::new("/repo/.ai/tickets/metrics/T-1/x.json")
    ));
    // Root level: CLAUDE.md only — sibling noise must not fire.
    assert!(relevant(root, Path::new("/repo/CLAUDE.md")));
    assert!(!relevant(root, Path::new("/repo/Cargo.lock")));
    assert!(!relevant(root, Path::new("/repo/target")));
    // Elsewhere: the one ROADMAP marker file, and nothing beside it.
    assert!(relevant(
        root,
        Path::new("/repo/documentation_v2/website/frontend/apps/editor/mission_creator_roadmap.md")
    ));
    assert!(!relevant(
        root,
        Path::new("/repo/documentation_v2/website/frontend/apps/editor/other.md")
    ));
    assert!(!relevant(
        Path::new("/elsewhere"),
        Path::new("/repo/CLAUDE.md")
    ));
}
