//! Ticket wave-lock command adapters.
pub use ticket_engine::wave_lock::{cmd_check, cmd_repack};
pub fn collisions(args: &[String]) -> anyhow::Result<u8> {
    let root = repository_layout::find_repository_root()?;
    ticket_engine::wave_lock::collisions::run(&root, args)
}

pub(crate) mod cli;
pub(crate) mod dispatch;
