use super::*;

/// Plain `git`, run from `dir`. The bash uses bare `git` everywhere except the calls listed on
/// [`git_lfs_safe`], and that distinction is deliberate rather than sloppy — see [`cmd_merge`].
pub(super) fn git_plain(dir: &Path) -> Run {
    Run::new("git").cwd(dir)
}

/// The bash's `GIT=(…)` array: git with the LFS filters and the hooks path neutralised. Both halves
/// are load-bearing and both were paid for.
///
/// **`core.hooksPath=/dev/null`:** the post-checkout/post-merge hooks are LFS hooks and with git-lfs
/// absent they exit non-zero, making `git worktree add` return **2 even though the worktree was
/// created successfully**. Under `set -e` that killed `new` after the tree appeared but before the
/// oracle symlinks were made, silently, with a real-looking "Preparing worktree" as the last line —
/// so every worktree the factory produced was missing both proof lanes. MEASURED: 2 with hooks, 0
/// without. **`filter.lfs.*`:** `GIT_LFS_SKIP_SMUDGE=1` alone is NOT enough, git still tries to
/// SPAWN the filter, which does not exist here; agents never touch `packages/map-assets`, so LFS
/// files stay ~133-byte pointers, keeping worktrees cheap too.
pub(super) fn git_lfs_safe(dir: &Path) -> Run {
    // Split on whitespace: none of these tokens contains a space, and the empty `=` values are
    // meant to be empty (`filter.lfs.smudge=` disables the filter rather than setting it).
    const CFG: &str = "-c core.hooksPath=/dev/null -c filter.lfs.smudge= -c filter.lfs.process= \
                       -c filter.lfs.clean=cat -c filter.lfs.required=false";
    git_plain(dir)
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .args(CFG.split_whitespace())
}

/// Run git, capturing both streams and preserving the raw exit code. `NotRun` (git absent, killed
/// by a signal) becomes an `Err`, never an exit code: `verification_core::proc` exists so "the OOM killer
/// shot git" is not reported as "git found a problem", and here a 137 misread as "no output, tree
/// is clean" is exactly how work gets deleted.
pub(super) fn git(run: Run) -> Result<Output> {
    run.output().map_err(|e| anyhow::anyhow!("{e:?}"))
}

/// Capture, plain git / LFS-neutralised git — the one-line call forms.
pub(super) fn gp(dir: &Path, args: &[&str]) -> Result<Output> {
    git(git_plain(dir).args(args))
}

pub(super) fn gn(dir: &Path, args: &[&str]) -> Result<Output> {
    git(git_lfs_safe(dir).args(args))
}

/// Run git and forward its output, the way the bash lets git inherit the terminal.
///
/// WHY `merged_output` AND NOT `output`, MEASURED 2026-08-12 (it corrected this port's first draft):
/// one `git worktree add` writes to BOTH streams — `Preparing worktree (new branch …)` to stderr,
/// `HEAD is now at <sha> base` to stdout — so draining into two strings and printing
/// stdout-then-stderr INVERTED those lines against the bash, which the harness caught. One shared
/// pipe is what a shell's `2>&1` does, so the order is the child's own. KNOWN DEVIATION, the only
/// one in the output contract: git progress lines bash left on stderr arrive on stdout. Combined
/// output — how `wave.sh`/`mod_wave.rs` capture this, and how the acceptance diff is taken — is
/// byte-identical, and no caller reads the streams apart.
pub(super) fn passthru(run: Run) -> Result<i32> {
    let m = run.merged_output().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    print!("{}", m.text);
    Ok(m.code)
}

/// Pass-through, plain git.
pub(super) fn pt(dir: &Path, args: &[&str]) -> Result<i32> {
    passthru(git_plain(dir).args(args))
}

/// Forward **stdout only**, swallowing stderr — the bash's `git … 2>/dev/null || true`.
///
/// Used for the two `git branch` deletions. Their stdout is load-bearing — `Deleted branch
/// slice/T-900 (was f7b03ad).` is the operator's only receipt that the branch went, and the harness
/// caught its absence when this discarded both streams. Their stderr is the "branch not found"
/// noise the bash hides, because deleting a branch that never existed is not an error.
pub(super) fn pt_stdout(dir: &Path, args: &[&str]) -> Result<()> {
    print!("{}", gp(dir, args)?.stdout);
    Ok(())
}

/// `git status --porcelain` in a worktree, LFS filters neutralised. The one call whose FAILURE MODE
/// decides whether work is destroyed — so all three callers route through here and all three check
/// `code` before reading `stdout`.
pub(super) fn status_of(dir: &Path) -> Result<Output> {
    gn(dir, &["status", "--porcelain"])
}

