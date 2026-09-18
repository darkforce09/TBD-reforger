use super::*;

pub(super) fn cmd_land(root: &Path) -> u8 {
    let Some(w) = current_wave(root) else {
        return 2;
    };
    if w == "done" {
        println!("nothing to land");
        return 0;
    }
    let mut merged = 0usize;
    let mut skipped = 0usize;

    // 1. Refuse dirty trees (uncommitted work would be lost).
    for s in wave_slices(root, &w) {
        if tree_state(root, &s) == TreeState::Dirty {
            eprintln!("REFUSING: {s} worktree has uncommitted changes.");
            // bash oddity: BASE/$s not parent_slice; status --short redirected to stderr
            let d = root.join(BASE).join(&s);
            if let Ok(o) = Command::new("git")
                .args(["-C", d.to_str().unwrap_or(""), "status", "--short"])
                .output()
            {
                eprint!("{}", String::from_utf8_lossy(&o.stdout));
            }
            return 1;
        }
    }

    // 2. Merge every slice that actually has commits.
    for s in wave_slices(root, &w) {
        if has_work(root, &s) {
            println!("── merging {s}");
            // T-853: was `bash scripts/mod/slice-worktree.sh merge <slice>`, in-process now.
            // The port CLOSED a fail-open here that this caller depended on: bash's dirty check
            // used plain git, and a `git status` exiting 128 produced an empty substitution that
            // `[ -n … ]` read as CLEAN — so a dirty worktree merged and the work was destroyed.
            let ok = crate::commands::platform::slice_worktree::run_at(
                root,
                &["merge".to_string(), s.clone()],
            )
            .map(|rc| rc == 0)
            .unwrap_or(false);
            if ok {
                merged += 1;
            } else {
                eprintln!("MERGE FAILED for {s} — resolve manually, then re-run land");
                return 1;
            }
        } else {
            println!("── skipping {s} (no commits)");
            skipped += 1;
        }
    }
    println!("merged {merged}, skipped {skipped}");

    // 3. Gate before reap.
    if cmd_gate(root) != 0 {
        println!();
        eprintln!(
            "Gate FAILED after merge. Worktrees kept for inspection. Fix on main, re-run: cargo run -q -p xtask -- mod wave gate"
        );
        return 1;
    }

    // 4. Reap.
    println!();
    // T-853: was `bash scripts/mod/slice-worktree.sh reap`, in-process now. `reap` is the
    // DESTRUCTIVE one, and the port left every guard intact: uncommitted work, "unstarted is not
    // merged" (the five-worktree incident), and git's own `worktree lock` refusal.
    let _ = crate::commands::platform::slice_worktree::run_at(root, &["reap".to_string()]);

    // 5. Push.
    println!();
    cmd_push(root)
}

pub(super) fn cmd_push(root: &Path) -> u8 {
    println!("═══ push ═══");
    let log = Command::new("git")
        .current_dir(root)
        .args(["log", "--oneline", "@{u}..HEAD"])
        .output();
    let n = match &log {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .count(),
        Err(_) => 0,
    };
    if n == 0 {
        println!("  nothing to push");
        return 0;
    }

    let diff = Command::new("git")
        .current_dir(root)
        .args(["diff", "--name-only", "@{u}..HEAD"])
        .output();
    let lfs = match &diff {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| l.starts_with("packages/map-assets/"))
            .count(),
        Err(_) => 0,
    };
    if lfs != 0 {
        eprintln!(
            "  REFUSING to bypass the LFS hook: {lfs} file(s) under packages/map-assets/ are in these"
        );
        eprintln!(
            "  commits and need real LFS objects uploaded. Install git-lfs, then: git push origin main"
        );
        return 1;
    }

    println!("  pushing {n} commit(s) (no LFS content — hook bypass is safe)");
    match Run::new("git")
        .args(["push", "--no-verify", "origin", "main"])
        .cwd(root)
        .merged_output()
    {
        Ok(m) => {
            let lines: Vec<&str> = m.text.lines().collect();
            let start = lines.len().saturating_sub(4);
            for line in &lines[start..] {
                println!("{line}");
            }
        }
        Err(nr) => {
            eprintln!("{nr:?}");
            return 1;
        }
    }
    0
}
