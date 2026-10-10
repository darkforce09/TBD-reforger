//! The wave gate's `trunk build --release`, isolated from the operator's dev server.
//!
//! **Role:** `gate_trunk_build` runs the release Trunk build of the SPA in the gate's private dist
//! and target folders and checks that it produced fresh output.
//!
//! **Position:** `mk gate-step trunk-build`, which the ticket manager's wave gate runs when the
//! SPA's scope changed this wave (`when_scope = "wasm_scope"`); the folders are
//! `Ctx::gate_trunk_dist` and `Ctx::gate_trunk_target`.
//!
//! **Signals & state:** none held; spawns trunk through the host bridge and reads the dist folder.
//!
//! **Invariants:** `cargo xtask mk leptos` runs `trunk serve --release` over the same crate, and
//! two trunks sharing a dist or a target folder collide in ways that read like code defects; Trunk
//! writes only `<dist>/.stage`, `<dist>/*` and its `CARGO_TARGET_DIR` subfolders (`wasm-bindgen`,
//! `wasm-opt`, the wasm32 output), so a private `--dist` together with a private `CARGO_TARGET_DIR`
//! leaves no path both writers name; the gate refuses build folders that are not private; the build
//! passes only when its dist and its `wasm-opt` output hold files strictly newer than its start.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::changed::realpath_m;
use super::host;
use super::step_context::Ctx;
use super::{wprint, wprintln};