/// Trimmed stdout as a number, or 0 — the bash's `"$(… 2>/dev/null || echo 0)"`. A rev-list that
/// cannot run counts as ZERO commits, the SAFE direction in both callers: in `drop` Guard A then
/// abstains so Guard B decides, and in `reap` zero plus no merge in main's history means KEEP.
pub(super) fn count(out: &Output) -> u64 {
    if out.code == 0 {
        out.stdout.trim().parse().unwrap_or(0)
    } else {
        0
    }
}

/// A sub-slice belongs to its parent's tree: `T-181.7.1` → `T-181.7`; `T-181.7` stays put.
///
/// The bash is `sed -E 's/^(T-[0-9]+\.[0-9]+).*/\1/'`, whose oddities are the contract, not
/// accidents to be tidied (`pins_the_sed_regex_oddities` covers each): `^`-anchored with a greedy
/// `.*` tail and not global, so `T-181.7junk` → `T-181.7` while `xT-181.7.1` is UNCHANGED; a bare id
/// with no dot (`T-181`) does not match and is returned unchanged, which is how the factory's flat
/// ids survive (every live worktree in the real repo is that shape); and `T-181.` needs a digit
/// after the dot, so it too is unchanged.
pub(super) fn parent_slice(s: &str) -> String {
    // Per call: runs at most once per process on a ~10-byte string, so a `OnceLock` buys nothing.
    let re = regex::Regex::new(r"^(T-[0-9]+\.[0-9]+).*").expect("static regex");
    match re.captures(s) {
        Some(c) => c[1].to_string(),
        None => s.to_string(),
    }
}

/// Repo root. `TBD_SLICE_WORKTREE_ROOT` overrides it, mirroring `TBD_PREFLIGHT_ROOT` in the T-889
/// port, so the tests and the acceptance harness drive throwaway repos under `/tmp` rather than the
/// real `.ai/artifacts/worktrees/`, which holds live slices. The bash has no equivalent — it derives
/// `$ROOT` from `dirname $0/../..`.
pub(super) fn resolve_root() -> Result<PathBuf> {
    match std::env::var("TBD_SLICE_WORKTREE_ROOT") {
        Ok(v) if !v.is_empty() => Ok(PathBuf::from(v)),
        _ => find_repo_root(),
    }
}

pub fn run(args: &[String]) -> Result<u8> {
    dispatch(&resolve_root()?, args)
}

/// Same dispatch, against an EXPLICIT root.
///
/// T-853: `crate::commands::mod_ops::wave_execution`'s `prep` and `land` used to `bash scripts/mod/slice-worktree.sh`. They
/// call this instead — in-process, so there is no second cargo resolution and no chance of the
/// child picking a different `CARGO_TARGET_DIR` than the process that launched it. They already
/// hold the root they mean, so they pass it rather than re-deriving it through
/// `TBD_SLICE_WORKTREE_ROOT`/`find_repo_root`.
pub fn run_at(root: &Path, args: &[String]) -> Result<u8> {
    dispatch(root, args)
}

/// The `case "$cmd" in` block. Split from [`run`] so the tests can exercise every arm — including
/// the `*)` fallthrough — against a throwaway root, rather than asserting a copy of this `match`.
pub(super) fn dispatch(root: &Path, args: &[String]) -> Result<u8> {
    let cmd = args.first().map(String::as_str).unwrap_or("");
    let slice = args.get(1).map(String::as_str).unwrap_or("");
    // `${3:-}` in the bash — POSITIONAL, so `drop T-x --force` puts `--force` here. There is no
    // flag parsing: `drop --force T-x` does NOT force, it tries to drop a slice named `--force`.
    let third = args.get(2).map(String::as_str).unwrap_or("");

    match cmd {
        "new" => cmd_new(root, slice),
        "list" => cmd_list(root),
        "merge" => cmd_merge(root, slice),
        "drop" => cmd_drop(root, slice, third),
        "reap" => cmd_reap(root),
        // The `*)` arm, which catches the EMPTY command too — and `create`, a spelling the factory
        // docs warn about twice, so it prints usage rather than doing anything.
        _ => {
            print!("{USAGE}");
            Ok(2)
        }
    }
}

