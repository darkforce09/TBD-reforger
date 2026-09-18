//! T-868 — port of `scripts/mod/debug-direct-join.sh` → `cargo xtask debug direct-join`.
//!
//! Path pins MIRROR the values `scripts/mod/lib/paths.sh` defined; nothing here reads that file.
//! T-853 deleted `lib/xtask-run.sh` (zero consumers); `lib/paths.sh` survives only until
//! `deploy-staging.sh`, its last consumer, is ported.
//! `MONO_ROOT`, `MOD_ROOT=apps/mod`, `SCHEMA=packages/tbd-schema`, `WEB=apps/website/api_v2`,
//! `DEPLOY_ENV=scripts/deploy/deploy.env`.
//!
//! Orchestrator only: keeps `debug a2s-probe` / `debug direct-join-log` / `debug ndjson-append`.
//!
//! Fail-opens closed / pinned vs bash:
//! - Steam `buildid`: missing file / no match → `unknown` (pinned soft probe). A hit whose awk
//!   `$3` is empty (typical two-field Steam `"buildid"\t"N"` lines) stays **empty**, not
//!   `unknown` — preserved oddity of the bash pipeline.
//! - Symlink: `readlink -f` fail → `missing` (pinned soft probe).
//! - Ping: no `time=` match → `fail` (pinned soft probe).
//! - Remote SSH: bash `2>/dev/null || true` collapsed ToolAbsent and transport errors into empty
//!   `REMOTE_OUT`. **Closed for ToolAbsent** when `TBD_SSH_HOST` is set (`service=tool_absent`);
//!   transport / nonzero / stderr-only failures still collapse to empty (pinned preserved oddity —
//!   the debug helper must not abort the local summary).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use regex::Regex;
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

use crate::core::repository_root::find_repo_root;

const PING_HOST: &str = "192.168.0.140";
const A2S_PORTS: &[u16] = &[2001, 17777];

/// Remote probe body — byte-stable with the former bash `<<'RS'` heredoc.
const REMOTE_SCRIPT: &str = r#"SVC=$(systemctl --user is-active tbd-reforger.service 2>/dev/null || echo inactive)
P2001=$(ss -ulnp 2>/dev/null | grep -c ':2001 ' || echo 0)
P17777=$(ss -ulnp 2>/dev/null | grep -c ':17777 ' || echo 0)
LOG=$(ls -td /home/sam/tbd/profile/logs/logs_* 2>/dev/null | head -1)/console.log
LISTEN=$(grep "listening on address" "$LOG" 2>/dev/null | tail -1 || echo none)
A2S=$(grep -i A2S "$LOG" 2>/dev/null | tail -2 || echo none)
CLIENT=$(grep -iE "connect|client|join|session" "$LOG" 2>/dev/null | tail -3 || echo none)
echo "service=$SVC udp2001=$P2001 udp17777=$P17777"
echo "listen=$LISTEN"
echo "a2s=$A2S"
echo "client_lines=$CLIENT"
"#;

/// Paths mirroring `scripts/mod/lib/paths.sh`.
struct Paths {
    mono_root: PathBuf,
    #[allow(dead_code)]
    mod_root: PathBuf,
    #[allow(dead_code)]
    schema: PathBuf,
    #[allow(dead_code)]
    web: PathBuf,
    deploy_env: PathBuf,
}

impl Paths {
    fn from_root(root: &Path) -> Self {
        Self {
            mono_root: root.to_path_buf(),
            mod_root: root.join("apps/mod"),
            schema: root.join("packages/tbd-schema"),
            web: root.join("apps/website/api_v2"),
            deploy_env: root.join("scripts/deploy/deploy.env"),
        }
    }

    fn debug_log(&self) -> PathBuf {
        self.mono_root.join(".cursor/debug-8fc1e0.log")
    }
}

/// Entry for `xtask debug direct-join [RUN_ID]`.
pub fn run(run_id: Option<&str>) -> Result<u8> {
    let root = find_repo_root()?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    run_with(
        root.as_path(),
        home.as_path(),
        run_id.unwrap_or("user-repro"),
    )
}

/// Testable entry with explicit mono root + HOME (throwaway Steam trees).
pub fn run_with(root: &Path, home: &Path, run_id: &str) -> Result<u8> {
    let paths = Paths::from_root(root);

    // bash: PATH="$HOME/.local/bin:$PATH" — restore on drop so unit tests never leak PATH.
    let _path = crate::core::test_environment::PathGuard::prepend_dir(&home.join(".local/bin"));

    // bash: `[ -f "$ENV_FILE" ] && source "$ENV_FILE"` then read TBD_SSH_*.
    let (ssh_host, ssh_pass) = load_ssh_vars(&paths.deploy_env);

    let client_build = steam_build_id(home, "1874880");
    let server_build = steam_build_id(home, "1874900");
    let symlink = read_symlink(home);
    let remote_out = remote_probe(ssh_host.as_deref(), ssh_pass.as_deref());
    let ping = ping_ms(PING_HOST);
    let a2s_json = crate::commands::debug::probes::a2s_probe_json(PING_HOST, A2S_PORTS);

    let log = paths.debug_log();
    crate::commands::debug::probes::cmd_direct_join_log(
        &log,
        run_id,
        &remote_out,
        &client_build,
        &server_build,
        &symlink,
        &ping,
        &a2s_json,
    )?;

    println!("Wrote debug log: {}", log.display());
    println!("--- summary ---");
    println!("Client build: {client_build} | Server build: {server_build}");
    println!("Symlink: {symlink}");
    // bash: `echo "$REMOTE_OUT"` — empty still prints a blank line.
    println!("{remote_out}");
    Ok(0)
}

