use super::*;

#[test]
fn set_status_refuses_empty_without_write() {
    // Class-R: empty status must not overwrite a live registry field.
    let root = worktree_root();
    let registry_path = root.join(crate::repository::TICKETS_DIR).join("T-001.toml");
    let before = fs::read_to_string(&registry_path).expect("read registry before");
    let mut registry = load_registry(&root).expect("load tip registry");
    let status_before = opt_str(require_ticket(&registry, "T-001"), "status")
        .unwrap_or("")
        .to_string();

    let err = cmd_set_status(&root, &mut registry, "T-001", "")
        .expect_err("set-status must refuse empty status");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing empty write") || msg.contains("non-empty"),
        "expected empty refuse, got: {msg}"
    );

    let status_after = opt_str(require_ticket(&registry, "T-001"), "status")
        .unwrap_or("")
        .to_string();
    assert_eq!(
        status_before, status_after,
        "empty set-status must not mutate in-memory status"
    );
    let after = fs::read_to_string(&registry_path).expect("read registry after");
    assert_eq!(before, after, "empty set-status must not write T-001.toml");
}

#[test]
fn set_status_refuses_invalid_enum_without_write() {
    // Class-R: invalid enum must not overwrite a live registry field.
    let root = worktree_root();
    let registry_path = root.join(crate::repository::TICKETS_DIR).join("T-001.toml");
    let before = fs::read_to_string(&registry_path).expect("read registry before");
    let mut registry = load_registry(&root).expect("load tip registry");

    let err = cmd_set_status(&root, &mut registry, "T-001", "not-a-real-status")
        .expect_err("set-status must refuse invalid enum");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("invalid status") && msg.contains("not-a-real-status"),
        "expected invalid-enum refuse, got: {msg}"
    );

    let after = fs::read_to_string(&registry_path).expect("read registry after");
    assert_eq!(
        before, after,
        "invalid set-status must not write T-001.toml"
    );
}

#[test]
fn set_status_refuses_invalid_registry_without_write() {
    let root = worktree_root();
    let registry_path = root.join(crate::repository::TICKETS_DIR).join("T-001.toml");
    let before = fs::read_to_string(&registry_path).expect("read registry before");
    let mut registry = red_registry(&root);

    // Preflight must fail before any mutator body runs — if this Err is missing,
    // cmd_set_status would write the live tip (see the perturbation below).
    let preflight = require_check_ok(&root, &registry, "set-status T-001");
    assert!(
        preflight.is_err(),
        "preflight must be red before calling cmd_set_status"
    );

    let err = cmd_set_status(&root, &mut registry, "T-001", "shipped")
        .expect_err("set-status must refuse a schema-red registry");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing set-status T-001"),
        "expected refuse message, got: {msg}"
    );
    assert!(
        msg.contains("ticket check failed"),
        "expected check-failed note, got: {msg}"
    );

    let after = fs::read_to_string(&registry_path).expect("read registry after");
    assert_eq!(
        before, after,
        "set-status must not write T-001.toml when check is red"
    );
}

#[test]
fn mark_ready_refuses_invalid_registry() {
    let root = worktree_root();
    let mut registry = red_registry(&root);
    let err = cmd_mark_ready(&root, &mut registry, "T-001", None, None)
        .expect_err("mark-ready must refuse a schema-red registry");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing mark-ready T-001"),
        "expected refuse message, got: {msg}"
    );
}

