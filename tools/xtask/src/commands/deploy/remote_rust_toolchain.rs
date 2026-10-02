//! How a remote deploy step finds the deploy account's Rust toolchain on the host.
//!
//! **Role:** holds the one shell line, [`PUT_RUST_TOOLCHAIN_ON_PATH`], that a remote step runs
//! before it calls `cargo` or `trunk` on the host.
//!
//! **Position:** read by the remote steps of `cargo xtask deploy website`
//! (`tools/xtask/src/commands/deploy/website/remote_steps.rs`) and by the `bash -s` payloads of
//! `cargo xtask deploy staging` (`tools/xtask/src/commands/deploy/staging/payloads.rs`, the
//! remote profile, and `tools/xtask/src/commands/deploy/staging/host_agent.rs`, the host agent
//! build), which send shell text to the host over ssh.
//!
//! **Signals & state:** none; one constant.
//!
//! **Invariants:** a command that ssh runs on the host starts a non-login shell, so the rustup line
//! in the deploy account's `~/.profile` never runs there and `cargo` and `trunk` are not found
//! without this line; the line only prepends `$HOME/.cargo/bin`, where rustup and
//! `cargo install --locked trunk` put the binaries, and changes nothing else in the environment.

/// The shell line that puts `$HOME/.cargo/bin` in front of `PATH` for a remote step; the remote
/// shell expands `$HOME` and `$PATH`.
pub(crate) const PUT_RUST_TOOLCHAIN_ON_PATH: &str = "export PATH=\"$HOME/.cargo/bin:$PATH\"";
