//! T-742 — ad-hoc `cargo test` into a PER-SLICE private `CARGO_TARGET_DIR`.
//!
//! DEFECT: concurrent slice worktrees export the same shared cache
//! (`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target` / `$MAIN_ROOT/target`). `cargo test` BUILDS
//! AND THEN RUNS a binary, so one worktree can execute another's `website_frontend-<hash>` (T-649
//! live: arsenal.rs:4733 failure that did not exist in that tree; line tracked a sibling). The
//! per-slice gate and the wave-gate `test frontend` step already use private dirs; ad-hoc agent
//! invocations did not, and the brief only ADVISED a private dir — nothing enforced it.
//!
//! THIS PATH: `wave test --slice T-nnn -p <crate> …` pins `$HOME/.cache/tbd-target-<SLICE>`
//! (override with `TBD_ADHOC_TARGET_DIR` only when it resolves to that same default, or to a
//! non-`T-*` verifier path — see F2 below), refuses `/tmp` and any collapse onto the TRUE shared
//! roots (`$HOME/.cache/tbd-target` and `$MAIN_ROOT/target` — NEVER against whatever
//! `CARGO_TARGET_DIR` currently holds; that false-refused the sanctioned per-slice path when an
//! agent had already exported it), mtime-bumps via `touch_changed` (same fingerprint cure as the
//! gate), and runs with `CARGO_INCREMENTAL=0`. It does NOT take the shared gate lock — that lock
//! serialises the SHARED gate dirs (`target-gate-*`); isolation here is the private directory
//! itself (measured: frontend-only private dir ~2.7 GB, not a 57 GB shared-cache clone). Cargo does
//! NOT rebuild across worktrees that share a target dir — the private path is the mitigator;
//! shared-dir `cargo test` remains the foreign-binary class.
//!
//! Keep it lean: require an explicit `-p` / `--package` among args. Delete the private dir before
//! reporting — never print that for a foreign-slice or live-worktree path.

use std::path::{Path, PathBuf};

use super::changed::realpath_m;
use super::{Ctx, host, touch};
use crate::{werr, wprintln};

