use super::*;

pub(super) fn cmd_drop(root: &Path, slice_arg: &str, third: &str) -> Result<u8> {
    if slice_arg.is_empty() {
        eprintln!("usage: {PROG} drop <slice>");
        return Ok(2);
    }
    let slice = parent_slice(slice_arg);
    let dir = format!("{WORKTREES_DIR}/{slice}");
    let branch = format!("slice/{slice}");
    let abs_dir = root.join(&dir);
    let forced = third == "--force";

    // ── GUARD A: unmerged commits ────────────────────────────────────────────────────────────
    // `git branch -D` below is a FORCE delete, so dropping an unmerged branch leaves its commits as
    // unreferenced objects — recoverable only by someone who thinks to look, which nobody does.
    // OBSERVED 2026-07-26: the command center landed a slice's first two commits and dropped the
    // worktree, then RESUMED the agent. It found its own worktree and branch gone mid-session, its
    // commits surviving only as loose objects, and had to recreate the branch and restore the tree
    // before it could finish. It reported that rather than losing the work, which is the only
    // reason this was noticed at all.
    if !forced && gp(root, &["rev-parse", "--verify", &branch])?.code == 0 {
        let range = format!("main..{branch}");
        let ahead = count(&gp(root, &["rev-list", "--count", &range])?);
        if ahead > 0 {
            eprintln!("REFUSED: {branch} has {ahead} commit(s) not on main.");
            eprintln!("         Merge them, or re-run with: {PROG} drop {slice} --force");
            for line in gp(root, &["log", "--oneline", &range])?.stdout.lines() {
                eprintln!("           {line}");
            }
            return Ok(1);
        }
    }

    // ── GUARD B: dirty tree. THE ONE THAT ACTUALLY COVERS THE CITED INCIDENT. ─────────────────
    // The bash's own heading: "I PORTED THE WRONG GUARD." Guard A alone would have been SILENT on
    // Measured, `main..branch` was 0 at drop time because the work had all landed. What
    // mattered was never "unmerged commits" but "an agent is still writing here", i.e. a DIRTY TREE.
    // `reap` had that guard all along; the port took one of its three and not the one that applies.
    // Measured live at 1128c1e3 with only Guard A in place:
    //   ahead=0  dirty=3  -> would DESTROY 3 UNSTAGED files (not in the object DB)
    //   ahead=0  dirty=2  -> would DESTROY 2 staged files
    // And `platform wave land` calls this in a loop AUTOMATICALLY, minutes after selecting the slice, with
    // a merge and a wave gate in between — a resumed agent writing then is the sequence.
    if !forced && abs_dir.is_dir() {
        let st = status_of(&abs_dir)?;
        // ── FAIL-OPEN CLOSED (3 of 3) ────────────────────────────────────────────────────────
        // The bash wrote `dirty="$(git … | wc -l)"` then `rc=$?`, but `$?` there is the exit of the
        // ASSIGNMENT — of the pipeline, i.e. of `wc -l`, which always succeeds. `rc` was therefore
        // always 0 and this arm was DEAD CODE. Worse, `pipefail` + `set -e` meant a git failure
        // killed the script AT the assignment with no message (git's stderr is swallowed by
        // `2>/dev/null`), so an operator saw a bare exit 128 from a tool that had just been asked
        // to delete a directory. The branch the author intended is live here.
        if st.code != 0 {
            eprintln!(
                "REFUSED: cannot read {dir}'s status (rc={}) — refusing to drop a tree I cannot inspect.",
                st.code
            );
            return Ok(1);
        }
        let dirty = st.stdout.lines().count();
        if dirty > 0 {
            eprintln!(
                "REFUSED: {dir} has {dirty} uncommitted change(s) — an agent may still be working."
            );
            eprintln!(
                "         Unstaged work is not in the object database and cannot be recovered."
            );
            for line in st.stdout.lines().take(10) {
                eprintln!("           {line}");
            }
            eprintln!("         Wait for its report, or re-run with: {PROG} drop {slice} --force");
            return Ok(1);
        }
    }

    // Past the guards, deletion is unconditional and `--force` here overrides git's OWN refusal to
    // remove a dirty tree. `2>/dev/null || rm -rf "$dir"` in the bash: a directory git does not
    // recognise as a worktree is still removed.
    if gn(root, &["worktree", "remove", "--force", &dir])?.code != 0 && abs_dir.exists() {
        fs::remove_dir_all(&abs_dir).with_context(|| format!("rm -rf {dir}"))?;
    }
    pt_stdout(root, &["branch", "-D", &branch])?; // `|| true`: a branch that never existed is fine
    pt(root, &["worktree", "prune"])?;
    println!("dropped {dir} + {branch}");
    Ok(0)
}

