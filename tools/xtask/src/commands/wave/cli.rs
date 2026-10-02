use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum WaveLockCmd {
    /// Compile `.ai/tickets/wave.lock` from the ticket files — the ONLY legal writer.
    Repack {
        /// Freeze these shipped ids as a pending close target (space-separated). For a wave that
        /// dissolved id by id and left no `[[emptied]]` entry — see `ticket_engine::wave_lock::reserved_entry`.
        #[arg(long, value_name = "IDS")]
        reserve: Option<String>,
    },
    /// Recompute from the tickets and structurally compare against the committed lock.
    Check,
}
