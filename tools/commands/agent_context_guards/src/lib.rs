//! Guards that keep an AI agent's context small without ever hiding a failure.
//!
//! **Role:** [`run_tool_call_guard`] answers the PreToolUse hook an agent harness runs before each
//! Read or Bash call (exit 0 allow, 2 deny) from the Bash rules and the Read rules;
//! [`run_filtered_command`] runs a noisy command and prints only the lines that carry a verdict,
//! a failure or the tail. They are `cargo xtask ai guard` and `cargo xtask ai run`.
//! **Position:** a command crate of `tools/commands`, over `time_source` (the wall clock of the
//! session read set) and `serde_json` (the hook payload). The `ai` group of `xtask` parses the
//! command line and calls the two entry points; `.claude/settings.json` runs the guard.
//! **Signals & state:** the read set of each session, a file under the system temp folder the
//! Read rules append to; nothing else.
//! **Invariants:** the guard fails open and denies only on a rule it positively matched; the
//! filter never turns a non-zero exit into a clean-looking run, and returns the command's own
//! exit code.

mod bash_command_guard;
mod error;
mod output_filter;
pub mod prelude;
mod read_guard;
mod tool_call_guard;

pub use error::{Error, Result};
pub use output_filter::run_filtered_command;
pub use tool_call_guard::run_tool_call_guard;
