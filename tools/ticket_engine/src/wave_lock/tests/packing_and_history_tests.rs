use super::*;

#[test]
fn overlapping_owns_never_share_a_wave() {
    let dir = scratch(
        "overlap",
        &[
            (
                "T-1.toml",
                &work("T-1", 10, &["crates/foo/a.rs"], &[], "queued"),
            ),
            (
                "T-2.toml",
                &work("T-2", 20, &["crates/foo/a.rs"], &[], "queued"),
            ),
        ],
    );
    let lock = compile(&dir, &BTreeSet::new(), None).unwrap();
    let open: Vec<&LockWave> = lock.waves.iter().filter(|w| w.n > 0).collect();
    assert_eq!(open.len(), 2, "colliding owns must split: {lock:?}");
    assert_eq!(open[0].tickets, vec!["T-1".to_string()]);
    assert_eq!(open[1].tickets, vec!["T-2".to_string()]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn dependent_packs_strictly_after_unshipped_dependency() {
    // T-2 has the LOWER order but depends on T-9 — it must still land strictly later.
    let dir = scratch(
        "deps",
        &[
            ("T-2.toml", &work("T-2", 10, &["a.rs"], &["T-9"], "queued")),
            ("T-9.toml", &work("T-9", 20, &["b.rs"], &[], "queued")),
        ],
    );
    let lock = compile(&dir, &BTreeSet::new(), None).unwrap();
    let wave_of = |id: &str| {
        lock.waves
            .iter()
            .find(|w| w.tickets.iter().any(|t| t == id))
            .map(|w| w.n)
            .unwrap()
    };
    assert!(
        wave_of("T-2") > wave_of("T-9"),
        "dependent same-or-earlier than dependency: {lock:?}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn candidates_sort_by_order_then_id_never_glob_order() {
    // File names sort "-10" before "-9" lexically; order says the single digit first.
    let dir = scratch(
        "order",
        &[
            ("T-10.toml", &work("T-10", 20, &["a.rs"], &[], "queued")),
            ("T-9.toml", &work("T-9", 10, &["b.rs"], &[], "queued")),
            ("T-8.toml", &work("T-8", 20, &["c.rs"], &[], "queued")),
        ],
    );
    let lock = compile(&dir, &BTreeSet::new(), None).unwrap();
    // One wave (disjoint, under cap) — candidates sort by order, then by the numeric id key,
    // so the tie on order 20 puts the single-digit id first.
    assert_eq!(
        lock.tickets_in_wave(1),
        vec!["T-9".to_string(), "T-8".to_string(), "T-10".to_string()]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn compile_render_is_deterministic_and_roundtrips() {
    let dir = scratch(
        "det",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["b/dir"], &["T-1"], "queued")),
            ("T-3.toml", &work("T-3", 30, &["c.rs"], &[], "shipped")),
        ],
    );
    let a = compile(&dir, &BTreeSet::new(), None).unwrap();
    let b = compile(&dir, &BTreeSet::new(), None).unwrap();
    assert_eq!(a, b);
    let ra = render(&a).unwrap();
    assert_eq!(
        ra,
        render(&b).unwrap(),
        "double compile must render identical bytes"
    );
    assert!(ra.starts_with(HEADER));
    assert!(ra.ends_with('\n'));
    let parsed = parse(&ra).unwrap();
    assert_eq!(parsed, a, "render → parse must be lossless");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wave_zero_is_baseline_union_parked_minus_reopened() {
    let dir = scratch(
        "zero",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["b.rs"], &[], "shipped")),
        ],
    );
    let baseline: BTreeSet<String> = ["T-1".to_string(), "T-777".to_string()]
        .into_iter()
        .collect();
    let views = load_views(&dir).unwrap();
    // The reopened id leaves wave 0; the id with no file is kept by the ledger; one stays parked.
    assert_eq!(
        wave_zero(&views, &baseline),
        vec!["T-2".to_string(), "T-777".to_string()]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn wave_zero_orders_a_four_digit_id_after_every_three_digit_id() {
    let dir = scratch(
        "zero-order",
        &[
            (
                "T-1000.toml",
                &work("T-1000", 10, &["a.rs"], &[], "shipped"),
            ),
            ("T-101.toml", &work("T-101", 20, &["b.rs"], &[], "shipped")),
            ("T-100.toml", &work("T-100", 30, &["c.rs"], &[], "shipped")),
        ],
    );
    let views = load_views(&dir).unwrap();
    assert_eq!(
        wave_zero(&views, &BTreeSet::new()),
        vec![
            "T-100".to_string(),
            "T-101".to_string(),
            "T-1000".to_string()
        ]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn missing_lock_is_a_did_not_run_refusal() {
    let dir = scratch("missing", &[]);
    let err = format!("{:#}", load(&dir).unwrap_err());
    assert!(
        err.contains("DidNotRun"),
        "want DidNotRun refusal, got: {err}"
    );
    assert!(err.contains("never an empty plan"), "got: {err}");
    let errs = check_as_errors(&dir);
    assert_eq!(errs.len(), 1);
    assert!(errs[0].contains("DidNotRun"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn check_reds_on_perturbed_owns_and_edges_and_stale_membership() {
    let dir = scratch(
        "perturb",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["b.rs"], &["T-1"], "queued")),
        ],
    );
    let lock = compile(&dir, &BTreeSet::new(), None).unwrap();
    write(&dir, &lock).unwrap();
    assert!(check_as_errors(&dir).is_empty(), "fresh lock must be green");

    // Perturb owns without repack → red.
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-1.toml"),
        work("T-1", 10, &["zzz.rs"], &[], "queued"),
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter().any(|e| e.contains("owns snapshot")),
        "owns perturbation must be red: {errs:?}"
    );
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-1.toml"),
        work("T-1", 10, &["a.rs"], &[], "queued"),
    )
    .unwrap();
    assert!(check_as_errors(&dir).is_empty(), "restore must be green");

    // Remove a depends_on edge without repack → red.
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-2.toml"),
        work("T-2", 20, &["b.rs"], &[], "queued"),
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter().any(|e| e.contains("depends_on snapshot")),
        "edge removal must be red: {errs:?}"
    );
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-2.toml"),
        work("T-2", 20, &["b.rs"], &["T-1"], "queued"),
    )
    .unwrap();
    assert!(check_as_errors(&dir).is_empty());

    // Ship without repack → membership + wave 0 red.
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-1.toml"),
        work("T-1", 10, &["a.rs"], &[], "shipped"),
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter().any(|e| e.contains("wave 0 is stale")),
        "ship without repack must red wave 0: {errs:?}"
    );
    assert!(
        errs.iter().any(|e| e.contains("not dispatchable")),
        "shipped id left in an open wave must be red: {errs:?}"
    );
    // Repack (the ship hook path) → green again, id parked.
    let lock = repack_quiet(&dir).unwrap();
    assert!(check_as_errors(&dir).is_empty());
    assert!(lock.tickets_in_wave(0).contains(&"T-1".to_string()));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reorder_changes_open_waves_only_never_wave_zero() {
    let dir = scratch(
        "reorder",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["a.rs"], &[], "queued")),
            ("T-3.toml", &work("T-3", 30, &["c.rs"], &[], "shipped")),
        ],
    );
    let before = repack_quiet(&dir).unwrap();
    assert_eq!(before.tickets_in_wave(1), vec!["T-1".to_string()]);
    // Swap the two open orders.
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-1.toml"),
        work("T-1", 40, &["a.rs"], &[], "queued"),
    )
    .unwrap();
    let after = repack_quiet(&dir).unwrap();
    assert_eq!(after.tickets_in_wave(1), vec!["T-2".to_string()]);
    assert_eq!(
        before.tickets_in_wave(0),
        after.tickets_in_wave(0),
        "order edits must never move wave 0"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A wave repacked once its whole set has landed freezes that whole set; a wave
/// repacked after EVERY id freezes at most a remnant.
///
/// This is why wave 248 could not be closed on 2026-09-05. `wave --close` closes a pending
/// `[[emptied]]` entry and `carry_emptied` freezes one only when a repack sees a wave whose
/// every ticket has landed — but `ticket ship` repacked per id, and each repack re-packs from
/// scratch, so the wave shrank between ships and no repack ever saw the full set. The repair
/// is `ship --no-repack` plus one repack at the end of the wave (`cmds::cmd_ship_opt`); this
/// pins the lock-side half of it, both directions in one test so neither can rot alone.
#[test]
fn a_wave_freezes_its_whole_set_only_when_repacked_after_the_last_ship() {
    let files = |s1: &str, s2: &str| {
        vec![
            ("T-1.toml".to_string(), work("T-1", 10, &["a.rs"], &[], s1)),
            ("T-2.toml".to_string(), work("T-2", 20, &["b.rs"], &[], s2)),
        ]
    };
    let write = |dir: &Path, rows: &[(String, String)]| {
        for (name, body) in rows {
            fs::write(dir.join(crate::repository::TICKETS_DIR).join(name), body).unwrap();
        }
    };
    let set_of = |lock: &WaveLock| -> Vec<Vec<String>> {
        lock.emptied.iter().map(|e| e.tickets.clone()).collect()
    };

    // Disjoint owns, so the packer puts T-1 and T-2 in ONE wave.
    let seed = files("queued", "queued");
    let rows: Vec<(&str, &str)> = seed.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let dir = scratch_git("t946-carry-per-id", &rows, &["wave 41 CLOSED — prior"]);
    let base = repack_quiet(&dir).unwrap();
    assert_eq!(
        base.waves.iter().find(|w| w.n > 0).map(|w| w.tickets.len()),
        Some(2),
        "the fixture must pack both tickets into one wave: {:?}",
        base.waves
    );

    // Per-id: ship T-1, repack, ship T-2, repack.
    write(&dir, &files("shipped", "queued"));
    repack_quiet(&dir).unwrap();
    write(&dir, &files("shipped", "shipped"));
    let per_id = repack_quiet(&dir).unwrap();
    println!("── per-id repack ── emptied sets = {:?}", set_of(&per_id));
    assert!(
        !set_of(&per_id).iter().any(|t| t.len() == 2),
        "a per-id repack cannot freeze the whole wave — the defect: {:?}",
        set_of(&per_id)
    );

    // Batched: both ship, then ONE repack.
    let dir2 = scratch_git("t946-carry-batch", &rows, &["wave 41 CLOSED — prior"]);
    repack_quiet(&dir2).unwrap();
    write(&dir2, &files("shipped", "shipped"));
    let batched = repack_quiet(&dir2).unwrap();
    println!("── one repack ── emptied sets = {:?}", set_of(&batched));
    assert!(
        set_of(&batched).iter().any(|t| t.len() == 2
            && t.contains(&"T-1".to_string())
            && t.contains(&"T-2".to_string())),
        "one repack after the last ship freezes the wave's whole set: {:?}",
        set_of(&batched)
    );
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&dir2);
}

