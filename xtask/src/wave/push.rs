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

use std::io::Write;
use std::process::{Command, Stdio};

use super::Ctx;
use crate::wprintln;

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

            let mut child = Command::new("git")
                .env("GIT_INDEX_FILE", &idx)
                .args(["check-attr", "--cached", "-z", "--stdin", "filter"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|_| ())?;
            child
                .stdin
                .take()
                .ok_or(())?
                .write_all(&list.stdout)
                .map_err(|_| ())?;
            let attrs = child.wait_with_output().map_err(|_| ())?;
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
        wprintln!("        Ask HEAD about it:                   git check-attr filter -- <path>");
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
    super::flush();
    match Command::new("git")
        .args(["push", "--no-verify", "origin", "main"])
        .status()
    {
        Ok(st) => super::host::status_code(&st) as u8,
        Err(_) => 127,
    }
}