#[test]
fn add_refuses_invalid_registry_without_write() {
    let root = worktree_root();
    let registry_path = root.join(crate::repository::TICKETS_DIR).join("T-001.toml");
    let before = fs::read_to_string(&registry_path).expect("read registry before");
    let mut registry = red_registry(&root);
    let next_before = registry
        .get("next_id")
        .and_then(|n| n.as_u64())
        .expect("next_id");
    let tickets_before = tickets(&registry).len();

    let err = cmd_add(
        &root,
        &mut registry,
        "should-not-land",
        "platform",
        "xtask",
        "ops",
        "",
    )
    .expect_err("add must refuse a schema-red registry");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing add"),
        "expected refuse message, got: {msg}"
    );
    assert!(
        msg.contains("ticket check failed"),
        "expected check-failed note, got: {msg}"
    );

    let next_after = registry
        .get("next_id")
        .and_then(|n| n.as_u64())
        .expect("next_id");
    assert_eq!(
        next_before, next_after,
        "add must not bump next_id when check is red"
    );
    assert_eq!(
        tickets_before,
        tickets(&registry).len(),
        "add must not push a row in-memory when check is red"
    );

    let after = fs::read_to_string(&registry_path).expect("read registry after");
    assert_eq!(
        before, after,
        "add must not write T-001.toml when check is red"
    );
}

#[test]
fn remove_refuses_invalid_registry_without_write() {
    let root = worktree_root();
    let registry_path = root.join(crate::repository::TICKETS_DIR).join("T-001.toml");
    let before = fs::read_to_string(&registry_path).expect("read registry before");
    let mut registry = red_registry(&root);
    let tickets_before = tickets(&registry).len();

    let err = cmd_remove(&root, &mut registry, "T-001", false)
        .expect_err("remove must refuse a schema-red registry");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing remove T-001"),
        "expected refuse message, got: {msg}"
    );
    assert!(
        msg.contains("ticket check failed"),
        "expected check-failed note, got: {msg}"
    );
    assert_eq!(
        tickets_before,
        tickets(&registry).len(),
        "remove must not drop a row in-memory when check is red"
    );

    let after = fs::read_to_string(&registry_path).expect("read registry after");
    assert_eq!(
        before, after,
        "remove must not write T-001.toml when check is red"
    );
}

#[test]
fn advance_slice_refuses_invalid_registry_without_write() {
    // Advance-slice must share the add/remove preflight — red registry
    // never mutates active_slice in-memory or on disk.
    let root = worktree_root();
    let registry_path = root.join(crate::repository::TICKETS_DIR).join("T-001.toml");
    let before = fs::read_to_string(&registry_path).expect("read registry before");
    let mut registry = red_registry(&root);

    let active_before =
        opt_str(require_ticket(&registry, "T-090"), "active_slice").map(|s| s.to_string());

    let err = cmd_advance_slice(&root, &mut registry, "T-090")
        .expect_err("advance-slice must refuse a schema-red registry");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("refusing advance-slice T-090"),
        "expected refuse message, got: {msg}"
    );
    assert!(
        msg.contains("ticket check failed"),
        "expected check-failed note, got: {msg}"
    );

    let active_after =
        opt_str(require_ticket(&registry, "T-090"), "active_slice").map(|s| s.to_string());
    assert_eq!(
        active_before, active_after,
        "advance-slice must not mutate active_slice in-memory when check is red"
    );

    let after = fs::read_to_string(&registry_path).expect("read registry after");
    assert_eq!(
        before, after,
        "advance-slice must not write T-001.toml when check is red"
    );
}

#[test]
fn require_check_ok_err_matches_set_status_gate() {
    // Same gate surface ship/set-status/mark-ready/add/remove/advance-slice share

    let root = worktree_root();
    let registry = red_registry(&root);
    let err = require_check_ok(&root, &registry, "set-status T-001")
        .expect_err("red registry must fail require_check_ok");
    assert!(format!("{err:#}").contains("refusing set-status T-001"));
}

