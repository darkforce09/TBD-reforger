//! Tests for [`crate::slice_worktree`] — T-853.
//!
//! Split out of `slice_worktree.rs` purely for size: that file reached 1010 lines, one over
//! SIZE-3's hard fail, and the seam between a 700-line implementation and 300 lines of tests is
//! the obvious one. Not one assertion changed in the move.
//!
//! Every destructive guard in `drop`/`reap` is pinned here, including `ancestor_clause_is_inert`,
//! which records a branch that was DEAD in the original bash (if `b` is an ancestor of `main`
//! then `merge-base(b,main) == b`, so the inequality can never hold) so that nobody later
//! "repairs" it into something that actually reaps.
use crate::slice_worktree::*;

/// git in `dir`, args split on spaces. Returns (code, stdout).
fn g(dir: &Path, args: &str) -> (i32, String) {
    let a: Vec<&str> = args.split(' ').collect();
    let o = gn(dir, &a).expect("git ran");
    (o.code, o.stdout)
}

fn commit(dir: &Path, file: &str, msg: &str) {
    // Body = `msg`, which is unique per call. A fixed body silently breaks any test that
    // touches the same path twice on branches taken from an already-merged main: the content
    // matches, `git commit` exits 1 with "nothing to commit", and the helper's assert fires
    // ~40 lines from the real cause. Caught exactly that way while compacting this module.
    fs::write(dir.join(file), msg).unwrap();
    assert_eq!(g(dir, "add -A").0, 0);
    assert_eq!(g(dir, &format!("commit -q -m {msg}")).0, 0);
}

/// A repo shaped like TBD-Reforger: `main`, both required oracle sources, `apps/mod` tracked.
/// Purges any previous run's directory on entry, so tests need no cleanup of their own.
fn scratch(name: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("tbd-sw-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    for lane in ["crf_framework", "vanilla_reference"] {
        fs::create_dir_all(p.join("apps/mod").join(lane)).unwrap();
        fs::write(p.join("apps/mod").join(lane).join("m.txt"), lane).unwrap();
    }
    assert_eq!(g(&p, "init -q -b main .").0, 0);
    assert_eq!(g(&p, "config user.email t@t.t").0, 0);
    assert_eq!(g(&p, "config user.name t").0, 0);
    // ALL THREE LANES MUST BE IGNORED — load-bearing, not tidiness. `new` symlinks them INSIDE
    // the worktree, so an un-ignored lane shows as `?? apps/mod/<lane>` and the tree is
    // PERMANENTLY DIRTY: every guard here then refuses forever. FOUND BY THIS TEST — the fixture
    // first omitted `playable_selector` and `drop` refused a supposedly clean tree. The real
    // `apps/mod/.gitignore` has the same three with the same NO-trailing-slash rule (a slash
    // lets the link itself be staged, committing an absolute path — and unlicensed code for
    // playable_selector). Adding a lane means editing the lane table AND that .gitignore.
    let ignores = "crf_framework\nvanilla_reference\nplayable_selector\n";
    fs::write(p.join("apps/mod/.gitignore"), ignores).unwrap();
    commit(&p, "file.txt", "base");
    p
}

fn tree(root: &Path, slice: &str) -> PathBuf {
    root.join(BASE).join(slice)
}

fn branch_exists(root: &Path, slice: &str) -> bool {
    g(
        root,
        &format!("show-ref --verify --quiet refs/heads/slice/{slice}"),
    )
    .0 == 0
}

/// Is `lane` symlinked into `slice`'s worktree from the root checkout's copy?
fn lane_ok(root: &Path, slice: &str, lane: &str) -> bool {
    let dst = tree(root, slice).join("apps/mod").join(lane);
    lane_is_linked(&dst, &root.join("apps/mod").join(lane))
}

/// new + one commit in the tree + merge to main: the "work has all landed" state.
fn landed(root: &Path, slice: &str) {
    assert_eq!(cmd_new(root, slice).unwrap(), 0);
    commit(&tree(root, slice), "s.txt", slice);
    assert_eq!(cmd_merge(root, slice).unwrap(), 0);
}

