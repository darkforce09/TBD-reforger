use super::*;
use std::path::PathBuf;

use crate::commands::platform::wave_execution::{capture_step, testcwd};

#[test]
fn the_allowlist_refuses_anything_that_is_not_wave_or_a_ticket() {
    // `land T-204` was byte-for-byte `land` before this allowlist existed.
    assert!(is_ticket_glob("T-204"));
    assert!(is_ticket_glob("T-204.3"));
    assert!(!is_ticket_glob("t-204"), "the glob is case-sensitive");
    assert!(!is_ticket_glob("T-x"));
    assert!(!is_ticket_glob("--force"));
    assert!(!is_ticket_glob("T-"));
}

// ── T-923: the close ceremony ───────────────────────────────────────────────────────────

#[test]
fn the_close_argument_parser_is_an_allowlist() {
    // T-946 added the third element: the operator-vouched `--tickets` set, `None` by default.
    assert_eq!(parse_close_args(&[]).unwrap(), (None, false, None));
    assert_eq!(
        parse_close_args(&["--dry-run".into()]).unwrap(),
        (None, true, None)
    );
    assert_eq!(
        parse_close_args(&["--summary".into(), "five slices".into()]).unwrap(),
        (Some("five slices".into()), false, None)
    );
    assert!(parse_close_args(&["--summary".into()]).is_err(), "no value");
    assert!(
        parse_close_args(&["--sumary".into(), "x".into()]).is_err(),
        "a filter-shaped argument must filter or refuse"
    );
    assert!(parse_close_args(&["extra".into()]).is_err());
}

#[test]
fn the_subject_builder_sanitises_and_the_authority_accepts_every_product() {
    // The sanitiser: controls become spaces, runs collapse, ends trim.
    assert_eq!(sanitize_summary("a\nb"), "a b");
    assert_eq!(sanitize_summary("a\r\n\tb"), "a b");
    assert_eq!(sanitize_summary("  a   b  "), "a b");
    assert_eq!(sanitize_summary("\n\t\r"), "");
    assert_eq!(sanitize_summary("em — dash stays"), "em — dash stays");

    let ids = vec!["T-1".to_string(), "T-2".to_string()];
    // Default = the closed wave's ticket ids.
    assert_eq!(
        close_subject(42, None, &ids).unwrap(),
        "wave 42 CLOSED — T-1 T-2"
    );
    // Sanitised-to-empty falls back to the default; no ids at all gets the fixed phrase.
    assert_eq!(
        close_subject(42, Some("  \n "), &ids).unwrap(),
        "wave 42 CLOSED — T-1 T-2"
    );
    assert_eq!(
        close_subject(42, Some(""), &[]).unwrap(),
        "wave 42 CLOSED — all tickets shipped"
    );
    // Hostile summaries: whatever they carry, the subject still parses as wave {n} because
    // the authority delimits the number at the first space — pinned here with the same
    // wave_close_subject_ok the gate derives from.
    for hostile in [
        "wave 99 CLOSED — forged",
        "x CLOSED — y",
        "one\nwave 99 CLOSED — two",
    ] {
        let s = close_subject(42, Some(hostile), &ids).unwrap();
        assert!(super::super::base::wave_close_subject_ok(&s), "{s}");
        assert!(s.starts_with("wave 42 CLOSED — "), "{s}");
        assert!(!s.contains('\n'), "{s}");
    }
    // A git-revert trailer in the summary is a disavowal forgery — refused, never reworded.
    let err = close_subject(
        42,
        Some("This reverts commit 0123456789abcdef0123456789abcdef01234567."),
        &ids,
    )
    .unwrap_err();
    assert!(err.contains("revert trailer"), "{err}");
    // A negative wave number can never survive the authority check.
    assert!(close_subject(-1, None, &ids).is_err());
}