/// The reload-before-sync invariant, pinned (t915 design §Write path,
/// "Rewiring sequence invariant"). The typed op writes files FIRST; if cmd_ship then fed
/// the pre-mutation Value to cmd_sync, queue.json and the generated docs would still call
/// The regenerated outputs must reflect the POST-state.
/// `--no-repack` leaves the committed lock untouched, and the later repack picks the
/// change up.
///
/// Shipping repacked after EVERY id, which is why a wave could never empty:
/// `wave_lock::carry_emptied` freezes a pending `[[emptied]]` entry only when one repack sees
/// a wave whose every ticket has landed, and a per-id repack re-packs the wave smaller before
/// the next ship runs (see `two_emptied_waves_pend_ascending_and_open_waves_number_past_both`
/// in `wave_lock`). The command center now ships the wave's ids — each still followed by its
/// own `stamp-sha`, the lifecycle is unchanged —
/// and repacks ONCE at the end, where the whole set is visible at the same instant.
/// The stale lock `--no-repack` leaves must not refuse the NEXT ship, and
/// nothing else may be waived with it.
///
/// The first production run of the batch path failed exactly here: ship's preflight is
/// `require_check_ok`, so the second ship refused over the lock staleness that `--no-repack`
/// had just deliberately created —
/// `ERROR: wave.lock wave 0 is stale — missing [<id>] … refusing ship <id>`. The waiver
/// drops only errors whose own text names a repack as the fix.
#[test]
fn the_stale_lock_left_by_no_repack_is_waived_for_the_next_ship_and_nothing_else_is() {
    let root = scratch_registry("ship-batch-waiver");
    let mut registry = load_registry(&root).expect("scratch registry loads");
    crate::wave_lock::repack_quiet(&root).expect("baseline lock");

    cmd_ship_opt(&root, &mut registry, "T-002", false).expect("batch ship");
    let registry = load_registry(&root).expect("reload after ship");

    // The lock now lags the tickets, exactly as --no-repack announced.
    let plain = require_check_ok(&root, &registry, "ship NEXT")
        .expect_err("a stale lock must still refuse the ordinary path");
    let plain = format!("{plain:#}");
    println!("── plain require_check_ok ── {plain}");

    let deferred =
        crate::validation::require_check_ok_deferring_repack(&root, &registry, "ship NEXT");
    let raw = crate::validation::check(&root, &registry, false);
    let lock: Vec<&String> = raw
        .iter()
        .filter(|e| e.contains("run `cargo xtask wave repack`"))
        .collect();
    let other: Vec<&String> = raw
        .iter()
        .filter(|e| !e.contains("run `cargo xtask wave repack`"))
        .collect();
    println!(
        "── repack-fixable ── {}\n── everything else ── {}",
        lock.len(),
        other.len()
    );
    assert!(
        lock.len() == 2 && lock.iter().all(|e| e.contains("wave.lock")),
        "the fixture must carry the stale-lock pair --no-repack creates: {raw:?}"
    );
    assert!(
        !other.is_empty(),
        "and at least one error the waiver must NOT touch — here the unstamped ship: {raw:?}"
    );
    // The waiver is exactly the difference between the two counts: nothing more, nothing less.
    let deferred_msg = format!(
        "{:#}",
        deferred.expect_err("the unstamped ship still refuses")
    );
    println!("── deferring ── {deferred_msg}");
    assert!(
        deferred_msg.contains(&format!("({} error(s))", other.len())),
        "deferring must drop the {} lock error(s) and keep the other {}: {deferred_msg}",
        lock.len(),
        other.len()
    );
    assert!(
        plain.contains(&format!("({} error(s))", raw.len())),
        "and the ordinary path still counts them all: {plain}"
    );
    let _ = fs::remove_dir_all(&root);
}

