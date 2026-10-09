//! `wave push`: push main, guarding against publishing LFS pointers without their objects.
//!
//! **Role:** `cmd_push` decides the push mode from whether git-lfs is usable here: with git-lfs it
//! runs `git push origin main` with hooks; without it, it lists the LFS files the range would
//! publish (`lfs_paths_in_range`), refuses when there are any, and otherwise pushes `--no-verify`.
//!
//! **Position:** reached through the wave command table after `land`; spawns git through
//! [`process_runner::Run`].
//!
//! **Signals & state:** none held; reads git and pushes main.
//!
//! **Invariants:** the guard asks `git check-attr` which files are LFS, never a path pattern, and
//! names each offending file; it examines every commit in the range (`diff-tree -c --root`, so
//! merges and the initial commit are read) against that commit's own `.gitattributes`, read through
//! a throwaway index; the path list is written to `check-attr` from a scoped thread while the
//! caller drains its output, so a long list cannot deadlock the pipe pair; every error refuses,
//! because a guard that cannot answer must not answer "go ahead"; a git-lfs probe that cannot run
//! counts as absent, which keeps the guard on; each mode names itself before it acts.

use process_runner::{BinaryOutput, Run};

use super::Ctx;
use crate::wave_execution::wprintln;

/// Run `run`, feeding `input` to its stdin from a writer thread while its stdout drains to EOF.
///
/// The whole point is that the two happen AT THE SAME TIME, which [`Run::binary_output`]
/// guarantees: the stdin body is written on its own thread, closed when written (that close is
/// the EOF check-attr waits for), while both pipes drain. See the note in the module header for
/// the ten-minute hang this replaced: filling one 64 KB pipe from the same thread that must empty
/// the other is a deadlock the moment either list outgrows the buffer, and no amount of ordering
/// fixes it.
///
/// `Err(())` = the child could not be started, could not be waited on, or died on a signal — the
/// caller's CANNOT-TELL, never "nothing found". A failed WRITE is deliberately not an error here:
/// the child exiting early makes it `EPIPE`, and what the caller judges is the child's own status
/// and output, which a short answer already fails.
fn feed_and_capture(run: Run, input: &[u8]) -> Result<BinaryOutput, ()> {
    run.stdin_bytes(input.to_vec())
        .binary_output()
        .map_err(|_| ())
}

/// Does `cmd args…` run and exit 0?
///
/// Anything else — a non-zero exit, or a binary that will not spawn — is `false`, because the one
/// caller reads `false` as "keep the guard on". Fail closed, same as everything else here.
fn probe_ok(run: Run, args: &[&str]) -> bool {
    // Both streams are captured and dropped: the probe's answer is its exit code alone.
    run.args(args)
        .output()
        .map(|out| out.code == 0)
        .unwrap_or(false)
}

/// Is git-lfs usable on this machine?
fn git_lfs_present() -> bool {
    probe_ok(Run::new("git"), &["lfs", "version"])
}