#[test]
fn pins_the_sed_regex_oddities() {
    assert_eq!(parent_slice("T-181.7.1"), "T-181.7");
    assert_eq!(parent_slice("T-181.7"), "T-181.7");
    // Flat factory ids have no dot and must survive untouched — every live worktree in the real
    // repo (T-212, T-654, T-673…) is this shape.
    assert_eq!(parent_slice("T-181"), "T-181");
    assert_eq!(parent_slice("T-181.7junk"), "T-181.7"); // greedy `.*` tail
    assert_eq!(parent_slice("xT-181.7.1"), "xT-181.7.1"); // `^`-anchored
    assert_eq!(parent_slice("T-181."), "T-181."); // needs a digit after the dot
    assert_eq!(parent_slice(""), "");
}

#[test]
fn usage_matches_the_bash_header() {
    // ODDITY PIN. `sed -n '2,14p'` overshoots by two lines, so what a user sees ends with a
    // shell directive AND A TRAILING BLANK LINE. Asserted STRUCTURALLY and unconditionally, not
    // only against the script: the first draft did only that, the script was not on the
    // expected path, it SILENTLY SKIPPED, and the missing blank line shipped until the harness
    // caught it — a test that quietly examines nothing is the defect tbd-gate exists to make
    // unrepresentable.
    let lines: Vec<&str> = USAGE.lines().collect();
    assert_eq!(lines.len(), 13, "sed prints lines 2..14 inclusive");
    assert_eq!(lines[11], "set -euo pipefail");
    assert_eq!(lines[12], "", "line 14 is blank and sed prints it");
    assert!(USAGE.ends_with("set -euo pipefail\n\n"), "{USAGE:?}");
    // While the script still exists, prove the embedded copy has not drifted from it.
    let xtask = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sh = xtask
        .parent()
        .unwrap()
        .join("scripts/mod/slice-worktree.sh");
    if let Ok(body) = fs::read_to_string(&sh) {
        let want: Vec<&str> = body.lines().skip(1).take(13).collect();
        assert_eq!(USAGE, want.join("\n") + "\n", "usage drifted from the bash");
    }
}

#[test]
fn unknown_and_empty_subcommands_print_usage_and_exit_2() {
    let root = scratch("dispatch");
    // `create` is the spelling the factory docs warn about twice (PLATFORM_FACTORY.md:276,
    // FACTORY_FOR_CURSOR.md:469). It must do NOTHING — no tree, no branch.
    for cmd in ["create", "", "wat", "--help", "reaP"] {
        assert_eq!(dispatch(&root, &[cmd.to_string()]).unwrap(), 2, "`{cmd}`");
    }
    assert_eq!(dispatch(&root, &[]).unwrap(), 2, "no arguments at all");
    assert!(!root.join(BASE).exists(), "usage must not create the base");
}

#[test]
fn missing_slice_id_is_exit_2_on_every_arm() {
    let root = scratch("usage2");
    assert_eq!(cmd_new(&root, "").unwrap(), 2);
    assert_eq!(cmd_merge(&root, "").unwrap(), 2);
    assert_eq!(cmd_drop(&root, "", "").unwrap(), 2);
}

