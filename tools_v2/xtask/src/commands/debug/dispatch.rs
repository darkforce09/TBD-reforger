use super::cli::DebugCmd;
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
            a2s_json,
        } => {
            crate::commands::debug::probes::cmd_direct_join_log(
                &log,
                &run_id,
                &remote,
                &client_build,
                &server_build,
                &symlink,
                &ping,
                &a2s_json,
            )?;
            Ok(0)
        }
        DebugCmd::DirectJoin { run_id } => {
            crate::commands::debug::direct_join::run(run_id.as_deref())
        }
    }
}
