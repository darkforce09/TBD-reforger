use super::*;

#[test]
fn two_emptied_waves_pend_ascending_and_open_waves_number_past_both() {
    // Three colliding tickets → waves 42/43/44 over base 41. Ship 42's whole set, repack;
    // ship 43's whole set, repack: two pending entries, ascending, and the surviving open
    // wave keeps numbering past the HIGHEST pending label.
    let dir = scratch_git(
        "emptied-two",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["a.rs"], &[], "queued")),
            ("T-3.toml", &work("T-3", 30, &["a.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — test ledger"],
    );
    let first = repack_quiet(&dir).unwrap();
    assert_eq!(first.tickets_in_wave(42), vec!["T-1".to_string()]);
    assert_eq!(first.tickets_in_wave(44), vec!["T-3".to_string()]);

    fs::write(
        dir.join(".ai/tickets/T-1.toml"),
        work("T-1", 10, &["a.rs"], &[], "shipped"),
    )
    .unwrap();
    let mid = repack_quiet(&dir).unwrap();
    assert_eq!(mid.emptied.len(), 1);
    assert_eq!(mid.emptied[0].n, 42);

    fs::write(
        dir.join(".ai/tickets/T-2.toml"),
        work("T-2", 20, &["a.rs"], &[], "shipped"),
    )
    .unwrap();
    let lock = repack_quiet(&dir).unwrap();
    let pending: Vec<(u32, Vec<String>)> = lock
        .emptied
        .iter()
        .map(|e| (e.n, e.tickets.clone()))
        .collect();
    assert_eq!(
        pending,
        vec![(42, vec!["T-1".to_string()]), (43, vec!["T-2".to_string()]),],
        "entries pend ascending with their frozen sets: {lock:?}"
    );
    let open: Vec<u32> = lock.waves.iter().filter(|w| w.n > 0).map(|w| w.n).collect();
    assert_eq!(open, vec![44], "numbering starts past pending 43: {lock:?}");
    assert_eq!(lock.tickets_in_wave(44), vec!["T-3".to_string()]);
    assert!(check_as_errors(&dir).is_empty());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn lock_without_an_emptied_section_parses_and_renders_without_one() {
    // A lock blob that predates the section still parses through the serde default, and an EMPTY
    // section must not touch the render at all — `emptied = []` after the `[[waves]]` tables
    // would not even be valid TOML, and the committed lock must repack byte-identically.
    let text =
        "version = 1\nmax_concurrent = 8\npack_last = []\nwaves = []\n\n[owns]\n\n[depends_on]\n";
    let lock = parse(text).unwrap();
    assert!(lock.emptied.is_empty(), "absent section parses as empty");
    let rendered = render(&lock).unwrap();
    assert!(
        !rendered.contains("emptied"),
        "an empty section renders NOTHING: {rendered}"
    );
    // And a nonempty section round-trips losslessly.
    let mut with = lock.clone();
    with.emptied.push(LockWave {
        n: 7,
        tickets: vec!["T-9".to_string()],
    });
    let r2 = render(&with).unwrap();
    assert!(r2.contains("[[emptied]]"), "{r2}");
    assert_eq!(parse(&r2).unwrap(), with, "render → parse must be lossless");
}

#[test]
fn check_reds_on_a_perturbed_emptied_entry_until_restored() {
    // The recorded single-pending state from the full-ship test, then three hand-edits to
    // the committed section — each red NAMING the entry, each restored to green. The
    // frozen SET is not recomputable from the tree (that is the point of freezing), so
    // the catchable perturbations are the invariant ones: an emptied set, a set pointing
    // at unshipped work, a label the ledger already closed.
    let dir = scratch_git(
        "emptied-perturb",
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["a.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — test ledger"],
    );
    repack_quiet(&dir).unwrap();
    fs::write(
        dir.join(".ai/tickets/T-1.toml"),
        work("T-1", 10, &["a.rs"], &[], "shipped"),
    )
    .unwrap();
    repack_quiet(&dir).unwrap();
    let good = fs::read_to_string(lock_path(&dir)).unwrap();
    assert!(check_as_errors(&dir).is_empty(), "recorded state is green");
    let block = "[[emptied]]\nn = 42\ntickets = [\"T-1\"]";
    assert!(good.contains(block), "fixture block drifted: {good}");

    // (a) drop the ticket from the frozen set → the set empties → red.
    fs::write(
        lock_path(&dir),
        good.replace(block, "[[emptied]]\nn = 42\ntickets = []"),
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter()
            .any(|e| e.contains("emptied wave 42 lists no tickets")),
        "an emptied-out set must be red: {errs:?}"
    );
    println!("── perturbed (set emptied) ── {}", errs.join(" | "));

    // (b) point the set at an UNSHIPPED ticket → red naming ticket and status.
    fs::write(
        lock_path(&dir),
        good.replace(block, "[[emptied]]\nn = 42\ntickets = [\"T-2\"]"),
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter()
            .any(|e| e.contains("emptied wave 42 lists T-2 (queued)")),
        "an unshipped member must be red: {errs:?}"
    );
    println!("── perturbed (unshipped member) ── {}", errs.join(" | "));

    // (c) pull the label to the ledger base → red: that marker already landed.
    fs::write(
        lock_path(&dir),
        good.replace(block, "[[emptied]]\nn = 41\ntickets = [\"T-1\"]"),
    )
    .unwrap();
    let errs = check_as_errors(&dir);
    assert!(
        errs.iter()
            .any(|e| e.contains("emptied wave 41 is at or below wave_base 41")),
        "a landed label must be red: {errs:?}"
    );

    // Restore the exact recorded bytes → green.
    fs::write(lock_path(&dir), &good).unwrap();
    let errs = check_as_errors(&dir);
    assert!(errs.is_empty(), "restore must be green: {errs:?}");
    println!("── restored ── check green ({} error(s))", errs.len());
    let _ = fs::remove_dir_all(&dir);
}