/// An incidental repack keeps the plan's own width.
///
/// `ticket ship`'s lifecycle hook repacks with no environment, and `max_concurrent()` defaults
/// to 8. Measured 2026-09-06: wave 236 was packed 3 wide, a ship hook re-packed
/// it at 8, and the wave that had just been gated no longer existed in the plan — `wave
/// --close` had nothing to close. An explicit `TBD_MAX_CONCURRENT` still wins.
#[test]
fn an_incidental_repack_keeps_the_locks_own_width() {
    let dir = scratch_git(
        "t946-cap",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["b.rs"], &[], "queued")),
            ("T-3.toml", &work("T-3", 30, &["c.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — prior"],
    );
    // Pack it deliberately narrow, the way the run does — via the explicit cap, never
    // `set_var`: TBD_MAX_CONCURRENT is process state and a sibling test would be re-packed
    // under it (that is the race documented on `compile_with_cap`).
    let narrow = compile_with_cap(&dir, &BTreeSet::new(), None, Some(1)).unwrap();
    write(&dir, &narrow).unwrap();
    println!(
        "── deliberate ── max_concurrent = {}, waves = {:?}",
        narrow.max_concurrent,
        narrow.waves.iter().map(|w| w.n).collect::<Vec<_>>()
    );
    assert_eq!(narrow.max_concurrent, 1);
    let narrow_open: Vec<usize> = narrow
        .waves
        .iter()
        .filter(|w| w.n > 0)
        .map(|w| w.tickets.len())
        .collect();
    assert_eq!(narrow_open, vec![1, 1, 1], "three singleton waves");

    // An incidental repack — no environment, exactly the ship hook's shape.
    let again = repack_quiet(&dir).unwrap();
    println!(
        "── incidental ── max_concurrent = {}, wave sizes = {:?}",
        again.max_concurrent,
        again
            .waves
            .iter()
            .filter(|w| w.n > 0)
            .map(|w| w.tickets.len())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        again.max_concurrent, 1,
        "the hook must not silently widen the plan to the default 8"
    );
    assert_eq!(
        again
            .waves
            .iter()
            .filter(|w| w.n > 0)
            .map(|w| w.tickets.len())
            .collect::<Vec<_>>(),
        vec![1, 1, 1],
        "and the wave membership must not move under a wave that is being run"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The lock numbers from the HIGHEST CLAIM, not merely the newest marker, and a
/// prefixed subject is not a marker at all.
///
/// This is the measured shape of the real ledger on 2026-09-05: a newer marker claiming a
/// LOWER wave than an older one (two programmes closed alongside each other),
/// plus prefixed `<id> wave N CLOSED` subjects that the anchored authority rejects. Before
/// the fix the lock numbered from `newest_close_base` alone while the close ceremony's oracle
/// would accept only `highest claim + 1`, so the two drifted and NO close could be written.
#[test]
fn numbering_seats_on_the_highest_claim_not_the_newest_marker() {
    let dir = scratch_git(
        "t946-claim",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["a.rs"], &[], "queued")),
        ],
        &[
            "wave 41 CLOSED — the highest claim",
            "T-853 wave 45 CLOSED — prefixed, so NOT a marker",
            "wave 40 CLOSED — newer by commit order, lower by claim",
        ],
    );

    let base = crate::wave_lock::history::newest_close_base(&dir).unwrap();
    let claim = crate::wave_lock::history::max_close_claim(&dir).unwrap();
    println!("── newest_close_base = {base:?}   max_close_claim = {claim:?}");
    assert_eq!(base, Some(40), "newest by commit order");
    assert_eq!(
        claim,
        Some(41),
        "highest claim among REAL markers — the prefixed 45 is not one"
    );

    let lock = repack_quiet(&dir).unwrap();
    let open: Vec<u32> = lock.waves.iter().map(|w| w.n).filter(|n| *n > 0).collect();
    println!(
        "── open wave labels ── {open:?} (wave_base = {})",
        lock.wave_base
    );
    assert_eq!(
        open.first().copied(),
        Some(42),
        "the first open wave is highest-claim + 1 = the only label the close oracle accepts"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn repack_continues_the_close_marker_ledger() {
    // The ledger's newest marker says wave 41; an ordinary commit sits at HEAD, so this is
    // the fallback (non-HEAD) derivation path. Colliding owns split T-1/T-2 into two waves.
    let dir = scratch_git(
        "ledger",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["a.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — test ledger", "post-close housekeeping"],
    );
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 41, "base = the newest close marker");
    let open: Vec<u32> = lock.waves.iter().filter(|w| w.n > 0).map(|w| w.n).collect();
    assert_eq!(open, vec![42, 43], "open waves continue base+1: {lock:?}");
    assert_eq!(lock.tickets_in_wave(42), vec!["T-1".to_string()]);
    assert!(
        check_as_errors(&dir).is_empty(),
        "fresh repack must be green"
    );
    let text = std::fs::read_to_string(lock_path(&dir)).unwrap();
    assert!(
        text.contains("\nwave_base = 41\n"),
        "wave_base is always emitted: {text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn shallow_clone_refuses_base_derivation() {
    // A depth-1 clone hides the ledger. Deriving base 0 there would be a wrong answer,
    // not a fallback (red against a full-history lock, silently green if the committed
    // base happens to be 0) — so both repack and check refuse loudly instead.
    let dir = scratch_git(
        "shallow-src",
        &[("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued"))],
        &["wave 41 CLOSED — test ledger", "post-close housekeeping"],
    );
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 41, "full history derives the marker");
    let clone = dir.with_file_name("t914-shallow-clone");
    let _ = fs::remove_dir_all(&clone);
    git_in_dir(
        dir.parent().unwrap(),
        &[
            "clone",
            "-q",
            "--depth",
            "1",
            &format!("file://{}", dir.display()),
            clone.file_name().unwrap().to_str().unwrap(),
        ],
    );
    let err = repack_quiet(&clone).unwrap_err().to_string();
    assert!(err.contains("shallow"), "repack refuses on shallow: {err}");
    fs::copy(lock_path(&dir), lock_path(&clone)).unwrap();
    let errors = check_as_errors(&clone);
    assert!(
        errors.iter().any(|e| e.contains("base derivation refused")),
        "check refuses, never derives 0: {errors:?}"
    );
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_dir_all(&clone);
}

