//! The `gate` binary: the headless browser gates of the single-page app.

fn main() -> std::process::ExitCode {
    browser_gate_suites::command_lines::gate::run()
}
