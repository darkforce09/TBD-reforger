//! T-853 — port of `scripts/mod/deploy-staging.sh` (1889 lines) → `cargo xtask deploy staging`.
//!
//! Rsync the platform to the staging box, rebuild the API, refresh the Reforger profile,
//! render + push `server.config.json`, restart the game server, and then **assert the boot**
//! rather than assume it.
//!
//! ── WHY THIS PORT IS THE KEYSTONE ────────────────────────────────────────────────────────────
//!
//! `deploy-staging.sh` was the LAST consumer of both `scripts/mod/lib/paths.sh` and
//! `scripts/mod/lib/gate-grep.sh`. Everything it used from them is inlined here:
//!
//! * `paths.sh` gave it `MONO_ROOT` / `SCHEMA` / `DEPLOY_ENV`. Those are now
//!   [`Paths`] below, derived from [`crate::root::find_repo_root`] instead of from
//!   `$(dirname $0)`. Note the deliberate divergence from `gate_deploy_website.rs`: that port
//!   honours a `DEPLOY_ENV` env override, and this one does **not**, because `paths.sh`
//!   *unconditionally overwrote* `DEPLOY_ENV` after sourcing. Honouring an inherited value here
//!   would be new behaviour wearing a port's clothes.
//! * `gate-grep.sh` gave it `gate_require` / `gate_ban` inside `validate_agent_files()`. Those
//!   are now `tbd_gate::gate::{require, ban}` + `Pattern`, which render byte-for-byte the same
//!   `FAIL: <msg>` and carry the four-outcome [`tbd_gate::Verdict`] the bash helper could only
//!   describe in a comment.
//!
//! With this file landed, both libs have zero consumers (T-879 / T-880).
//!
//! ── MODULE SPLIT (what each file owns) ───────────────────────────────────────────────────────
//!
//! 1889 lines of bash do not fit one Rust module under the 1000-line SIZE-3 ceiling, so the port
//! is cut at the bash's own seams — the four artefacts it builds (agent, boot verdict, server
//! config, remote host), each of which the bash already gave its own local-only entry point, plus
//! two supporting modules split off purely for size and named for what they own.
//!
//! | file | bash lines | owns |
//! |------|-----------|------|
//! | this file | 26–101 | `Paths` (the `paths.sh` inline), CLI parse, mode dispatch |
//! | [`agent`] | 103–411, 597–679 | T-289 agent: the rendered artefact + its structural gates |
//! | [`agent_selftest`] | 413–595 | T-289: driving that artefact against a stub systemd |
//! | [`boot`] | 681–1070 | T-607 boot verdict over a `console.log`, plus its selftest |
//! | [`config`] | 1072–1250 | `deploy.env`, the `:=` defaults, the mode gate |
//! | [`render`] | 1252–1522 | T-288 modpack resolution + `server.config.json` render/validate |
//! | [`pycompat`] | (the 14 `python3` sites) | the python behaviours that were OBSERVABLE in output |
//! | [`remote`] | 1524–1889 | ssh/rsync/compose transport, the deploy pipeline, the V6 read |
//! | [`payloads`] | 1591–1849 | the exact text of every remote `bash -s` heredoc |
//!
//! ── WHAT IS AND IS NOT VERIFIED ──────────────────────────────────────────────────────────────
//!
//! `scripts/deploy/deploy.env` does not exist on a dev machine (it is gitignored *and*
//! rsync-excluded — see the exclude list in [`remote`]), so every ssh/rsync/compose path in
//! [`remote`] is **unreachable locally and was never executed by this port's author**. Those
//! paths are covered by argv-construction unit tests (`remote::tests`) that assert the exact
//! program + argument vector, in order, that would be spawned. That is structural fidelity, not
//! live proof, and the tests say so in their names.
//!
//! Everything reachable offline — `--help`, `--render-only`, `--render-agent`,
//! `--agent-selftest`, `--verify-boot`, `--verify-boot-selftest`, `--dry-run`, bad-flag and
//! missing-env handling — was diffed byte-for-byte against the bash before it was deleted.
//!
//! ── FAIL-OPENS CLOSED (each one named) ───────────────────────────────────────────────────────
//!
//! The brief allows exactly one class of fix: a path where the script could report having run a
//! check it did not run. Three were found and closed; they are listed at their sites and
//! repeated here so a reader does not have to find them:
//!
//! 1. [`config`] — `deploy.env` was `source`d, i.e. executed as shell. A syntax error in it
//!    aborted under `set -e`, but a *stray command* in it ran silently with the deploy's
//!    privileges. Now KEY=VALUE parsed; anything else is inert. (Same call as
//!    `gate_deploy_website.rs`.)
//! 2. [`agent`] — `agent_selftest()`'s JSON-contract case ran only `if command -v python3`. On a
//!    box without python3 the check silently vanished and the selftest still printed
//!    `AGENT SELFTEST: N passed, 0 failed`. The JSON parse is now `serde_json`, compiled in, so
//!    the case cannot be skipped. Same for `systemd-analyze verify`, which was also
//!    `if command -v`-guarded — that one *stays* conditional (see the note there) because it
//!    tests the host's systemd, not our artefact, but it now says out loud that it was skipped
//!    instead of silently shrinking the denominator.
//! 3. [`remote`] — `ssh_cmd "cat '$log/console.log'" > "$_local_log" 2>/dev/null || true`
//!    swallowed the ssh status entirely; only the follow-up `[ ! -s ]` caught it, so a
//!    *non-empty but truncated* pull read as a complete log. The status is now checked.
//!
//! ── ODDITIES PRESERVED ON PURPOSE ────────────────────────────────────────────────────────────
//!
//! Reproduced, pinned with a test, and documented at their sites:
//!
//! * `--render-only` is documented in the bash header as "no rsync, no ssh, no deploy" but sits
//!   *after* the `deploy.env` existence check and the `${VAR:?}` requirements, so it cannot run
//!   without a filled deploy.env. `--render-agent` / `--agent-selftest` / `--verify-boot*` sit
//!   *before* it and are genuinely credential-free. Kept: callers depend on the ordering, and
//!   the render legitimately needs `TBD_PROFILE_DIR` for `TBD_SERVER_CONFIG_REMOTE`.
//! * `--render-only --dry-run` consumes `--dry-run` as the output path. Bash's `shift`-then-read
//!   never looked at whether the next word was a flag.
//! * `deploy.env` values override the process environment, because `source` ran after the
//!   command line was already in `environ`. So `TBD_A2S_PORT=1 bash deploy-staging.sh` is
//!   ignored if deploy.env sets it.
//! * `TBD_SCENARIO` is assigned with an `if [ -z ]` and NOT `: "${VAR:=default}"` — the `}` of
//!   the ResourceGUID would close the parameter expansion and truncate the default to
//!   `{69A85365FC09E2CA`. The Rust has no such hazard, but the *validator* that catches the
//!   truncation is kept, because it is the check that caught it (bash line 1414).
//! * Usage text still says `deploy-staging.sh`, and the `--verify-boot` hint still shows the
//!   `bash scripts/mod/deploy-staging.sh …` invocation. Byte-parity with the captured baseline.
//! * The V1 mission-JSON validate prints its banner and then does nothing under `--dry-run`.
//! * `TBD_WORKSHOP_MOD_ID` emptiness is checked against the *sourced* value, so exporting an
//!   empty one on the command line does not trip the config-mode requirement.