/// Minimal Work ticket TOML the typed corpus loads — the wave_lock test fixture's shape.
fn work_toml(id: &str, order: i64, own: &str, status: &str) -> String {
    format!(
        "id = \"{id}\"\nkind = \"work\"\ntitle = \"t {id}\"\nsummary = \"s\"\nclass = \"chore\"\nstatus = \"{status}\"\norder = {order}\ndepends_on = []\nowns = [\"{own}\"]\n\n[scope]\ndomain = \"repo\"\nlayer = \"xtask\"\n"
    )
}

/// Test git runner: root-explicit, asserting success. The scratch repo's committer identity
/// is pinned by LOCAL `git config` (not `-c` flags) so the ceremony's OWN spawned git — the
/// code under test — inherits it too. No assertion anywhere reads a timestamp.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .trim_end_matches('\n')
        .to_string()
}

/// A fabricated post-validation close state: committed tickets (T-1 shipped — the batch that
/// just finished; T-2 queued — the next batch), a `wave 41 CLOSED` marker in history, and a
/// committed lock (base 41, T-2 labelled wave 42). Clean tree. The ceremony under test
/// closes wave 42.
///
/// The dir is NOT deleted at test end on purpose: cwd-guarded tests must never delete a
/// directory another thread may have captured as its restore target (the pre-existing
/// chdir-test hazard this suite refuses to widen). Each rerun reclaims its own dir here.
fn close_scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t923-close-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tickets = dir.join(".ai/tickets");
    std::fs::create_dir_all(&tickets).unwrap();
    std::fs::write(tickets.join("ROOT"), "# ticket-registry root marker\n").unwrap();
    std::fs::write(tickets.join("scope-vocab.toml"), "[repo.xtask]\n").unwrap();
    std::fs::write(
        tickets.join("T-1.toml"),
        work_toml("T-1", 10, "a.rs", "shipped"),
    )
    .unwrap();
    std::fs::write(
        tickets.join("T-2.toml"),
        work_toml("T-2", 20, "b.rs", "queued"),
    )
    .unwrap();
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t923@test"]);
    git(&dir, &["config", "user.name", "t923"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    git(&dir, &["add", "--", ".ai"]);
    git(&dir, &["commit", "-q", "-m", "seed tickets"]);
    std::fs::write(dir.join("c0.txt"), "0\n").unwrap();
    git(&dir, &["add", "--", "c0.txt"]);
    git(&dir, &["commit", "-q", "-m", "wave 41 CLOSED — prior wave"]);
    ticket_engine::wave_lock::repack_quiet(&dir).unwrap();
    git(&dir, &["add", "--", ticket_engine::repository::WAVE_LOCK]);
    git(&dir, &["commit", "-q", "-m", "wave.lock: baseline"]);
    dir
}

#[test]
fn close_ceremony_commits_the_marker_and_the_lock_refresh_and_ends_check_green() {
    let dir = close_scratch("e2e");
    let cwd = testcwd::CwdGuard::enter(&dir);
    let before: i64 = git(&dir, &["rev-list", "--count", "HEAD"]).parse().unwrap();

    let (out, rc) = capture_step(|| close_ceremony(&dir, "42", &["T-1".to_string()], None, false));
    println!("── ceremony stdout ──\n{out}");
    assert_eq!(rc, 0, "{out}");

    // Exactly two commits: the marker, then the lock refresh riding it.
    let after: i64 = git(&dir, &["rev-list", "--count", "HEAD"]).parse().unwrap();
    assert_eq!(after, before + 2, "marker + lock refresh, nothing else");
    let log3 = git(&dir, &["log", "--oneline", "-3"]);
    println!("── git log --oneline -3 ──\n{log3}");
    let marker = git(&dir, &["rev-parse", "HEAD~1"]);
    let subject = git(&dir, &["log", "-1", "--format=%s", "HEAD~1"]);
    println!("── accepted subject ── {subject}");
    assert_eq!(subject, "wave 42 CLOSED — T-1");
    assert_eq!(
        git(&dir, &["log", "-1", "--format=%s", "HEAD"]),
        "wave.lock: repack after close"
    );

    // The anchored authority accepts the committed marker with number 42 (cwd is the
    // scratch repo, which is what these cwd-bound readers key on).
    assert_eq!(super::super::base::wave_close_number(&marker), Some(42));
    assert_eq!(super::super::base::wave_close_is_newest_wave(&marker), 0);
    assert_eq!(
        ticket_engine::wave_lock::history::newest_close_base(&dir).unwrap(),
        Some(42)
    );

    // Repack derived base 42 and renumbered the open wave to 43; check is green; the tree
    // is clean — no manual step left.
    let lock = ticket_engine::wave_lock::load(&dir).unwrap();
    println!("── lock head ── wave_base = {}", lock.wave_base);
    assert_eq!(lock.wave_base, 42);
    assert_eq!(lock.tickets_in_wave(43), vec!["T-2".to_string()]);
    let errs = ticket_engine::wave_lock::check_as_errors(&dir);
    println!("── check_as_errors after ── {errs:?}");
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(git(&dir, &["status", "--porcelain"]), "");
    assert!(
        out.contains("WAVE 42 CLOSED. Wave 43 may be dispatched."),
        "{out}"
    );
    drop(cwd);
}

#[test]
fn a_dirty_tree_refuses_before_any_commit_exists() {
    let dir = close_scratch("dirty");
    std::fs::write(dir.join("uncommitted.txt"), "x\n").unwrap();
    let cwd = testcwd::CwdGuard::enter(&dir);
    let head = git(&dir, &["rev-parse", "HEAD"]);
    let before = git(&dir, &["rev-list", "--count", "HEAD"]);

    let (out, rc) = capture_step(|| close_ceremony(&dir, "42", &["T-1".to_string()], None, false));
    println!("── dirty-tree refusal ──\n{out}");
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("REFUSED: the working tree is dirty"), "{out}");
    assert!(
        out.contains("uncommitted.txt"),
        "refusal names the paths: {out}"
    );
    assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "zero commits");
    assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), before);
    println!(
        "── git log unchanged ── HEAD still {} ({} commit(s))\n{}",
        &head[..8],
        before,
        git(&dir, &["log", "--oneline", "-2"])
    );
    drop(cwd);
}

