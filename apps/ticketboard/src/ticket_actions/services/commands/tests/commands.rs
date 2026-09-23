use super::*;
use crate::test_support::Scratch;

fn tail_of(req: &TicketCommand) -> Vec<&str> {
    assert_eq!(
        &req.args[..5],
        &TICKET_PREFIX[..],
        "every verb argv starts with the alias expansion"
    );
    req.args[5..].iter().map(String::as_str).collect()
}

#[test]
fn ship_builder_argv_and_display() {
    let req = ship("T-915.4");
    assert_eq!(tail_of(&req), vec!["ship", "T-915.4"]);
    assert_eq!(req.display, "cargo xtask ticket ship T-915.4");
    assert_eq!(req.guard, None);
}

#[test]
fn set_status_builder_covers_the_whole_enum() {
    for status in board::STATUS_ORDER {
        let req = set_status("T-9", status);
        assert_eq!(tail_of(&req), vec!["set-status", "T-9", status.as_str()]);
        assert_eq!(
            req.display,
            format!("cargo xtask ticket set-status T-9 {}", status.as_str())
        );
    }
}

#[test]
fn mark_ready_with_and_without_spec() {
    let bare = mark_ready("T-9", None);
    assert_eq!(tail_of(&bare), vec!["mark-ready", "T-9"]);
    let with = mark_ready("T-9", Some("docs/platform/spec.md"));
    assert_eq!(
        tail_of(&with),
        vec!["mark-ready", "T-9", "docs/platform/spec.md"]
    );
    assert_eq!(
        with.display,
        "cargo xtask ticket mark-ready T-9 docs/platform/spec.md"
    );
}

#[test]
fn reorder_and_advance_slice_builders() {
    assert_eq!(
        tail_of(&reorder("T-9", "T-8")),
        vec!["reorder", "T-9", "T-8"]
    );
    assert_eq!(
        reorder("T-9", "T-8").display,
        "cargo xtask ticket reorder T-9 T-8"
    );
    assert_eq!(tail_of(&advance_slice("T-9")), vec!["advance-slice", "T-9"]);
}

#[test]
fn add_builder_summary_flag_and_display_quoting() {
    let bare = add("Solo", "   ");
    assert_eq!(tail_of(&bare), vec!["add", "Solo"]);
    let full = add("Two words", "a summary");
    assert_eq!(
        tail_of(&full),
        vec!["add", "Two words", "--summary", "a summary"]
    );
    // argv carries the raw strings; only the DISPLAY quotes them.
    assert_eq!(
        full.display,
        "cargo xtask ticket add 'Two words' --summary 'a summary'"
    );
}

#[test]
fn add_child_flag_combos() {
    let plain = add_child("T-9", "kid", "", false);
    assert_eq!(tail_of(&plain), vec!["add-child", "T-9", "kid"]);
    let sum = add_child("T-9", "kid", "why", false);
    assert_eq!(
        tail_of(&sum),
        vec!["add-child", "T-9", "kid", "--summary", "why"]
    );
    let promote = add_child("T-9", "kid", "", true);
    assert_eq!(
        tail_of(&promote),
        vec!["add-child", "T-9", "kid", "--promote"]
    );
    let both = add_child("T-9", "kid", "why", true);
    assert_eq!(
        tail_of(&both),
        vec!["add-child", "T-9", "kid", "--summary", "why", "--promote"]
    );
    assert_eq!(
        both.display,
        "cargo xtask ticket add-child T-9 kid --summary why --promote"
    );
}

#[test]
fn remove_force_combo() {
    assert_eq!(tail_of(&remove("T-9", false)), vec!["remove", "T-9"]);
    assert_eq!(
        tail_of(&remove("T-9", true)),
        vec!["remove", "T-9", "--force"]
    );
    assert_eq!(
        remove("T-9", true).display,
        "cargo xtask ticket remove T-9 --force"
    );
}

