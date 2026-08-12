//! T-853 — port of `scripts/mod/slice-worktree.sh` → `cargo xtask platform slice-worktree`.
//!
//! One worktree per SLICE, under `.ai/artifacts/worktrees/<slice>`, on branch `slice/<slice>`.
//! Sub-slices (`T-181.7.1`) live in their parent's tree (`T-181.7`) because they are the same
//! slice's work. Subcommands: `new` `list` `merge` `drop` `reap`.
//!
//! ── THIS FILE DESTROYS WORK IF IT IS WRONG ───────────────────────────────────────────────────
//! `drop` and `reap` delete git worktrees, and a worktree's UNCOMMITTED files exist nowhere else —
//! not in the object database, not in a reflog, nowhere. Both of the bash's incident reports are
//! about this file deleting live agents' work: `reap` wiped FIVE mid-slice worktrees ([`cmd_reap`])
//! and `drop` did the same to T-352 ([`cmd_drop`]). Every guard is load-bearing scar tissue with a
//! test proving its refusal still fires; do not "simplify" one for looking redundant with another —
//! the bash records that `drop`'s first guard was ported from `reap` and was THE WRONG ONE OF THE
//! THREE, measured silent on the incident it cited.
//!
//! OUTPUT IS A CONTRACT: `wave.sh` and `xtask mod wave` scrape this. Accepted by diffing
//! stdout+stderr+rc against the bash over 29 scenarios covering every subcommand and error path, in
//! throwaway repos with pinned commit dates so even the short SHAs match. Intended deviations: the
//! three marked FAIL-OPEN CLOSED (1..3) and the note on [`passthru`].

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tbd_gate::proc::{Output, Run};

use crate::root::find_repo_root;

/// Where slice worktrees live, relative to the repo root. Kept RELATIVE because the bash `cd`s to
/// `$ROOT` and interpolates `$dir` straight into its messages (`already exists:
/// .ai/artifacts/worktrees/T-212`). Only the final `worktree:` line is absolute, built as
/// `$ROOT/$dir`.
const BASE: &str = ".ai/artifacts/worktrees";

/// What the bash prints for `$0` — whatever the caller typed, so "byte-identical" is only defined
/// against one invocation, and the baselines were captured as `bash scripts/mod/slice-worktree.sh`.
/// Not a stale lie *yet*: the `.sh` cannot be deleted while `xtask/src/mod_wave.rs:317,471` and
/// `scripts/platform/wave.sh:3236` still shell out to it, so the `--force` advice below runs today.
/// WHEN THOSE THREE CALL SITES MOVE TO `xtask`, delete the script and repoint this one constant at
/// the `cargo xtask` spelling — that is the whole edit.
/// How the operator re-runs this tool, as printed in every refusal message.
///
/// T-853 REPOINTED THIS when the bash was deleted. It was
/// `scripts/mod/slice-worktree.sh`, which the port had to keep verbatim while the byte-for-byte
/// diff against that script was the acceptance criterion. The moment the script went away, that
/// contract became moot and the string became actively harmful: every guard refusal
/// ("Merge them, or re-run with: …") was telling the operator to run a file that does not exist,
/// at exactly the moment they are trying to get unstuck.
///
/// MEASURED 2026-08-12 — this was found by using the tool for real, dropping six stale slices;
/// the refusal fired correctly and then named a deleted script.
const PROG: &str = "cargo xtask platform slice-worktree --";

/// `usage()` in the bash is `sed -n '2,14p' "$0"` — it prints its own header back.
///
/// ODDITY PRESERVED: the range runs off the end of the header — line 13 is `set -euo pipefail` and
/// line 14 is blank, so the usage a user sees ends with a shell directive and a trailing empty line
/// (`2,12p` was plainly meant). Reproduced verbatim because the port is diffed byte-for-byte and
/// `wave.sh` greps this; a port has no `$0` to `sed`, so `usage_matches_the_bash_header` pins it.
const USAGE: &str = "\
# Slice worktree lifecycle — see docs/mod/SLICE_WORKFLOW.md (operator-defined, binding).
#
# One worktree per SLICE. Sub-slices (T-181.7.1) live in their parent's worktree (T-181.7),
# because they are the same slice's work. Three worktrees at a time; merge when all three are
# complete; DELETE immediately after merging — leftover trees fill the disk.
#
#   bash scripts/mod/slice-worktree.sh new   T-181.7
#   bash scripts/mod/slice-worktree.sh list
#   bash scripts/mod/slice-worktree.sh merge T-181.7
#   bash scripts/mod/slice-worktree.sh drop  T-181.7
#   bash scripts/mod/slice-worktree.sh reap
set -euo pipefail

