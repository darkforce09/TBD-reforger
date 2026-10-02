//! `cargo xtask deploy website` — rsync the monorepo to the server, start the staging Postgres,
//! build the release API, the staging host tools and the Leptos SPA there, start and reload the
//! Caddy web server, and restart the API's user-systemd unit.
//!
//! **Role:** reads the deploy settings, refuses what the `--delete` rsync must never touch, and
//! runs the steps in order: the map-asset probe, the rsync, the remote steps of
//! [`remote_steps`], and the restart.
//!
//! **Position:** called by `cargo xtask deploy website` from the development machine; the
//! settings come from `deploy.env` through [`crate::core::deploy_environment`], and every remote
//! step's text comes from [`remote_steps`], so `--dry-run` prints exactly what a live run sends.
//!
//! **Signals & state:** none held; one run spawns `rsync`, `ssh` and `sshpass` through
//! `verification_core::proc::Run` and prints their merged output.
//!
//! **Invariants:** a missing, unreadable or malformed `deploy.env` exits 1 rather than deploying
//! with defaults, and the file is parsed, never executed; a missing tool or a killed child is
//! reported as itself and never folds into "deploy succeeded"; `TBD_REMOTE_DIR` sits under the
//! deploy user's `/home/<user>/tbd` (from `TBD_SSH_HOST`), and a host named without a user is
//! refused; a failing remote step stops the deploy before the restart, so the running API keeps
//! serving the previous build.
//!
//! Two behaviours are deliberate and easy to misread as bugs. Trailing slashes on
//! `TBD_REMOTE_DIR` are stripped only for that prefix check — echoed remote paths and `cd '…'`
//! payloads keep the operator's raw value, so what is printed is what runs. And a failed
//! `systemctl --user restart` warns instead of aborting: the code and the database are already on
//! the server by then, so the deploy is done and the restart is the operator's to finish.

use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

use crate::core::deploy_environment::{
    DeployEnvironment, DeployHostFolder, deploy_environment_path,
};
use crate::core::repository_layout;
use crate::core::repository_root::find_repo_root;

pub mod asset_preflight;
pub mod help_text;
pub mod remote_steps;
pub mod rsync_argv;
pub mod systemd_unit;

use help_text::usage;
use remote_steps::RemoteStep;

/// Entry for `xtask deploy website`.
pub fn run(args: &[String]) -> Result<u8> {
    let mut dry_run = false;
    for arg in args {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "-h" | "--help" => {
                print!("{}", usage());
                return Ok(0);
            }
            other => {
                eprintln!("Unknown option: {other}");
                eprint!("{}", usage());
                return Ok(2);
            }
        }
    }

    let root = find_repo_root()?;
    let environment = match DeployEnvironment::load_required(&deploy_environment_path(&root)) {
        Ok(environment) => environment,
        Err(error) => {
            eprintln!("{error}");
            return Ok(1);
        }
    };
    let deploy_host = match environment.deploy_host() {
        Ok(host) => host,
        Err(error) => {
            eprintln!("{error}");
            return Ok(1);
        }
    };
    let Some(tbd_folder) = deploy_host.tbd_folder() else {
        eprintln!(
            "Refusing to deploy: TBD_SSH_HOST must name the deploy user (user@host), because \
             TBD_REMOTE_DIR must stay under that user's /home/<user>/tbd/ (got: {})",
            deploy_host.ssh_destination()
        );
        return Ok(1);
    };
    let remote_dir = match DeployHostFolder::Checkout.resolve(&environment, &deploy_host) {
        Ok(folder) => folder,
        Err(error) => {
            eprintln!("{error}");
            return Ok(1);
        }
    };
    let host = deploy_host.ssh_destination();
    let postgres_port = environment.value_or("TBD_POSTGRES_HOST_PORT", "5432");
    let systemd_unit = environment
        .value_or(
            "TBD_WEBSITE_SYSTEMD_UNIT",
            systemd_unit::default_unit_name(),
        )
        .to_string();
    let skip_compose = environment.value("TBD_SKIP_COMPOSE") == Some("1");
    let skip_spa = environment.value("TBD_SKIP_SPA_BUILD") == Some("1");
    let skip_api = environment.value("TBD_SKIP_API_BUILD") == Some("1");
    let ssh_pass = environment.value("TBD_SSH_PASS").map(str::to_string);
    let ssh_identity = environment
        .value("TBD_SSH_IDENTITY_FILE")
        .map(str::to_string);
    let profile_dir = environment.value("TBD_PROFILE_DIR");

    if let Err(code) = refuse_prairielearn("TBD_REMOTE_DIR", &remote_dir) {
        return Ok(code);
    }
    if let Err(code) = refuse_prairielearn("TBD_SSH_HOST", &host) {
        return Ok(code);
    }
    if let Some(p) = profile_dir
        && let Err(code) = refuse_prairielearn("TBD_PROFILE_DIR", p)
    {
        return Ok(code);
    }
    if let Err(code) = require_tbd_remote_prefix(&remote_dir, &tbd_folder) {
        return Ok(code);
    }

    let cfg = DeployCfg {
        root,
        host,
        remote_dir,
        postgres_port: postgres_port.to_string(),
        systemd_unit,
        skip_compose,
        skip_spa,
        skip_api,
        ssh_pass,
        ssh_identity,
        dry_run,
    };
    cfg.execute()
}

