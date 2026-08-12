//! T-858 — port of `scripts/deploy/deploy-website.sh` → `cargo xtask deploy website`.
//!
//! Rsync the monorepo to `TBD_REMOTE_DIR`, bring up staging Postgres, build the release
//! API + Leptos SPA on the server, restart the user-systemd unit, print Caddy hints.
//!
//! Fail-opens closed vs bash:
//! - `deploy.env` is KEY=VALUE parsed, not `source`d. A read/parse failure is a hard exit 1
//!   (bash `source` of a missing/unreadable file also fails; we additionally refuse to execute
//!   arbitrary shell from that file — that was a silent footgun, not a gate verdict).
//! - Live `rsync` / `ssh` / `sshpass` go through `tbd_gate::proc::Run`. `ToolAbsent` /
//!   `Signalled` cannot fold into "deploy succeeded" (bash `set -e` would also stop, but a
//!   swallowed status could not).
//!
//! Preserved oddities:
//! - Usage text still says `deploy-website.sh` (byte-parity with the former `--help`).
//! - Required-var errors keep the bash `: ${VAR:?}` shape including the historical
//!   `scripts/deploy/deploy-website.sh: line N:` prefix.
//! - Trailing slashes on `TBD_REMOTE_DIR` are stripped only for the prefix check; echoed
//!   remote paths and `cd '…'` payloads keep the raw value (including `////` joins).
//! - `systemctl --user restart` soft-fails with WARN (does not abort the deploy).
//! - Unknown option short-circuits left-to-right before later `--help` (bash `case` loop).

use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tbd_gate::proc::{self, Run};
use tbd_gate::verdict::NotRun;

use crate::root::find_repo_root;

/// Historical usage block — kept byte-identical to the bash `usage()` heredoc.
const USAGE: &str = "\
Usage: deploy-website.sh [--dry-run] [--help]

  Rsync the monorepo to TBD_REMOTE_DIR, bring up staging Postgres (compose),
  build the release API binary + Leptos SPA on the server, restart the
  user-systemd API unit, and print Caddy reload hints.

  --dry-run   Print the plan (rsync/ssh/compose/build/restart) without executing.
  -h, --help  Show this help.

Environment (scripts/deploy/deploy.env):
  TBD_SSH_HOST              required (e.g. sam@192.168.0.140)
  TBD_REMOTE_DIR            required (must be under /home/sam/tbd/ — never prairielearn)
  TBD_SSH_PASS              optional (sshpass)
  TBD_SSH_IDENTITY_FILE     optional (ssh -i)
  TBD_POSTGRES_HOST_PORT    optional (default 5432) — compose host port
  TBD_WEBSITE_SYSTEMD_UNIT  optional (default tbd-website-api.service)
  TBD_SKIP_COMPOSE          set to 1 to skip docker compose postgres up
  TBD_SKIP_SPA_BUILD        set to 1 to skip remote trunk build
  TBD_SKIP_API_BUILD        set to 1 to skip remote cargo build

Smoke (no SSH):
  bash scripts/deploy/deploy-website.sh --help
  bash scripts/deploy/deploy-website.sh --dry-run   # needs a filled deploy.env

Compose validate (local):
  docker compose -f apps/website/docker-compose.staging.yml config
  # on hosts with Podman only:
  podman compose -f apps/website/docker-compose.staging.yml config
";