#[test]
fn display_quoting_escapes_embedded_single_quotes() {
    let req = add("it's broken", "");
    assert_eq!(tail_of(&req), vec!["add", "it's broken"]);
    assert_eq!(req.display, "cargo xtask ticket add 'it'\\''s broken'");
}

// ---- queue ----

#[test]
fn queue_is_single_flight_fifo() {
    let mut q = TicketCommandQueue::default();
    assert!(!q.busy());
    let started = q.submit(ship("T-1"));
    assert_eq!(
        started.as_ref().map(|r| r.display.as_str()),
        Some("cargo xtask ticket ship T-1")
    );
    assert!(q.busy(), "in-flight from submit");
    assert_eq!(q.running_display(), Some("cargo xtask ticket ship T-1"));

    assert!(q.submit(ship("T-2")).is_none(), "second submit parks FIFO");
    assert!(q.submit(ship("T-3")).is_none());
    assert_eq!(q.pending_len(), 2);

    let fin = q.finish(true);
    assert_eq!(fin.dropped, 0);
    assert_eq!(
        fin.next.as_ref().map(|r| r.display.as_str()),
        Some("cargo xtask ticket ship T-2"),
        "FIFO order"
    );
    assert!(q.busy(), "in-flight flag stays up across the handoff");

    let fin = q.finish(true);
    assert_eq!(
        fin.next.as_ref().map(|r| r.display.as_str()),
        Some("cargo xtask ticket ship T-3")
    );
    let fin = q.finish(true);
    assert_eq!(
        fin,
        Finish {
            dropped: 0,
            next: None
        }
    );
    assert!(!q.busy(), "idle only after the last verb finishes");
}

#[test]
fn queue_failure_drops_the_pending_tail_and_never_retries() {
    let mut q = TicketCommandQueue::default();
    let _ = q.submit(ship("T-1"));
    assert!(q.submit(ship("T-2")).is_none());
    assert!(q.submit(ship("T-3")).is_none());
    let fin = q.finish(false);
    assert_eq!(fin.dropped, 2, "both pending requests dropped");
    assert_eq!(fin.next, None, "nothing dispatches after a failure");
    assert!(!q.busy());
    assert_eq!(q.pending_len(), 0);
    // The queue is reusable afterwards — a fresh submit starts clean.
    assert!(q.submit(ship("T-4")).is_some());
}

#[test]
fn queued_requests_keep_their_captured_fingerprints() {
    let mut q = TicketCommandQueue::default();
    let guard = FileChangeGuard {
        path: PathBuf::from("/repo/.ai/tickets/T-2.toml"),
        pre: Some(42),
    };
    let _ = q.submit(ship("T-1"));
    assert!(q.submit(ship("T-2").with_guard(guard.clone())).is_none());
    let fin = q.finish(true);
    assert_eq!(
        fin.next.unwrap().guard,
        Some(guard),
        "the pre-dispatch fingerprint rides the queue untouched"
    );
}

// ---- CAS guard ----

#[test]
fn cas_guard_passes_unchanged_and_refuses_changed_or_deleted() {
    let s = Scratch::new("cas");
    let file = s.path().join("T-1.toml");
    fs::write(&file, b"id = \"T-1\"\n").unwrap();
    let g = guard_for(&file);
    assert!(g.pre.is_some());
    assert!(cas_ok(Some(&g)), "unchanged bytes pass");

    // Same length, different bytes — content hash catches what len+mtime miss.
    fs::write(&file, b"id = \"T-2\"\n").unwrap();
    assert!(!cas_ok(Some(&g)), "changed bytes refuse");

    let g2 = guard_for(&file);
    assert!(cas_ok(Some(&g2)));
    fs::remove_file(&file).unwrap();
    assert!(!cas_ok(Some(&g2)), "deleted file refuses");

    assert!(cas_ok(None), "`add` has no target file — no guard");
}

#[test]
fn hash_is_content_sensitive() {
    assert_ne!(hash_bytes(b"aaaa"), hash_bytes(b"aaab"));
    assert_eq!(hash_bytes(b""), 0xcbf2_9ce4_8422_2325);
}

