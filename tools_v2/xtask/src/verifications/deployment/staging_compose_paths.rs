//! T-438 / T-461 — the `deploy-staging.sh` compose-path pin (T-853 port of
//! `scripts/mod/verify-t438-deploy-staging-compose-path.sh`).
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Staging deploys must point `docker compose -f` at `apps/website/docker-compose.staging.yml`
//! (T-251), never at the `apps/website/api/` sibling that used to live there. Getting it wrong
//! does not fail loudly — compose happily starts *a* stack from *a* file, so the deploy goes green
//! and staging quietly runs the wrong topology. Hence a static pin rather than a smoke test.
//!
//! T-461 (wave 23 adversarial) found the previous Class-R false-green, in the script's own words:
//!
//! > (1) a `//` / `#` comment containing the good path counted as presence;
//! > (2) only one exact `cd '$TBD_REMOTE_DIR/apps/website/api'` string was banned, so live could
//! >     use api/compose while dry-run stayed good (or good path lived only in a comment).
//!
//! So the gate strips comments first, then requires the good `-f` path on **both** the dry-run
//! `echo` and the live `ssh_cmd` line, requires those two to agree with each other, and separately
//! rejects the stale api/ path on either. "Dry-run says one thing, live does another" is the
//! failure this shape exists to catch: a dry run is the only rehearsal anyone gets.
//!
//! OWNS WIDEN (from the script): wave_plan T-438 lists only `scripts/mod/deploy-staging.sh`; this
//! is the Class-R perturbation guard for that path contract. T-461 owns the script hardening.
//!
//! ── WHAT THE PORT REMOVES ────────────────────────────────────────────────────────────────────
//!
//! 1. **`python3`.** The bash delegated comment-stripping and quote-safe `-f` extraction to a
//!    heredoc'd Python program — the single reason this gate is on `scripts/python-inventory.txt`.
//!    `strip_shell_comments` and `f_path` are that program in Rust, unit-tested, which a
//!    heredoc never could be. **That inventory line can go**; nothing here shells out at all.
//! 2. **A fail-quiet that printed nothing.** `pin_out="$(… python3 …)" || pin_rc=$?` captured only
//!    stdout, so every way Python could die before its own `print`s — absent interpreter (127),
//!    non-UTF-8 input, an unreadable file — left `pin_out` EMPTY: the operator got `verify-…:
//!    FAIL` with **zero diagnosis on stdout** and a traceback on stderr that `wave.sh`'s `tail
//!    -15` may or may not have reached. Closed on *status*, the important half, but mute. Those
//!    states are named `NotRun` causes now, printed with the rest of the report.
//! 3. **`pin_rc` observable only as "non-zero".** The script header claims `2 = internal error`,
//!    but the Python never exits 2 — an unhandled exception exits 1 — so "found a violation" and
//!    "never examined the file" were indistinguishable. They are [`Verdict::Failed`] and
//!    [`Verdict::DidNotRun`] here, and the distinction survives even though the status stays 0/1.
//!
//! It does NOT remove any `2>/dev/null` or `|| true`: this script had none, and it stats `$FILE`
//! before reading it. Those fail-open shapes were already absent; claiming otherwise is theatre.
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! `wave.sh`'s `run()` captures `"$@" 2>&1` and prints `tail -15` of a failed step, so every line
//! below is operator-facing evidence. Acceptance was a byte-for-byte stdout+stderr+rc diff against
//! the script on a clean tree and on 13 perturbed throwaway roots (see `tests::bites`). Status
//! stays bash's binary 0/1 — see [`verify_t438`].
//!
//! ── WHEN `deploy-staging.sh` ITSELF IS PORTED ────────────────────────────────────────────────
//!
//! T-853 ports `scripts/mod/deploy-staging.sh` later in the same program, and the file this gate
//! reads then stops being shell. **Everything about *what* is pinned lives in the consts below** —
//! [`DEPLOY_SCRIPT`], [`GOOD_PATH`], [`BAD_PATH`], [`CD_INTO_API_SQ`], [`CD_INTO_API_DQ`],
//! [`DRY_RUN_KEY`], [`LIVE_KEY`] — and every message is `format!`ed from them. Repointing:
//! change [`DEPLOY_SCRIPT`]; if the new host is not shell, replace `strip_shell_comments` and
//! the two `cd` bans — a Rust deploy driver has no `cd` line to ban, it has a
//! `Command::current_dir`, and the ban should follow it there rather than be deleted.

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use verification_core::{Finding, Kind, NotRun, Pattern, Verdict, gate};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// Printed on the PASS/FAIL summary line: the script's filename stem verbatim, because operator
/// logs and `cargo xtask verify t438` transcripts are grepped for exactly this string.
const GATE_NAME: &str = "verify-t438-deploy-staging-compose-path";

/// Source containing both the dry-run plan and live SSH compose invocation, repo-relative.
/// The transport facade delegates to this module; auditing the facade alone would miss both
/// command strings and cannot establish that rehearsal and execution use the same compose file.
const DEPLOY_SCRIPT: &str = "tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs";

/// The one true compose file (T-251). Double duty exactly as in the bash: the string that must
/// follow `-f`, **and** — joined onto the repo root — the file that must exist. The script spelled
/// it twice (`$COMPOSE`, `$GOOD_PATH`); two spellings of one contract drift, so there is one here.
const GOOD_PATH: &str = "apps/website/docker-compose.staging.yml";

/// The stale pre-T-251 location. Must appear on neither compose line and must not exist on disk —
/// a file left there is what makes the wrong `-f` path a *plausible* edit rather than an obvious
/// typo, so the gate removes the temptation as well as the reference.
const BAD_PATH: &str = "apps/website/api/docker-compose.staging.yml";

/// Banned outright: `cd`-ing the remote shell into `api/` before compose. Both quotings, because
/// T-461's finding was that banning one exact string is banning nothing.
const CD_INTO_API_SQ: &str = "cd '$TBD_REMOTE_DIR/apps/website/api'";
/// The double-quoted twin of [`CD_INTO_API_SQ`].
const CD_INTO_API_DQ: &str = r#"cd "$TBD_REMOTE_DIR/apps/website/api""#;

/// How the dry-run compose line is recognised: the script prints its plan with this prefix.
const DRY_RUN_KEY: &str = "[dry-run]";
/// How the live compose line is recognised: the remote-exec helper it is handed to.
/// Names the live dispatch in operator-facing messages only — it is NOT a matcher any more.
///
/// T-853: was `ssh_cmd`, bash's one-line ssh wrapper, and `line.contains(LIVE_KEY)` was how the
/// live compose invocation was found. The Rust equivalent is `Runner::ssh_ok`, whose call spans
/// lines, so classification moved to "does this compose line carry the dry-run marker". This const
/// survives so the failure text still tells the reader WHICH invocation is wrong.
const LIVE_KEY: &str = "ssh_ok";

/// The two compose invocations, classified out of the stripped source.
///
/// `None` is itself a finding: the script treated a *missing* dry-run or live compose line as a
/// failure, not as vacuously satisfied. T-556 anti-vacuity applied to the gate's own inputs — a
/// gate that goes quiet because the thing it audits was deleted checks nothing.
struct ComposeLines<'a> {
    dry: Option<&'a str>,
    live: Option<&'a str>,
}

#[cfg(test)]
#[path = "tests/staging_compose_paths/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_t438;

#[cfg(test)]
use source_audit::{audit, f_path, f_regex, script_basename, strip_shell_comments};
