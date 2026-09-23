//! `cargo xtask deploy staging` — put the platform on the staging box and prove it booted.
//!
//! Rsync the monorepo, rebuild the API, refresh the Reforger profile, render and push
//! `server.config.json`, restart the game server, then read the server's own console log and
//! assert the boot rather than assume it.
//!
//! ── MODULE SPLIT (what each file owns) ───────────────────────────────────────────────────────
//!
//! Staging operations are grouped by the artefact they build; supporting modules own the shared
//! configuration and rendering.
//!
//! | file | owns |
//! |------|------|
//! | this file | [`Paths`], the CLI parse, and mode dispatch |
//! | [`host_agent`] | the host agent's settings, its server-config `rcon` block and its install |
//! | [`boot`] | the boot verdict over a `console.log`, plus its selftest |
//! | [`config`] | the deploy file, the defaults it fills in, and the launch-mode gate |
//! | [`render`] | modpack resolution and the `server.config.json` render and validate |
//! | [`pycompat`] | JSON behaviours a `python3` implementation made observable in output |
//! | [`remote`] | ssh, rsync and compose transport, the deploy pipeline, the console-log read |
//! | [`payloads`] | the exact text of every remote `bash -s` heredoc |
//!
//! ── WHAT IS AND IS NOT VERIFIED ──────────────────────────────────────────────────────────────
//!
//! [`crate::core::repository_layout::DEPLOY_ENV`] exists on no development machine: it is
//! gitignored and rsync-excluded (see the exclude list in [`remote`]), so every ssh, rsync and
//! compose path in [`remote`] is unreachable from a checkout. Those paths are covered by
//! argv-construction tests that assert the exact program and argument vector, in order, that
//! would be spawned. That is structural fidelity rather than live proof, and each test says so
//! in its name.
//!
//! Everything reachable offline — `--help`, `--render-only`, `--verify-boot`,
//! `--verify-boot-selftest`, `--dry-run`, bad-flag and missing-value handling, and the exact text
//! of every remote payload — is exercised by this crate's tests.
//!
//! ── WHY THREE CHECKS REFUSE RATHER THAN SKIP ─────────────────────────────────────────────────
//!
//! A deploy that reports having run a check it did not run is worse than one that stops, so
//! three places refuse instead:
//!
//! 1. [`config`] parses the deploy file as `KEY=VALUE` and never executes it, so a stray command
//!    in it is inert rather than run with the deploy's privileges.
//! 2. [`host_agent`] refuses a credential, RCON password or API origin the agent or the engine
//!    would refuse, before anything is deployed, and its install reads the unit's state back.
//! 3. [`remote`] checks the status of the console-log pull. A partial transfer yields a
//!    non-empty file, so a size check alone would read a truncated log as a complete one.
//!
//! ── BEHAVIOURS THAT LOOK LIKE BUGS AND ARE NOT ───────────────────────────────────────────────
//!
//! Each is pinned by a test and documented at its site:
//!
//! * `--render-only` runs after the deploy file's existence check and its required values, so it
//!   needs a filled deploy file even though it touches no server; the render legitimately reads
//!   `TBD_PROFILE_DIR` for `TBD_SERVER_CONFIG_REMOTE`. `--verify-boot*` runs before that point
//!   and is genuinely credential-free.
//! * `--render-only --dry-run` takes `--dry-run` as the output path: a value argument is read as
//!   a value, with no lookahead for a leading dash, so a file may legitimately be named that.
//! * Deploy-file values override the process environment, so `TBD_A2S_PORT=1 cargo xtask deploy
//!   staging` is ignored when the deploy file sets that key.
//! * `TBD_SCENARIO`'s default carries a `{ResourceGUID}`; the validator that catches a truncated
//!   GUID is kept, because a truncated default is silent everywhere else.
//! * `TBD_WORKSHOP_MOD_ID` emptiness is read from the deploy file's value, so exporting an empty
//!   one on the command line does not trip the config-mode requirement.

use anyhow::Result;
use std::path::PathBuf;

mod boot;
mod config;
mod host_agent;
mod payloads;
mod pycompat;
mod remote;
mod render;

/// The three checkout locations this command reads.
///
/// The root comes from the repository marker, like every other xtask module, so the answer is
/// the worktree the command is run from rather than the one that built the binary.
#[derive(Debug, Clone)]
pub struct Paths {
    /// Repository root: the rsync source, and the base for every other path.
    pub mono_root: PathBuf,
    /// Host secrets and remote paths. Gitignored, rsync-excluded, development machine only.
    pub deploy_env: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Paths> {
        let mono_root = crate::core::repository_root::find_repo_root()?;
        Ok(Paths {
            deploy_env: mono_root.join(crate::core::repository_layout::DEPLOY_ENV),
            mono_root,
        })
    }
}