// ---- offered transitions ----

/// The acceptance matrix, pinned literally per status. The exhaustive
/// wildcard-free match in `offered_transitions` (and the STATUS_ORDER walk
/// in `board::column_of`) makes a 9th status a compile error, not a silent
/// fall-through.
#[test]
fn offered_transitions_matrix_pinned() {
    use Transition::*;
    assert_eq!(offered_transitions(StatusName::Idea), vec![QueueAfter]);
    assert_eq!(offered_transitions(StatusName::Queued), vec![MarkReady]);
    assert_eq!(
        offered_transitions(StatusName::Ready),
        vec![Ship, DemoteToQueued, Defer, CancelTicket]
    );
    assert_eq!(
        offered_transitions(StatusName::Review),
        vec![Ship, DemoteToQueued, Defer, CancelTicket],
        "review mirrors ready"
    );
    assert_eq!(
        offered_transitions(StatusName::Running),
        vec![],
        "running is the runner's claim — no manual transitions"
    );
    assert_eq!(
        offered_transitions(StatusName::Shipped),
        vec![ReopenToQueued]
    );
    assert_eq!(
        offered_transitions(StatusName::Deferred),
        vec![ReopenToQueued]
    );
    assert_eq!(
        offered_transitions(StatusName::Cancelled),
        vec![ReopenToQueued]
    );
}

#[test]
fn running_is_never_a_dispatch_target_in_the_normal_ui() {
    for status in board::STATUS_ORDER {
        for t in offered_transitions(status) {
            if let Some(req) = confirm_request(t, "T-1") {
                assert!(
                    !req.args.iter().any(|a| a == "running"),
                    "{t:?} on {status:?} must not target running: {}",
                    req.display
                );
            }
        }
    }
    // The two form transitions carry no status literal at all.
    assert_eq!(confirm_request(Transition::QueueAfter, "T-1"), None);
    assert_eq!(confirm_request(Transition::MarkReady, "T-1"), None);
}

#[test]
fn confirm_requests_map_to_exactly_one_verb_line() {
    let cases = [
        (Transition::Ship, "cargo xtask ticket ship T-9"),
        (
            Transition::DemoteToQueued,
            "cargo xtask ticket set-status T-9 queued",
        ),
        (
            Transition::ReopenToQueued,
            "cargo xtask ticket set-status T-9 queued",
        ),
        (
            Transition::Defer,
            "cargo xtask ticket set-status T-9 deferred",
        ),
        (
            Transition::CancelTicket,
            "cargo xtask ticket set-status T-9 cancelled",
        ),
    ];
    for (t, want) in cases {
        assert_eq!(confirm_request(t, "T-9").unwrap().display, want);
    }
    assert!(confirm_note(Transition::ReopenToQueued).is_some());
    assert!(confirm_note(Transition::Ship).is_none());
}

#[test]
fn advanced_matrix_per_kind_and_status() {
    use AdvancedAction::*;
    // Work tickets: raw set-status + remove; no advance-slice.
    assert_eq!(
        advanced_actions(StatusName::Queued, false),
        vec![RawSetStatus, Remove]
    );
    // Programs add advance-slice.
    assert_eq!(
        advanced_actions(StatusName::Ready, true),
        vec![RawSetStatus, AdvanceSlice, Remove]
    );
    // Running adds the one manual escape hatch — Cancel — behind Advanced.
    assert_eq!(
        advanced_actions(StatusName::Running, false),
        vec![RawSetStatus, Remove, CancelRunning]
    );
    assert_eq!(
        advanced_actions(StatusName::Running, true),
        vec![RawSetStatus, AdvanceSlice, Remove, CancelRunning]
    );
    for status in board::STATUS_ORDER {
        let has_cancel = advanced_actions(status, false).contains(&CancelRunning);
        assert_eq!(
            has_cancel,
            status == StatusName::Running,
            "CancelRunning is running-only"
        );
    }
}

// ---- recovery hint ----

