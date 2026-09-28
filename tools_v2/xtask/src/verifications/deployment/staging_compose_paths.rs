//! The staging compose file and its one owner, `cargo xtask deploy website`.
//!
//! **Role:** holds that every compose command the website deploy sends to the host names
//! `apps/website/docker-compose.staging.yml`, that the game server deploy sends none, and that
//! the compose file sits where both expect it.
//!
//! **Position:** the body of `cargo xtask verify staging-compose-paths`, which `ci-local`, the
//! wave gate's `VERIFY_STEPS` and the `mod-gates-hosted` job of `.github/workflows/ci.yml` run;
//! it reads [`WEBSITE_DEPLOY_SOURCE`] and [`STAGING_DEPLOY_SOURCE`] as text and runs nothing.
//!
//! **Signals & state:** none; reads of the checkout.
//!
//! **Invariants:** the status is binary, 0 when every check held and 1 otherwise, and an input it
//! could not read is named as "did not run" in the report; a comment never counts as a compose
//! command, so it can neither satisfy the pin nor trip the ban.
//!
//! ── WHAT THE GATE IS FOR ─────────────────────────────────────────────────────────────────────
//!
//! Compose happily starts *a* stack from *a* file, so a compose command that names the wrong file
//! does not fail loudly: the deploy goes green and the host quietly runs the wrong topology. And a
//! second deploy that runs compose on the same host starts services the website deploy owns,
//! without the settings the website deploy passes (the Postgres host port), and with the website
//! deploy's plan no longer describing the host. Hence a static pin rather than a smoke test.
//!
//! Two false-green shapes the gate refuses:
//!
//! 1. a `//` or `#` comment naming the good path counting as a compose command;
//! 2. a relative `-f` path that looks right but resolves against the wrong folder: the gate
//!    requires the exact checkout-relative path, and separately bans a `cd` into
//!    `apps/website/api_v2`.
//!
//! The website deploy prints each command under `--dry-run` from the same string it runs live, so
//! there is no separate rehearsal text that could disagree with the real one.
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
//! **Everything about *what* is pinned lives in the consts below** — [`WEBSITE_DEPLOY_SOURCE`],
//! [`STAGING_DEPLOY_SOURCE`], [`GOOD_PATH`], [`BAD_PATH`], [`COMPOSE_COMMAND`] and
//! [`CD_INTO_API`] — and every message is `format!`ed from them. To follow the deploy code
//! elsewhere, change the two sources; if the new code sets its working folder another way,
//! replace [`CD_INTO_API`] with the equivalent ban rather than deleting it.

use std::path::Path;

use anyhow::Result;
use regex::Regex;
use verification_core::{Finding, Kind, NotRun, Pattern, Verdict, gate};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// Printed on the PASS/FAIL summary line: operator logs and
/// `cargo xtask verify staging-compose-paths` transcripts are grepped for exactly this string.
const GATE_NAME: &str = "staging-compose-paths";

/// The source that builds every compose command `cargo xtask deploy website` sends to the host,
/// repo-relative. The deploy's step runner in front of it only prints and sends those strings, so
/// auditing the runner alone would see no compose command at all.
const WEBSITE_DEPLOY_SOURCE: &str = "tools_v2/xtask/src/commands/deploy/website/remote_steps.rs";

/// The game server deploy's pipeline, repo-relative. It runs no compose command: the staging
/// compose stack belongs to the website deploy.
const STAGING_DEPLOY_SOURCE: &str = "tools_v2/xtask/src/commands/deploy/staging/remote/ssh_argv.rs";

/// The one staging compose file. Double duty: the string that must follow `-f`, **and** — joined
/// onto the repo root — the file that must exist. Two spellings of one contract drift, so there
/// is exactly one here.
const GOOD_PATH: &str = "apps/website/docker-compose.staging.yml";

/// A staging compose file beside the API. It must appear on no compose line and must not exist on
/// disk: a file there is what makes the wrong `-f` path a *plausible* edit rather than an obvious
/// typo, so the gate removes the temptation as well as the reference.
const BAD_PATH: &str = "apps/website/api_v2/docker-compose.staging.yml";

/// A compose command, under either provider and in either spelling: `docker compose`,
/// `podman compose`, `docker-compose`, `podman-compose`. The word after it must end in a space,
/// a tab or the line's end, so the compose file's own name, `docker-compose.staging.yml`, is not
/// a command.
const COMPOSE_COMMAND: &str = r"\b(?:docker|podman)(?:[ \t]+|-)compose(?:[ \t]|$)";

/// A `cd` into `apps/website/api_v2` under any quoting, as the Rust source spells it: single
/// quotes, escaped double quotes or none, after any prefix such as `{remote_dir}/`. One pattern
/// for every quoting, because banning one spelling bans nothing.
const CD_INTO_API: &str =
    r#"\bcd[ \t]+(?:\\?["'])?[^ \t"'\\;&|]*apps/website/api_v2(?:[/\\"' \t;&|]|$)"#;

#[cfg(test)]
#[path = "tests/staging_compose_paths/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_staging_compose_paths;

#[cfg(test)]
use source_audit::{audit, compose_lines, f_path, f_regex, source_basename, strip_comments};
