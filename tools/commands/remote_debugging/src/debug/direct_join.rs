//! `cargo xtask debug direct-join` — one summary of why a client cannot join the staging server.
//!
//! **Role:** an orchestrator: it runs the probes of `debug a2s-probe`, `debug direct-join-log` and
//! `debug ndjson-append` in process and collects their answers into one report.
//!
//! **Position:** called by [`crate::debug::run`]. The staging host comes from
//! `deploy.env` through [`deploy_settings`]; it is resolved to an IPv4 address
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
//! section empty, because the unreachable server is the thing being diagnosed. `--instance N`
//! probes fleet instance N as `cargo xtask deploy staging` addresses it
//! (`staging_fleet_instance`; by default unit `tbd-reforger@N.service`,
//! game port 2000 + N, A2S port 17776 + N and the profile `~/tbd/fleet/instance-N/profile`); an
//! instance outside the fleet exits 1 before any probe, and fleet settings the deploy refuses skip
//! the host's probes as a `deploy.env` that does not load does. Without it the single server is
//! probed, and on a host that holds `~/tbd/fleet` the remote section reads `service=fleet_host`
//! with a line naming `--instance`. The H1 row keys its listener counts by the probed server's
//! own game and A2S ports.

use std::fs;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

use process_runner::Run;
use regex::Regex;
use verification_core::verdict::NotRun;

use crate::debug::debug_log_ids::RunId;
use crate::debug::probes::{self, DirectJoinObservations};
use crate::debug::staging_fleet_instance::{
    InstanceSelectionError, profile_under_home, select_fleet_instance,
};
use crate::error::Result;
use deploy_settings::{DeployEnvironment, DeployHostFolder, deploy_environment_path};
use deployment::staging::fleet_instances::{
    FLEET_ROOT_UNDER_HOME, FleetInstance, MAXIMUM_FLEET_INSTANCES,
};
use repository_layout::find_repository_root;

/// The single server's game port.
pub const SINGLE_SERVER_GAME_PORT: u16 = 2001;
/// The single server's A2S port.
pub const SINGLE_SERVER_A2S_PORT: u16 = 17777;
/// The single server's game and A2S ports.
const SINGLE_SERVER_PORTS: [u16; 2] = [SINGLE_SERVER_GAME_PORT, SINGLE_SERVER_A2S_PORT];
/// The single server's unit.
const SINGLE_SERVER_UNIT: &str = "tbd-reforger.service";

/// What a probe that needs the staging host records when there is none.
const SKIPPED: &str = "skipped";