struct DeployCfg {
    root: PathBuf,
    host: String,
    remote_dir: String,
    postgres_port: String,
    systemd_unit: String,
    skip_compose: bool,
    skip_spa: bool,
    skip_api: bool,
    ssh_pass: Option<String>,
    ssh_identity: Option<String>,
    dry_run: bool,
}

impl DeployCfg {
    fn execute(&self) -> Result<u8> {
        println!("==> deploy-website → {}:{}", self.host, self.remote_dir);

        println!("==> preflight: remote map assets");
        if let Err(code) = self.check_remote_assets() {
            return Ok(code);
        }

        println!(
            "==> rsync (excludes secrets, build artifacts, worktrees and local tool state, the \
             terrain + scratch asset trees, the packages/ tree, oracle lanes)"
        );
        if self.dry_run {
            for line in rsync_argv::dry_run_lines(&self.host, &self.remote_dir) {
                println!("{line}");
            }
        } else if let Err(code) = self.rsync_to_remote() {
            return Ok(code);
        }

        for step in self.remote_plan() {
            if let Err(code) = self.remote_step(&step) {
                return Ok(code);
            }
        }

        println!("==> remote: restart {}", self.systemd_unit);
        let restart = remote_steps::restart(&self.systemd_unit);
        if self.dry_run {
            println!("[dry-run] ssh … {restart}");
        } else {
            // A failed restart warns and the deploy continues: the code is already on the
            // server, and aborting here would leave the operator without the hints below.
            match self.ssh_login_shell_status(&restart) {
                Ok(0) => {}
                Ok(_) | Err(_) => {
                    eprintln!(
                        "WARN: systemctl restart failed — is {} installed?",
                        self.systemd_unit
                    );
                    eprintln!(
                        "      The unit ships at {}; install it once on the server:",
                        systemd_unit::template_for(&self.systemd_unit)
                    );
                    eprintln!(
                        "        {}",
                        systemd_unit::install_command(&self.remote_dir, &self.systemd_unit)
                    );
                }
            }
        }

        println!(
            "==> unit: {} is installed by hand (see {} Phase D)",
            systemd_unit::template_for(&self.systemd_unit),
            crate::core::repository_layout::documentation::HOME_SERVER_RUNBOOK
        );
        println!(
            "    Every deployment template lives in {}",
            repository_layout::DEPLOY_DIR
        );
        println!("==> smoke hints");
        println!("    curl -sf http://127.0.0.1:8080/healthz");
        println!("    curl -sfI http://127.0.0.1:3080/");
        println!("==> done");
        Ok(0)
    }

