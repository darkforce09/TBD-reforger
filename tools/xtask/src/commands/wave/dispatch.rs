use super::cli::WaveLockCmd;
use anyhow::Result;
use repository_layout::prelude::find_repository_root;

pub(crate) fn run(cmd: WaveLockCmd) -> Result<u8> {
    {
        let root = find_repository_root()?;
        match cmd {
            WaveLockCmd::Repack { reserve } => {
                let ids: Vec<ticket_model::TicketId> = reserve
                    .as_deref()
                    .unwrap_or_default()
                    .split_whitespace()
                    .map(ticket_model::TicketId::new)
                    .collect();
                Ok(ticket_wave_lock::cmd_repack(&root, &ids)?)
            }
            WaveLockCmd::Check => Ok(ticket_wave_lock::cmd_check(&root)?),
        }
    }
}
