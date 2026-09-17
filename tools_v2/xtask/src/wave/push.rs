//! ── T-599 — THE PUSH GUARD ASKS GIT WHICH FILES ARE LFS. IT DOES NOT MATCH THE PATH. ────────
//!
//! git-lfs is not installed in this container (any checkout needing it dies with
//! `git-lfs filter-process: 1: git-lfs: not found`), so `--no-verify` is how work leaves this
//! machine at all. The guard is real: pushing `--no-verify` over genuine LFS content publishes
//! commits whose LFS objects were never uploaded, and every later clone breaks on them.
//!
//! WHAT THIS USED TO BE, AND WHY IT WAS WRONG:
//!
//! ```text
//! if git diff --name-only origin/main..HEAD | grep -q '^packages/map-assets/'; then refuse
//! ```
//!
//! It matched the DIRECTORY and assumed everything under it was LFS. `.gitattributes` has never
//! said that — LFS covers exactly three globs there:
//!
//! ```text
//! packages/map-assets/**/*.png   **/*.r16   **/*.tbd-sat
//! ```
//!
//! Everything else beneath that tree is ordinary bytes. MEASURED 2026-07-31 while closing wave 74:
//! a legitimate 19-commit push was refused, and all 30 files in the range resolved to
//! `filter: unspecified` — including T-594's regenerated `everon/objects/prefabs.json.gz` and
//! `everon/objects/type-inventory.json`, which are real content, not pointers. ZERO files in that
//! range were LFS. The operator overrode the guard by hand, correctly.
//!
//! THE OVERRIDE IS THE DAMAGE, not the lost minutes. A guard that is wrong about ordinary work
//! teaches whoever runs it that overriding is the normal way to push. The one time it is right, it
//! gets overridden by reflex too — and that is the push that breaks the remote. Precision here is a
//! safety property, not tidiness.
//!
//! So ask `git check-attr`, which consults the same `.gitattributes` git itself would, and refuse
//! only on a genuine `filter: lfs`. And NAME the offending files: the old message named a
//! directory, which the reader had no way to verify, so the only available responses were trust and
//! override. A named path can be checked in one command, which the message prints.
//!
//! FAIL CLOSED. Every error path refuses. A guard that cannot answer the LFS question must not
//! answer "go ahead" — that is the one direction where being wrong cannot be undone, because the
//! remote is shared. This is deliberately NOT symmetric with the false-positive fix above.
//!
//! ── T-600 — EVERY COMMIT IN THE RANGE, AND EACH COMMIT'S OWN `.gitattributes`. ───────────────
//!
//! T-599 fixed WHICH question this asks (check-attr, not path matching). It kept the wrong INPUT:
//! `git diff --name-only origin/main..HEAD` is the ENDPOINT diff, so a file living only in an
//! INTERMEDIATE commit — added, then deleted or renamed before HEAD — was never examined at all.
//! MEASURED in a scratch repo with this function sourced verbatim: a `.tbd-sat` added in commit 2
//! of 3 and deleted in commit 3 gave `rc=0` and empty output — the guard ALLOWED the push. The
//! commit publishing that pointer still reaches the remote, and every later checkout, bisect or
//! `lfs fetch --all` of it breaks. A tool reporting success over an input it never read is the
//! exact failure this guard exists to prevent, so it is not acceptable that it was pre-existing.
//!
//! So walk `git rev-list <range>` and diff EACH commit. Two flags are load-bearing:
//!   - `-c` — plain `diff-tree` prints NOTHING for a merge commit — trading one blind spot for
//!     another (an evil merge that adds an LFS file in the merge itself). `-c` reports what a
//!     merge introduced beyond ALL its parents, and is an ordinary diff on non-merges. MEASURED:
//!     evil-merge case goes empty without it, names the file with it.
//!   - `--root` — a range containing the initial commit is otherwise silently empty.
//!
//! WHICH `.gitattributes` — HEAD'S, OR THE COMMIT'S OWN? THE COMMIT'S OWN, deliberately.
//! `.gitattributes` can change inside the range. If `filter=lfs` was in force when the file landed
//! and is gone by HEAD, `check-attr` at HEAD answers `unspecified` and the guard allows the push —
//! MEASURED, the same blind spot in a second disguise. The commit's own rule is also the CORRECT
//! predicate, not just the safer one: that rule is what decided whether git-lfs's clean filter ran,
//! i.e. whether the blob in that commit is a pointer needing an uploaded object or ordinary bytes.
//! HEAD's opinion of a historical blob is hearsay.
//!
//! It cuts both ways, and that is intended. A file committed as ordinary bytes BEFORE some later
//! commit in the range adds an lfs rule is NOT refused: its blob is real content, nothing was ever
//! cleaned, nothing needs uploading. Refusing it would be a fresh false positive of exactly the
//! kind T-599 removed — and the false-positive fix is the reason the guard is believed at all.
//!
//! Git is 2.39 here, so `check-attr --source=<tree-ish>` (2.40+) does not exist. `--cached` does,
//! and reads attributes from the index ONLY — so a throwaway `GIT_INDEX_FILE` filled by
//! `read-tree <c>` is that answer. MEASURED: HEAD says `unspecified` for the Case-7 path, the temp
//! index says `lfs`.
//!
//! ── T-943 — THE GUARD DEADLOCKED ON ITS OWN QUESTION, AND ANSWERED IT WHERE NOBODY ASKED. ────
//!
//! Two defects, both measured 2026-09-04 while closing wave 248 on `origin/main..HEAD` (28 commits,
//! one of them T-090.12.2's 1,691 `packages/map-assets/everon/prefabs/blas/*.bvh`).
//!
//! FIRST: THE PIPE. This function used to hand `check-attr` the whole path list —
//! `child.stdin.take()?.write_all(&list.stdout)` — and only afterwards call `wait_with_output` to
//! read the answer. Both pipes hold 64 KB. 1,691 paths are ~90 KB in and ~120 KB back, so
//! check-attr filled its stdout pipe, blocked on write, and therefore stopped reading; the list
//! then filled ITS stdin pipe and the parent blocked on write. MEASURED: both processes in state S
//! for ten minutes until the operator killed them. Neither side is at fault alone — a pipe pair is
//! a deadlock unless somebody drains while somebody fills, and one thread cannot be both.
//!
//! So the write moves to a scoped thread and the caller keeps `wait_with_output`. `std::thread::scope`
//! and not `spawn`: the writer borrows the list, and scope guarantees it is joined before this
//! function returns — a detached writer outliving a failed spawn is the bug this fix would
//! otherwise trade in. The regression pin drives a stub check-attr that answers incrementally and
//! wraps the call in a 30 s receive; with the inline `write_all` restored it TIMES OUT, which is
//! what a deadlock looks like from the outside and the only honest red for one. How big the list
//! has to be is NOT "bigger than one pipe buffer" — see `deadlock_threshold` in the test module,
//! which is the arithmetic the first draft of that pin got wrong and the perturbation caught.
//!
//! SECOND: THE GUARD RAN ON A HOST THAT HAS GIT-LFS. Everything above exists because git-lfs is
//! missing from the CONTAINER, where `--no-verify` is the only way out. On the operator's host
//! git-lfs 3.7.1 is installed, the pre-push hook uploads the objects, and the guard's refusal is a
//! false positive of exactly the kind T-599 was written to remove — the operator overrode it by
//! hand with `git push origin main`, and was right. `git lfs version` is the question that decides
//! it, and its answer picks BOTH the mode line and the argv (see `push_plan`): present → plain
//! `git push origin main` WITH hooks and no guard at all; absent → the guard, then `--no-verify`.
//!
//! A probe that cannot run counts as ABSENT. That keeps the guard on, which is the direction where
//! being wrong is recoverable — same asymmetry as the fail-closed rule above, for the same reason.
//! And each branch NAMES itself on stdout before it acts: a tool that silently changes behaviour
//! with the machine it is on is the defect this program keeps finding, so the mode is printed even
//! though nothing reads it but a human.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use super::Ctx;
use crate::wprintln;