/// The remote probe, run through `ssh … bash -s`: the game server unit, its two UDP listeners and
/// the last listen, A2S and client lines of the newest `console.log` under `profile_word`, a
/// profile folder already written as one shell word. With `fleet_guard` a host that holds the
/// fleet folder answers `service=fleet_host` and a line naming `--instance` instead.
fn remote_probe_script(
    unit: &str,
    profile_word: &str,
    [game, a2s]: [u16; 2],
    fleet_guard: bool,
) -> String {
    let guard = if fleet_guard {
        format!(
            "if [ -e \"$HOME\"/{FLEET_ROOT_UNDER_HOME} ]; then\n  echo \"service=fleet_host\"\n  \
             echo \"refused=this host runs a fleet under ~/{FLEET_ROOT_UNDER_HOME}; pass --instance N \
             (1 to {MAXIMUM_FLEET_INSTANCES})\"\n  exit 0\nfi\n"
        )
    } else {
        String::new()
    };
    format!(
        r#"{guard}SVC=$(systemctl --user is-active {unit} 2>/dev/null || echo inactive)
PG=$(ss -ulnp 2>/dev/null | grep -c ':{game} ' || echo 0)
PA=$(ss -ulnp 2>/dev/null | grep -c ':{a2s} ' || echo 0)
LOG=$(ls -td {profile_word}/logs/logs_* 2>/dev/null | head -1)/console.log
LISTEN=$(grep "listening on address" "$LOG" 2>/dev/null | tail -1 || echo none)
A2S=$(grep -i A2S "$LOG" 2>/dev/null | tail -2 || echo none)
CLIENT=$(grep -iE "connect|client|join|session" "$LOG" 2>/dev/null | tail -3 || echo none)
echo "service=$SVC udp{game}=$PG udp{a2s}=$PA"
echo "listen=$LISTEN"
echo "a2s=$A2S"
echo "client_lines=$CLIENT"
"#
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
    /// The remote probe's script, or why the remote probe is skipped.
    remote_script: Result<String, String>,
    /// The game and A2S ports the A2S probe queries.
    ports: [u16; 2],
    ssh_pass: Option<String>,
}

/// The game and A2S ports of the server a run probes: the fleet instance's, else the single
/// server's.
fn probed_ports(instance: Option<&FleetInstance>) -> [u16; 2] {
    instance.map_or(SINGLE_SERVER_PORTS, |instance| {
        [instance.game_port, instance.a2s_port]
    })
}

/// The staging host named in `environment` with the server to probe on it, or the reason the
/// probes that need one are skipped.
fn staging_host(
    environment: &DeployEnvironment,
    instance: Option<&FleetInstance>,
) -> Result<StagingHost, String> {
    let host = environment
        .deploy_host()
        .map_err(|error| error.to_string())?;
    let ports = probed_ports(instance);
    let remote_script = match instance {
        Some(instance) => {
            let profile = format!("\"$HOME\"/{}", single_quoted(&profile_under_home(instance)));
            let unit = instance.game_server_unit();
            Ok(remote_probe_script(&unit, &profile, ports, false))
        }
        None => DeployHostFolder::Profile
            .resolve(environment, &host)
            .map(|profile| {
                let profile = single_quoted(&profile);
                remote_probe_script(SINGLE_SERVER_UNIT, &profile, ports, true)
            })
            .map_err(|error| error.to_string()),
    };
    Ok(StagingHost {
        destination: host.ssh_destination(),
        name: host.host().to_string(),
        address: host.resolve_ipv4(),
        remote_script,
        ports,
        ssh_pass: environment.value("TBD_SSH_PASS").map(str::to_string),
    })
}

/// The report file, in the checkout's `.cursor/`.
fn debug_log_path(root: &Path) -> PathBuf {
    root.join(".cursor/debug-8fc1e0.log")
}

/// Entry for `xtask debug direct-join [RUN_ID] [--instance N]`.
pub fn run(run_id: Option<RunId>, instance: Option<u16>) -> Result<u8> {
    let root = find_repository_root()?;
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    let run_id = run_id.unwrap_or_else(|| RunId::new("user-repro"));
    match DeployEnvironment::load_if_present(&deploy_environment_path(&root)) {
        Ok(environment) => run_with(&root, &home, &environment, &run_id, instance),
        // Without `--instance` the probed server is the single server, whose ports are fixed; an
        // instance's ports come from the settings that did not load.
        Err(error) => write_report(
            &root,
            &home,
            Err(error.to_string()),
            instance.is_none().then_some(SINGLE_SERVER_PORTS),
            &run_id,
        ),
    }
}

/// Testable entry with the checkout root, `HOME` and the deploy settings injected.
pub fn run_with(
    root: &Path,
    home: &Path,
    environment: &DeployEnvironment,
    run_id: &RunId,
    instance: Option<u16>,
) -> Result<u8> {
    let server = match instance.map(|number| select_fleet_instance(environment, number)) {
        None => None,
        Some(Ok(instance)) => Some(instance),
        Some(Err(refusal @ InstanceSelectionError::OutOfRange { .. })) => {
            eprintln!("debug direct-join: {refusal}");
            return Ok(1);
        }
        Some(Err(setting)) => {
            return write_report(root, home, Err(setting.to_string()), None, run_id);
        }
    };
    let target = staging_host(environment, server.as_ref());
    let ports = probed_ports(server.as_ref());
    write_report(root, home, target, Some(ports), run_id)
}

/// Runs the probes against `target` and writes the six rows; `listener_ports` are the probed
/// server's game and A2S ports, `None` when the settings name no server.
fn write_report(
    root: &Path,
    home: &Path,
    target: Result<StagingHost, String>,
    listener_ports: Option<[u16; 2]>,
    run_id: &RunId,
) -> Result<u8> {
    // Restored on drop, so a test never leaks the prepended PATH.
    let _path = process_runner::PathGuard::prepend_dir(&home.join(".local/bin"));

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
            let remote = match &host.remote_script {
                Ok(script) => remote_probe(&host.destination, host.ssh_pass.as_deref(), script),
                Err(reason) => {
                    eprintln!("debug direct-join: {reason}; the remote probe is skipped");
                    SKIPPED.to_string()
                }
            };
            let (ping_ms, ping_address) = match &host.address {
                Ok(address) => (ping(&address.to_string()), address.to_string()),
                Err(_) => ("fail".to_string(), String::new()),
            };
            let a2s_json = probes::a2s_probe_json_for(&host.name, &host.address, &host.ports);
            (remote, ping_ms, host.name.clone(), ping_address, a2s_json)
        }
    };

    let log = debug_log_path(root);
    probes::cmd_direct_join_log(
        &log,
        &DirectJoinObservations {
            run_id,
            remote: &remote,
            listener_ports,
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
    if let Err(NotRun::ToolAbsent(_)) = process_runner::which(program_check) {
        return "service=tool_absent".into();
    }
    if pass.is_some() {
        // sshpass invokes ssh — both must exist.
        if let Err(NotRun::ToolAbsent(_)) = process_runner::which("ssh") {
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
    // A ping that cannot be spawned, or is killed by a signal, has no round trip to read.
    let output = Run::new("ping")
        .args(["-c", "1", "-W", "2", address])
        .output();
    let Ok(out) = output else {
        return "fail".into();
    };
    let combined = format!("{}{}", out.stdout, out.stderr);
    let re = Regex::new(r"time=([0-9.]+)").expect("ping time regex");
    re.captures(&combined)
        .and_then(|c| c.get(1))
        .map_or_else(|| "fail".into(), |m| m.as_str().to_string())
}

#[cfg(test)]
#[path = "tests/direct_join/tests.rs"]
mod tests;
