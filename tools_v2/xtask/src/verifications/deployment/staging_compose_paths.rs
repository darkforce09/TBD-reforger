//! The staging-deploy compose-path pin.
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Staging deploys must point `docker compose -f` at `apps/website/docker-compose.staging.yml`,
//! never at an `apps/website/api_v2/` sibling. Getting it wrong does not fail loudly — compose
//! happily starts *a* stack from *a* file, so the deploy goes green and staging quietly runs the
//! wrong topology. Hence a static pin rather than a smoke test.
//!
//! Two false-green shapes this gate is built to refuse:
//!
//! 1. a `//` or `#` comment containing the good path counting as presence;
//! 2. banning one exact `cd` string, so the live path can use the api/ compose file while the
//!    dry-run plan stays good.
//!
//! So the gate strips comments first, then requires the good `-f` path on **both** the dry-run
//! plan line and the live ssh line, requires those two to agree with each other, and separately
//! rejects the stale api/ path on either. "Dry-run says one thing, live does another" is the
//! failure this shape exists to catch: a dry run is the only rehearsal anyone gets.
//!
//! ── FAILED AND DID-NOT-RUN ARE DIFFERENT ─────────────────────────────────────────────────────
//!
//! "Found a violation" and "never examined the file" are different operator actions, so they are
//! [`Verdict::Failed`] and [`Verdict::DidNotRun`] here, and the distinction survives in the
//! printed report even though the exit status stays 0/1. Every `NotRun` cause — unreadable input,
//! missing target — is named on stdout with the rest of the report rather than left as a bare
//! non-zero status.
//!
//! ── OUTPUT AND STATUS ARE A CONTRACT ─────────────────────────────────────────────────────────
//!
//! The wave gate captures a step's stdout+stderr and prints its last 15 lines on failure, so every
//! line below is operator-facing evidence. Status is binary 0/1 — see
//! [`verify_staging_compose_paths`].
//!
//! ── REPOINTING THE PIN ───────────────────────────────────────────────────────────────────────
//!
//! **Everything about *what* is pinned lives in the consts below** — [`DEPLOY_SOURCE`],
//! [`GOOD_PATH`], [`BAD_PATH`], [`CD_INTO_API_SQ`], [`CD_INTO_API_DQ`], [`DRY_RUN_KEY`],
//! [`LIVE_KEY`] — and every message is `format!`ed from them. To follow the deploy driver
//! elsewhere, change [`DEPLOY_SOURCE`]; if the new host has no remote shell line, replace
//! `strip_comments` and the two `cd` bans with the equivalent for whatever sets the working
//! directory there, rather than deleting them.

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use verification_core::{Finding, Kind, NotRun, Pattern, Verdict, gate};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// Printed on the PASS/FAIL summary line: operator logs and
/// `cargo xtask verify staging-compose-paths` transcripts are grepped for exactly this string.
const GATE_NAME: &str = "staging-compose-paths";

/// Source containing both the dry-run plan and live SSH compose invocation, repo-relative.
/// The transport facade delegates to this module; auditing the facade alone would miss both
/// command strings and cannot establish that rehearsal and execution use the same compose file.
const DEPLOY_SOURCE: &str = "tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs";

/// The one true compose file. Double duty: the string that must follow `-f`, **and** — joined
/// onto the repo root — the file that must exist. Two spellings of one contract drift, so there
/// is exactly one here.
const GOOD_PATH: &str = "apps/website/docker-compose.staging.yml";

/// The stale location. Must appear on neither compose line and must not exist on disk — a file
/// left there is what makes the wrong `-f` path a *plausible* edit rather than an obvious typo,
/// so the gate removes the temptation as well as the reference.
const BAD_PATH: &str = "apps/website/api_v2/docker-compose.staging.yml";

/// Banned outright: `cd`-ing the remote shell into `api/` before compose. Both quotings, because
/// banning one exact string is banning nothing.
const CD_INTO_API_SQ: &str = "cd '$TBD_REMOTE_DIR/apps/website/api_v2'";
/// The double-quoted twin of [`CD_INTO_API_SQ`].
const CD_INTO_API_DQ: &str = r#"cd "$TBD_REMOTE_DIR/apps/website/api_v2""#;

/// How the dry-run compose line is recognised: the driver prints its plan with this prefix.
const DRY_RUN_KEY: &str = "[dry-run]";
/// Names the live dispatch (`Runner::ssh_ok`) in operator-facing messages. It is NOT a matcher:
/// that call spans lines, so classification is "does this compose line carry the dry-run marker".
/// The const exists so a failure still tells the reader WHICH invocation is wrong.
const LIVE_KEY: &str = "ssh_ok";

/// The two compose invocations, classified out of the stripped source.
///
/// `None` is itself a finding: a *missing* dry-run or live compose line is a failure, never
/// vacuously satisfied. A gate that goes quiet because the thing it audits was deleted checks
/// nothing.
struct ComposeLines<'a> {
    dry: Option<&'a str>,
    live: Option<&'a str>,
}

#[cfg(test)]
#[path = "tests/staging_compose_paths/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_staging_compose_paths;

#[cfg(test)]
use source_audit::{audit, f_path, f_regex, source_basename, strip_comments};