pub(crate) fn gate_trunk_build(ctx: &Ctx) -> i32 {
    let fdir = ctx.root.join("crates/frontend/shell/frontend_application");
    // Refuse to build UN-ISOLATED rather than race. Once either private path is collapsed onto a
    // shared one, every trunk failure past this line is an environment race wearing a compile
    // error's clothes, and the agent reading it has no way to tell.
    //
    // TWO CORRECTIONS, both measured 2026-07-26:
    //   * The dist this guard must protect is the one `trunk serve` OWNS, which is MAIN's — but
    //     $ROOT inside a worktree is the WORKTREE, so the old compare checked
    //     .ai/artifacts/worktrees/T-nnn/crates/frontend/shell/frontend_application/dist and never
    //     looked at the path the dev server actually writes. Check both: main's (the collision that matters) and this
    //     tree's (still not somewhere a gate should be writing).
    //   * Both compares were plain strings, so a symlink or a `./` spelling of the same directory
    //     walked straight through a guard whose entire job is "are these two the same place".
    //     Canonicalise first. `readlink -f` resolves symlinks and normalises lexically, and still
    //     answers for a path that does not exist yet (the gate's private dirs on a cold machine).
    // Only reachable by setting TBD_GATE_TRUNK_DIST/TARGET — the defaults never collapse — which is
    // precisely why it must be right: the one caller who ever trips it is overriding on purpose.
    let c_gt = canon(&ctx.gate_trunk_target);
    let c_gd = canon(&ctx.gate_trunk_dist);
    let c_shared = canon(&ctx.cargo_target_dir);
    let c_serve = canon(
        &ctx.main_root
            .join("crates/frontend/shell/frontend_application/dist")
            .display()
            .to_string(),
    );
    let c_wt = canon(&fdir.join("dist").display().to_string());
    if c_gt == c_shared || c_gd == c_serve || c_gd == c_wt {
        wprintln!(
            "trunk: gate build paths are not private — refusing to race the operator's dev server."
        );
        wprintln!(
            "        gate target={}  ->  {}",
            ctx.gate_trunk_target,
            c_gt
        );
        wprintln!(
            "        gate dist  ={}    ->  {}",
            ctx.gate_trunk_dist,
            c_gd
        );
        wprintln!("        shared cargo target = {c_shared}");
        wprintln!(
            "        dev server's dist   = {c_serve}   (main — this is the one trunk serve owns)"
        );
        wprintln!("        this tree's dist    = {c_wt}");
        return 1;
    }
    let t0 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // `return $?`, not `return 1`: hostrun applies the timeout host-side and the step runner
    // reports 124 as "FAIL (TIMEOUT)" rather than a build failure. Flattening it here would
    // relabel the single most expensive step's timeout as a code error — the same category mistake
    // this whole function is about.
    //
    // MEASURED 2026-07-26: Cursor/agent shells export NO_COLOR=1. trunk 0.21.14's clap binds that
    // env to `--no-color` and then rejects the value `1` (`possible values: true, false`), so the
    // wave gate printed `trunk build FAIL` over a healthy tree. Unset for this step only.
    let script = format!(
        "cd '{}' && unset NO_COLOR && CARGO_TARGET_DIR='{}' trunk build --release --dist '{}'",
        fdir.display(),
        ctx.gate_trunk_target,
        ctx.gate_trunk_dist
    );
    let (out, rc) = host::capture(&ctx.host.hostrun_argv(&host::v(&["sh", "-c", &script])));
    wprint!("{out}");
    if rc != 0 {
        return rc;
    }

    // NON-VACUITY. Exit 0 only says trunk was happy; it does not say trunk HONOURED either flag. A
    // Trunk.toml key, a config-precedence change on upgrade, or one dropped quote in the sh -c above
    // would put the output back into the shared paths and the isolation would be gone SILENTLY —
    // the gate would keep printing PASS right up until the day it raced again. So prove it every
    // run: both private paths must have taken a write from THIS build.
    //
    // NO SLACK on t0: there is no grace window here at all, not even a reduced one. `date +%s`
    // truncates downward, so t0 <= the real start instant T0; the build takes minutes, so every file
    // it writes has mtime T_w > T0 >= t0; and `-newermt` is STRICTLY greater (verified 2026-07-26: a
    // file whose mtime equals the argument does not match). So T_w > t0 holds with certainty and the
    // slack bought nothing. It cost something, though: `@$((t0 - 5))` accepted a wasm written up to
    // five seconds BEFORE this build started — i.e. exactly the stale artifact from a just-finished
    // build that this guard exists to reject. The one assumption is sub-second mtime granularity;
    // measured on the real gate paths, both are btrfs recording nanoseconds.
    if newer_than(Path::new(&ctx.gate_trunk_dist), t0, |n| {
        n.ends_with("_bg.wasm")
    })
    .is_none()
    {
        wprintln!(
            "trunk: reported success but {} holds no wasm from this run.",
            ctx.gate_trunk_dist
        );
        wprintln!(
            "        --dist was not honoured — the gate is writing into a dist the dev server owns."
        );
        return 1;
    }
    let wasm_opt = Path::new(&ctx.gate_trunk_target).join("wasm-opt");
    if newer_than(&wasm_opt, t0, |n| n.ends_with(".wasm")).is_none() {
        wprintln!(
            "trunk: reported success but {}/wasm-opt holds no wasm from this run.",
            ctx.gate_trunk_target
        );
        wprintln!(
            "        CARGO_TARGET_DIR was not honoured — wasm-opt staging is shared with the dev server."
        );
        return 1;
    }
    0
}

/// `readlink -f -- "$p" 2>/dev/null || printf '%s' "$p"` — resolve symlinks when possible, and
/// still answer for a path that does not exist yet.
fn canon(p: &str) -> String {
    match std::fs::canonicalize(p) {
        Ok(c) => c.display().to_string(),
        Err(_) => realpath_m(Path::new(p)).display().to_string(),
    }
}

/// `find <dir> -name <pat> -newermt "@$t0" | head -1` — STRICTLY newer than `t0`.
fn newer_than(dir: &Path, t0: u64, name_ok: impl Fn(&str) -> bool) -> Option<PathBuf> {
    for e in walkdir::WalkDir::new(dir).into_iter().flatten() {
        if !e.file_type().is_file() {
            continue;
        }
        let n = e.file_name().to_string_lossy().into_owned();
        if !name_ok(&n) {
            continue;
        }
        let Ok(md) = e.metadata() else { continue };
        let Ok(mt) = md.modified() else { continue };
        let Ok(d) = mt.duration_since(UNIX_EPOCH) else {
            continue;
        };
        // `-newermt` compares with sub-second precision and is strict, so a file whose mtime equals
        // the argument does not match.
        if d.as_secs() > t0 || (d.as_secs() == t0 && d.subsec_nanos() > 0) {
            return Some(e.path().to_path_buf());
        }
    }
    None
}
