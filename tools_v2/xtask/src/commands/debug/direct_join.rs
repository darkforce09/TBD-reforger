//! `cargo xtask debug direct-join` — one summary of why a client cannot join the staging server.
//!
//! **Role:** an orchestrator: it runs the probes of `debug a2s-probe`, `debug direct-join-log` and
//! `debug ndjson-append` in process and collects their answers into one report.
//!
//! **Position:** called by [`crate::commands::debug::dispatch`]. The staging host comes from
//! `deploy.env` through [`crate::core::deploy_environment`]; it is resolved to an IPv4 address
//! once, and the ping and the A2S probe both use that address.
//!
//! **Signals & state:** prepends `$HOME/.local/bin` to `PATH` for the run and restores it on
//! return; appends six rows to `.cursor/debug-8fc1e0.log` in the checkout.
//!
//! **Invariants:** every local probe is soft, because the summary is worth more complete than
//! strict: a missing Steam manifest or an unmatched `buildid` reports `unknown`, a symlink that
//! does not resolve reports `missing`, and a ping with no `time=` reports `fail`. A `buildid`
//! line with only two fields — which is what Steam writes — reports empty rather than `unknown`,
//! so an operator can tell "the file said nothing here" from "there was no file". With no host,
//! or a `deploy.env` that does not load, the remote, ping and A2S probes record `skipped`, one
//! stderr line names the settings file, and the command still exits 0. With a host set and no
//! `ssh`, the remote probe reports `service=tool_absent`; any other ssh failure leaves the remote
//! section empty, because the unreachable server is the thing being diagnosed.

use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Result;
use regex::Regex;
use verification_core::proc::{self, Run};
use verification_core::verdict::NotRun;

use crate::commands::debug::probes::{self, DirectJoinObservations};
use crate::core::deploy_environment::{
    DeployEnvironment, DeployHostFolder, deploy_environment_path,
};
use crate::core::repository_root::find_repo_root;

const A2S_PORTS: &[u16] = &[2001, 17777];

/// What a probe that needs the staging host records when there is none.
const SKIPPED: &str = "skipped";

/// The remote probe, run through `ssh … bash -s`: the game server unit, its UDP listeners and the
/// last listen, A2S and client lines of the newest `console.log` under the profile folder.
fn remote_probe_script(profile: &str) -> String {
    format!(
        r#"SVC=$(systemctl --user is-active tbd-reforger.service 2>/dev/null || echo inactive)
P2001=$(ss -ulnp 2>/dev/null | grep -c ':2001 ' || echo 0)
P17777=$(ss -ulnp 2>/dev/null | grep -c ':17777 ' || echo 0)
LOG=$(ls -td {profile}/logs/logs_* 2>/dev/null | head -1)/console.log
LISTEN=$(grep "listening on address" "$LOG" 2>/dev/null | tail -1 || echo none)
A2S=$(grep -i A2S "$LOG" 2>/dev/null | tail -2 || echo none)
CLIENT=$(grep -iE "connect|client|join|session" "$LOG" 2>/dev/null | tail -3 || echo none)
echo "service=$SVC udp2001=$P2001 udp17777=$P17777"
echo "listen=$LISTEN"
echo "a2s=$A2S"
echo "client_lines=$CLIENT"
"#,
        profile = single_quoted(profile)
    )
}

/// `value` as one POSIX shell word, whatever it holds.
fn single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// The staging host the remote, ping and A2S probes reach, resolved once.
struct StagingHost {
    destination: String,
    name: String,
    address: Result<Ipv4Addr, String>,
    /// The profile folder, or why the remote probe is skipped.
    profile: Result<String, String>,
    ssh_pass: Option<String>,
}

/// The staging host named in `environment`, or the reason the probes that need one are skipped.
fn staging_host(environment: &DeployEnvironment) -> Result<StagingHost, String> {
    let host = environment
        .deploy_host()
        .map_err(|error| error.to_string())?;
    Ok(StagingHost {
        destination: host.ssh_destination(),
        name: host.host().to_string(),
        address: host.resolve_ipv4(),
        profile: DeployHostFolder::Profile
            .resolve(environment, &host)
            .map_err(|error| error.to_string()),
        ssh_pass: environment.value("TBD_SSH_PASS").map(str::to_string),
    })
}

/// The report file, in the checkout's `.cursor/`.
fn debug_log_path(root: &Path) -> PathBuf {
    root.join(".cursor/debug-8fc1e0.log")
}