/// The three real wave-stale shapes from xtask (`wave_lock.rs` base drift,
/// wave-0 membership drift, and the missing-lock DidNotRun) all trigger; a
/// generic check ERROR or rustc noise does not.
#[test]
fn recovery_hint_triggers_on_the_wave_stale_signature_only() {
    let stale_base = [
        "ERROR: wave.lock wave_base 131 is stale — the close-marker ledger \
             derives 132: run `cargo xtask wave repack`",
    ];
    assert!(wants_recovery_hint(stale_base));
    let stale_membership = [
        "some earlier line",
        "ERROR: wave.lock wave 0 is stale — missing [\"T-9\"], extra []: run \
             `cargo xtask wave repack`",
    ];
    assert!(wants_recovery_hint(stale_membership));
    let missing_lock = [
        "/repo/.ai/tickets/wave.lock missing — DidNotRun: run `cargo xtask wave \
             repack`. A missing lock is a refusal, never an empty plan.",
    ];
    assert!(wants_recovery_hint(missing_lock));

    let unrelated = [
        "ERROR: T-915.9 status ready without order",
        "error[E0308]: mismatched types",
        "Unknown ticket: T-999",
    ];
    assert!(!wants_recovery_hint(unrelated));
    assert!(
        RECOVERY_HINT.contains(WAVE_STALE_SIGNATURE),
        "the hint shows the same command the refusals name"
    );
}

/// The mid-verb SIGKILL case: a signal-killed verb printed
/// nothing about the lock, but the crash IS the wave-stale hazard — the
/// hint shows. A clean nonzero exit still needs the signature.
#[test]
fn recovery_hint_applies_on_signal_kill_even_with_a_silent_log() {
    assert!(recovery_hint_applies(true, []));
    assert!(recovery_hint_applies(
        true,
        ["   Compiling xtask v0.1.0 (/repo/xtask)"]
    ));
    assert!(!recovery_hint_applies(false, ["Unknown ticket: T-999"]));
    assert!(recovery_hint_applies(
        false,
        [
            "ERROR: wave.lock wave 0 is stale — missing [\"T-9\"], extra []: run `cargo xtask wave repack`"
        ]
    ));
}

#[test]
fn success_tail_skips_cargo_noise_and_blank_lines() {
    let ship_stream = [
        "    Blocking waiting for file lock on build directory",
        "   Compiling xtask v0.1.0 (/repo/xtask)",
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 3m",
        "     Running `target/debug/xtask ticket ship T-905`",
        "T-905 -> shipped",
        "",
    ];
    assert_eq!(
        success_tail(ship_stream.into_iter()),
        Some("T-905 -> shipped".to_owned())
    );
    // set-status is silent on success — only cargo noise on the stream.
    let silent = [
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.1s",
        "     Running `target/debug/xtask ticket set-status T-9 queued`",
    ];
    assert_eq!(success_tail(silent.into_iter()), None);
}

// ---- remove gates ----

#[test]
fn remove_type_to_confirm_gate_requires_the_exact_id() {
    assert!(remove_gate_ok("T-915.4", "T-915.4"));
    assert!(
        remove_gate_ok("  T-915.4  ", "T-915.4"),
        "whitespace trimmed"
    );
    assert!(!remove_gate_ok("t-915.4", "T-915.4"), "case-sensitive");
    assert!(!remove_gate_ok("T-915", "T-915.4"));
    assert!(!remove_gate_ok("T-915.40", "T-915.4"));
    assert!(!remove_gate_ok("", "T-915.4"));
}

#[test]
fn descendants_closure_by_dotted_prefix() {
    let ids = [
        "T-9", "T-9.1", "T-9.10", "T-9.2", "T-9.2.1", "T-90", "T-90.1", "T-8",
    ];
    assert_eq!(
        descendants(ids, "T-9"),
        vec!["T-9.1", "T-9.2", "T-9.2.1", "T-9.10"],
        "numeric sort, dot-boundary (never T-90)"
    );
    assert!(descendants(ids, "T-8").is_empty());
}