";

/// Plain `git`, run from `dir`. The bash uses bare `git` everywhere except the calls listed on
/// [`git_lfs_safe`], and that distinction is deliberate rather than sloppy — see [`cmd_merge`].
fn git_plain(dir: &Path) -> Run {
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
fn git_lfs_safe(dir: &Path) -> Run {
    // Split on whitespace: none of these tokens contains a space, and the empty `=` values are
    // meant to be empty (`filter.lfs.smudge=` disables the filter rather than setting it).
    const CFG: &str = "-c core.hooksPath=/dev/null -c filter.lfs.smudge= -c filter.lfs.process= \
                       -c filter.lfs.clean=cat -c filter.lfs.required=false";
    git_plain(dir)
        .env("GIT_LFS_SKIP_SMUDGE", "1")
        .args(CFG.split_whitespace())
}

/// Run git, capturing both streams and preserving the raw exit code. `NotRun` (git absent, killed
/// by a signal) becomes an `Err`, never an exit code: `tbd_gate::proc` exists so "the OOM killer
/// shot git" is not reported as "git found a problem", and here a 137 misread as "no output, tree
/// is clean" is exactly how work gets deleted.
fn git(run: Run) -> Result<Output> {
    run.output().map_err(|e| anyhow::anyhow!("{e:?}"))
}

/// Capture, plain git / LFS-neutralised git — the one-line call forms.
fn gp(dir: &Path, args: &[&str]) -> Result<Output> {
    git(git_plain(dir).args(args))
}
fn gn(dir: &Path, args: &[&str]) -> Result<Output> {
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
fn passthru(run: Run) -> Result<i32> {
    let m = run.merged_output().map_err(|e| anyhow::anyhow!("{e:?}"))?;
    print!("{}", m.text);
    Ok(m.code)
}

/// Pass-through, plain git.
fn pt(dir: &Path, args: &[&str]) -> Result<i32> {
    passthru(git_plain(dir).args(args))
}

/// Forward **stdout only**, swallowing stderr — the bash's `git … 2>/dev/null || true`.
///
/// Used for the two `git branch` deletions. Their stdout is load-bearing — `Deleted branch
/// slice/T-900 (was f7b03ad).` is the operator's only receipt that the branch went, and the harness
/// caught its absence when this discarded both streams. Their stderr is the "branch not found"
/// noise the bash hides, because deleting a branch that never existed is not an error.
fn pt_stdout(dir: &Path, args: &[&str]) -> Result<()> {
    print!("{}", gp(dir, args)?.stdout);
    Ok(())
}

/// `git status --porcelain` in a worktree, LFS filters neutralised. The one call whose FAILURE MODE
/// decides whether work is destroyed — so all three callers route through here and all three check
/// `code` before reading `stdout`.
fn status_of(dir: &Path) -> Result<Output> {
    gn(dir, &["status", "--porcelain"])
}

/// Trimmed stdout as a number, or 0 — the bash's `"$(… 2>/dev/null || echo 0)"`. A rev-list that
/// cannot run counts as ZERO commits, the SAFE direction in both callers: in `drop` Guard A then
/// abstains so Guard B decides, and in `reap` zero plus no merge in main's history means KEEP.
fn count(out: &Output) -> u64 {
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
fn parent_slice(s: &str) -> String {
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
fn resolve_root() -> Result<PathBuf> {
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
/// T-853: `crate::mod_wave`'s `prep` and `land` used to `bash scripts/mod/slice-worktree.sh`. They
/// call this instead — in-process, so there is no second cargo resolution and no chance of the
/// child picking a different `CARGO_TARGET_DIR` than the process that launched it. They already
/// hold the root they mean, so they pass it rather than re-deriving it through
/// `TBD_SLICE_WORKTREE_ROOT`/`find_repo_root`.
pub fn run_at(root: &Path, args: &[String]) -> Result<u8> {
    dispatch(root, args)
}

/// The `case "$cmd" in` block. Split from [`run`] so the tests can exercise every arm — including
/// the `*)` fallthrough — against a throwaway root, rather than asserting a copy of this `match`.
fn dispatch(root: &Path, args: &[String]) -> Result<u8> {
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

/// Whether a missing oracle lane is fatal. See the licence/policy essay in [`cmd_new`].
#[derive(PartialEq, Clone, Copy)]
enum Policy {
    Required,
    Optional,
}

fn cmd_new(root: &Path, slice_arg: &str) -> Result<u8> {
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
fn ln_sfn(src: &Path, dst: &Path) -> std::io::Result<()> {
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
fn lane_is_linked(dst: &Path, src: &Path) -> bool {
    match fs::symlink_metadata(dst) {
        Ok(md) if md.file_type().is_symlink() => {
            // `-d` semantics on top: a dangling link is not a usable lane.
            dst.is_dir() && fs::read_link(dst).map(|t| t == src).unwrap_or(false)
        }
        _ => false,
    }
}

fn cmd_list(root: &Path) -> Result<u8> {
    // Plain `git`, exactly as the bash: no LFS/hooks neutralisation, and git's exit code is ours.
    Ok(pt(root, &["worktree", "list"])? as u8)
}

fn cmd_merge(root: &Path, slice_arg: &str) -> Result<u8> {
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

fn cmd_drop(root: &Path, slice_arg: &str, third: &str) -> Result<u8> {
    if slice_arg.is_empty() {
        eprintln!("usage: {PROG} drop <slice>");
        return Ok(2);
    }
    let slice = parent_slice(slice_arg);
    let dir = format!("{BASE}/{slice}");
    let branch = format!("slice/{slice}");
    let abs_dir = root.join(&dir);
    let forced = third == "--force";

    // ── GUARD A: unmerged commits ────────────────────────────────────────────────────────────
    // `git branch -D` below is a FORCE delete, so dropping an unmerged branch leaves its commits as
    // unreferenced objects — recoverable only by someone who thinks to look, which nobody does.
    // OBSERVED 2026-07-26: the command center landed T-352's first two commits and dropped the
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
    // T-352 — measured, `main..branch` was 0 at drop time because the work had all landed. What
    // mattered was never "unmerged commits" but "an agent is still writing here", i.e. a DIRTY TREE.
    // `reap` had that guard all along; the port took one of its three and not the one that applies.
    // Measured live at 1128c1e3 with only Guard A in place:
    //   T-365  ahead=0  dirty=3  -> would DESTROY 3 UNSTAGED files (not in the object DB)
    //   T-369  ahead=0  dirty=2  -> would DESTROY 2 staged files
    // And `wave.sh land` calls this in a loop AUTOMATICALLY, minutes after selecting the slice, with
    // a merge and a wave gate in between — a resumed agent writing then is the T-352 sequence.
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
fn cmd_reap(root: &Path) -> Result<u8> {
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
        let d_slash = format!("{BASE}/{s}/");

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
        // any-char wildcard there (`slice/T-181.7$` also matches `slice/T-181x7`); passed through
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

/// The bash's `for d in "$BASE"/*/` — subdirectories only, in glob (byte-sorted) order.
///
/// Two behaviours ride on that `*/` suffix: `BASE/README.md` is a FILE and is not matched, and when
/// the glob matches nothing bash leaves the literal pattern in `$d`, which `[ -d "$d" ] || continue`
/// discards — an empty iterator is the same. `-d` and `is_dir()` both follow symlinks.
fn slice_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(root.join(BASE)) else {
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

// ── tests ────────────────────────────────────────────────────────────────────────────────────
// PROPOSED SPLIT (SIZE): this module would move verbatim to `xtask/src/slice_worktree/tests.rs`
// behind `#[cfg(test)] mod tests;`, leaving the implementation near the 600-line target; T-853
// scoped this agent to one file, so it is inline. Every test builds its OWN throwaway repo under
// `temp_dir()` and calls `cmd_*` with an explicit `root` — nothing here can reach the real
// `.ai/artifacts/worktrees/`, because `resolve_root()` is never invoked, so there is no env var to
// race on and no path to a live slice.

#[cfg(test)]
mod tests;