pub fn cmd_test(ctx: &Ctx, argv: &[String]) -> u8 {
    let mut tid = String::new();
    let mut args: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < argv.len() {
        match argv[i].as_str() {
            "--slice" => {
                tid = argv.get(i + 1).cloned().unwrap_or_default();
                if tid.is_empty() {
                    werr!("test: REFUSING — --slice needs a ticket id (T-nnn)");
                    return 2;
                }
                i += 2;
            }
            "--" => {
                // Keep cargo's `--` separator (e.g. `… -- --list`). Dropping it made `cargo test`
                // see `--list` as its own flag and refuse.
                args.push("--".into());
                i += 1;
                while i < argv.len() {
                    args.push(argv[i].clone());
                    i += 1;
                }
                break;
            }
            other => {
                args.push(other.to_string());
                i += 1;
            }
        }
    }
    if tid.is_empty() {
        wprintln!("test: REFUSING — --slice T-nnn is required.");
        wprintln!("        Bare `cargo test` against the shared CARGO_TARGET_DIR is the T-742");
        wprintln!("        cross-worktree false-binary class. Sanctioned path:");
        wprintln!("          bash scripts/platform/wave.sh test --slice T-742 -p website-frontend");
        return 2;
    }
    // `case "$tid" in [Tt]-[0-9]*)`
    let shaped = (tid.starts_with("T-") || tid.starts_with("t-"))
        && tid[2..]
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false);
    if !shaped {
        werr!("test: REFUSING — slice id '{tid}' (expected T-nnn)");
        return 2;
    }
    // Normalise t-742 → T-742 without touching digits. `${tid#*[Tt]-}` strips through the FIRST
    // `T-`/`t-`.
    let tid = format!("T-{}", strip_through_first_t_dash(&tid));

    if args.is_empty() {
        wprintln!("test: REFUSING — pass cargo test args (at least -p <crate>).");
        wprintln!(
            "        An unbounded invocation would inflate the private dir toward a full workspace"
        );
        wprintln!("        build. Keep ad-hoc dirs lean (frontend-only measured ~2.7 GB).");
        wprintln!(
            "        Example: bash scripts/platform/wave.sh test --slice {tid} -p website-frontend"
        );
        return 2;
    }

    // NIT: prose said "at least -p <crate>"; enforce it — non-empty args without -p/--package still
    // accept unbounded / mis-aimed invocations that inflate the private dir.
    let has_pkg = args.iter().any(|a| {
        a == "-p"
            || a == "--package"
            || (a.starts_with("-p") && a.len() > 2)
            || a.starts_with("--package=")
    });
    if !has_pkg {
        wprintln!("test: REFUSING — cargo test args must include -p / --package <crate>.");
        wprintln!(
            "        Example: bash scripts/platform/wave.sh test --slice {tid} -p website-frontend"
        );
        return 2;
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let default_priv = format!("{home}/.cache/tbd-target-{tid}");
    let adhoc_env = std::env::var("TBD_ADHOC_TARGET_DIR").unwrap_or_default();
    let privd = if adhoc_env.is_empty() {
        default_priv.clone()
    } else {
        adhoc_env.clone()
    };
    if privd.starts_with("/tmp/") || privd.starts_with("/var/tmp/") {
        wprintln!("test: REFUSING — private target dir must not be under /tmp ({privd}).");
        wprintln!("        Host-native rule: never /tmp for CARGO_TARGET_DIR.");
        return 2;
    }
    if std::fs::create_dir_all(&privd).is_err() {
        werr!("test: cannot create {privd}");
        return 2;
    }
    let _ = std::fs::create_dir_all(&default_priv);

    let priv_r = canon(&privd);
    let default_r = canon(&default_priv);
    // F1: compare ONLY against true shared roots — never against whatever CARGO_TARGET_DIR
    // currently holds (that false-refused when env already pointed at the per-slice private dir).
    let cache_r = canon(&format!("{home}/.cache/tbd-target"));
    let main_r = canon(&ctx.main_root.join("target").display().to_string());
    if priv_r == cache_r || priv_r == main_r {
        wprintln!(
            "test: REFUSING — private dir collapsed onto the shared CARGO_TARGET_DIR ({priv_r})."
        );
        wprintln!(
            "        That is exactly the T-742 defect. Unset TBD_ADHOC_TARGET_DIR or point it at"
        );
        wprintln!("        a per-slice path under $HOME/.cache/tbd-target-{tid}.");
        return 2;
    }

    // F2: TBD_ADHOC_TARGET_DIR must resolve to this slice's default OR a non-`T-*` verifier path
    // (basename lacks `tbd-target-T-<digits>` — e.g. `tbd-target-wave138-verify`). A foreign-slice
    // `tbd-target-T-739` under `--slice T-999` is REFUSED — never print rm -rf for it.
    let base = Path::new(&priv_r)
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let token = adhoc_token(&base);
    if !adhoc_env.is_empty() && priv_r != default_r {
        if let Some(tok) = &token {
            wprintln!(
                "test: REFUSING — TBD_ADHOC_TARGET_DIR is not the default per-slice path ({priv_r})."
            );
            if *tok != tid {
                wprintln!("        Foreign-slice token '{tok}' != --slice '{tid}'.");
            }
            wprintln!(
                "        Allowed overrides: $HOME/.cache/tbd-target-{tid}, or a non-T-* verifier"
            );
            wprintln!("        path (e.g. $HOME/.cache/tbd-target-wave138-verify).");
            return 2;
        }
        // token empty → non-T-* verifier path — allowed (documented above).
    }

    // Never advertise rm -rf for a path whose ticket token differs from --slice or is a live
    // worktree's foreign cache. Default per-slice for THIS tid + non-T-* verifier OK.
    //
    // PRESERVED AS WRITTEN: the bash sets `allow_rm=0` unconditionally inside the foreign-token
    // branch AND repeats the same test twice more afterwards ("defence in depth"), so the
    // live-worktree scan cannot change the outcome. Reproduced rather than simplified — the
    // redundant arms are how the bash guarantees a foreign token never reaches the banner.
    let mut allow_rm = true;
    if token.as_deref().map(|t| t != tid).unwrap_or(false) {
        allow_rm = false;
    }
    if token.is_none() || priv_r == default_r {
        allow_rm = true;
    }
    // Foreign token always blocks the banner even if somehow past the refuse (defence in depth).
    if token.as_deref().map(|t| t != tid).unwrap_or(false) {
        allow_rm = false;
    }

    wprintln!("═══ ad-hoc test {tid} ═══");
    wprintln!("CARGO_TARGET_DIR={priv_r}  (private — not the shared cache)");
    if allow_rm {
        wprintln!("delete before report: rm -rf '{priv_r}'");
    } else {
        wprintln!("delete before report: (omitted — path token is foreign or live; do not rm -rf)");
    }

    // Same mtime-bump the gate uses so a WARM private dir cannot keep fingerprints from before this
    // worktree's own edits. Cross-worktree isolation is the private dir; this covers the
    // same-worktree stale-fingerprint half of the pattern.
    let rc = touch::touch_changed("");
    if rc != 0 {
        return rc as u8;
    }

    // hostrun + explicit env: distrobox-host-exec does not forward the shell environment.
    let mut cmd: Vec<String> = vec![
        "env".into(),
        format!("CARGO_TARGET_DIR={priv_r}"),
        "CARGO_INCREMENTAL=0".into(),
        "cargo".into(),
        "test".into(),
    ];
    cmd.extend(args);
    host::inherit(&ctx.host.hostrun_argv(&cmd)) as u8
}

