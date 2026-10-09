//! The `cargo xtask wave` group and the `slice-collisions` verb over the wave lock crate.

/// Runs the slice collision report over the checkout this command runs in.
pub(crate) fn collisions(args: &[String]) -> anyhow::Result<u8> {
    let root = repository_layout::prelude::find_repository_root()?;
    Ok(ticket_wave_lock::collisions::run(&root, args)?)
}

pub(crate) mod cli;
pub(crate) mod dispatch;