pub(super) fn cmd_new(root: &Path, slice_arg: &str) -> Result<u8> {
    if slice_arg.is_empty() {
        eprintln!("usage: {PROG} new <slice>");
        return Ok(2);
    }
    let p = parent_slice(slice_arg);
    let slice = if p != slice_arg {
        println!(
            "note: {slice_arg} is a sub-slice — it belongs in {p}'s worktree (SLICE_WORKFLOW.md rule 1)"
        );
        p
    } else {
        slice_arg.to_string()
    };
    let dir = format!("{BASE}/{slice}");
    let branch = format!("slice/{slice}");
    let abs_dir = root.join(&dir);

    // An existing tree is NOT skipped outright — it falls through to the oracle link step, which is
    // idempotent. Early-returning here is what made the missing-oracle bug UNREPAIRABLE: the trees
    // existed, so every subsequent `new` said "already exists" and changed nothing.
    if abs_dir.is_dir() {
        println!("already exists: {dir} (re-checking oracles)");
    } else {
        fs::create_dir_all(root.join(BASE)).with_context(|| format!("mkdir -p {BASE}"))?;
        // Branch from the CURRENT main tip so the agent gets the committed factory.
        let refname = format!("refs/heads/{branch}");
        let have = gp(root, &["show-ref", "--verify", "--quiet", &refname])?;
        let add: Vec<&str> = if have.code == 0 {
            vec!["worktree", "add", &dir, &branch]
        } else {
            vec!["worktree", "add", "-b", &branch, &dir, "main"]
        };
        let code = passthru(git_lfs_safe(root).args(&add))?;
        // The bash's `set -e`. Kept as a hard stop: continuing to the symlink step with no tree
        // would `ln` into a path that does not exist, and the operator would then be told
        // "REFUSING: no usable oracle lane" for a worktree that was never created.
        if code != 0 {
            return Ok(code as u8);
        }
    }

    // ── ORACLE LANES ─────────────────────────────────────────────────────────────────────────
    // The oracle sources are GITIGNORED, so a fresh worktree has none of them, and an agent with no
    // way to query CRF or read vanilla source falls back on training-data guesses about Enfusion,
    // which are wrong. Link them in (read-only; no disk cost, no risk of a slice mutating them).
    // Idempotent, so re-running `new` REPAIRS a missing lane rather than skipping it.
    //
    // LICENCE — the lanes are NOT equivalent, and the next agent must know which is which:
    //   crf_framework      Arma Public License. Read, cite, design-mirror. Never vendored.
    //   vanilla_reference  Bohemia game source, carved by `enf carve`. Read-only, never committed.
    //   playable_selector  NO LICENCE AT ALL — DESIGN-MIRROR ONLY. Absence of a licence is worse
    //                      than APL, not better: default copyright applies, so there is no
    //                      permission to copy, adapt or redistribute a single line. Read it to
    //                      understand how a lobby/slot-picker is SHAPED, then write our own.
    // `xtask verify no-crf-leak` enforces that (CRF_ and PS_ identifier + asset-GUID gates).
    //
    // REQUIRED vs OPTIONAL (T-181.52): the refuse-on-missing rule exists for ONE failure mode — an
    // agent with no way to CHECK an Enfusion API fact will invent one. crf_framework and
    // vanilla_reference answer that, live in the repo, and are provisioned by repo tooling, so their
    // absence means a broken local setup: REFUSE. playable_selector is a DESIGN mirror proving no
    // Enfusion fact, cannot be compiled against, and sits OUTSIDE the repo on one operator's disk,
    // so on CI it is legitimately absent — refusing would break `new` for everyone but Sam over a
    // non-correctness problem while the two required lanes still cover the real failure mode. So:
    // WARN, naming what the agent lost. Licence risk is unaffected either way.
    let home = std::env::var("HOME").unwrap_or_default();
    // `${TBD_PS_ORACLE:-…}` — the default applies when the variable is unset OR EMPTY.
    let ps_oracle = match std::env::var("TBD_PS_ORACLE") {
        Ok(v) if !v.is_empty() => v,
        _ => format!("{home}/Projects/Archive/Reforger_Lobby/PlayableSelector-main"),
    };
    let r = root.display();
    let mo = format!("{r}/apps/mod");
    let (crf, van) = (
        format!("{mo}/crf_framework"),
        format!("{mo}/vanilla_reference"),
    );
    let lanes = [
        ("crf_framework", crf, Policy::Required),
        ("vanilla_reference", van, Policy::Required),
        ("playable_selector", ps_oracle, Policy::Optional),
    ];

    let mut missing_oracle = false;
    for (name, src, policy) in &lanes {
        // `[ -d "$src" ]` follows symlinks, and so does `is_dir()`.
        if !Path::new(src).is_dir() {
            if *policy == Policy::Required {
                eprintln!("  ERROR: {src} missing — cannot link the {name} oracle lane");
                missing_oracle = true;
            } else {
                eprintln!(
                    "  WARNING: no {name} oracle at {src} — this tree has NO PlayableSelector lane."
                );
                eprintln!(
                    "           Design work that would cite it must STOP and ask, not guess."
                );
                eprintln!(
                    "           If the checkout moved, re-run with TBD_PS_ORACLE=/path/to/PlayableSelector-main"
                );
            }
            continue;
        }

        let dst = abs_dir.join("apps/mod").join(name);
        if let Err(e) = ln_sfn(Path::new(src), &dst) {
            // The bash's bare `ln -sfn` under `set -e`: ln prints its own diagnostic and the script
            // dies on the spot. Reachable when the tree has no `apps/mod/` at all.
            eprintln!(
                "ln: failed to create symbolic link '{}': {e}",
                dst.display()
            );
            return Ok(1);
        }
        // Verify rather than trust. This whole block used to be unreachable and nothing noticed,
        // because nobody checked the result — the agents just quietly lost their proof lanes.
        if lane_is_linked(&dst, Path::new(src)) {
            println!("  oracle ok: apps/mod/{name} -> {src}");
        } else {
            eprintln!("  ERROR: failed to link apps/mod/{name} into {dir}");
            if *policy == Policy::Required {
                missing_oracle = true;
            }
        }
    }
    if missing_oracle {
        eprintln!(
            "REFUSING: {dir} has no usable oracle lane — an agent here would guess at Enfusion."
        );
        return Ok(1);
    }

    // Tempted to "fix" the LFS pointers? DON'T symlink them. Content is deliberately not smudged
    // (see [`git_lfs_safe`]), so `packages/map-assets/**` arrives as ~133-byte pointers, which makes
    // `cargo xtask ci schema-validate` die in a worktree at `schema height-labels` ("PNG decode: Invalid PNG
    // signature") while passing on main — two agents burned real effort on that. Symlinking the real
    // assets DOES fix the target, and was tried and REVERTED: git then reports all 983 tracked files
    // there as DELETED, leaving every worktree permanently dirty so `wave.sh land` refuses it.
    // Hiding that with `--skip-worktree` would make working-tree changes INVISIBLE, which in a
    // program merging unattended agent work silently loses a slice.
    println!(
        "  note: packages/map-assets is LFS pointers here — run 'xtask schema validate', not 'cargo xtask ci schema-validate'"
    );
    println!("worktree: {r}/{dir}   branch: {branch}");
    Ok(0)
}