use anyhow::Result;
use std::path::PathBuf;

mod agent;
mod agent_selftest;
mod boot;
mod config;
mod payloads;
mod pycompat;
mod remote;
mod render;

/// `paths.sh`, inlined. This is the whole of what `deploy-staging.sh` used from it.
///
/// The bash derived these from `$(dirname "$0")`; a cargo subcommand has no `$0` in the repo, so
/// the root comes from the ticket registry marker like every other xtask module. `SCHEMA`,
/// `MOD_ROOT` and `WEB` were exported by `paths.sh` for other consumers — only the three fields
/// below were ever read by this script, and inlining the unused ones would be importing a
/// dependency, not removing one.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `MONO_ROOT` — repo root. rsync source, and the base for every other path.
    pub mono_root: PathBuf,
    /// `SCHEMA` — `packages/tbd-schema`, home of the golden missions the V1 step validates.
    pub schema: PathBuf,
    /// `DEPLOY_ENV` — `scripts/deploy/deploy.env`. Gitignored, rsync-excluded, dev-PC only.
    pub deploy_env: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Paths> {
        let mono_root = crate::root::find_repo_root()?;
        Ok(Paths {
            schema: mono_root.join("packages/tbd-schema"),
            deploy_env: mono_root.join("scripts/deploy/deploy.env"),
            mono_root,
        })
    }
}

/// Usage block — byte-identical to the bash `-h|--help` arm (three `echo` lines).
const USAGE: &str = "\
Usage: deploy-staging.sh [--dry-run] [--render-only <path>]
                         [--render-agent <dir>] [--agent-selftest <dir>]
                         [--verify-boot <console.log>] [--verify-boot-selftest]";