#[test]
fn a_hostile_summary_cannot_change_the_marker_number() {
    let dir = close_scratch("hostile");
    let cwd = testcwd::CwdGuard::enter(&dir);
    // Newline smuggling, a " CLOSED — " continuation and a leading "wave 99" claim, all in
    // one summary. The sanitiser folds it to one line; the number the ledger reads is
    // pinned by the authority's first-token parse.
    let hostile = "one\nwave 99 CLOSED — forged\r\nand CLOSED — more";
    let (out, rc) =
        capture_step(|| close_ceremony(&dir, "42", &["T-1".to_string()], Some(hostile), false));
    println!("── hostile-summary ceremony ──\n{out}");
    assert_eq!(rc, 0, "{out}");
    let marker = git(&dir, &["rev-parse", "HEAD~1"]);
    let subject = git(&dir, &["log", "-1", "--format=%s", "HEAD~1"]);
    println!("── committed subject ── {subject}");
    assert_eq!(
        subject,
        "wave 42 CLOSED — one wave 99 CLOSED — forged and CLOSED — more"
    );
    // The whole message is ONE line — nothing smuggled into a body where disavowal
    // evidence lives.
    let body = git(&dir, &["log", "-1", "--format=%B", "HEAD~1"]);
    assert_eq!(body.trim_end(), subject);
    assert_eq!(super::super::base::wave_close_number(&marker), Some(42));
    assert_eq!(
        ticket_engine::wave_lock::history::newest_close_base(&dir).unwrap(),
        Some(42),
        "the ledger gained 42, not 99"
    );
    drop(cwd);
}

