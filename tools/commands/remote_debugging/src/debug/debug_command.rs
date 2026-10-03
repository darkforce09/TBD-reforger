//! The `cargo xtask debug` command line.
//!
//! **Role:** the [`DebugCmd`] clap enum: the three probe primitives and the direct-join
//! orchestrator, with their arguments and defaults.
//! **Position:** mounted by `xtask`'s `cli` as the `debug` group; routed by
//! [`crate::debug::run`].
//! **Signals & state:** none; plain data.
//! **Invariants:** the default ports of `direct-join-log` are the single server's
//! ([`SINGLE_SERVER_GAME_PORT`], [`SINGLE_SERVER_A2S_PORT`]).

use clap::Subcommand;
use std::path::PathBuf;

use crate::debug::direct_join::{SINGLE_SERVER_A2S_PORT, SINGLE_SERVER_GAME_PORT};

/// The `cargo xtask debug` subcommands.
#[derive(Subcommand, Debug)]
pub enum DebugCmd {
    /// A2S query of each port; prints one JSON object keyed `p<port>`.
    #[command(name = "a2s-probe")]
    A2sProbe {
        /// Host to query (default: the host of TBD_SSH_HOST in deploy.env).
        #[arg(long)]
        host: Option<String>,
        /// Comma-separated UDP ports to query.
        #[arg(long, default_value = "2001,17777")]
        ports: String,
    },
    /// Append one NDJSON debug row to a log.
    #[command(name = "ndjson-append")]
    NdjsonAppend {
        /// The log file the row is appended to.
        #[arg(long)]
        log: PathBuf,
        /// The hypothesis id the row records (`H1`…).
        #[arg(long)]
        hypothesis: String,
        /// The row's message.
        #[arg(long)]
        message: String,
        /// The row's data as JSON; text that is not JSON is written as `{}`.
        #[arg(long, default_value = "{}")]
        data: String,
        /// The run id the row records.
        #[arg(long, default_value = "")]
        run_id: String,
    },
    /// Append the six direct-join hypothesis rows (H1 to H6) to a log.
    #[command(name = "direct-join-log")]
    DirectJoinLog {
        /// The log file the rows are appended to.
        #[arg(long)]
        log: PathBuf,
        /// The run id the rows record.
        #[arg(long)]
        run_id: String,
        /// The remote probe's output: service state, UDP listeners and log lines.
        #[arg(long, default_value = "")]
        remote: String,
        /// The game port whose UDP listener count `--remote` carries (H1's `udp_<port>` key).
        #[arg(long, default_value_t = SINGLE_SERVER_GAME_PORT)]
        game_port: u16,
        /// The A2S port whose UDP listener count `--remote` carries.
        #[arg(long, default_value_t = SINGLE_SERVER_A2S_PORT)]
        a2s_port: u16,
        /// The client's Steam build id.
        #[arg(long)]
        client_build: String,
        /// The server's Steam build id.
        #[arg(long)]
        server_build: String,
        /// The resolved client addon link, or `missing`.
        #[arg(long)]
        symlink: String,
        /// The ping round trip in milliseconds, or `fail`.
        #[arg(long)]
        ping: String,
        /// The host name that was pinged and A2S-probed.
        #[arg(long, default_value = "")]
        host: String,
        /// The IPv4 address the ping and the A2S probe went to.
        #[arg(long, default_value = "")]
        address: String,
        /// The A2S probe's JSON answer.
        #[arg(long)]
        a2s_json: String,
    },
    /// Orchestrator: runs the probes above and prints one summary.
    #[command(name = "direct-join")]
    DirectJoin {
        /// Run id written into the NDJSON block (default: user-repro).
        run_id: Option<String>,
        /// Fleet instance to probe, 1 to 5: unit tbd-reforger@N, game port 2000+N, A2S 17776+N.
        #[arg(long)]
        instance: Option<u16>,
    },
}
