use clap::Subcommand;
use std::path::PathBuf;

use crate::commands::debug::direct_join::{SINGLE_SERVER_A2S_PORT, SINGLE_SERVER_GAME_PORT};

#[derive(Subcommand, Debug)]
pub(crate) enum DebugCmd {
    /// A2S query of each port; prints one JSON object keyed `p<port>`.
    #[command(name = "a2s-probe")]
    A2sProbe {
        /// Host to query (default: the host of TBD_SSH_HOST in deploy.env).
        #[arg(long)]
        host: Option<String>,
        #[arg(long, default_value = "2001,17777")]
        ports: String,
    },
    #[command(name = "ndjson-append")]
    NdjsonAppend {
        #[arg(long)]
        log: PathBuf,
        #[arg(long)]
        hypothesis: String,
        #[arg(long)]
        message: String,
        #[arg(long, default_value = "{}")]
        data: String,
        #[arg(long, default_value = "")]
        run_id: String,
    },
    #[command(name = "direct-join-log")]
    DirectJoinLog {
        #[arg(long)]
        log: PathBuf,
        #[arg(long)]
        run_id: String,
        #[arg(long, default_value = "")]
        remote: String,
        /// The game port whose UDP listener count `--remote` carries (H1's `udp_<port>` key).
        #[arg(long, default_value_t = SINGLE_SERVER_GAME_PORT)]
        game_port: u16,
        /// The A2S port whose UDP listener count `--remote` carries.
        #[arg(long, default_value_t = SINGLE_SERVER_A2S_PORT)]
        a2s_port: u16,
        #[arg(long)]
        client_build: String,
        #[arg(long)]
        server_build: String,
        #[arg(long)]
        symlink: String,
        #[arg(long)]
        ping: String,
        /// The host name that was pinged and A2S-probed.
        #[arg(long, default_value = "")]
        host: String,
        /// The IPv4 address the ping and the A2S probe went to.
        #[arg(long, default_value = "")]
        address: String,
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