    /// The ordered remote steps between the rsync and the restart.
    ///
    /// Postgres comes first, because the API build and the checksum repair need it. The staging
    /// host tools build right after the API, from the same checkout, and `TBD_SKIP_API_BUILD`
    /// drops both cargo steps. The web server follows the app build and belongs to compose, not
    /// to the build: `TBD_SKIP_COMPOSE` drops both compose steps, and `TBD_SKIP_SPA_BUILD` drops
    /// only the build, so Caddy still starts, reloads its Caddyfile and serves the `dist` already
    /// on the host. The checksum repair comes last, once the new tree is on the server and before
    /// the unit picks it up: a comments-only migration edit must be repointed before the new
    /// binary boots, or the boot refuses it.
    fn remote_plan(&self) -> Vec<RemoteStep> {
        let mut plan = Vec::new();
        if !self.skip_compose {
            plan.push(RemoteStep::new(
                "staging Postgres (docker compose)",
                remote_steps::postgres_start(&self.remote_dir, &self.postgres_port),
            ));
        }
        if !self.skip_api {
            plan.push(RemoteStep::new(
                "cargo build --release -p api --bin api",
                remote_steps::api_build(&self.remote_dir),
            ));
            plan.push(RemoteStep::new(
                "cargo build --release: the staging host tools (staging-fixtures, \
                 acknowledgement-dropping-relay)",
                remote_steps::staging_host_tools_build(&self.remote_dir),
            ));
        }
        if !self.skip_spa {
            plan.push(RemoteStep::new(
                "trunk build --release (Leptos SPA → frontend/dist)",
                remote_steps::spa_build(&self.remote_dir),
            ));
        }
        if !self.skip_compose {
            plan.push(RemoteStep::new(
                "staging Caddy on :3080 (docker compose), then reload its Caddyfile",
                remote_steps::web_server_start_and_reload(&self.remote_dir, &self.postgres_port),
            ));
        }
        plan.push(RemoteStep::new(
            "repoint the checksums of comments-only migration edits",
            remote_steps::migration_checksum_repair(&self.remote_dir),
        ));
        plan
    }

    /// Run one step over ssh, or print it under `--dry-run`. A non-zero exit aborts the deploy
    /// before the restart, so the running unit keeps serving the previous tree.
    fn remote_step(&self, step: &RemoteStep) -> Result<(), u8> {
        println!("==> remote: {}", step.title);
        if self.dry_run {
            println!("[dry-run] ssh … {}", step.command);
            return Ok(());
        }
        self.ssh_login_shell(&step.command)
    }

    fn ssh_base_program_args(&self) -> (String, Vec<String>) {
        if let Some(ref pass) = self.ssh_pass {
            (
                "sshpass".into(),
                vec![
                    "-p".into(),
                    pass.clone(),
                    "ssh".into(),
                    "-o".into(),
                    "StrictHostKeyChecking=no".into(),
                ],
            )
        } else if let Some(ref id) = self.ssh_identity {
            (
                "ssh".into(),
                vec![
                    "-i".into(),
                    id.clone(),
                    "-o".into(),
                    "StrictHostKeyChecking=no".into(),
                ],
            )
        } else {
            (
                "ssh".into(),
                vec!["-o".into(), "StrictHostKeyChecking=no".into()],
            )
        }
    }

    /// Runs `command` on the host in a login shell; a non-zero exit becomes the deploy's status.
    fn ssh_login_shell(&self, command: &str) -> Result<(), u8> {
        let code = self.ssh_login_shell_status(command)?;
        if code == 0 { Ok(()) } else { Err(code as u8) }
    }

    /// Runs `command` on the host in a login shell and returns its exit status. The command goes
    /// to ssh as the one quoted word [`remote_steps::login_shell`] builds, because ssh joins its
    /// remote arguments into a single line for the host's shell.
    fn ssh_login_shell_status(&self, command: &str) -> Result<i32, u8> {
        let (program, mut args) = self.ssh_base_program_args();
        args.push(self.host.clone());
        args.push(remote_steps::login_shell(command));
        // Closed fail-open: absent ssh/sshpass is NotRun, not a silent success.
        if let Err(e) = proc::which(&program) {
            return Err(not_run_exit(&e));
        }
        let mut run = Run::new(&program);
        for a in &args {
            run = run.arg(a);
        }
        match run.merged_output() {
            Ok(out) => {
                let _ = io::stdout().write_all(out.text.as_bytes());
                Ok(out.code)
            }
            Err(e) => Err(not_run_exit(&e)),
        }
    }

