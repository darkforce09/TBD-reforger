//! The staging compose file and its one owner, `cargo xtask deploy website`.
//!
//! **Role:** holds that every compose command the website deploy sends to the host names
//! `deploy/compose.staging.yml` and no other compose file, that the game server deploy sends none,
//! that the compose file sits where both expect it, and that no compose file sits in the checkout
//! root the commands run from.
//!
//! **Position:** the body of `cargo xtask verify staging-compose-paths`, which `ci-local`, the
//! wave gate's `VERIFY_STEPS` and the `mod-gates-hosted` job of `.github/workflows/ci.yml` run;
//! it reads `WEBSITE_DEPLOY_SOURCE` and every production source of `STAGING_DEPLOY_MODULE`
//! as text and runs nothing.
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
//!    requires the exact checkout-relative path, and separately bans a `cd` into `deploy/`,
//!    where the development stack's `compose.dev.yml` sits one word away from the staging file.
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
//! **Everything about *what* is pinned lives in the consts below** — `WEBSITE_DEPLOY_SOURCE`,
//! `STAGING_DEPLOY_PIPELINE`, `STAGING_DEPLOY_MODULE`, `GOOD_PATH`, `COMPOSE_FOLDER`,
//! `COMPOSE_FILE_NAME` and `COMPOSE_COMMAND` — and every message is `format!`ed from them. To
//! follow the deploy code elsewhere, change the sources; if the new code sets its working folder
//! another way, replace the `cd` ban (`cd_into_compose_folder`) with the equivalent ban rather
//! than deleting it.

use std::path::Path;

use crate::error::Result;
use regex::Regex;
use verification_core::{Finding, Kind, NotRun, Pattern, Verdict, gate};

// ── THE PIN, IN ONE PLACE ────────────────────────────────────────────────────────────────────

/// Printed on the PASS/FAIL summary line: operator logs and
/// `cargo xtask verify staging-compose-paths` transcripts are grepped for exactly this string.
const GATE_NAME: &str = "staging-compose-paths";

/// The source that builds every compose command `cargo xtask deploy website` sends to the host,
/// repo-relative. The deploy's step runner in front of it only prints and sends those strings, so
/// auditing the runner alone would see no compose command at all.
const WEBSITE_DEPLOY_SOURCE: &str = "tools/commands/deployment/src/website/remote_steps.rs";

/// The game server deploy's pipeline, repo-relative: the file that runs every step the deploy
/// sends to the host. It must exist, so a moved pipeline turns the gate red rather than blind.
const STAGING_DEPLOY_PIPELINE: &str =
    "tools/commands/deployment/src/staging/remote/fleet_deploy.rs";

/// The game server deploy's module, repo-relative. `<module>.rs` and every `.rs` file under
/// `<module>/` outside a `tests/` folder build what the deploy sends to the host, the pipeline and
/// each payload module alike, so each is audited: the deploy runs no compose command, because the
/// staging compose stack belongs to the website deploy.
const STAGING_DEPLOY_MODULE: &str = "tools/commands/deployment/src/staging";

/// The one staging compose file. Double duty: the string that must follow `-f`, **and** — joined
/// onto the repo root — the file that must exist. Two spellings of one contract drift, so there
/// is exactly one here.
const GOOD_PATH: &str = "deploy/compose.staging.yml";

/// The folder that holds every compose file of the checkout: [`GOOD_PATH`] and, beside it, the
/// development stack's `deploy/compose.dev.yml`, which would start the wrong topology on the host
/// without any error.
const COMPOSE_FOLDER: &str = "deploy";

/// One word that names a compose file, with any folder in front: `compose.yml`,
/// `docker-compose.staging.yml`, `deploy/compose.dev.yml`. Every such word on a compose line other
/// than [`GOOD_PATH`] is a wrong file — the development stack beside it or a compose file outside
/// [`COMPOSE_FOLDER`] — whether it follows `-f`, a second `-f` overlay (the later file wins for
/// conflicting keys) or an `--env-file=`; and a file of this name in the checkout root is one
/// compose loads by itself when `-f` is dropped.
const COMPOSE_FILE_NAME: &str = r"^(?:.*/)?(?:docker-)?compose(?:\.[A-Za-z0-9_-]+)*\.ya?ml$";

/// A compose command, under either provider and in either spelling: `docker compose`,
/// `podman compose`, `docker-compose`, `podman-compose`. The word after it must end in a space,
/// a tab or the line's end, so the compose file's own name, `docker-compose.staging.yml`, is not
/// a command.
const COMPOSE_COMMAND: &str = r"\b(?:docker|podman)(?:[ \t]+|-)compose(?:[ \t]|$)";

/// A `cd` into [`COMPOSE_FOLDER`] under any quoting, as the Rust source spells it: single quotes,
/// escaped double quotes or none, after at most one leading segment such as `{remote_dir}/` or
/// `$TBD_REMOTE_DIR/`. One pattern for every quoting, because banning one spelling bans nothing;
/// one leading segment, so a path that merely passes through a folder of that name, such as the
/// `/home/deploy/` of the host's deploy account, is not a `cd` into the checkout's folder.
fn cd_into_compose_folder() -> String {
    format!(
        r#"\bcd[ \t]+(?:\\?["'])?(?:[^ \t"'\\;&|/]*/)?{}(?:[/\\"' \t;&|]|$)"#,
        regex::escape(COMPOSE_FOLDER)
    )
}

#[cfg(test)]
#[path = "tests/staging_compose_paths/tests.rs"]
mod tests;

mod source_audit;
pub use source_audit::verify_staging_compose_paths;

#[cfg(test)]
use source_audit::{
    audit, compose_lines, f_path, f_regex, source_basename, strip_comments, wrong_compose_files,
};
