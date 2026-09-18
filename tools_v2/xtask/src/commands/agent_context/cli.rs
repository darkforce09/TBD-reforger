use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum AiCmd {
    /// PreToolUse hook: reads the harness hook JSON on stdin.
    /// exit 0 = allow, exit 2 = deny (reason on stderr). Fails OPEN on anything unexpected.
    Guard,
    /// Run a command and print a filtered view of its output. Never hides a failure: a
    /// non-zero exit also prints the raw tail, and verdict lines always pass through.
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