/// `${tid#*[Tt]-}` — drop everything through the first `T-` or `t-`.
fn strip_through_first_t_dash(s: &str) -> String {
    let b = s.as_bytes();
    for i in 0..b.len().saturating_sub(1) {
        if (b[i] == b'T' || b[i] == b't') && b[i + 1] == b'-' {
            return s[i + 2..].to_string();
        }
    }
    s.to_string()
}

/// `sed -n 's/^tbd-target-\([Tt]-[0-9][0-9]*\).*/\1/p'` then the same `T-` normalisation.
fn adhoc_token(base: &str) -> Option<String> {
    let rest = base.strip_prefix("tbd-target-")?;
    let head = rest.as_bytes();
    if head.len() < 3 {
        return None;
    }
    if !(head[0] == b'T' || head[0] == b't') || head[1] != b'-' {
        return None;
    }
    let digits: String = rest[2..].chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    Some(format!("T-{digits}"))
}

/// `readlink -f -- "$p" 2>/dev/null || printf '%s' "$p"`.
fn canon(p: &str) -> String {
    match std::fs::canonicalize(p) {
        Ok(c) => c.display().to_string(),
        Err(_) => realpath_m(&PathBuf::from(p)).display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_id_normalisation_matches_the_parameter_expansion() {
        assert_eq!(strip_through_first_t_dash("T-742"), "742");
        assert_eq!(strip_through_first_t_dash("t-742"), "742");
        // `${tid#*[Tt]-}` is the SHORTEST match, so a stray earlier `t-` wins.
        assert_eq!(strip_through_first_t_dash("xt-9T-742"), "9T-742");
    }

    #[test]
    fn foreign_slice_tokens_are_recognised_so_the_banner_can_be_withheld() {
        assert_eq!(adhoc_token("tbd-target-T-739"), Some("T-739".into()));
        assert_eq!(
            adhoc_token("tbd-target-t739"),
            None,
            "the sed needs the dash"
        );
        assert_eq!(adhoc_token("tbd-target-wave138-verify"), None);
    }
}
