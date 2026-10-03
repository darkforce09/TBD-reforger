//! `cargo xtask deploy staging` — put the staging fleet's game servers on the host and prove each
//! one booted.
//!
//! Check that the website API answers on the host and that the fleet's secret files are there,
//! rsync the monorepo, write every instance's profile and server config on the host, install the
//! template units, restart every game server, then read each server's own console log and assert
//! its boot rather than assume it; then start the relay and the host agents. The website stack
//! itself (API, Postgres, Caddy) is `cargo xtask deploy website`'s.
//!
//! ── MODULE SPLIT (what each file owns) ───────────────────────────────────────────────────────
//!
//! | file | owns |
//! |------|------|
//! | this file | [`Paths`], the CLI parse, and mode dispatch |
//! | [`config`] | the deploy file, its defaults, the retired settings it refuses |
//! | [`fleet_instances`] | the instances: ports, names, visibility, folders, units, the relay |
//! | [`fleet_server_config`] | each instance's `server.config.json` and `--render-only` |
//! | [`render`] | modpack resolution, `game.mods[]` and the server config check |
//! | [`payloads`] | the secret file check, each instance's files, profile writer and V2–V4 smoke |
//! | [`fleet_units`] | the three template units and their install |
//! | [`host_agent`] | each instance's `agent.toml` and the agents' install |
//! | [`acknowledgement_relay`] | the relay in front of the relay instance's agent |
//! | [`legacy_single_instance_migration`] | `--migrate-single-instance` |
//! | [`boot`] | the boot verdict over a `console.log`, plus its selftest |
//! | [`pycompat`] | JSON behaviours a `python3` implementation made observable in output |
//! | [`remote`] | ssh and rsync transport, the pipeline, the boot verdict per instance |
//!
//! ── WHAT IS AND IS NOT VERIFIED ──────────────────────────────────────────────────────────────
//!
//! [`crate::core::repository_layout::DEPLOY_ENV`] exists on no development machine: it is
//! gitignored and rsync-excluded, so every ssh and rsync path in [`remote`] is unreachable from a
//! checkout. Those paths are covered by tests that assert the exact argument vectors and payloads
//! that would be sent, and the secret file check is run under a local `bash`. That is structural
//! fidelity rather than live proof.
//!
//! Everything reachable offline — `--help`, `--render-only`, `--verify-boot`,
//! `--verify-boot-selftest`, the `--dry-run` plan, bad-flag and missing-value handling, and the
//! exact text of every remote payload — is exercised by this crate's tests.
//!
//! ── WHY CHECKS REFUSE RATHER THAN SKIP ───────────────────────────────────────────────────────
//!
//! A deploy that reports having run a check it did not run is worse than one that stops:
//!
//! 1. [`config`] reads the deploy file through [`crate::core::deploy_environment`], which parses it
//!    as `KEY=VALUE` and never executes it, and refuses a setting the fleet does not read.
//! 2. The secret files are checked on the host before anything changes there, and every unit the
//!    deploy starts has its state read back.
//! 3. [`remote`] judges only a log written by the boot it started, pulled whole.
//!
//! ── BEHAVIOURS THAT LOOK LIKE BUGS AND ARE NOT ───────────────────────────────────────────────
//!
//! * `--render-only` runs after the deploy file's existence check and its required values, so it
//!   needs a filled deploy file even though it touches no server. `--verify-boot*` runs before that
//!   point and is genuinely credential-free.
//! * `--render-only --dry-run` takes `--dry-run` as the output directory: a value argument is read
//!   as a value, with no lookahead for a leading dash.
//! * Deploy-file values override the process environment; an empty assignment in the file counts
//!   as unset and is not filled from the environment either.
//! * `TBD_SCENARIO`'s default carries a `{ResourceGUID}`; the validator that catches a truncated
//!   GUID is kept, because a truncated default is silent everywhere else.

use anyhow::Result;
use std::path::PathBuf;

mod acknowledgement_relay;
mod boot;
mod config;
pub(crate) mod fleet_instances;
mod fleet_server_config;
mod fleet_units;
mod host_agent;
mod legacy_single_instance_migration;
pub(crate) mod payloads;
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
    /// Host secrets and remote paths: the file `DEPLOY_ENV` names, else the checkout's
    /// gitignored, rsync-excluded `deploy.env`. Absolute whenever the root is.
    pub deploy_env: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Paths> {
        let mono_root = repository_layout::find_repository_root()?;
        Ok(Paths {
            deploy_env: crate::core::deploy_environment::deploy_environment_path(&mono_root),
            mono_root,
        })
    }
}

/// The `--help` block.
const USAGE: &str = "\
Usage: cargo xtask deploy staging [--dry-run] [--migrate-single-instance] [--render-only <directory>]
                                  [--verify-boot <console.log>] [--verify-boot-selftest]";

/// Everything the CLI loop can produce.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub dry_run: bool,
    /// `--migrate-single-instance`: retire the single-instance server so the fleet takes its ports.
    pub migrate_single_instance: bool,
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
            "--migrate-single-instance" => cli.migrate_single_instance = true,
            // Render every instance's server config into a LOCAL directory and exit before any
            // rsync/ssh runs: the render half, exercised without touching a real server.
            "--render-only" => {
                i += 1;
                match args.get(i) {
                    Some(v) if !v.is_empty() => cli.render_only_out = Some(v.clone()),
                    _ => {
                        eprintln!("--render-only requires an output directory");
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

    // ── Mode dispatch ────────────────────────────────────────────────────────────────────────
    //
    // The ORDER IS THE CONTRACT: both --verify-boot forms sit BEFORE the deploy.env existence
    // check, so they run on a machine with no staging settings at all. --render-only sits AFTER it:
    // the render reads the fleet's ports, the public address and the mod source from deploy.env,
    // and a render from invented values would be a different config than the deploy pushes.
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