/// `ln -sfn src dst`, GNU semantics, MEASURED rather than assumed (2026-08-12):
///
/// * dst absent → create the link.
/// * dst is a symlink, even one to a directory → `-n` stops the dereference and `-f` replaces it.
///   The idempotent re-run path that repairs a stale lane.
/// * dst is a REAL directory → `-n` does not apply (it covers only symlinks-to-directories) and ln
///   creates the link INSIDE it, exiting 0: `ln -sfn …/crf_framework realdir2` produced
///   `realdir2/crf_framework`. Reproduced, not corrected — that is ln's behaviour; the LIE it
///   enables is closed in [`lane_is_linked`].
pub(super) fn ln_sfn(src: &Path, dst: &Path) -> std::io::Result<()> {
    let target = match fs::symlink_metadata(dst) {
        // Real directory (symlink_metadata does not follow, so a symlink-to-dir is not this arm).
        Ok(md) if md.is_dir() => dst.join(src.file_name().unwrap_or_default()),
        // Symlink or file: `-f` removes it first. `remove_file` unlinks a symlink without touching
        // what it points at, which is what the oracle sources need.
        Ok(_) => {
            fs::remove_file(dst)?;
            dst.to_path_buf()
        }
        Err(_) => dst.to_path_buf(),
    };
    std::os::unix::fs::symlink(src, target)
}

/// Is the lane genuinely linked?
///
/// ── FAIL-OPEN CLOSED (1 of 3) ────────────────────────────────────────────────────────────────
/// The bash verifies with `[ -d "$dir/apps/mod/$ref" ]`, which FOLLOWS symlinks and so also passes
/// for a plain real directory ln just descended into (see [`ln_sfn`]) — printing `oracle ok` for a
/// lane that was never linked, the exact "reports success over an input it never examined" defect
/// its own comment says the check was added to stop. Requiring a symlink resolving to `src` closes
/// it. The happy path is unchanged so no baseline moves, and the bad path is unreachable today
/// (both lanes are gitignored) — but one `git add apps/mod/crf_framework/` makes it reachable, and
/// it fails silent.
pub(super) fn lane_is_linked(dst: &Path, src: &Path) -> bool {
    match fs::symlink_metadata(dst) {
        Ok(md) if md.file_type().is_symlink() => {
            // `-d` semantics on top: a dangling link is not a usable lane.
            dst.is_dir() && fs::read_link(dst).map(|t| t == src).unwrap_or(false)
        }
        _ => false,
    }
}