/// Entry for `xtask deploy website`.
pub fn run(args: &[String]) -> Result<u8> {
    let mut dry_run = false;
    for arg in args {
        match arg.as_str() {
            "--dry-run" => dry_run = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(0);
            }
            other => {
                eprintln!("Unknown option: {other}");
                eprint!("{USAGE}");
                return Ok(2);
            }
        }
    }

    let root = find_repo_root()?;
    let env_file = match std::env::var("DEPLOY_ENV") {
        Ok(p) if !p.is_empty() => PathBuf::from(p),
        _ => root.join("scripts/deploy/deploy.env"),
    };

    if !env_file.is_file() {
        eprintln!(
            "Missing {} — copy from scripts/deploy/deploy.env.example",
            env_file.display()
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
    let systemd_unit = map
        .get("TBD_WEBSITE_SYSTEMD_UNIT")
        .filter(|s| !s.is_empty())
        .map(|s| s.as_str())
        .unwrap_or("tbd-website-api.service");
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
        systemd_unit: systemd_unit.to_string(),
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

        println!("==> rsync (excludes secrets, build artifacts, LFS map-assets, oracle lanes)");
        if self.dry_run {
            println!(
                "[dry-run] rsync -avz --delete … {}:{}/",
                self.host, self.remote_dir
            );
        } else if let Err(code) = self.rsync_to_remote() {
            return Ok(code);
        }

        if !self.skip_compose {
            println!("==> remote: staging Postgres (docker compose)");
            let compose_remote = format!(
                "cd '{}' &&     export TBD_POSTGRES_HOST_PORT='{}' &&     if command -v docker >/dev/null 2>&1; then       docker compose -f apps/website/docker-compose.staging.yml up -d postgres;     else       podman compose -f apps/website/docker-compose.staging.yml up -d postgres;     fi",
                self.remote_dir, self.postgres_port
            );
            if self.dry_run {
                println!("[dry-run] ssh … {compose_remote}");
            } else if let Err(code) = self.ssh_cmd(&["bash", "-lc", &compose_remote]) {
                return Ok(code);
            }
        }

        if !self.skip_api {
            println!("==> remote: cargo build --release -p website-api --bin api");
            let api_build = format!(
                "cd '{}' &&     export PATH=\"$HOME/.cargo/bin:$PATH\" &&     cargo build --release -p website-api --bin api &&     test -x target/release/api",
                self.remote_dir
            );
            if self.dry_run {
                println!("[dry-run] ssh … {api_build}");
            } else if let Err(code) = self.ssh_cmd(&["bash", "-lc", &api_build]) {
                return Ok(code);
            }
        }

        if !self.skip_spa {
            println!("==> remote: trunk build --release (Leptos SPA → frontend/dist)");
            let spa_build = format!(
                "cd '{}/apps/website/frontend' &&     export PATH=\"$HOME/.cargo/bin:$PATH\" &&     trunk build --release",
                self.remote_dir
            );
            if self.dry_run {
                println!("[dry-run] ssh … {spa_build}");
            } else if let Err(code) = self.ssh_cmd(&["bash", "-lc", &spa_build]) {
                return Ok(code);
            }
        }

        println!("==> remote: restart {}", self.systemd_unit);
        let restart = format!(
            "systemctl --user restart '{}' &&   systemctl --user is-active '{}'",
            self.systemd_unit, self.systemd_unit
        );
        if self.dry_run {
            println!("[dry-run] ssh … {restart}");
        } else {
            // Preserved soft-fail: bash `ssh_cmd … || { WARN; }` — do not abort.
            match self.ssh_cmd_status(&["bash", "-lc", &restart]) {
                Ok(0) => {}
                Ok(_) | Err(_) => {
                    eprintln!(
                        "WARN: systemctl restart failed — is {} installed?",
                        self.systemd_unit
                    );
                    eprintln!("      See docs/website/HOME_SERVER.md Phase D for the unit sketch.");
                }
            }
        }

        println!("==> Caddy");
        println!("    Ensure scripts/deploy/Caddyfile.website is loaded on the server");
        println!("    (root → $TBD_REMOTE_DIR/apps/website/frontend/dist; proxy /api → :8080).");
        println!(
            "    Example: caddy reload --config '{}/scripts/deploy/Caddyfile.website'",
            self.remote_dir
        );
        println!("==> smoke hints");
        println!("    curl -sf http://127.0.0.1:8080/healthz");
        println!("    curl -sfI http://127.0.0.1:3080/");
        println!("==> done");
        Ok(0)
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

    fn rsync_to_remote(&self) -> Result<(), u8> {
        let rsync_e = if let Some(ref pass) = self.ssh_pass {
            // Preserved oddity: bash expands `$TBD_SSH_PASS` inside the -e string unquoted
            // relative to shell word-splitting of the remote side of -e.
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
        let run = Run::new("rsync")
            .arg("-e")
            .arg(&rsync_e)
            .arg("-avz")
            .arg("--delete")
            .arg("--exclude=.git/")
            .arg("--exclude=target/")
            .arg("--exclude=target-gate-*/")
            .arg("--exclude=dist-gate-*/")
            .arg("--exclude=**/node_modules/")
            .arg("--exclude=apps/website/frontend/dist/")
            .arg("--exclude=apps/website/api/.env")
            .arg("--exclude=apps/website/api/.tools/")
            .arg("--exclude=scripts/deploy/deploy.env")
            .arg("--exclude=packages/map-assets/")
            .arg("--exclude=apps/mod/crf_framework/")
            .arg("--exclude=apps/mod/vanilla_reference/")
            .arg("--exclude=apps/mod/playable_selector/")
            .arg("--exclude=apps/mod/.local-test-profile/")
            .arg(&mono)
            .arg(&dest);

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
            // Preserved bash `: ${VAR:?msg}` shape (historical script path + line).
            eprintln!(
                "scripts/deploy/deploy-website.sh: line {line}: {key}: {key} required in deploy.env"
            );
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
mod tests {
    use super::*;

    #[test]
    fn prairielearn_case_insensitive() {
        assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/sam/PrairieLearn/x").is_err());
        assert!(refuse_prairielearn("TBD_SSH_HOST", "sam@PRAIRIELEARN.local").is_err());
        assert!(refuse_prairielearn("TBD_REMOTE_DIR", "/home/sam/tbd/repo").is_ok());
    }

    #[test]
    fn remote_prefix_rejects_escape_and_outside() {
        assert!(require_tbd_remote_prefix("/home/sam/tbd/../elsewhere").is_err());
        assert!(require_tbd_remote_prefix("/tmp/not-tbd-at-all").is_err());
        assert!(require_tbd_remote_prefix("/home/sam/tbd").is_ok());
        assert!(require_tbd_remote_prefix("/home/sam/tbd/repo///").is_ok());
    }

    #[test]
    fn usage_mentions_dry_run() {
        assert!(USAGE.contains("--dry-run"));
        assert!(USAGE.contains("TBD_REMOTE_DIR"));
    }
}
