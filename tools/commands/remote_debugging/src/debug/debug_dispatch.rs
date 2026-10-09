//! The routing of `cargo xtask debug`.
//!
//! **Role:** [`run`] sends each [`DebugCmd`] to its probe; `a2s-probe`'s port list and its host
//! default (the host of `TBD_SSH_HOST`) are resolved here.
//! **Position:** called by `xtask`'s `cli` dispatch; calls [`crate::debug::probes`] and
//! [`crate::debug::direct_join`].
//! **Signals & state:** none held; `a2s-probe` without `--host` reads `deploy.env`.
//! **Invariants:** an explicit `--host` wins over `deploy.env`, which is then never read; no
//! usable host exits 1 with one stderr line.

use super::debug_command::DebugCmd;
use crate::debug::debug_log_ids::{HypothesisId, RunId};
use crate::debug::probes::DirectJoinObservations;
use crate::error::{Error, Result};
use deploy_settings::{DeployEnvironment, SettingError, deploy_environment_path};
use repository_root::find_repository_root;

/// Runs one `cargo xtask debug` command and returns its exit code.
pub fn run(cmd: DebugCmd) -> Result<u8> {
    match cmd {
        DebugCmd::A2sProbe { host, ports } => {
            let ports: Vec<u16> = ports
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if ports.is_empty() {
                return Err(Error::Refused("no ports".to_string()));
            }
            // An explicit --host beats deploy.env, which is read only when it is absent.
            let host = match host {
                Some(host) => host,
                None => {
                    let path = deploy_environment_path(&find_repository_root()?);
                    let default = DeployEnvironment::load_if_present(&path)
                        .map_err(|error| error.to_string())
                        .and_then(|environment| default_probe_host(&environment));
                    match default {
                        Ok(host) => host,
                        Err(message) => {
                            eprintln!("debug a2s-probe: {message}");
                            return Ok(1);
                        }
                    }
                }
            };
            crate::debug::probes::cmd_a2s_probe(&host, &ports)?;
            Ok(0)
        }
        DebugCmd::NdjsonAppend {
            log,
            hypothesis,
            message,
            data,
            run_id,
        } => {
            crate::debug::probes::cmd_ndjson_append(
                &log,
                &HypothesisId::new(hypothesis),
                &message,
                &data,
                &RunId::new(run_id),
            )?;
            Ok(0)
        }
        DebugCmd::DirectJoinLog {
            log,
            run_id,
            remote,
            game_port,
            a2s_port,
            client_build,
            server_build,
            symlink,
            ping,
            host,
            address,
            a2s_json,
        } => {
            crate::debug::probes::cmd_direct_join_log(
                &log,
                &DirectJoinObservations {
                    run_id: &RunId::new(run_id),
                    remote: &remote,
                    listener_ports: Some([game_port, a2s_port]),
                    client_build: &client_build,
                    server_build: &server_build,
                    symlink: &symlink,
                    ping_ms: &ping,
                    ping_host: &host,
                    ping_address: &address,
                    a2s_json: &a2s_json,
                },
            )?;
            Ok(0)
        }
        DebugCmd::DirectJoin { run_id, instance } => {
            crate::debug::direct_join::run(run_id.map(RunId::new), instance)
        }
    }
}

/// The host `a2s-probe` queries without `--host`: the host part of `TBD_SSH_HOST`, or the message
/// that says how to name one.
fn default_probe_host(environment: &DeployEnvironment) -> Result<String, String> {
    match environment.deploy_host() {
        Ok(host) => Ok(host.host().to_string()),
        Err(SettingError::Missing { .. }) => Err(format!(
            "pass --host, or set TBD_SSH_HOST in {}",
            environment.path().display()
        )),
        Err(error) => Err(error.to_string()),
    }
}
