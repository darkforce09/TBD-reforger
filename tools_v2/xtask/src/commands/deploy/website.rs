//! `cargo xtask deploy website` — rsync the monorepo to the server, bring up staging Postgres,
//! build the release API and the Leptos SPA there, restart the user-systemd unit, print the
//! Caddy hints.
//!
//! Two refusals shape the command. The host file at
//! [`repository_layout::DEPLOY_ENV`] is parsed as `KEY=VALUE` and never executed, so a deploy
//! cannot be turned into arbitrary shell by editing a configuration file; an unreadable or
//! malformed file exits 1 rather than deploying with defaults. Live `rsync`, `ssh` and `sshpass`
//! run through `verification_core::proc::Run`, so a missing tool or a killed child is reported as
//! itself and can never fold into "deploy succeeded".
//!
//! Two behaviours are deliberate and easy to misread as bugs. Trailing slashes on
//! `TBD_REMOTE_DIR` are stripped only for the `/home/sam/tbd/` prefix check — echoed remote paths
//! and `cd '…'` payloads keep the operator's raw value, so what is printed is what runs. And a
//! failed `systemctl --user restart` warns instead of aborting: the code and the database are
//! already on the server by then, so the deploy is done and the restart is the operator's to
//! finish.

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

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
    let env_file = match std::env::var("DEPLOY_ENV") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => root.join(repository_layout::DEPLOY_ENV),
    };

    if !env_file.is_file() {
        eprintln!(
            "Missing {} — copy from {}",
            env_file.display(),
            repository_layout::DEPLOY_ENV_EXAMPLE
        );
        return Ok(1);
    }

    let map = match parse_deploy_env(&env_file) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e:#}");
            return Ok(1);
        }
    };

    let host = match require_var(&map, "TBD_SSH_HOST", 79) {
        Ok(v) => v,
        Err(code) => return Ok(code),
    };
    let remote_dir = match require_var(&map, "TBD_REMOTE_DIR", 80) {
        Ok(v) => v,
        Err(code) => return Ok(code),
    };
    let postgres_port = map
        .get("TBD_POSTGRES_HOST_PORT")
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .unwrap_or("5432");
    let systemd_unit: String = map
        .get("TBD_WEBSITE_SYSTEMD_UNIT")
        .filter(|s| !s.is_empty())
        .cloned()
        .unwrap_or_else(|| systemd_unit::default_unit_name().to_string());
    let skip_compose = map.get("TBD_SKIP_COMPOSE").map(|s| s.as_str()) == Some("1");
    let skip_spa = map.get("TBD_SKIP_SPA_BUILD").map(|s| s.as_str()) == Some("1");
    let skip_api = map.get("TBD_SKIP_API_BUILD").map(|s| s.as_str()) == Some("1");
    let ssh_pass = map.get("TBD_SSH_PASS").filter(|s| !s.is_empty()).cloned();
    let ssh_identity = map
        .get("TBD_SSH_IDENTITY_FILE")
        .filter(|s| !s.is_empty())
        .cloned();
    let profile_dir = map
        .get("TBD_PROFILE_DIR")
        .filter(|s| !s.is_empty())
        .cloned();

    if let Err(code) = refuse_prairielearn("TBD_REMOTE_DIR", &remote_dir) {
        return Ok(code);
    }
    if let Err(code) = refuse_prairielearn("TBD_SSH_HOST", &host) {
        return Ok(code);
    }
    if let Some(ref p) = profile_dir
        && let Err(code) = refuse_prairielearn("TBD_PROFILE_DIR", p)
    {
        return Ok(code);
    }
    if let Err(code) = require_tbd_remote_prefix(&remote_dir) {
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
            "==> rsync (excludes secrets, build artifacts, the terrain + scratch asset trees, \
             the legacy packages/ tree, oracle lanes)"
        );
        if self.dry_run {
            println!(
                "[dry-run] rsync -avz --delete … {}:{}/",
                self.host, self.remote_dir
            );
            // The exclude list is the whole point of a dry run: with `--delete` and no
            // `--delete-excluded`, every entry is also what keeps rsync from removing that path
            // on the server. Print it rather than eliding it behind the ellipsis.
            let argv = rsync_argv::rsync_argv("", "", "");
            for excluded in rsync_argv::exclusions(&argv) {
                println!("[dry-run]   --exclude={excluded}");
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
            match self.ssh_cmd_status(&["bash", "-lc", &restart]) {
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

        println!("==> Caddy");
        println!(
            "    Ensure {} is loaded on the server",
            repository_layout::CADDYFILE
        );
        println!("    (root → $TBD_REMOTE_DIR/apps/website/frontend/dist; proxy /api → :8080).");
        println!(
            "    Example: caddy reload --config '{}/{}'",
            self.remote_dir,
            repository_layout::CADDYFILE
        );
        println!(
            "==> unit: {} is installed by hand (see docs/website/HOME_SERVER.md Phase D)",
            systemd_unit::template_for(&self.systemd_unit)
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

    /// The ordered remote steps between the rsync and the restart. The checksum repair and the
    /// state-directory move come last, once the new tree is on the server and before the unit picks
    /// it up: a comments-only migration edit must be repointed before the new binary boots, or the
    /// boot refuses it, and the runtime files must already be where the unit's environment points.
    fn remote_plan(&self) -> Vec<RemoteStep> {
        let mut plan = Vec::new();
        if !self.skip_compose {
            plan.push(RemoteStep::new(
                "staging Postgres (docker compose)",
                remote_steps::compose_up(&self.remote_dir, &self.postgres_port),
            ));
        }
        if !self.skip_api {
            plan.push(RemoteStep::new(
                "cargo build --release -p website-api --bin api",
                remote_steps::api_build(&self.remote_dir),
            ));
        }
        if !self.skip_spa {
            plan.push(RemoteStep::new(
                "trunk build --release (Leptos SPA → frontend/dist)",
                remote_steps::spa_build(&self.remote_dir),
            ));
        }
        plan.push(RemoteStep::new(
            "repoint the checksums of comments-only migration edits",
            remote_steps::migration_checksum_repair(&self.remote_dir),
        ));
        plan.push(RemoteStep::new(
            "move runtime files into the unit's state directory",
            remote_steps::runtime_state_move(&self.remote_dir),
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
        self.ssh_cmd(&["bash", "-lc", &step.command])
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

    fn ssh_cmd(&self, remote_args: &[&str]) -> Result<(), u8> {
        let code = self.ssh_cmd_status(remote_args)?;
        if code == 0 { Ok(()) } else { Err(code as u8) }
    }

    fn ssh_cmd_status(&self, remote_args: &[&str]) -> Result<i32, u8> {
        let (program, mut args) = self.ssh_base_program_args();
        args.push(self.host.clone());
        for a in remote_args {
            args.push((*a).into());
        }
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
        let code = self.ssh_cmd_status(&["bash", "-lc", &script])?;
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

fn require_var(map: &HashMap<String, String>, key: &str, line: u32) -> Result<String, u8> {
    match map.get(key) {
        Some(v) if !v.is_empty() => Ok(v.clone()),
        _ => {
            // `line` is the line of the deploy file the value is expected on, so the message
            // points at the edit to make rather than at the check that refused.
            eprintln!("deploy.env: line {line}: {key}: {key} required in deploy.env");
            Err(1)
        }
    }
}

fn refuse_prairielearn(label: &str, value: &str) -> Result<(), u8> {
    if value.to_ascii_lowercase().contains("prairielearn") {
        eprintln!("Refusing to deploy: {label} must not contain 'prairielearn' (got: {value})");
        eprintln!("TBD lives under /home/sam/tbd/ only — see docs/website/HOME_SERVER.md.");
        return Err(1);
    }
    Ok(())
}

fn require_tbd_remote_prefix(raw: &str) -> Result<(), u8> {
    let mut dir = raw.to_string();
    while dir.ends_with('/') && dir != "/" {
        dir.pop();
    }
    if dir.contains("..") {
        eprintln!("Refusing to deploy: TBD_REMOTE_DIR must not contain '..' (got: {raw})");
        eprintln!("TBD_REMOTE_DIR must be under /home/sam/tbd/ — see docs/website/HOME_SERVER.md.");
        return Err(1);
    }
    let allowed = "/home/sam/tbd";
    if dir != allowed && !dir.starts_with(&format!("{allowed}/")) {
        eprintln!("Refusing to deploy: TBD_REMOTE_DIR must be under /home/sam/tbd/ (got: {raw})");
        eprintln!("rsync --delete to paths outside /home/sam/tbd/ is forbidden.");
        return Err(1);
    }
    Ok(())
}

/// KEY=VALUE parser (not a shell `source`). Strips an optional leading `export `.
fn parse_deploy_env(path: &Path) -> Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some((k, v)) = line.split_once('=') {
            let v = v.trim().trim_matches('"').trim_matches('\'').to_string();
            map.insert(k.trim().to_string(), v);
        }
    }
    Ok(map)
}

#[cfg(test)]
#[path = "tests/website/tests.rs"]
mod tests;