    /// Asks the server where its map assets live, before the `--delete` rsync runs.
    ///
    /// Refuses the deploy when the server is still on the pre-relocation layout, because this build
    /// would answer every `/map-assets` request with a 404 and log nothing about why.
    fn check_remote_assets(&self) -> Result<(), u8> {
        let script = asset_preflight::probe_script(&self.remote_dir);
        if self.dry_run {
            println!("[dry-run] ssh … {script}");
            return Ok(());
        }
        let code = self.ssh_login_shell_status(&script)?;
        asset_preflight::report(asset_preflight::classify(code), &self.remote_dir)
    }

    fn rsync_to_remote(&self) -> Result<(), u8> {
        let rsync_e = if let Some(ref pass) = self.ssh_pass {
            // rsync splits `-e` on whitespace itself, so the password is embedded unquoted.
            // A password containing whitespace would split into extra argv entries here.
            format!("sshpass -p {pass} ssh -o StrictHostKeyChecking=no")
        } else if let Some(ref id) = self.ssh_identity {
            format!("ssh -i {id} -o StrictHostKeyChecking=no")
        } else {
            "ssh -o StrictHostKeyChecking=no".to_string()
        };

        if let Err(e) = proc::which("rsync") {
            return Err(not_run_exit(&e));
        }

        let dest = format!("{}:{}/", self.host, self.remote_dir);
        let mono = format!("{}/", self.root.display());
        let run = Run::new("rsync").args(rsync_argv::rsync_argv(&rsync_e, &mono, &dest));

        match run.merged_output() {
            Ok(out) => {
                let _ = io::stdout().write_all(out.text.as_bytes());
                if out.code == 0 {
                    Ok(())
                } else {
                    Err(out.code as u8)
                }
            }
            Err(e) => Err(not_run_exit(&e)),
        }
    }
}

fn not_run_exit(e: &NotRun) -> u8 {
    match e {
        NotRun::ToolAbsent(tool) => {
            eprintln!("{tool}: command not found");
            127
        }
        other => {
            eprintln!("{other:?}");
            1
        }
    }
}

fn refuse_prairielearn(label: &str, value: &str) -> Result<(), u8> {
    if value.to_ascii_lowercase().contains("prairielearn") {
        eprintln!("Refusing to deploy: {label} must not contain 'prairielearn' (got: {value})");
        eprintln!(
            "TBD lives only under the deploy user's /home/<user>/tbd/ — see {}.",
            crate::core::repository_layout::documentation::HOME_SERVER_RUNBOOK
        );
        return Err(1);
    }
    Ok(())
}

/// Refuses a `TBD_REMOTE_DIR` outside `tbd_folder` or holding `..`: the rsync runs with
/// `--delete`, so its destination must stay inside the one folder the deploy owns.
fn require_tbd_remote_prefix(raw: &str, tbd_folder: &str) -> Result<(), u8> {
    let mut dir = raw.to_string();
    while dir.ends_with('/') && dir != "/" {
        dir.pop();
    }
    if dir.contains("..") {
        eprintln!("Refusing to deploy: TBD_REMOTE_DIR must not contain '..' (got: {raw})");
        eprintln!(
            "TBD_REMOTE_DIR must be under {tbd_folder}/ — see {}.",
            crate::core::repository_layout::documentation::HOME_SERVER_RUNBOOK
        );
        return Err(1);
    }
    if dir != tbd_folder && !dir.starts_with(&format!("{tbd_folder}/")) {
        eprintln!("Refusing to deploy: TBD_REMOTE_DIR must be under {tbd_folder}/ (got: {raw})");
        eprintln!("rsync --delete to paths outside {tbd_folder}/ is forbidden.");
        return Err(1);
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/website/tests.rs"]
mod tests;
