//! The `capture` binary: Mission Creator screenshots, zoom sweeps and crops against a live app.

fn main() -> std::process::ExitCode {
    browser_gate_suites::command_lines::capture::run()
}
