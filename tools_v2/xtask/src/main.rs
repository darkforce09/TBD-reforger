mod commands;
mod verifications;

use std::process::ExitCode;

use commands::ticket::load_registry;

mod cli;

fn main() -> ExitCode {
    match cli::dispatch::run() {
        Ok(code) => ExitCode::from(code),
        Err(e) => {
            eprintln!("xtask: {e:#}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
#[path = "tests/tooling_dependency_boundaries.rs"]
mod tooling_dependency_boundaries;

pub(crate) mod core;