/// Entry for `xtask debug direct-join [RUN_ID]`.
pub fn run(run_id: Option<&str>) -> Result<u8> {
    let root = find_repo_root()?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let run_id = run_id.unwrap_or("user-repro");
    match DeployEnvironment::load_if_present(&deploy_environment_path(&root)) {
        Ok(environment) => run_with(&root, &home, &environment, run_id),
        Err(error) => write_report(&root, &home, Err(error.to_string()), run_id),
    }
}

/// Testable entry with the checkout root, `HOME` and the deploy settings injected.
pub fn run_with(
    root: &Path,
    home: &Path,
    environment: &DeployEnvironment,
    run_id: &str,
) -> Result<u8> {
    write_report(root, home, staging_host(environment), run_id)
}

fn write_report(
    root: &Path,
    home: &Path,
    target: Result<StagingHost, String>,
    run_id: &str,
) -> Result<u8> {
    // Restored on drop, so a test never leaks the prepended PATH.
    let _path = crate::core::test_environment::PathGuard::prepend_dir(&home.join(".local/bin"));

    let client_build = steam_build_id(home, "1874880");
    let server_build = steam_build_id(home, "1874900");
    let symlink = read_symlink(home);
    let (remote, ping_ms, ping_host, ping_address, a2s_json) = match &target {
        Err(reason) => {
            eprintln!("debug direct-join: {reason}; the remote, ping and A2S probes are skipped");
            let skipped = serde_json::json!({ "skipped": reason }).to_string();
            (
                SKIPPED.to_string(),
                SKIPPED.to_string(),
                String::new(),
                String::new(),
                skipped,
            )
        }
        Ok(host) => {
            let remote = match &host.profile {
                Ok(profile) => remote_probe(
                    &host.destination,
                    host.ssh_pass.as_deref(),
                    &remote_probe_script(profile),
                ),
                Err(reason) => {
                    eprintln!("debug direct-join: {reason}; the remote probe is skipped");
                    SKIPPED.to_string()
                }
            };
            let (ping_ms, ping_address) = match &host.address {
                Ok(address) => (ping(&address.to_string()), address.to_string()),
                Err(_) => ("fail".to_string(), String::new()),
            };
            let a2s_json = probes::a2s_probe_json_for(&host.name, &host.address, A2S_PORTS);
            (remote, ping_ms, host.name.clone(), ping_address, a2s_json)
        }
    };

    let log = debug_log_path(root);
    probes::cmd_direct_join_log(
        &log,
        &DirectJoinObservations {
            run_id,
            remote: &remote,
            client_build: &client_build,
            server_build: &server_build,
            symlink: &symlink,
            ping_ms: &ping_ms,
            ping_host: &ping_host,
            ping_address: &ping_address,
            a2s_json: &a2s_json,
        },
    )?;

    println!("Wrote debug log: {}", log.display());
    println!("--- summary ---");
    println!("Client build: {client_build} | Server build: {server_build}");
    println!("Symlink: {symlink}");
    let ping_target = match (ping_host.is_empty(), ping_address.is_empty()) {
        (true, _) => String::new(),
        (false, true) => format!(" {ping_host}"),
        (false, false) => format!(" {ping_host} ({ping_address})"),
    };
    println!("Ping{ping_target}: {ping_ms}");
    println!("A2S: {a2s_json}");
    // An empty remote section still prints its blank line.
    println!("{remote}");
    Ok(0)
}

/// The third whitespace field of each `buildid` line of the app's Steam manifest, quotes removed;
/// `unknown` when the manifest or the line is missing.
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

/// The resolved client addon link, or `missing`.
fn read_symlink(home: &Path) -> String {
    let path = home.join(".local/share/tbd-server-addons/tbd-framework");
    match fs::canonicalize(&path) {
        Ok(p) => p.display().to_string(),
        Err(_) => "missing".into(),
    }
}

fn remote_probe(destination: &str, pass: Option<&str>, script: &str) -> String {
    // An absent ssh or sshpass is reported as itself, never as an empty remote section.
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
    let program = match pass {
        Some(p) => {
            args.extend(["-p".into(), p.into(), "ssh".into()]);
            "sshpass"
        }
        None => "ssh",
    };
    args.extend([
        "-o".into(),
        "StrictHostKeyChecking=no".into(),
        destination.into(),
        "bash".into(),
        "-s".into(),
    ]);

    // A transport failure or a non-zero remote exit leaves the remote section empty.
    let mut run = Run::new(program).stdin(script);
    for a in &args {
        run = run.arg(a);
    }
    match run.merged_output() {
        Ok(out) if out.code == 0 => out.text,
        _ => String::new(),
    }
}

/// The round trip of one `ping -c 1 -W 2` in milliseconds, or `fail`.
fn ping(address: &str) -> String {
    let output = Command::new("ping")
        .args(["-c", "1", "-W", "2", address])
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
