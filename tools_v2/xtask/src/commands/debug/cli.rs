use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub(crate) enum DebugCmd {
    #[command(name = "a2s-probe")]
    A2sProbe {
        #[arg(long, default_value = "192.168.0.140")]
        host: String,
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
        #[arg(long)]
        client_build: String,
        #[arg(long)]
        server_build: String,
        #[arg(long)]
        symlink: String,
        #[arg(long)]
        ping: String,
        #[arg(long)]
        a2s_json: String,
    },
    /// Orchestrator: runs every probe below and prints one summary.
    #[command(name = "direct-join")]
    DirectJoin {
        /// Run id written into the NDJSON block (default: user-repro).
        run_id: Option<String>,
    },
}
