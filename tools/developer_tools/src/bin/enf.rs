//! The `enf` binary: symbol indexes, lookups and checks over Enfusion scripts.

fn main() -> std::process::ExitCode {
    enfusion_script_index::run_command_line()
}