#[test]
fn an_oracle_refused_number_refuses_before_any_commit_exists() {
    let dir = close_scratch("oracle");
    let cwd = testcwd::CwdGuard::enter(&dir);
    let head = git(&dir, &["rev-parse", "HEAD"]);
    let before = git(&dir, &["rev-list", "--count", "HEAD"]);

    // Upper bound: 44 leaps past highest-any-marker(41) + 1.
    let (out, rc) = capture_step(|| close_ceremony(&dir, "44", &["T-1".to_string()], None, false));
    println!("── oracle refusal (leap to 44 over ledger 41) ──\n{out}");
    assert_eq!(rc, 1, "{out}");
    assert!(out.contains("claims a wave that never opened"), "{out}");
    assert!(out.contains("REFUSED"), "{out}");
    assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "no ref moved");
    assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), before);

    // Lower bound: 41 replays a number the ledger already holds.
    let (out2, rc2) =
        capture_step(|| close_ceremony(&dir, "41", &["T-1".to_string()], None, false));
    println!("── oracle refusal (replay of 41) ──\n{out2}");
    assert_eq!(rc2, 1, "{out2}");
    assert!(out2.contains("CONTRADICTED by the marker ledger"), "{out2}");
    assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "no ref moved");
    assert_eq!(git(&dir, &["rev-list", "--count", "HEAD"]), before);
    println!(
        "── git log unchanged after both refusals ── HEAD still {} ({} commit(s))",
        &head[..8],
        before
    );
    drop(cwd);
}

#[test]
fn dry_run_prints_the_subject_and_writes_nothing() {
    let dir = close_scratch("dry");
    // No cwd guard on purpose: the dry-run path never touches git at all, and running it
    // from a foreign cwd proves that.
    let head = git(&dir, &["rev-parse", "HEAD"]);
    let (out, rc) = capture_step(|| {
        close_ceremony(
            &dir,
            "42",
            &["T-1".to_string()],
            Some("soak complete"),
            true,
        )
    });
    println!("── dry run ──\n{out}");
    assert_eq!(rc, 0, "{out}");
    assert!(out.contains("wave 42 CLOSED — soak complete"), "{out}");
    let porcelain = git(&dir, &["status", "--porcelain"]);
    println!("── porcelain after dry run ── {porcelain:?}");
    assert_eq!(porcelain, "", "porcelain unchanged and empty");
    assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "nothing committed");
    let _ = std::fs::remove_dir_all(&dir); // never chdir'd into — safe to reclaim now
}

// ── T-925: close targets the oldest pending emptied label ───────────────────────────────