/// The `--help` block.
const USAGE: &str = "\
Usage: cargo xtask deploy staging [--dry-run] [--render-only <path>]
                                  [--verify-boot <console.log>] [--verify-boot-selftest]";

/// Everything the CLI loop can produce. Mirrors the bash's five mode variables plus `DRY_RUN`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub dry_run: bool,
    pub render_only_out: Option<String>,
    pub verify_boot_log: Option<String>,
    pub verify_boot_selftest: bool,
}

/// One of the two terminal answers a parse can give: keep going, or stop with this status.
///
/// `Help` is separate from `Stop(0)` so the caller prints the usage block at exactly the point
/// the bash's `echo` ran — inside the loop, before any later argument is examined.
#[derive(Debug, PartialEq, Eq)]
pub enum Parsed {
    Run(Box<Cli>),
    Help,
    /// Message already rendered to stderr; carry only the status.
    Stop(u8),
}

/// The bash `while [ "$#" -gt 0 ]; do case "$1" in … esac; shift; done` loop.
///
/// ODDITY PRESERVED: the `--x <value>` arms `shift` and then read `${1:-}`, which means
/// `--render-only --dry-run` takes `--dry-run` as the output *path*. There is no lookahead for a
/// leading `-` in the bash and adding one here would change behaviour, not fix a bug — a caller
/// could legitimately want a file called `--dry-run` and, more to the point, nobody can depend on
/// a behaviour this port invented.
///
/// ODDITY PRESERVED: an unknown option short-circuits immediately, left to right, so
/// `--nope --help` exits 2 and never prints usage.
pub fn parse(args: &[String]) -> Parsed {
    let mut cli = Cli::default();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--dry-run" => cli.dry_run = true,
            // Render the server config to a LOCAL path and exit 0 before any rsync/ssh
            // runs. This is the only way to exercise the render half without touching a real
            // server, and it is what the perturbation gate drives.
            "--render-only" => {
                i += 1;
                match args.get(i) {
                    Some(v) if !v.is_empty() => cli.render_only_out = Some(v.clone()),
                    _ => {
                        eprintln!("--render-only requires an output path");
                        return Parsed::Stop(2);
                    }
                }
            }
            // Run the boot verdict over a console.log you already have — no ssh, no
            // deploy.env, no staging host. Same split --render-only made for the server config:
            // a check that only runs mid-deploy is a check nobody runs.
            "--verify-boot" => {
                i += 1;
                match args.get(i) {
                    Some(v) if !v.is_empty() => cli.verify_boot_log = Some(v.clone()),
                    _ => {
                        eprintln!("--verify-boot requires a path to a console.log");
                        return Parsed::Stop(2);
                    }
                }
            }
            // Prove the boot verdict can FAIL. A gate never observed failing is not a gate.
            "--verify-boot-selftest" => cli.verify_boot_selftest = true,
            "-h" | "--help" => return Parsed::Help,
            other => {
                eprintln!("Unknown option: {other}");
                return Parsed::Stop(2);
            }
        }
        i += 1;
    }
    Parsed::Run(Box::new(cli))
}

/// Entry for `cargo xtask deploy staging -- <args>`.
pub fn run(args: &[String]) -> Result<u8> {
    let cli = match parse(args) {
        Parsed::Help => {
            println!("{USAGE}");
            return Ok(0);
        }
        Parsed::Stop(code) => return Ok(code),
        Parsed::Run(cli) => *cli,
    };
    let paths = Paths::resolve()?;

    // ── Mode dispatch, in the bash's order ───────────────────────────────────────────────────
    //
    // The ORDER IS THE CONTRACT, not an implementation detail: both --verify-boot forms sit
    // BEFORE the deploy.env existence check, so they run on a
    // machine with no staging credentials at all. --render-only sits AFTER it (bash line 1514 vs
    // the check at 1072) and therefore needs a filled deploy.env despite the header advertising
    // it as "no rsync, no ssh, no deploy". That is preserved, not fixed: the render genuinely
    // reads TBD_PROFILE_DIR (for TBD_SERVER_CONFIG_REMOTE) and TBD_GAME_PORT, so a deploy.env-less
    // render would have to invent values and would then be rendering a different config than the
    // deploy does — the exact "validating something you did not deploy" defect this file is
    // written against.
    if cli.verify_boot_selftest {
        println!("==> boot verdict selftest (local only, no deploy, no ssh)");
        return Ok(boot::selftest(&paths));
    }
    if let Some(log) = cli.verify_boot_log.as_deref() {
        return Ok(boot::verify_boot_cli(&paths, std::path::Path::new(log)));
    }

    remote::deploy(&paths, &cli)
}

#[cfg(test)]
#[path = "tests/staging/tests.rs"]
mod tests;