#[test]
fn ln_sfn_replaces_a_symlink_but_descends_into_a_real_directory() {
    let p = std::env::temp_dir().join(format!("tbd-sw-ln-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(p.join("srcA")).unwrap();
    fs::create_dir_all(p.join("srcB")).unwrap();
    // absent -> created; existing symlink -> replaced (the idempotent repair path).
    ln_sfn(&p.join("srcA"), &p.join("lane")).unwrap();
    assert!(lane_is_linked(&p.join("lane"), &p.join("srcA")));
    ln_sfn(&p.join("srcB"), &p.join("lane")).unwrap();
    assert!(lane_is_linked(&p.join("lane"), &p.join("srcB")));
    // MEASURED GNU behaviour: a REAL directory is descended into, exit 0.
    fs::create_dir_all(p.join("real")).unwrap();
    ln_sfn(&p.join("srcA"), &p.join("real")).unwrap();
    assert!(p.join("real/srcA").exists(), "ln descends into a real dir");
    // FAIL-OPEN CLOSED (1 of 3): bash's `[ -d … ]` says "oracle ok" here. We do not.
    assert!(!lane_is_linked(&p.join("real"), &p.join("srcA")));
    // A dangling link is not a lane either.
    std::os::unix::fs::symlink(p.join("gone"), p.join("dead")).unwrap();
    assert!(!lane_is_linked(&p.join("dead"), &p.join("gone")));
    let _ = fs::remove_dir_all(&p);
}

#[test]
fn new_creates_a_tree_links_the_lanes_and_is_idempotent() {
    let root = scratch("new");
    assert_eq!(cmd_new(&root, "T-900").unwrap(), 0);
    let t = tree(&root, "T-900");
    assert!(t.is_dir() && branch_exists(&root, "T-900"));
    for lane in ["crf_framework", "vanilla_reference"] {
        assert!(lane_ok(&root, "T-900", lane), "{lane} lane missing");
    }
    // THE UNREPAIRABLE BUG: delete a lane, re-run `new`, and it must come back rather than be
    // skipped with "already exists".
    fs::remove_file(t.join("apps/mod/crf_framework")).unwrap();
    assert_eq!(cmd_new(&root, "T-900").unwrap(), 0);
    let ok = lane_ok(&root, "T-900", "crf_framework");
    assert!(ok, "re-running `new` did not repair the lane");
}

#[test]
fn new_refuses_when_a_required_oracle_is_missing() {
    let root = scratch("noracle");
    fs::remove_dir_all(root.join("apps/mod/vanilla_reference")).unwrap();
    // The tree is still created — that is the bash's order — but the command REFUSES, so a
    // caller that checks the status never hands the tree to an agent.
    assert_eq!(cmd_new(&root, "T-901").unwrap(), 1, "must REFUSE, not warn");
}

#[test]
fn sub_slice_shares_the_parent_tree() {
    let root = scratch("subslice");
    assert_eq!(cmd_new(&root, "T-181.7.1").unwrap(), 0);
    let parent = tree(&root, "T-181.7");
    assert!(parent.is_dir(), "sub-slice did not use the parent's tree");
    assert!(!tree(&root, "T-181.7.1").exists());
    // ODDITY: `merge`/`drop` do the same rewrite but SILENTLY. Proven by operating on the
    // sub-slice id and having the PARENT's tree be what moves.
    commit(&tree(&root, "T-181.7"), "s.txt", "w");
    let rc = cmd_merge(&root, "T-181.7.1").unwrap();
    assert_eq!(rc, 0, "merge did not rewrite the sub-slice to its parent");
    assert!(root.join("s.txt").exists());
    assert_eq!(cmd_drop(&root, "T-181.7.1", "").unwrap(), 0);
    assert!(!tree(&root, "T-181.7").exists(), "dropped the parent");
}

#[test]
fn merge_reports_an_absent_worktree_and_lands_a_clean_one() {
    let root = scratch("merge");
    assert_eq!(cmd_merge(&root, "T-902").unwrap(), 1, "no worktree at …");
    landed(&root, "T-902");
    // NON-VACUITY: the merge really landed, so the refusals below refuse something that would
    // otherwise have succeeded.
    assert!(root.join("s.txt").exists(), "merge did not land the file");
}

#[test]
fn merge_refuses_a_dirty_tree() {
    let root = scratch("mergedirty");
    assert_eq!(cmd_new(&root, "T-903").unwrap(), 0);
    commit(&tree(&root, "T-903"), "s.txt", "w");
    fs::write(tree(&root, "T-903").join("uncommitted.txt"), "in flight").unwrap();
    assert_eq!(cmd_merge(&root, "T-903").unwrap(), 1, "REFUSING dirty tree");
    assert!(!root.join("s.txt").exists(), "nothing may have merged");
}

#[test]
fn drop_refuses_unmerged_commits_and_force_overrides() {
    let root = scratch("dropahead");
    assert_eq!(cmd_new(&root, "T-904").unwrap(), 0);
    commit(&tree(&root, "T-904"), "s.txt", "w");
    // GUARD A fires: the commits are not on main.
    assert_eq!(cmd_drop(&root, "T-904", "").unwrap(), 1);
    let t904 = tree(&root, "T-904");
    assert!(t904.is_dir(), "tree must survive the refusal");
    assert!(branch_exists(&root, "T-904"), "branch must survive");
    // NON-VACUITY: `--force` is the documented override, so the guard is not merely unreachable.
    assert_eq!(cmd_drop(&root, "T-904", "--force").unwrap(), 0);
    assert!(!tree(&root, "T-904").exists() && !branch_exists(&root, "T-904"));
}

#[test]
fn drop_refuses_a_dirty_tree_even_when_nothing_is_unmerged() {
    // THE T-352 SHAPE EXACTLY: the work all landed (`main..branch` == 0) so Guard A abstains,
    // and an agent is still writing. Guard A alone was measured SILENT here.
    let root = scratch("dropdirty");
    landed(&root, "T-905");
    let ahead = count(&gp(&root, &["rev-list", "--count", "main..slice/T-905"]).unwrap());
    assert_eq!(ahead, 0, "precondition: Guard A must have nothing to say");
    fs::write(tree(&root, "T-905").join("inflight.txt"), "unstaged").unwrap();
    let rc = cmd_drop(&root, "T-905", "").unwrap();
    assert_eq!(rc, 1, "GUARD B must fire");
    let f = tree(&root, "T-905").join("inflight.txt");
    assert!(f.exists(), "unstaged work destroyed — unrecoverable");
}

#[test]
fn reap_guards_every_destructive_case_in_one_pass() {
    // All five refusals and the one deletion, in a single `reap` over a single repo — which is
    // also how the bash meets them (a loop over every tree), so an early `continue` that
    // swallowed the rest of the list would show up here.
    let root = scratch("reapall");
    // 1. UNSTARTED — THE FIVE-WORKTREE INCIDENT. A fresh tree sits at the branch point, so
    //    `is-ancestor` is trivially TRUE and the old code reaped exactly this. GUARD 2.
    assert_eq!(cmd_new(&root, "T-907").unwrap(), 0);
    // 2. DIRTY but merged — GUARD 1.
    landed(&root, "T-908");
    fs::write(tree(&root, "T-908").join("inflight.txt"), "unstaged").unwrap();
    // 3. UNMERGED commits — kept as "N commit(s) not merged into main".
    assert_eq!(cmd_new(&root, "T-909").unwrap(), 0);
    commit(&tree(&root, "T-909"), "a.txt", "a");
    // 4. NO BRANCH — a bare directory. Plus a README.md, which the `*/` glob must not match.
    fs::create_dir_all(root.join(BASE).join("T-911")).unwrap();
    fs::write(root.join(BASE).join("README.md"), "docs").unwrap();
    // 5. LOCKED and merged — GUARD 3, git's own refusal (`worktree remove` needs `--force
    //    --force` for a locked tree, and even `drop`'s single `--force` would not be enough).
    landed(&root, "T-913");
    assert_eq!(g(&root, &format!("worktree lock {BASE}/T-913")).0, 0);
    // 6. MERGED and CLEAN — the one that must actually be reaped. NON-VACUITY for all of the
    //    above: without this the test would pass on a `cmd_reap` that did nothing at all.
    landed(&root, "T-910");

    assert_eq!(cmd_reap(&root).unwrap(), 0);

    // All five set up above must have SURVIVED; only the sixth may be gone.
    for t in ["T-907", "T-908", "T-909", "T-911", "T-913"] {
        let kept = tree(&root, t).is_dir();
        assert!(kept, "{t} was reaped — a guard did not fire");
    }
    let inflight = tree(&root, "T-908").join("inflight.txt");
    assert!(inflight.exists(), "GUARD 1: uncommitted work destroyed");
    assert!(branch_exists(&root, "T-907") && branch_exists(&root, "T-909"));
    assert!(branch_exists(&root, "T-913"));
    let readme = root.join(BASE).join("README.md");
    assert!(readme.exists(), "the `*/` glob matched README.md");
    assert!(!tree(&root, "T-910").exists(), "reap never reaps anything");
    assert!(!branch_exists(&root, "T-910"));
}

#[test]
fn ancestor_clause_is_inert() {
    // Pins the arithmetic behind the DEAD CLAUSE in `cmd_reap`: whenever `is-ancestor(b, main)`
    // holds, `merge-base(b, main)` IS `b`, so the `!=` can never be true.
    let root = scratch("ancestor");
    landed(&root, "T-912");
    let b = "slice/T-912";
    let anc = format!("merge-base --is-ancestor {b} main");
    let rc = g(&root, &anc).0;
    assert_eq!(rc, 0, "precondition: b is an ancestor of main");
    let tip = g(&root, &format!("rev-parse {b}")).1;
    let mb = g(&root, &format!("merge-base {b} main")).1;
    let (tip, mb) = (tip.trim(), mb.trim());
    assert_eq!(tip, mb, "if these differed the clause would fire");
}
