//! Argument-shape tests for `cargo xtask mcp wb-logs`: the empty-value spellings of `--file`
//! must reach this command's own exit codes instead of clap's.

use crate::cli::{Cli, TopCmd};
use crate::commands::mcp::cli::McpCmd;
use clap::Parser;
use std::ffi::OsString;
use std::path::PathBuf;

#[test]
fn file_equals_empty_parses_via_clap() {
    // `PathBufValueParser` rejects an empty value with clap rc=2; `parse_file_arg` accepts it
    // so `--file=` becomes this command's ENVIRONMENT (3), not a clap usage error.
    let args = crate::commands::mcp::workbench_logs::preprocess_cli_args(
        ["xtask", "mcp", "wb-logs", "--file="]
            .into_iter()
            .map(OsString::from)
            .collect(),
    );
    let cli = Cli::try_parse_from(args).expect("--file= must parse (not clap empty-value)");
    match cli.cmd {
        TopCmd::Mcp {
            cmd: McpCmd::WbLogs { file, .. },
        } => {
            assert_eq!(file, Some(PathBuf::new()));
        }
        other => panic!("unexpected cmd: {other:?}"),
    }
}
