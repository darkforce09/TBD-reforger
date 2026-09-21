use super::*;

use std::fs;

fn scratch(tag: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("t912-lock-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let tickets = dir.join(crate::repository::TICKETS_DIR);
    fs::create_dir_all(&tickets).unwrap();
    fs::write(tickets.join("ROOT"), "# ticket-registry root marker\n").unwrap();
    // `Corpus::load` resolves scope legality fail-closed — every scratch
    // tree carries the minimal vocabulary its fixtures use.
    fs::write(tickets.join("scope-vocab.toml"), "[repo.xtask]\n").unwrap();
    for (name, body) in files {
        fs::write(tickets.join(name), body).unwrap();
    }
    dir
}

fn work(id: &str, order: i64, owns: &[&str], deps: &[&str], status: &str) -> String {
    let owns = owns
        .iter()
        .map(|o| format!("\"{o}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let deps = deps
        .iter()
        .map(|d| format!("\"{d}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "id = \"{id}\"\nkind = \"work\"\ntitle = \"t {id}\"\nsummary = \"s\"\nclass = \"chore\"\nstatus = \"{status}\"\norder = {order}\ndepends_on = [{deps}]\nowns = [{owns}]\n\n[scope]\ndomain = \"repo\"\nlayer = \"xtask\"\n"
    )
}

// ── The close-marker ledger base ────────────────────────────────────────────────────────
/// Run git in `dir` with a pinned identity — the metrics.rs scratch-repo pattern. Never a
/// `Date`-dependent assertion downstream: subjects are the only degree of freedom.
fn git_in_dir(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args([
            "-c",
            "user.email=t914@test",
            "-c",
            "user.name=t914",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// [`scratch`] plus a fabricated history: init, commit the ticket files, then one commit
/// per subject. Each subject commit touches its own file so a later `git revert` of it is
/// never empty.
fn scratch_git(tag: &str, files: &[(&str, &str)], subjects: &[&str]) -> PathBuf {
    let dir = scratch(tag, files);
    git_in_dir(&dir, &["init", "-q"]);
    git_in_dir(&dir, &["add", "--", ".ai"]);
    git_in_dir(&dir, &["commit", "-q", "-m", "seed tickets"]);
    for (i, s) in subjects.iter().enumerate() {
        fs::write(dir.join(format!("c{i}.txt")), format!("{i}\n")).unwrap();
        git_in_dir(&dir, &["add", "--", &format!("c{i}.txt")]);
        git_in_dir(&dir, &["commit", "-q", "-m", s]);
    }
    dir
}

// ── The [[emptied]] section — pending close targets ─────────────────────────────────────
/// The lock text from `[[emptied]]` to the section after it — the proof-block slice.
fn emptied_slice(text: &str) -> &str {
    let start = text.find("[[emptied]]").expect("emptied section present");
    let end = text[start..]
        .find("\n[owns]")
        .map(|i| start + i)
        .unwrap_or(text.len());
    &text[start..end]
}

// ── The reservation, for a wave that dissolved id by id ─────────────────────────────────
/// Build the exact wave-240 shape: a wave whose set ships ONE ID AT A TIME, so no repack
/// ever sees the whole set under its own label and the freed label is reissued.
fn dissolved_by_id(name: &str) -> std::path::PathBuf {
    // T-3 collides with T-1 only, so it starts a wave behind. Shipping T-1 then T-2 dissolves
    // the first wave in two steps, and the label lands on T-3.
    let dir = scratch_git(
        name,
        &[
            ("T-1.toml", &work("T-1", 10, &["a.rs"], &[], "queued")),
            ("T-2.toml", &work("T-2", 20, &["b.rs"], &[], "queued")),
            ("T-3.toml", &work("T-3", 30, &["a.rs"], &[], "queued")),
        ],
        &["wave 41 CLOSED — test ledger"],
    );
    let before = repack_quiet(&dir).unwrap();
    assert_eq!(
        before.tickets_in_wave(42),
        vec!["T-1".to_string(), "T-2".to_string()],
        "the wave that will dissolve: {before:?}"
    );
    for (id, order, owns) in [("T-1", 10, "a.rs"), ("T-2", 20, "b.rs")] {
        fs::write(
            dir.join(format!("{}/{id}.toml", crate::repository::TICKETS_DIR)),
            work(id, order, &[owns], &[], "shipped"),
        )
        .unwrap();
        repack_quiet(&dir).unwrap(); // the per-id ship hook
    }
    dir
}

mod packing_and_history_tests;

mod emptied_wave_tests;