/// The mode line and the `git push` argv, decided by that one question.
///
/// Split out from [`cmd_push`] because `--no-verify` is the entire safety property and a unit test
/// can pin it here without pushing anything: present → the flag is GONE and the pre-push hook
/// uploads the LFS objects; absent → the flag is back and the guard above has already run.
fn push_plan(git_lfs: bool) -> (&'static str, &'static [&'static str]) {
    if git_lfs {
        ("git-lfs present: normal push", &["push", "origin", "main"])
    } else {
        (
            "git-lfs absent: guarded --no-verify push",
            &["push", "--no-verify", "origin", "main"],
        )
    }
}

/// Every path in `<range>` that genuinely resolves to `filter: lfs`, one per line.
///
/// `Ok(vec![])` = nothing LFS in the range. `Err(())` = COULD NOT TELL (never "nothing found").
pub(crate) fn lfs_paths_in_range(range: &str) -> Result<Vec<String>, ()> {
    if range.is_empty() {
        return Err(());
    }
    // A bad range dies here, before anything is examined — the cannot-tell answer, not
    // "nothing found".
    let commits = Run::new("git")
        .args(["rev-list", range])
        .output()
        .map_err(|_| ())?;
    if commits.code != 0 {
        return Err(());
    }
    let commits = commits.stdout;

    // The throwaway index the `--cached` read is aimed at.
    let idx = std::env::temp_dir().join(format!("tbd-wave-lfs-idx-{}", std::process::id()));
    let _ = std::fs::remove_file(&idx);
    let _ = std::fs::remove_file(idx.with_extension("lock"));

    let mut found: Vec<String> = Vec::new();
    let result = (|| -> Result<(), ()> {
        for c in commits.lines().filter(|l| !l.is_empty()) {
            // `-z` end to end: NUL-separated paths, so a filename containing a space, a quote or a
            // newline cannot split into two entries and get another file's attribute pinned on it.
            //
            // `--diff-filter=d` EXCLUDES deletions (lowercase excludes; uppercase D would select
            // only them). MEASURED on b5c1a8f7c: 4 files total, 3 deleted, `d` yields 1 and none of
            // the 3. Deleting an LFS file uploads nothing and so cannot leave a dangling object —
            // counting deletions would reintroduce a false refusal of exactly the kind this
            // function exists to remove. Getting this flag backwards is the one edit here that
            // fails OPEN, which is why it is measured and not assumed.
            let list = Run::new("git")
                .args([
                    "diff-tree",
                    "-z",
                    "--no-commit-id",
                    "--name-only",
                    "-r",
                    "-c",
                    "--root",
                    "--diff-filter=d",
                    c,
                ])
                .binary_output()
                .map_err(|_| ())?;
            if list.code != 0 {
                return Err(());
            }

            // Attributes AS OF $c: a fresh index holding that commit's tree, read with `--cached`
            // so the working tree's (i.e. HEAD's) .gitattributes cannot answer for a historical
            // commit.
            let _ = std::fs::remove_file(&idx);
            let _ = std::fs::remove_file(idx.with_extension("lock"));
            let rt = Run::new("git")
                .env("GIT_INDEX_FILE", idx.display().to_string())
                .args(["read-tree", c])
                .output()
                .map_err(|_| ())?;
            if rt.code != 0 {
                return Err(());
            }

            // `feed_and_capture`, never an inline `write_all` — this is the list that hit
            // 1,691 paths and wedged both processes for ten minutes.
            let check_attr = Run::new("git")
                .env("GIT_INDEX_FILE", idx.display().to_string())
                .args(["check-attr", "--cached", "-z", "--stdin", "filter"]);
            let attrs = feed_and_capture(check_attr, &list.stdout)?;
            if attrs.code != 0 {
                return Err(());
            }

            // `check-attr -z` emits NUL-separated triples: <path> <attr-name> <value>.
            let mut it = attrs.stdout.split(|b| *b == 0);
            while let (Some(path), Some(_name), Some(value)) = (it.next(), it.next(), it.next()) {
                if value == b"lfs" {
                    found.push(String::from_utf8_lossy(path).into_owned());
                }
            }
        }
        Ok(())
    })();

    let _ = std::fs::remove_file(&idx);
    let _ = std::fs::remove_file(idx.with_extension("lock"));
    result?;

    // One line per path even when several commits touched it; nothing at all if we could not tell.
    found.sort();
    found.dedup();
    Ok(found)
}

pub(crate) fn cmd_push(_ctx: &Ctx) -> u8 {
    let range = "origin/main..HEAD";
    // One question, asked once, and printed. Everything below it — whether the guard runs
    // at all, and whether the push carries --no-verify — is this answer.
    let git_lfs = git_lfs_present();
    let (mode, argv) = push_plan(git_lfs);
    wprintln!("{mode}");
    if !git_lfs {
        let lfs = match lfs_paths_in_range(range) {
            Ok(v) => v,
            Err(()) => {
                wprintln!("REFUSING --no-verify: could not determine LFS status for {range}.");
                wprintln!(
                    "        One of `git rev-list` / `diff-tree` / `read-tree` / `check-attr` failed, so this"
                );
                wprintln!(
                    "        guard has no answer. It refuses rather than guessing — an unchecked --no-verify"
                );
                wprintln!("        push is the unrecoverable one.");
                return 1;
            }
        };
        if !lfs.is_empty() {
            let n = lfs.len();
            wprintln!(
                "REFUSING --no-verify: {n} file(s) in the commits of {range} resolve to `filter: lfs`:"
            );
            for p in &lfs {
                wprintln!("          {p}");
            }
            wprintln!(
                "        Find the commit that publishes one:  git log --oneline {range} -- <path>"
            );
            wprintln!(
                "        Ask HEAD about it:                   git check-attr filter -- <path>"
            );
            wprintln!(
                "        HEAD may answer `unspecified` and this guard still be right: it asks each commit's"
            );
            wprintln!(
                "        OWN .gitattributes, because that is the rule that decided whether the blob in that"
            );
            wprintln!("        commit is an LFS pointer. See lfs_paths_in_range above.");
            wprintln!(
                "        git-lfs is absent here, so --no-verify would publish commits whose LFS objects"
            );
            wprintln!("        are never uploaded. Install git-lfs and push normally.");
            return 1;
        }
    }
    super::flush();
    super::host::status_code(Run::new("git").args(argv).terminal()) as u8
}