pub(super) fn cmd_list(root: &Path) -> Result<u8> {
    // Plain `git`, exactly as the bash: no LFS/hooks neutralisation, and git's exit code is ours.
    Ok(pt(root, &["worktree", "list"])? as u8)
}

pub(super) fn cmd_merge(root: &Path, slice_arg: &str) -> Result<u8> {
    if slice_arg.is_empty() {
        eprintln!("usage: {PROG} merge <slice>");
        return Ok(2);
    }
    // ODDITY: unlike `new`, the sub-slice rewrite here is SILENT — `merge T-181.7.1` merges
    // `slice/T-181.7` and never says so. Preserved; `sub_slice_shares_the_parent_tree` pins it.
    let slice = parent_slice(slice_arg);
    let branch = format!("slice/{slice}");
    let dir = format!("{BASE}/{slice}");
    let abs_dir = root.join(&dir);
    if !abs_dir.is_dir() {
        eprintln!("no worktree at {dir}");
        return Ok(1);
    }

    // Refuse to merge a dirty tree — uncommitted slice work would be silently lost.
    //
    // ── FAIL-OPEN CLOSED (2 of 3) ────────────────────────────────────────────────────────────
    // The bash runs this status with PLAIN git, unlike `drop`, which neutralises the LFS filters for
    // the same call and says why: without git-lfs installed `status` can exit 128 on an LFS-adjacent
    // tree. Inside `if [ -n "$(…)" ]` a failed substitution yields an empty string, `set -e` does
    // not fire in a condition, and A DIRTY TREE THEREFORE READS AS CLEAN — the merge proceeds and
    // the work is lost, the one outcome this guard exists to prevent. Two changes: neutralised git,
    // and a non-zero status is a refusal rather than "clean".
    let st = status_of(&abs_dir)?;
    if st.code != 0 {
        eprintln!(
            "REFUSING: cannot read {dir}'s status (rc={}) — refusing to merge a tree I cannot inspect.",
            st.code
        );
        return Ok(1);
    }
    if !st.stdout.is_empty() {
        eprintln!("REFUSING: {dir} has uncommitted changes. Commit them in the worktree first:");
        eprint!("{}", gn(&abs_dir, &["status", "--short"])?.stdout);
        return Ok(1);
    }

    // T-946 — THE GATE-VERDICT RECEIPT GUARDS THIS PATH TOO.
    //
    // T-924 put the receipt check in `platform wave land`, and the wave 237 verifier found that
    // `land` is one of THREE ways a slice branch reaches main: this command is the second, and
    // `mod wave land` is the third — which calls this one. A guard that covers a third of the
    // doors is a guard nobody can rely on, and its main goal is literally "nothing mechanical lets
    // an ungated or stale-gated slice land". So the check moves to the chokepoint both paths share.
    //
    // Same fail-closed shape as `land`: the tip of the branch about to be merged is compared with
    // the sha the gate recorded, and a missing, stale, red or unreadable receipt refuses. Nothing
    // here waives it — a merge is a merge whoever typed it.
    let tip = gn(root, &["rev-parse", &branch])?.stdout;
    if let Some(msg) =
        crate::commands::platform::wave_execution::verdict::land_refusal(root, &slice, tip.trim())
    {
        eprintln!("{msg}");
        return Ok(2);
    }

    // ODDITY: the message is hard-coded to `T-181:` whatever program the slice belongs to, so the
    // platform factory's merges are all tagged with the mod program's ticket. Preserved — `reap`'s
    // `git log --grep` keys off `slice/<id>` in git's auto-generated "Merge branch" line, not off
    // this prefix, so changing it would rewrite history for no gain.
    // ODDITY: nothing checks that HEAD is `main`. This merges into whatever is checked out.
    let msg = format!("T-181: merge {branch}\n\nCo-Authored-By: Claude <noreply@anthropic.com>");
    let code = pt(root, &["merge", "--no-ff", &branch, "-m", &msg])?;
    if code != 0 {
        return Ok(code as u8); // bash `set -e`
    }
    println!("merged {branch} -> main");
    Ok(0)
}