#[test]
fn head_itself_as_the_marker_counts() {
    // The include-HEAD path: the marker IS HEAD — the state every post-close repack runs
    // in, and the one prev_wave_close deliberately refuses to see.
    let dir = scratch_git(
        "ledger-head",
        &[("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued"))],
        &["wave 41 CLOSED — test ledger"],
    );
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 41, "a marker at HEAD is the base");
    assert_eq!(lock.tickets_in_wave(42), vec!["T-1".to_string()]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_disavowed_marker_is_not_a_base() {
    // wave 42's close is reverted — git revert writes the exact trailer the disavowal
    // evidence reads — so the base falls through to wave 41, mirroring prev_wave_close's
    // F6 handling on a root that is not the cwd.
    let dir = scratch_git(
        "ledger-disavow",
        &[("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued"))],
        &[
            "wave 41 CLOSED — test ledger",
            "wave 42 CLOSED — reverted below",
        ],
    );
    git_in_dir(&dir, &["revert", "--no-edit", "HEAD"]);
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 41, "a reverted close is not a boundary");
    assert_eq!(lock.tickets_in_wave(42), vec!["T-1".to_string()]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn stale_base_reds_check_until_repack() {
    let dir = scratch_git(
        "ledger-stale",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["a.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — test ledger"],
    );
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 41);
    assert!(check_as_errors(&dir).is_empty());

    // A new close marker lands without a repack → the committed base is stale → red,
    // naming the fix. This is the close → check-red → repack loop.
    fs::write(dir.join("c-next.txt"), "x\n").unwrap();
    git_in_dir(&dir, &["add", "--", "c-next.txt"]);
    git_in_dir(
        &dir,
        &["commit", "-q", "-m", "wave 42 CLOSED — test ledger"],
    );
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter()
            .any(|e| e.contains("wave_base 41") && e.contains("wave repack")),
        "a stale base must be red and name the repack: {errs:?}"
    );

    // Repack renumbers from the new base → green, open waves from 43.
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 42);
    assert_eq!(lock.tickets_in_wave(43), vec!["T-1".to_string()]);
    assert!(check_as_errors(&dir).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn no_marker_tree_keeps_base_zero_and_waves_from_one() {
    let dir = scratch_git(
        "ledger-none",
        &[("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued"))],
        &["an ordinary commit, no marker anywhere"],
    );
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 0, "no marker → base 0");
    assert_eq!(
        lock.tickets_in_wave(1),
        vec!["T-1".to_string()],
        "open waves stay 1..N on a markerless tree"
    );
    assert!(check_as_errors(&dir).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn lock_without_a_wave_base_parses_as_zero() {
    // The serde default that keeps lock blobs without the key parsing: the ones
    // `wave_plan_tickets_at` reads at historical revisions, and the raw-TOML stubs in the mod
    // wave tests.
    let text =
        "version = 1\nmax_concurrent = 8\npack_last = []\nwaves = []\n\n[owns]\n\n[depends_on]\n";
    let lock = parse(text).unwrap();
    assert_eq!(lock.wave_base, 0);
    assert!(
        render(&lock).unwrap().contains("\nwave_base = 0\n"),
        "the render emits wave_base even at 0"
    );
}

#[test]
fn full_ship_freezes_the_wave_into_emptied_and_open_waves_number_past_it() {
    // Colliding owns split T-1/T-2 into waves 42 and 43 over ledger base 41.
    let dir = scratch_git(
        "emptied-full",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["a.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — test ledger"],
    );
    let before = repack_quiet(&dir).unwrap();
    assert_eq!(before.tickets_in_wave(42), vec!["T-1".to_string()]);
    assert!(before.emptied.is_empty(), "nothing emptied yet: {before:?}");

    // Ship ALL of wave 42 (its whole set is T-1), then the ship-hook repack.
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-1.toml"),
        work("T-1", 10, &["a.rs"], &[], "shipped"),
    )
    .unwrap();
    let lock = repack_quiet(&dir).unwrap();
    assert_eq!(lock.wave_base, 41, "no marker landed — base unchanged");
    assert_eq!(
        lock.emptied,
        vec![LockWave {
            n: 42,
            tickets: vec!["T-1".to_string()]
        }],
        "the dissolved wave froze: {lock:?}"
    );
    // Open waves number PAST the pending label: max(wave_base 41, pending 42) + 1.
    let open: Vec<u32> = lock.waves.iter().filter(|w| w.n > 0).map(|w| w.n).collect();
    assert_eq!(open, vec![43], "{lock:?}");
    assert_eq!(lock.tickets_in_wave(43), vec!["T-2".to_string()]);
    assert!(
        check_as_errors(&dir).is_empty(),
        "recorded state must be green"
    );
    let text = fs::read_to_string(lock_path(&dir)).unwrap();
    println!("── [[emptied]] block after the full-ship repack ──");
    println!("{}", emptied_slice(&text));
    println!("── renumbered open wave ──");
    for w in lock.waves.iter().filter(|w| w.n > 0) {
        println!("wave {} = {:?}", w.n, w.tickets);
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn partial_ship_records_no_emptied_entry() {
    // Disjoint owns → ONE wave holding both tickets. Shipping only half of it must leave
    // no trace: an open wave records nothing until its WHOLE set has landed.
    let dir = scratch_git(
        "emptied-partial",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["b.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — test ledger"],
    );
    let before = repack_quiet(&dir).unwrap();
    assert_eq!(
        before.tickets_in_wave(42),
        vec!["T-1".to_string(), "T-2".to_string()]
    );
    fs::write(
        dir.join(crate::repository::TICKETS_DIR).join("T-1.toml"),
        work("T-1", 10, &["a.rs"], &[], "shipped"),
    )
    .unwrap();
    let lock = repack_quiet(&dir).unwrap();
    assert!(
        lock.emptied.is_empty(),
        "partial ship froze a wave: {lock:?}"
    );
    assert_eq!(lock.tickets_in_wave(42), vec!["T-2".to_string()]);
    assert!(check_as_errors(&dir).is_empty());
    let text = fs::read_to_string(lock_path(&dir)).unwrap();
    assert!(
        !text.contains("emptied"),
        "the empty section renders NOTHING: {text}"
    );
    println!(
        "── grep emptied on the partial-ship lock ── {} match(es)",
        text.matches("emptied").count()
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_wave_dissolved_id_by_id_leaves_no_entry_and_its_label_is_reissued() {
    // The defect itself, pinned before the repair: this is why `--close --tickets` refuses.
    let dir = dissolved_by_id("reserve-defect");
    let lock = load(&dir).unwrap();
    assert!(
        lock.emptied.is_empty(),
        "nothing froze — the carry never saw the whole set under one label: {lock:?}"
    );
    assert_eq!(
        lock.tickets_in_wave(42),
        vec!["T-3".to_string()],
        "and 42 — the label the close will ask for — now names UNSHIPPED work: {lock:?}"
    );
    println!(
        "── dissolved: emptied = {:?}, wave 42 = {:?}",
        lock.emptied,
        lock.tickets_in_wave(42)
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reserving_freezes_the_lost_set_and_the_open_wave_numbers_past_it() {
    let dir = dissolved_by_id("reserve-repair");
    let ids = vec!["T-1".to_string(), "T-2".to_string()];
    let lock = repack_reserving(&dir, &ids).unwrap();
    assert_eq!(
        lock.emptied,
        vec![LockWave {
            n: 42,
            tickets: ids.clone()
        }],
        "the vouched set holds the label the ceremony will claim: {lock:?}"
    );
    assert_eq!(
        lock.tickets_in_wave(42),
        Vec::<String>::new(),
        "42 is no longer an OPEN wave, so the close-label collision is gone"
    );
    assert_eq!(
        lock.tickets_in_wave(43),
        vec!["T-3".to_string()],
        "the live ticket moved past the reserved label: {lock:?}"
    );
    assert!(
        check_as_errors(&dir).is_empty(),
        "the reserved lock must be RECORDED-green, not just written: {:?}",
        check_as_errors(&dir)
    );
    // Self-sustaining: the derived carry keeps a pending entry above the base, so an ordinary
    // repack (every `ticket ship` runs one) must not undo the reservation.
    let again = repack_quiet(&dir).unwrap();
    assert_eq!(
        again.emptied, lock.emptied,
        "a plain repack dropped it: {again:?}"
    );
    let text = fs::read_to_string(lock_path(&dir)).unwrap();
    println!("── [[emptied]] after --reserve, then after a plain repack ──");
    println!("{}", emptied_slice(&text));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn reserving_refuses_a_live_ticket_and_an_unknown_id() {
    // MEMBERSHIP is vouched for; STATUS never is. T-3 is live and must be refused, or the
    // reservation would record a close target the ceremony refuses a step later.
    let dir = dissolved_by_id("reserve-refusals");
    for (ids, want) in [
        (vec!["T-1".to_string(), "T-3".to_string()], "not shipped"),
        (vec!["T-9".to_string()], "not a ticket in this tree"),
        (vec!["T-1".to_string(), "T-1".to_string()], "named twice"),
    ] {
        let err = format!(
            "{:#}",
            repack_reserving(&dir, &ids).expect_err("must refuse")
        );
        println!("── reserve {ids:?} ── {err}");
        assert!(err.contains(want), "wrong refusal for {ids:?}: {err}");
    }
    assert!(
        load(&dir).unwrap().emptied.is_empty(),
        "a refused reservation must not have written anything"
    );
    let _ = fs::remove_dir_all(&dir);
}