/// Everything the CLI loop can produce. Mirrors the bash's five mode variables plus `DRY_RUN`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cli {
    pub dry_run: bool,
    pub render_only_out: Option<String>,
    pub render_agent_out: Option<String>,
    pub agent_selftest_out: Option<String>,
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
            // T-288: render the server config to a LOCAL path and exit 0 before any rsync/ssh
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
            // T-289: render the host control agent + its systemd units into a LOCAL directory and
            // exit 0, before any rsync/ssh. Same split T-288 made for the server config: the
            // artefact is inspectable without deploying it.
            "--render-agent" => {
                i += 1;
                match args.get(i) {
                    Some(v) if !v.is_empty() => cli.render_agent_out = Some(v.clone()),
                    _ => {
                        eprintln!("--render-agent requires an output directory");
                        return Parsed::Stop(2);
                    }
                }
            }
            // T-289: render the agent, then RUN it against a stub systemctl whose answers this
            // program controls, and assert the agent reports the unit's real state. See
            // `agent::selftest` for why that is the whole point.
            "--agent-selftest" => {
                i += 1;
                match args.get(i) {
                    Some(v) if !v.is_empty() => cli.agent_selftest_out = Some(v.clone()),
                    _ => {
                        eprintln!("--agent-selftest requires a working directory");
                        return Parsed::Stop(2);
                    }
                }
            }
            // T-607: run the boot verdict over a console.log you already have — no ssh, no
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
            // T-607: prove the boot verdict can FAIL. A gate never observed failing is not a gate.
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
    // The ORDER IS THE CONTRACT, not an implementation detail: --render-agent, --agent-selftest
    // and both --verify-boot forms sit BEFORE the deploy.env existence check, so they run on a
    // machine with no staging credentials at all. --render-only sits AFTER it (bash line 1514 vs
    // the check at 1072) and therefore needs a filled deploy.env despite the header advertising
    // it as "no rsync, no ssh, no deploy". That is preserved, not fixed: the render genuinely
    // reads TBD_PROFILE_DIR (for TBD_SERVER_CONFIG_REMOTE) and TBD_GAME_PORT, so a deploy.env-less
    // render would have to invent values and would then be rendering a different config than the
    // deploy does — the exact "validating something you did not deploy" defect this file is
    // written against.
    if let Some(out) = cli.render_agent_out.as_deref() {
        println!("==> render host control agent (local only, no deploy) -> {out}");
        return agent::render_and_validate(std::path::Path::new(out));
    }
    if let Some(out) = cli.agent_selftest_out.as_deref() {
        println!("==> agent selftest (local only, no deploy) -> {out}");
        return agent_selftest::run(std::path::Path::new(out));
    }
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
mod tests {
    use super::*;

    fn v(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn usage_matches_the_captured_baseline() {
        // /tmp/t853/ds--help.old, three lines, rc=0.
        let lines: Vec<&str> = USAGE.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(
            lines[0],
            "Usage: deploy-staging.sh [--dry-run] [--render-only <path>]"
        );
        assert!(lines[1].starts_with("                         [--render-agent <dir>]"));
        assert!(lines[2].ends_with("[--verify-boot-selftest]"));
    }

    #[test]
    fn missing_value_stops_with_two() {
        for flag in [
            "--render-only",
            "--render-agent",
            "--agent-selftest",
            "--verify-boot",
        ] {
            assert_eq!(parse(&v(&[flag])), Parsed::Stop(2), "{flag}");
        }
    }

    #[test]
    fn unknown_option_short_circuits_before_help() {
        // Bash `case` runs left to right and exits 2 on the first unknown word, so the later
        // --help is never reached. Pinned because "helpful" reordering would change the status.
        assert_eq!(parse(&v(&["--nope", "--help"])), Parsed::Stop(2));
        assert_eq!(parse(&v(&["--help", "--nope"])), Parsed::Help);
    }

    #[test]
    fn oddity_flag_is_eaten_as_a_value() {
        // PRESERVED: `shift` then `${1:-}` has no lookahead, so --dry-run becomes the path.
        match parse(&v(&["--render-only", "--dry-run"])) {
            Parsed::Run(cli) => {
                assert_eq!(cli.render_only_out.as_deref(), Some("--dry-run"));
                assert!(
                    !cli.dry_run,
                    "--dry-run was consumed as a value, not a flag"
                );
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn empty_string_value_is_rejected_like_bash() {
        // `${1:-}` yields "" for an explicit empty arg too, and `[ -z ]` then fires.
        assert_eq!(parse(&v(&["--render-only", ""])), Parsed::Stop(2));
    }

    #[test]
    fn flags_accumulate() {
        match parse(&v(&["--dry-run", "--verify-boot-selftest"])) {
            Parsed::Run(cli) => {
                assert!(cli.dry_run && cli.verify_boot_selftest);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn paths_inline_the_three_fields_paths_sh_supplied() {
        let p = Paths::resolve().expect("repo root");
        assert!(
            p.mono_root.join(".ai/tickets/ROOT").is_file()
                || p.mono_root.join(".ai/tickets/registry.json").is_file()
        );
        assert!(p.schema.ends_with("packages/tbd-schema"));
        assert!(p.deploy_env.ends_with("scripts/deploy/deploy.env"));
    }
}