/// A fabricated PENDING-EMPTIED close state: `n` colliding tickets packed into singleton
/// waves 42..41+n over a `wave 41 CLOSED` ledger, lock committed, then the first `ship`
/// tickets shipped one at a time — each ship followed by the ship-hook repack, each pair
/// committed together (the T-917 lifecycle shape) — so the committed lock holds `ship`
/// pending `[[emptied]]` entries with frozen singleton sets, ascending, over a clean
/// tree. Same no-delete rule as [`close_scratch`]: cwd-guarded tests never reclaim their
/// dir; each rerun reclaims its own.
fn emptied_scratch(tag: &str, n: usize, ship: usize) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t925-close-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tickets = dir.join(".ai/tickets");
    std::fs::create_dir_all(&tickets).unwrap();
    std::fs::write(tickets.join("ROOT"), "# ticket-registry root marker\n").unwrap();
    std::fs::write(tickets.join("scope-vocab.toml"), "[repo.xtask]\n").unwrap();
    for i in 1..=n {
        std::fs::write(
            tickets.join(format!("T-{i}.toml")),
            work_toml(&format!("T-{i}"), (i * 10) as i64, "a.rs", "queued"),
        )
        .unwrap();
    }
    git(&dir, &["init", "-q"]);
    git(&dir, &["config", "user.email", "t925@test"]);
    git(&dir, &["config", "user.name", "t925"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    git(&dir, &["add", "--", ".ai"]);
    git(&dir, &["commit", "-q", "-m", "seed tickets"]);
    std::fs::write(dir.join("c0.txt"), "0\n").unwrap();
    git(&dir, &["add", "--", "c0.txt"]);
    git(&dir, &["commit", "-q", "-m", "wave 41 CLOSED — prior wave"]);
    ticket_engine::wave_lock::repack_quiet(&dir).unwrap();
    git(&dir, &["add", "--", ticket_engine::repository::WAVE_LOCK]);
    git(&dir, &["commit", "-q", "-m", "wave.lock: baseline"]);
    for i in 1..=ship {
        std::fs::write(
            tickets.join(format!("T-{i}.toml")),
            work_toml(&format!("T-{i}"), (i * 10) as i64, "a.rs", "shipped"),
        )
        .unwrap();
        ticket_engine::wave_lock::repack_quiet(&dir).unwrap();
        git(&dir, &["add", "--", ".ai"]);
        git(&dir, &["commit", "-q", "-m", &format!("T-{i}: ship")]);
    }
    dir
}

/// T-946 — the close-time registry view must see CHILD ids.
///
/// RED before the fix: `is_shipped("T-1.1")` was false for a ticket file that reads
/// `status = "shipped"`, because the view loaded parents only, and `wave --close` printed
/// `REFUSED: wave N still open: T-1.1` forever.
#[test]
fn registry_view_reports_a_shipped_child_ticket_as_shipped() {
    let dir = std::env::temp_dir().join(format!("t946-child-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let tickets = dir.join(".ai/tickets");
    std::fs::create_dir_all(&tickets).unwrap();
    std::fs::write(tickets.join("ROOT"), "# ticket-registry root marker\n").unwrap();
    std::fs::write(tickets.join("scope-vocab.toml"), "[repo.xtask]\n").unwrap();
    // `queued`, not `ready`: the T-917 schema gate requires a spec on a ready ticket, and
    // the typed corpus this view now loads through enforces it.
    std::fs::write(
        tickets.join("T-1.toml"),
        work_toml("T-1", 10, "a.rs", "queued"),
    )
    .unwrap();
    std::fs::write(
        tickets.join("T-1.1.toml"),
        work_toml("T-1.1", 11, "b.rs", "shipped"),
    )
    .unwrap();
    std::fs::write(
        tickets.join("T-2.toml"),
        work_toml("T-2", 20, "c.rs", "queued"),
    )
    .unwrap();

    match ticket_engine::wave_lock::load_views(&dir) {
        Ok(v) => println!(
            "── load_views ── {:?}",
            v.iter()
                .map(|t| (t.id.as_str(), t.status.as_str()))
                .collect::<Vec<_>>()
        ),
        Err(e) => println!("── load_views ERR ── {e:#}"),
    }
    let reg = ledger::Registry::load_repo(&dir);
    println!("── is_shipped ── T-1.1 = {}", reg.is_shipped("T-1.1"));
    assert!(
        reg.is_shipped("T-1.1"),
        "a shipped CHILD must read as shipped — the parents-only view is what made \
         `wave --close` refuse a wave of slices"
    );
    assert!(!reg.is_shipped("T-2"), "a queued parent is not shipped");
    assert!(
        !reg.is_shipped("T-404"),
        "an id with no ticket file is not shipped"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-946 — `--tickets` closes an operator-vouched set, and still validates every id.
#[test]
fn close_tickets_flag_parses_and_still_refuses_an_unshipped_id() {
    let parsed = parse_close_args(&[
        "--tickets".to_string(),
        "T-1, T-2".to_string(),
        "--dry-run".to_string(),
    ])
    .expect("parse");
    println!("── parsed ── {parsed:?}");
    assert_eq!(parsed.2, Some(vec!["T-1".to_string(), "T-2".to_string()]));
    assert!(parsed.1, "--dry-run still parses alongside --tickets");
    assert!(
        parse_close_args(&["--tickets".to_string(), "  ".to_string()]).is_err(),
        "an empty id list must refuse, not silently close everything"
    );
    assert!(
        parse_close_args(&["--tickets".to_string()]).is_err(),
        "a value-less --tickets must refuse"
    );

    // T-2 is NOT shipped in this tree, so the vouched set is still rejected on status.
    let dir = emptied_scratch("vouch", 2, 1);
    let cwd = testcwd::CwdGuard::enter(&dir);
    let ctx = Ctx::enter().expect("ctx");
    let (out, rc) = capture_step(|| {
        cmd_wave_close(
            &ctx,
            &[
                "--tickets".to_string(),
                "T-1,T-2".to_string(),
                "--dry-run".to_string(),
            ],
        )
    });
    println!("── --tickets with an unshipped id ──\n{out}");
    assert_eq!(rc, 1, "an unshipped id in the vouched set must refuse");
    assert!(
        out.contains("still open: T-2"),
        "the refusal names the unshipped id: {out}"
    );
    drop(cwd);
    let _ = std::fs::remove_dir_all(&dir);
}

/// T-946 — `--tickets` must refuse a label the lock still calls OPEN.
///
/// This is the guard for the marker that had to be disavowed on 2026-09-06: wave 236 shipped
/// per id, no pending entry formed, the repack gave label 236 to the next batch, and the close
/// wrote `wave 236 CLOSED` over it. Oracle 2 then refused every gate until the marker was
/// reverted.
#[test]
fn close_tickets_refuses_a_label_that_is_still_an_open_wave() {
    // No pending entry, so the label falls back to wave_base + 1 — and an OPEN wave holds it.
    let dir = emptied_scratch("open-label", 2, 0);
    let cwd = testcwd::CwdGuard::enter(&dir);
    let lock = ticket_engine::wave_lock::load(&dir).expect("lock");
    let n = lock.wave_base + 1;
    println!(
        "── lock ── base {} · open waves {:?} · pending {:?}",
        lock.wave_base,
        lock.waves
            .iter()
            .filter(|w| w.n > 0)
            .map(|w| (w.n, w.tickets.clone()))
            .collect::<Vec<_>>(),
        lock.emptied.iter().map(|e| e.n).collect::<Vec<_>>()
    );
    assert!(
        lock.waves.iter().any(|w| w.n == n),
        "the fixture must actually have an open wave at {n}"
    );

    let ctx = Ctx::enter().expect("ctx");
    let (out, rc) = capture_step(|| {
        cmd_wave_close(
            &ctx,
            &[
                "--tickets".to_string(),
                "T-1,T-2".to_string(),
                "--dry-run".to_string(),
            ],
        )
    });
    println!("── close --tickets over an open label ──\n{out}");
    assert_eq!(rc, 1, "must refuse");
    assert!(
        out.contains(&format!("wave {n} is still an OPEN wave")),
        "the refusal names the label and why: {out}"
    );
    drop(cwd);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn close_targets_the_recorded_emptied_label_end_to_end() {
    let dir = emptied_scratch("e2e", 2, 1);
    let cwd = testcwd::CwdGuard::enter(&dir);

    let lock = ticket_engine::wave_lock::load(&dir).unwrap();
    let (out, tgt) = capture_step(|| close_target(&lock));
    println!("── close_target ──\n{out}");
    let (w, ids) = tgt.expect("one pending entry");
    assert_eq!(w, "42");
    assert_eq!(
        ids,
        vec!["T-1".to_string()],
        "the FROZEN set, not a recompute"
    );

    let (out, rc) = capture_step(|| close_ceremony(&dir, &w, &ids, None, false));
    println!("── ceremony stdout ──\n{out}");
    assert_eq!(rc, 0, "{out}");

    let log3 = git(&dir, &["log", "--oneline", "-3"]);
    println!("── git log --oneline -3 ──\n{log3}");
    assert_eq!(
        git(&dir, &["log", "-1", "--format=%s", "HEAD~1"]),
        "wave 42 CLOSED — T-1",
        "the marker closes exactly the recorded label, naming its frozen set"
    );
    let marker = git(&dir, &["rev-parse", "HEAD~1"]);
    assert_eq!(super::super::base::wave_close_number(&marker), Some(42));

    let lock = ticket_engine::wave_lock::load(&dir).unwrap();
    println!(
        "── lock after close ── wave_base = {}, emptied = {:?}",
        lock.wave_base, lock.emptied
    );
    assert_eq!(lock.wave_base, 42, "base advanced to the closed label");
    assert!(
        lock.emptied.is_empty(),
        "the post-close repack dropped the entry: {lock:?}"
    );
    assert_eq!(lock.tickets_in_wave(43), vec!["T-2".to_string()]);
    let errs = ticket_engine::wave_lock::check_as_errors(&dir);
    println!("── check_as_errors after ── {errs:?}");
    assert!(errs.is_empty(), "{errs:?}");
    assert_eq!(git(&dir, &["status", "--porcelain"]), "");
    assert!(
        out.contains("WAVE 42 CLOSED. Wave 43 may be dispatched."),
        "{out}"
    );
    drop(cwd);
}

#[test]
fn two_pending_labels_drain_oldest_first() {
    let dir = emptied_scratch("two", 3, 2);
    let cwd = testcwd::CwdGuard::enter(&dir);

    let lock = ticket_engine::wave_lock::load(&dir).unwrap();
    let pend: Vec<u32> = lock.emptied.iter().map(|e| e.n).collect();
    assert_eq!(pend, vec![42, 43], "entries pend ascending: {lock:?}");

    // First close drains the OLDEST (42) — the only number the marker oracle accepts.
    let (sel1, tgt) = capture_step(|| close_target(&lock));
    let (w1, ids1) = tgt.expect("pending");
    assert_eq!(w1, "42");
    assert_eq!(ids1, vec!["T-1".to_string()]);
    let (out1, rc1) = capture_step(|| close_ceremony(&dir, &w1, &ids1, None, false));
    assert_eq!(rc1, 0, "{out1}");

    // Second close targets the NEXT label — the queue drains in ledger order.
    let lock = ticket_engine::wave_lock::load(&dir).unwrap();
    let (sel2, tgt2) = capture_step(|| close_target(&lock));
    let (w2, ids2) = tgt2.expect("still one pending");
    assert_eq!(w2, "43");
    assert_eq!(ids2, vec!["T-2".to_string()]);
    let (out2, rc2) = capture_step(|| close_ceremony(&dir, &w2, &ids2, None, false));
    assert_eq!(rc2, 0, "{out2}");
    println!("── selections ──\n{sel1}{sel2}");

    // Both markers landed, in oracle order (log lists newest first).
    let closes = git(&dir, &["log", "--format=%s", "--grep=CLOSED", "HEAD"]);
    println!("── close subjects (newest first) ──\n{closes}");
    assert_eq!(
        closes.lines().collect::<Vec<_>>(),
        vec![
            "wave 43 CLOSED — T-2",
            "wave 42 CLOSED — T-1",
            "wave 41 CLOSED — prior wave",
        ]
    );
    let lock = ticket_engine::wave_lock::load(&dir).unwrap();
    assert_eq!(lock.wave_base, 43);
    assert!(lock.emptied.is_empty(), "queue drained: {lock:?}");
    assert_eq!(lock.tickets_in_wave(44), vec!["T-3".to_string()]);
    assert!(ticket_engine::wave_lock::check_as_errors(&dir).is_empty());
    drop(cwd);
}

#[test]
fn nothing_pending_refuses_with_zero_writes() {
    // close_scratch's lock has NO pending entry: T-1 was shipped before the first compile,
    // so no open wave ever emptied — the state every tree is in right after a close.
    let dir = close_scratch("no-pending");
    let head = git(&dir, &["rev-parse", "HEAD"]);
    let lock = ticket_engine::wave_lock::load(&dir).unwrap();
    assert!(lock.emptied.is_empty());
    let (out, tgt) = capture_step(|| close_target(&lock));
    println!("── refusal ──\n{out}");
    assert!(tgt.is_none(), "{out}");
    assert!(
        out.contains("REFUSED: no emptied wave pending — nothing to close"),
        "{out}"
    );
    let porcelain = git(&dir, &["status", "--porcelain"]);
    println!("── porcelain after refusal ── {porcelain:?}");
    assert_eq!(porcelain, "", "zero writes");
    assert_eq!(git(&dir, &["rev-parse", "HEAD"]), head, "no ref moved");
    let _ = std::fs::remove_dir_all(&dir); // never chdir'd into — safe to reclaim
}