fn load_ssh_vars(deploy_env: &Path) -> (Option<String>, Option<String>) {
    let mut host = std::env::var("TBD_SSH_HOST").ok().filter(|s| !s.is_empty());
    let mut pass = std::env::var("TBD_SSH_PASS").ok().filter(|s| !s.is_empty());
    if deploy_env.is_file() {
        if let Ok(map) = parse_deploy_env(deploy_env) {
            // bash `source` overlays file onto the shell.
            if let Some(v) = map.get("TBD_SSH_HOST").filter(|s| !s.is_empty()) {
                host = Some(v.clone());
            }
            if let Some(v) = map.get("TBD_SSH_PASS").filter(|s| !s.is_empty()) {
                pass = Some(v.clone());
            }
        }
    }
    (host, pass)
}

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

/// bash: `grep buildid FILE | awk '{print $3}' | tr -d '"' || echo unknown`
fn steam_build_id(home: &Path, app_id: &str) -> String {
    let path = home
        .join(".local/share/Steam/steamapps")
        .join(format!("appmanifest_{app_id}.acf"));
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => return "unknown".into(),
    };
    let mut out = String::new();
    let mut any = false;
    for line in text.lines() {
        if !line.contains("buildid") {
            continue;
        }
        any = true;
        let fields: Vec<&str> = line.split_whitespace().collect();
        let third = fields.get(2).copied().unwrap_or("");
        let cleaned: String = third.chars().filter(|&c| c != '"').collect();
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&cleaned);
    }
    if any { out } else { "unknown".into() }
}

/// bash: `readlink -f "$HOME/.local/share/tbd-server-addons/tbd-framework" || echo missing`
fn read_symlink(home: &Path) -> String {
    let path = home.join(".local/share/tbd-server-addons/tbd-framework");
    match fs::canonicalize(&path) {
        Ok(p) => p.display().to_string(),
        Err(_) => "missing".into(),
    }
}

fn remote_probe(host: Option<&str>, pass: Option<&str>) -> String {
    let Some(host) = host.filter(|s| !s.is_empty()) else {
        return String::new();
    };

    // Closed fail-open: bash would `|| true` over a missing ssh/sshpass into empty remote.
    let program_check = if pass.is_some() { "sshpass" } else { "ssh" };
    if let Err(NotRun::ToolAbsent(_)) = proc::which(program_check) {
        return "service=tool_absent".into();
    }
    if pass.is_some() {
        // sshpass invokes ssh — both must exist.
        if let Err(NotRun::ToolAbsent(_)) = proc::which("ssh") {
            return "service=tool_absent".into();
        }
    }

    let mut args: Vec<String> = Vec::new();
    let program;
    if let Some(p) = pass.filter(|s| !s.is_empty()) {
        program = "sshpass";
        args.push("-p".into());
        args.push(p.into());
        args.push("ssh".into());
        args.push("-o".into());
        args.push("StrictHostKeyChecking=no".into());
        args.push(host.into());
        args.push("bash".into());
        args.push("-s".into());
    } else {
        program = "ssh";
        args.push("-o".into());
        args.push("StrictHostKeyChecking=no".into());
        args.push(host.into());
        args.push("bash".into());
        args.push("-s".into());
    }

    // Preserved oddity: transport / nonzero → empty remote (bash `2>/dev/null || true`).
    let mut run = Run::new(program).stdin(REMOTE_SCRIPT);
    for a in &args {
        run = run.arg(a);
    }
    match run.merged_output() {
        Ok(out) if out.code == 0 => out.text,
        _ => String::new(),
    }
}

/// bash: `ping -c 1 -W 2 HOST 2>&1 | grep -oP 'time=\K[0-9.]+' || echo fail`
fn ping_ms(host: &str) -> String {
    let output = Command::new("ping")
        .args(["-c", "1", "-W", "2", host])
        .output();
    let Ok(out) = output else {
        return "fail".into();
    };
    let combined = {
        let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&out.stderr));
        s
    };
    let re = Regex::new(r"time=([0-9.]+)").expect("ping time regex");
    if let Some(c) = re.captures(&combined) {
        c.get(1).unwrap().as_str().to_string()
    } else {
        "fail".into()
    }
}

#[cfg(test)]
#[path = "tests/direct_join/tests.rs"]
mod tests;