/// Run `cmd`, feeding `input` to its stdin from a scoped writer thread while THIS thread drains
/// its stdout to EOF.
///
/// The whole point is that the two happen AT THE SAME TIME. See the T-943 note in the module
/// header for the ten-minute hang this replaced: filling one 64 KB pipe from the same thread that
/// must empty the other is a deadlock the moment either list outgrows the buffer, and no amount of
/// ordering fixes it.
///
/// `Err(())` = the child could not be started or could not be waited on — the caller's
/// CANNOT-TELL, never "nothing found". A failed WRITE is deliberately not an error here: the child
/// exiting early makes it `EPIPE`, and what the caller judges is the child's own status and
/// output, which a short answer already fails.
fn feed_and_capture(cmd: &mut Command, input: &[u8]) -> Result<Output, ()> {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| ())?;
    let mut sink = child.stdin.take().ok_or(())?;
    std::thread::scope(|s| {
        s.spawn(move || {
            // Dropping `sink` at the end of this closure is what sends EOF. Without it check-attr
            // waits for more paths forever and the `wait_with_output` below never returns.
            let _ = sink.write_all(input);
        });
        child.wait_with_output()
    })
    .map_err(|_| ())
}

/// Does `cmd args…` run and exit 0?
///
/// Anything else — a non-zero exit, or a binary that will not spawn — is `false`, because the one
/// caller reads `false` as "keep the guard on". Fail closed, same as everything else here.
fn probe_ok(cmd: &mut Command, args: &[&str]) -> bool {
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Is git-lfs usable on this machine?
fn git_lfs_present() -> bool {
    probe_ok(&mut Command::new("git"), &["lfs", "version"])
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
pub fn lfs_paths_in_range(range: &str) -> Result<Vec<String>, ()> {
    if range.is_empty() {
        return Err(());
    }
    // A bad range dies here, before anything is examined — the cannot-tell answer, not
    // "nothing found".
    let commits = Command::new("git")
        .args(["rev-list", range])
        .output()
        .map_err(|_| ())?;
    if !commits.status.success() {
        return Err(());
    }
    let commits = String::from_utf8_lossy(&commits.stdout).into_owned();

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
            let list = Command::new("git")
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
                .output()
                .map_err(|_| ())?;
            if !list.status.success() {
                return Err(());
            }

            // Attributes AS OF $c: a fresh index holding that commit's tree, read with `--cached`
            // so the working tree's (i.e. HEAD's) .gitattributes cannot answer for a historical
            // commit.
            let _ = std::fs::remove_file(&idx);
            let _ = std::fs::remove_file(idx.with_extension("lock"));
            let rt = Command::new("git")
                .env("GIT_INDEX_FILE", &idx)
                .args(["read-tree", c])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map_err(|_| ())?;
            if !rt.success() {
                return Err(());
            }

            // T-943: `feed_and_capture`, never an inline `write_all` — this is the list that hit
            // 1,691 paths and wedged both processes for ten minutes.
            let mut check_attr = Command::new("git");
            check_attr
                .env("GIT_INDEX_FILE", &idx)
                .args(["check-attr", "--cached", "-z", "--stdin", "filter"])
                .stderr(Stdio::null());
            let attrs = feed_and_capture(&mut check_attr, &list.stdout)?;
            if !attrs.status.success() {
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

pub fn cmd_push(_ctx: &Ctx) -> u8 {
    let range = "origin/main..HEAD";
    // T-943. One question, asked once, and printed. Everything below it — whether the guard runs
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
    match Command::new("git").args(argv).status() {
        Ok(st) => super::host::status_code(&st) as u8,
        Err(_) => 127,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::mpsc;
    use std::time::Duration;

    /// A stand-in for `git check-attr --cached -z --stdin filter`, in the ONE shape that decides
    /// whether T-943 reproduces: it answers INCREMENTALLY. It reads a chunk of stdin, emits
    /// `<path>\0filter\0unspecified\0` for every complete path in it, and loops — so its stdout
    /// fills while its stdin is still being written, which is what real check-attr does and what
    /// makes a single-threaded fill-then-drain caller deadlock.
    ///
    /// A stub rather than real check-attr on purpose: real git could one day buffer differently and
    /// the regression pin would quietly stop reproducing anything — the vacuous-pass shape this
    /// program keeps finding. This one is pinned by construction.
    ///
    /// The 60 s watchdog is hygiene: when the parent deadlocks, its assertion fires at 30 s and
    /// this process must not survive as an orphan holding a pipe.
    const STUB_SRC: &str = r#"
use std::io::{Read, Write};

fn main() {
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(60));
        std::process::exit(0);
    });
    let mut stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = match stdin.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        while let Some(i) = buf.iter().position(|b| *b == 0) {
            let path: Vec<u8> = buf.drain(..=i).collect();
            let path = &path[..path.len() - 1];
            if path.is_empty() {
                continue;
            }
            if out.write_all(path).is_err() {
                return;
            }
            if out.write_all(b"\0filter\0unspecified\0").is_err() {
                return;
            }
        }
        if out.flush().is_err() {
            return;
        }
    }
}
"#;

    /// The `rustc` that built THIS test, wherever possible.
    ///
    /// PATH last, not first: a stub built by some other toolchain's rustc would still run and the
    /// test would still pass, and it would be reporting on something other than this build.
    fn rustc() -> PathBuf {
        if let Some(p) = std::env::var_os("RUSTC") {
            return PathBuf::from(p);
        }
        // Cargo bakes its own path in at compile time; under rustup rustc is its sibling.
        if let Some(sib) = Path::new(env!("CARGO")).parent().map(|d| d.join("rustc")) {
            if sib.is_file() {
                return sib;
            }
        }
        PathBuf::from("rustc")
    }

    /// Compile [`STUB_SRC`] into a private temp dir and return the binary.
    fn build_stub(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tbd-t943-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("stub temp dir");
        let src = dir.join("stub_check_attr.rs");
        std::fs::write(&src, STUB_SRC).expect("write stub source");
        let bin = dir.join("stub_check_attr");
        let out = Command::new(rustc())
            .args(["--edition", "2021", "-O", "-o"])
            .arg(&bin)
            .arg(&src)
            .output()
            .expect("rustc must be runnable to build the stub check-attr");
        assert!(
            out.status.success(),
            "rustc could not build the stub check-attr:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        bin
    }

    /// `git diff-tree -z --name-only` shape: NUL-separated, trailing NUL included.
    fn path_list(n: usize) -> Vec<u8> {
        let mut v = Vec::new();
        for i in 0..n {
            v.extend_from_slice(
                format!("packages/map-assets/everon/prefabs/blas/obj{i:06}.bvh").as_bytes(),
            );
            v.push(0);
        }
        v
    }

    /// A Linux pipe holds 64 KB, both directions.
    const PIPE_CAPACITY: usize = 64 * 1024;

    /// `filter\0unspecified\0` — what the stub appends to every path it answers.
    const ANSWER_OVERHEAD: usize = 19;

    /// The smallest stdin list that can wedge a fill-then-drain caller.
    ///
    /// THIS IS THE NUMBER THE FIRST VERSION OF THIS TEST GOT WRONG, and the perturbation caught it:
    /// 2,000 paths (108 KB) passed the PERTURBED build — a green regression pin over the very
    /// defect it was written for. "Bigger than one pipe buffer" is not the condition. The caller
    /// only blocks once it has written everything the child can absorb, and the child absorbs the
    /// 64 KB sitting in its stdin pipe PLUS every path it consumed before its own stdout pipe
    /// filled — `PIPE_CAPACITY / per_out` paths, worth `× per_in` bytes of input. Below that sum
    /// the whole list fits and both sides finish, defect or no defect.
    fn deadlock_threshold(per_in: usize) -> usize {
        PIPE_CAPACITY + (PIPE_CAPACITY / (per_in + ANSWER_OVERHEAD)) * per_in
    }

    /// THE T-943 REGRESSION PIN.
    ///
    /// The assertion is a receive timeout because that is the only honest red for a deadlock: the
    /// unfixed code does not fail, it never returns. MEASURED with the inline `write_all` restored:
    /// no output, killed at 30 s, red.
    #[test]
    fn a_path_list_larger_than_the_pipe_buffer_does_not_deadlock() {
        let bin = build_stub("deadlock");
        let n = 20_000usize;
        let list = path_list(n);
        let bytes = list.len();
        let threshold = deadlock_threshold(bytes / n);
        assert!(
            bytes > 4 * threshold,
            "a list of {bytes} bytes cannot wedge the pre-fix code (threshold {threshold}), so \
             this test would pass over the defect and prove nothing — see deadlock_threshold"
        );

        let (tx, rx) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let mut cmd = Command::new(&bin);
            let _ = tx.send(feed_and_capture(&mut cmd, &list).map(|o| o.stdout));
        });
        let out = match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(out)) => out,
            Ok(Err(())) => panic!("the stub check-attr did not run"),
            Err(_) => panic!(
                "DEADLOCK: feed_and_capture did not finish within 30 s for a {bytes} byte list. \
                 The T-943 defect is back — stdin is being written before stdout is drained."
            ),
        };
        worker.join().expect("writer worker");

        // Finishing is only half of it. A caller that returned early with a truncated answer would
        // also "not deadlock", and lfs_paths_in_range reads a short answer as `nothing is LFS` —
        // the fail-OPEN direction this whole module exists to keep shut.
        let mut it = out.split(|b| *b == 0);
        let mut seen = 0usize;
        while let (Some(path), Some(name), Some(value)) = (it.next(), it.next(), it.next()) {
            assert_eq!(name, b"filter", "triple {seen} is not a filter answer");
            assert_eq!(value, b"unspecified", "triple {seen} has the wrong value");
            assert!(!path.is_empty(), "triple {seen} has an empty path");
            seen += 1;
        }
        assert_eq!(
            seen, n,
            "every path must come back; a short read is the same bug in a different mask"
        );
        let _ = std::fs::remove_dir_all(
            std::env::temp_dir().join(format!("tbd-t943-deadlock-{}", std::process::id())),
        );
    }

    /// The child's own failure must reach the caller as a failure, not as an empty answer.
    #[test]
    fn feed_and_capture_reports_a_child_that_exits_non_zero() {
        let mut cmd = Command::new("git");
        cmd.args(["t943-no-such-subcommand"]).stderr(Stdio::null());
        let out = feed_and_capture(&mut cmd, b"anything\0").expect("git spawns");
        assert!(
            !out.status.success(),
            "a failing child must not look like an empty LFS answer"
        );
    }

    /// `--no-verify` is the safety property; this pins exactly when it is carried.
    ///
    /// Not a tautology over a bool: [`cmd_push`] pushes with THIS argv, so flipping either arm
    /// here is a behaviour change and turns this red.
    #[test]
    fn push_plan_drops_no_verify_exactly_when_git_lfs_is_present() {
        let (mode, argv) = push_plan(true);
        assert_eq!(mode, "git-lfs present: normal push");
        assert_eq!(argv, ["push", "origin", "main"].as_slice());
        assert!(
            !argv.contains(&"--no-verify"),
            "with git-lfs installed the pre-push hook must run — that is what uploads the objects"
        );

        let (mode, argv) = push_plan(false);
        assert_eq!(mode, "git-lfs absent: guarded --no-verify push");
        assert_eq!(argv, ["push", "--no-verify", "origin", "main"].as_slice());
    }

    /// Only a zero exit is `true` — a probe that cannot run keeps the guard on.
    ///
    /// Written against commands whose exit codes are fixed rather than against `git lfs version`
    /// itself: this host HAS git-lfs, so asserting the absent case from the machine would assert
    /// nothing here and something else in the container.
    #[test]
    fn probe_ok_is_true_only_for_a_zero_exit() {
        assert!(probe_ok(&mut Command::new("git"), &["--version"]));
        assert!(!probe_ok(
            &mut Command::new("git"),
            &["t943-no-such-subcommand"]
        ));
        assert!(!probe_ok(
            &mut Command::new("/nonexistent/t943-not-a-binary"),
            &[]
        ));
    }
}
