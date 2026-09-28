use super::cli::DebugCmd;
use crate::commands::debug::probes::DirectJoinObservations;
use crate::core::deploy_environment::{DeployEnvironment, SettingError, deploy_environment_path};
use crate::core::repository_root::find_repo_root;
use anyhow::{Result, bail};

pub(crate) fn run(cmd: DebugCmd) -> Result<u8> {
    match cmd {
        DebugCmd::A2sProbe { host, ports } => {
            let ports: Vec<u16> = ports
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if ports.is_empty() {
                bail!("no ports");
            }
            // An explicit --host beats deploy.env, which is read only when it is absent.
            let host = match host {
                Some(host) => host,
                None => {
                    let path = deploy_environment_path(&find_repo_root()?);
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
            crate::commands::debug::probes::cmd_a2s_probe(&host, &ports)?;
            Ok(0)
        }
        DebugCmd::NdjsonAppend {
            log,
            hypothesis,
            message,
            data,
            run_id,
        } => {
            crate::commands::debug::probes::cmd_ndjson_append(
                &log,
                &hypothesis,
                &message,
                &data,
                &run_id,
            )?;
            Ok(0)
        }
        DebugCmd::DirectJoinLog {
            log,
            run_id,
            remote,
            client_build,
            server_build,
            symlink,
            ping,
            host,
            address,
            a2s_json,
        } => {
            crate::commands::debug::probes::cmd_direct_join_log(
                &log,
                &DirectJoinObservations {
                    run_id: &run_id,
                    remote: &remote,
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
        DebugCmd::DirectJoin { run_id } => {
            crate::commands::debug::direct_join::run(run_id.as_deref())
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