/// Delete every slice worktree whose branch is already merged into main — the post-wave cleanup
/// step. Disk is the constraint; do not skip this.
///
/// ── THIS DESTROYED FIVE LIVE AGENTS' WORK ONCE. Read before changing. ────────────────────────
/// The old test was `merge-base --is-ancestor <branch> main` ALONE, which is TRUE for a branch with
/// NO COMMITS AT ALL — a fresh worktree sits at the branch point, trivially an ancestor of main — so
/// "no work done yet" was indistinguishable from "work merged". Combined with `worktree remove
/// --force`, which deliberately overrides git's own refusal to delete a dirty tree, one reap wiped
/// five worktrees whose agents were mid-slice with uncommitted files. Nothing in git could recover
/// them; only the agents' own context could.
///
/// Three independent guards now, because any one alone would have prevented it: (1) never reap a
/// tree with uncommitted changes or untracked files; (2) never reap a branch with no commits beyond
/// main (unstarted != merged); (3) NO `--force`, so git's own check is a backstop, not suppressed.
pub(super) fn cmd_reap(root: &Path) -> Result<u8> {
    let mut n = 0u32;
    for d in slice_dirs(root) {
        let s = d
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let b = format!("slice/{s}");
        let refname = format!("refs/heads/{b}");
        // The bash's `$d` carries a TRAILING SLASH from the `*/` glob and interpolates it straight
        // into `git worktree remove "$d"`. Rebuilt so git sees an identical argument.
        let d_slash = format!("{WORKTREES_DIR}/{s}/");

        if gp(root, &["show-ref", "--verify", "--quiet", &refname])?.code != 0 {
            println!("kept   {s} (no branch)");
            continue;
        }

        // GUARD 1 — uncommitted changes or untracked files.
        let st = status_of(&d)?;
        if st.code != 0 {
            // The bash dies here instead, silently: `pipefail` makes the assignment fail and
            // `set -e` aborts the WHOLE loop, so every tree after this one is never even considered.
            // Refusing this one tree and carrying on deletes nothing and says so.
            eprintln!(
                "KEPT   {s} — cannot read its status (rc={}); inspect by hand.",
                st.code
            );
            continue;
        }
        let dirty = st.stdout.lines().count();
        if dirty != 0 {
            eprintln!(
                "KEPT   {s} — {dirty} uncommitted change(s). An agent may still be working here."
            );
            continue;
        }

        // GUARD 2 — "no commits beyond main" is AMBIGUOUS: it means UNSTARTED (fresh tree) AND it
        // means MERGED (commits now in main). Those need opposite outcomes, and conflating them is
        // what destroyed five worktrees. The distinguisher is main's own history: `merge` leaves a
        // "Merge branch 'slice/<id>'" commit, so if that exists the work landed.
        let range = format!("main..{b}");
        let commits = count(&gp(root, &["rev-list", "--count", &range])?);
        // `--grep="slice/$s\$"` — a git BRE with an end anchor. The `.` in a sub-slice id is an
        // any-char wildcard there (a dotted branch pattern also matches one with any character
        // in the dot's place); passed through
        // unchanged so git applies exactly the semantics the bash got.
        let grep = format!("--grep=slice/{s}$");
        let log = gp(root, &["log", "main", "--merges", "--oneline", &grep])?;
        let mut landed = log.stdout.lines().next().unwrap_or("").to_string(); // `| head -1`
        // ── DEAD CLAUSE, REPRODUCED DELIBERATELY ─────────────────────────────────────────────
        // The bash adds a second "landed" shape for a conflicted merge resolved inside a normal
        // commit: `is-ancestor(b, main) && rev-parse(b) != merge-base(b, main)`. Those can NEVER
        // both hold — if `b` is an ancestor of main then `merge-base(b, main)` IS `b`, so the
        // inequality is always false. Kept because presence and absence are behaviourally identical,
        // and because deleting it would erase the record that the conflicted-merge shape is NOT
        // covered: such a tree is KEPT (the safe side), not reaped. `ancestor_clause_is_inert` pins
        // the arithmetic so nobody "repairs" this into something that reaps.
        if landed.is_empty() && gp(root, &["merge-base", "--is-ancestor", &b, "main"])?.code == 0 {
            let tip = gp(root, &["rev-parse", &b])?;
            let mb = gp(root, &["merge-base", &b, "main"])?;
            if tip.stdout.trim() != mb.stdout.trim() {
                landed = "ancestor".to_string();
            }
        }
        if commits == 0 && landed.is_empty() {
            eprintln!(
                "KEPT   {s} — no commits beyond main and no merge in main's history (unstarted)."
            );
            continue;
        }

        if gp(root, &["merge-base", "--is-ancestor", &b, "main"])?.code != 0 {
            println!("kept   {s} ({commits} commit(s) not merged into main)");
            continue;
        }

        // GUARD 3 — NO `--force`, so git refuses a tree it considers unsafe and we honour that.
        if gn(root, &["worktree", "remove", &d_slash])?.code == 0 {
            // `-d`, not `-D`: git refuses to delete an unmerged branch here too.
            pt_stdout(root, &["branch", "-d", &b])?;
            println!("reaped {s} (merged, clean)");
            n += 1;
        } else {
            eprintln!("KEPT   {s} — git refused to remove it; inspect by hand.");
        }
    }
    pt(root, &["worktree", "prune"])?;
    println!("reaped {n} worktree(s)");
    // `df -h "$ROOT" | tail -1`. Kept: the whole point of reap is disk, and the operator wants the
    // number afterwards. A `df` that cannot run prints nothing rather than aborting the reap.
    if let Ok(o) = Run::new("df")
        .args(["-h", &root.display().to_string()])
        .output()
        && let Some(last) = o.stdout.lines().next_back()
    {
        println!("{last}");
    }
    Ok(0)
}

/// The bash's `for d in "$WORKTREES_DIR"/*/` — subdirectories only, in glob (byte-sorted) order.
///
/// Two behaviours ride on that `*/` suffix: `WORKTREES_DIR/README.md` is a FILE and is not matched, and when
/// the glob matches nothing bash leaves the literal pattern in `$d`, which `[ -d "$d" ] || continue`
/// discards — an empty iterator is the same. `-d` and `is_dir()` both follow symlinks.
pub(super) fn slice_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(root.join(WORKTREES_DIR)) else {
        return Vec::new();
    };
    let mut v: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v
}