/// The batch waiver must NOT swallow the missing-lock refusal.
///
/// `wave_lock::missing_lock_error` carries the same ``run `cargo xtask wave repack` `` phrase
/// the waiver keys on, and `check_as_errors` returns it ALONE — every other lock check is
/// skipped behind it. Waiving it would let `ship --no-repack` write ticket status with no plan
/// on disk at all. Found by the wave 236 adversarial verifier, 2026-09-06.
#[test]
fn the_batch_waiver_never_swallows_a_missing_lock() {
    let root = scratch_registry("ship-batch-nolock");
    let registry = load_registry(&root).expect("scratch registry loads");
    crate::wave_lock::repack_quiet(&root).expect("baseline lock");
    let lock_path = root.join(crate::repository::WAVE_LOCK);
    fs::remove_file(&lock_path).expect("remove the lock");

    let raw = crate::validation::check(&root, &registry, false);
    println!("── with no lock ── {raw:?}");
    assert!(
        raw.iter().any(|e| e.contains("DidNotRun")),
        "the fixture must actually produce the DidNotRun refusal: {raw:?}"
    );
    let err = crate::validation::require_check_ok_deferring_repack(&root, &registry, "ship NEXT")
        .expect_err("a missing lock must refuse even inside the batch window");
    println!("── deferring ── {err:#}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ship_no_repack_leaves_the_lock_untouched_until_the_next_repack() {
    let root = scratch_registry("ship-no-repack");
    let mut registry = load_registry(&root).expect("scratch registry loads");
    crate::wave_lock::repack_quiet(&root).expect("baseline lock");
    let lock_path = root.join(crate::repository::WAVE_LOCK);
    let before = fs::read_to_string(&lock_path).expect("lock on disk");
    assert!(
        before.contains("\"T-002\""),
        "pre-state: T-002 is packed in an open wave"
    );

    cmd_ship_opt(&root, &mut registry, "T-002", false).expect("ship --no-repack");
    let after_ship = fs::read_to_string(&lock_path).expect("lock on disk");
    println!(
        "── lock bytes ── before={} after --no-repack={}",
        before.len(),
        after_ship.len()
    );
    assert_eq!(
        before, after_ship,
        "--no-repack must not write the lock at all"
    );

    crate::wave_lock::repack_quiet(&root).expect("the end-of-wave repack");
    let after_repack = fs::read_to_string(&lock_path).expect("lock on disk");
    println!("── lock bytes ── after repack={}", after_repack.len());
    assert_ne!(
        after_ship, after_repack,
        "the deferred repack still parks the shipped ticket — the refresh moved, not vanished"
    );
    let wave0 = crate::wave_lock::load(&root)
        .expect("lock")
        .waves
        .iter()
        .find(|w| w.n == 0)
        .map(|w| w.tickets.clone())
        .unwrap_or_default();
    assert!(
        wave0.contains(&"T-002".to_string()),
        "T-002 parked at wave 0 by the deferred repack: {wave0:?}"
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ship_regenerates_queue_from_post_state_reload_pin() {
    let root = scratch_registry("reload-pin");
    let mut registry = load_registry(&root).expect("scratch registry loads");
    assert!(
        queue_rows(&root).iter().any(|(id, _)| id == "T-002"),
        "pre-state: ready T-002 sits in queue.json"
    );

    cmd_ship(&root, &mut registry, "T-002").expect("ship on scratch");

    // The in-memory Value was reloaded from disk (not hand-patched):
    assert_eq!(
        opt_str(require_ticket(&registry, "T-002"), "status"),
        Some("shipped")
    );
    assert!(
        !queue_rows(&root).iter().any(|(id, _)| id == "T-002"),
        "queue.json regenerated from the POST-state must drop the shipped ticket"
    );
    // The ship hook repacked: the shipped parent is parked at wave 0, out of the open waves.
    let lock = crate::wave_lock::load(&root).expect("lock");
    assert!(lock.tickets_in_wave(0).contains(&"T-002".to_string()));
    assert!(!lock.open_ids().contains(&"T-002".to_string()));
    let _ = fs::remove_dir_all(&root);
}

/// Acceptance 3 — dotted child ship end-to-end through the typed verb. The
/// A parents-only view refuses this exact invocation with "Unknown ticket" for the child id
/// (`require_ticket` walked the parents-only Value; children were shipped by hand TOML
/// edit + repack — the documented hole this program closes).
#[test]
fn child_ship_end_to_end_typed_path() {
    let root = scratch_registry("child-ship");
    let mut registry = load_registry(&root).expect("scratch registry loads");
    // Pre-state: the program's queue row carries the ACTIVE CHILD's spec via slice_plan.
    assert!(
        queue_rows(&root)
            .iter()
            .any(|(id, spec)| id == "T-001" && spec == "docs/child-spec.md"),
        "pre-state: active slice spec reaches queue.json"
    );

    cmd_ship(&root, &mut registry, "T-001.1").expect("dotted child ship must resolve");

    // Child file flipped: shipped, completed_at stamped, order preserved, no SHA invented.
    match parse_scratch_ticket(&root, "T-001.1") {
        Ticket::Work(w) => {
            assert_eq!(
                w.status,
                crate::Status::Shipped {
                    shipped_at: None,
                    order: Some(20)
                }
            );
            assert!(w.completed_at.is_some(), "completed_at stamped");
        }
        Ticket::Program(_) => panic!("T-001.1 must stay work"),
    }
    // Parent active cleared because it named the shipped child.
    match parse_scratch_ticket(&root, "T-001") {
        Ticket::Program(p) => assert_eq!(p.active, None, "stale active cleared"),
        Ticket::Work(_) => panic!("T-001 must stay program"),
    }
    // Repack ran (the ship hook): the lock parks the child id.
    let lock = crate::wave_lock::load(&root).expect("lock");
    assert!(lock.tickets_in_wave(0).contains(&"T-001.1".to_string()));
    assert!(!lock.open_ids().contains(&"T-001.1".to_string()));
    // queue.json regenerated post-state THROUGH the reload: with the active slice gone,
    // The program's row falls back to its own spec. A stale pre-mutation Value would
    // still print docs/child-spec.md here.
    assert!(
        queue_rows(&root)
            .iter()
            .any(|(id, spec)| id == "T-001" && spec == "docs/spec.md"),
        "post-state: {:?}",
        queue_rows(&root)
    );

    // Lifecycle: between ship and stamp-sha the tree is transiently
    // gate-red (shipped_at + tokens missing), so the NEXT check-gated verb must
    // be preceded by the stamp — exactly the documented flow. Injected inputs:
    // no subject commits, the landing sha carries 12 included LOC.
    let sha_loc: std::collections::BTreeMap<String, u64> =
        [("deadbeef1234".to_string(), 12)].into_iter().collect();
    let lines = stamp_sha_with_inputs(
        &root,
        "T-001.1",
        "deadbeef1234",
        &std::collections::BTreeMap::new(),
        &sha_loc,
        "2026-08-15T10:00:00Z",
    )
    .expect("stamp closes the ship");
    assert!(
        lines.iter().any(|l| l.contains("shipped_at -> ")),
        "{lines:?}"
    );

    // Child set-status rides the same wiring: defer the idea sibling by dotted id.
    cmd_set_status(&root, &mut registry, "T-001.2", "deferred").expect("child set-status");
    match parse_scratch_ticket(&root, "T-001.2") {
        Ticket::Work(w) => {
            assert_eq!(
                w.status,
                crate::Status::Deferred { order: None },
                "idea child deferred (order-less)"
            );
        }
        Ticket::Program(_) => panic!("T-001.2 must stay work"),
    }
    let lock = crate::wave_lock::load(&root).expect("lock after set-status repack");
    assert!(
        lock.tickets_in_wave(0).contains(&"T-001.2".to_string()),
        "set-status repacked the deferred child into wave 0"
    );
    let _ = fs::remove_dir_all(&root);
}

/// Acceptance — the scratch end-to-end ship cycle: `ship` (stamps
/// completed_at; tree transiently gate-red) → "commit" (a fake landing sha) →
/// `stamp-sha` → ship gate GREEN with the auto-estimate written and its factor
/// matching the documented constant. Then the idempotence contract: re-stamp of
/// the same sha is a no-op, a different sha refuses, a non-shipped ticket and a
/// garbage sha refuse (op-level, corpus untouched — pinned in ops tests too).
#[test]
fn stamp_sha_end_to_end_scratch_cycle() {
    use std::collections::BTreeMap;
    let root = scratch_registry("stamp-sha-e2e");
    let mut registry = load_registry(&root).expect("scratch registry loads");
    let now = "2026-08-15T10:00:00Z";
    let subjects: BTreeMap<String, Vec<crate::cli::commit_subjects::SubjectCommit>> =
        BTreeMap::new();
    let sha_loc: BTreeMap<String, u64> = [("beefbeef00".to_string(), 20)].into_iter().collect();

    cmd_ship(&root, &mut registry, "T-002").expect("ship");
    // Between ship and stamp the gate is red BY DESIGN, naming both holes.
    let errs = crate::validation::check(&root, &registry, false);
    assert!(
        errs.iter()
            .any(|e| e.contains("T-002") && e.contains("without shipped_at")),
        "{errs:?}"
    );
    assert!(
        errs.iter()
            .any(|e| e.contains("T-002") && e.contains("no token accounting")),
        "{errs:?}"
    );

    let lines = stamp_sha_with_inputs(&root, "T-002", "beefbeef00", &subjects, &sha_loc, now)
        .expect("stamp-sha closes the ship");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("shipped_at -> \"beefbeef00\"")),
        "{lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("diff_loc estimate written") && l.contains("20 LOC")),
        "{lines:?}"
    );
    match parse_scratch_ticket(&root, "T-002") {
        Ticket::Work(w) => {
            assert_eq!(w.shipped_at.as_deref(), Some("beefbeef00"));
            assert!(w.estimated.iter().any(|e| e == "tokens"));
        }
        Ticket::Program(_) => panic!("work"),
    }
    let est: crate::metrics::estimates::EstimateRecord = serde_json::from_str(
        &fs::read_to_string(
            root.join(crate::repository::ESTIMATES_DIR)
                .join("T-002.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(est.source, "diff_loc");
    assert_eq!(
        est.factor,
        crate::metrics::estimates::TOKENS_PER_LOC,
        "factor matches the documented constant"
    );
    assert_eq!(
        est.tokens_estimated,
        20 * crate::metrics::estimates::TOKENS_PER_LOC
    );
    // Gate green: the full check has no finding left on the shipped parent (the fixture itself
    // stays green on every other rule).
    let errs = crate::validation::check(&root, &registry, false);
    assert!(
        errs.is_empty(),
        "gate must be green after stamp-sha:\n{}",
        errs.join("\n")
    );

    // Re-stamp same sha: no-op (bytes untouched), estimate untouched.
    let before =
        fs::read_to_string(root.join(crate::repository::TICKETS_DIR).join("T-002.toml")).unwrap();
    let lines = stamp_sha_with_inputs(&root, "T-002", "beefbeef00", &subjects, &sha_loc, now)
        .expect("re-stamp same sha");
    assert!(lines.iter().any(|l| l.contains("no-op")), "{lines:?}");
    assert_eq!(
        fs::read_to_string(root.join(crate::repository::TICKETS_DIR).join("T-002.toml")).unwrap(),
        before,
        "no-op must not rewrite the ticket"
    );
    // Different sha refuses; non-shipped refuses; garbage refuses.
    let err = stamp_sha_with_inputs(&root, "T-002", "0000000a", &subjects, &sha_loc, now)
        .expect_err("different sha");
    assert!(format!("{err:#}").contains("never overwritten"), "{err:#}");
    let err = stamp_sha_with_inputs(&root, "T-001.1", "beefbeef00", &subjects, &sha_loc, now)
        .expect_err("ready ticket refuses");
    assert!(format!("{err:#}").contains("not shipped"), "{err:#}");
    let err = stamp_sha_with_inputs(&root, "T-002", "not-a-sha", &subjects, &sha_loc, now)
        .expect_err("garbage sha");
    assert!(format!("{err:#}").contains("lowercase hex"), "{err:#}");
    let _ = fs::remove_dir_all(&root);
}
