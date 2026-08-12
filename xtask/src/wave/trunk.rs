//! `trunk build --release`, ISOLATED FROM THE OPERATOR'S DEV SERVER — read before simplifying.
//!
//! `cargo xtask mk leptos` is `trunk serve --release`: the same binary, running the same pipeline, over the
//! same crate, continuously, for hours. Two trunks over one working tree collide, and the collision
//! reads exactly like a code defect — the worst failure shape there is, because an unattended fix
//! agent burns its whole retry budget on working code.
//!
//! MEASURED 2026-07-26 against the pinned trunk 0.21.14, two release builds started together:
//! ```text
//!   shared dist + shared CARGO_TARGET_DIR   COLLIDED IN ALL 5 TRIALS — 4 lost by the gate, 1 by the
//!                                           dev server, every one of them
//!                                             "running wasm-opt / error copying (optimized) wasm
//!                                              file to dist dir / No such file or directory (os
//!                                              error 2)"
//!                                           which is the reported symptom byte for byte
//!   private --dist only                     15/15 clean. The ticket records this as a disproven
//!                                           dead end; it did not reproduce here. Do not read that
//!                                           as safe — $CARGO_TARGET_DIR/wasm-opt/<profile>/
//!                                           website-frontend_bg.wasm is still one path two writers
//!                                           share, and the adversarial verifier lost a build there.
//!                                           A window that is merely narrow is the exact thing that
//!                                           costs an unattended agent its retry budget.
//!   private --dist AND CARGO_TARGET_DIR     15/15 clean, then 10 consecutive gate builds clean with
//!                                           `trunk serve --release` live on :3000 throughout. This
//!                                           one is not luck: after both flags there is no path both
//!                                           writers can name.
//! ```
//!
//! THE WHOLE TRUNK WORKING SET, enumerated from 0.21.14 rather than assumed:
//! ```text
//!   <dist>/.stage                              staging — trunk removes and recreates this at the
//!                                              START of every build and deletes it at the end,
//!                                              which is what "error writing JS loader file to stage
//!                                              dir / No such file or directory" was
//!   <dist>/*                                   the applied distribution
//!   $CARGO_TARGET_DIR/wasm32-unknown-unknown/  cargo output
//!   $CARGO_TARGET_DIR/wasm-bindgen/<profile>/  bindgen staging
//!   $CARGO_TARGET_DIR/wasm-opt/<profile>/      wasm-opt staging   <- the one --dist does NOT move
//! ```
//! There is no staging env var to set. `TRUNK_STAGING_DIR` exists in the binary only as a variable
//! trunk EXPORTS to build hooks, never one it reads; staging is `<final dist>/.stage`, so `--dist`
//! is what isolates it. `--dist` plus `CARGO_TARGET_DIR` is therefore the COMPLETE set — nothing
//! else is written outside the read-only tool cache in `~/.cache/trunk`. That completeness is the
//! argument, not the trial count: a private dist alone left one shared writable path and so could
//! only ever be lucky, while both flags together leave none, which is why this is a cure and not a
//! mitigation. The operator's dev server stays up and serving on :3000 throughout.
//!
//! CORRECTION, measured: the comment this replaces claimed the gate provokes the race itself,
//! because `touch_changed` bumps mtime on ~18 frontend files "which is exactly what trunk serve
//! watches". FALSE for 0.21.14 — touching 18 real `.rs` files whose CONTENT was unchanged produced
//! no rebuild at all, twice over; only a content change wakes the watcher. Deleting `touch_changed`
//! would therefore not have cured anything: the operator's own edit or a merge wakes a
//! multi-minute rebuild, and the gate can arrive at any point inside it.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::changed::realpath_m;
use super::{Ctx, host};
use crate::{wprint, wprintln};

pub fn gate_trunk_build(ctx: &Ctx) -> i32 {
    let fdir = ctx.root.join("apps/website/frontend");
    // Refuse to build UN-ISOLATED rather than race. Once either private path is collapsed onto a
    // shared one, every trunk failure past this line is an environment race wearing a compile
    // error's clothes, and the agent reading it has no way to tell.
    //
    // TWO CORRECTIONS, both measured 2026-07-26:
    //   * The dist this guard must protect is the one `trunk serve` OWNS, which is MAIN's — but
    //     $ROOT inside a worktree is the WORKTREE, so the old compare checked
    //     .ai/artifacts/worktrees/T-nnn/apps/website/frontend/dist and never looked at the path the
    //     dev server actually writes. Check both: main's (the collision that matters) and this
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
            .join("apps/website/frontend/dist")
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
    // NO SLACK on t0, and the 5 s that used to be here is REMOVED rather than reduced. `date +%s`
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
