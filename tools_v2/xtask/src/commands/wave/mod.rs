//! Ticket wave-lock command adapters.
pub use ticket_engine::wave_lock::{cmd_check, cmd_repack};
pub fn collisions(args: &[String]) -> anyhow::Result<u8> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| anyhow::anyhow!(e).context("git rev-parse --show-toplevel"))?;
    let root = std::path::PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string());
    ticket_engine::wave_lock::collisions::run(&root, args)
}
