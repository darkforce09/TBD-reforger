//! The command line of the `staging-load` executable: a load plan as JSON in, the report as JSON
//! out.
//!
//! - **Role:** parses `--plan` and `--report`, reads the plan from the named file or from standard
//!   input, runs it, writes the report to the named file or to standard output, and maps the
//!   outcome to the exit code: 0 the report was written, 1 the load did not run or its report was
//!   not written, 2 a usage error (clap's).
//! - **Position:** `developer_tools`' `src/bin/staging_load.rs` calls [`entrypoint`]; the xtask
//!   load procedure runs that executable with the plan on standard input and reads the report from
//!   its standard output, through `staging_load_plan`'s process-boundary codec on both sides.
//! - **Signals & state:** none beyond the run it starts.
//! - **Invariants:** standard output carries the report as one JSON line and nothing else; every
//!   error goes to standard error as one `staging-load: …` line; a refused plan sends no request.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use staging_load_plan::{decode_plan, encode_report};

use crate::error::{Error, Result};
use crate::load_run::run;

/// The exit code after the report was written.
const EXIT_REPORT_WRITTEN: u8 = 0;
/// The exit code when the load did not run or its report was not written.
const EXIT_LOAD_NOT_RUN: u8 = 1;

/// Run the staging member load a plan describes and write what it measured as JSON.
#[derive(Debug, Parser)]
#[command(name = "staging-load")]
struct StagingLoadCommandLine {
    /// The load plan (a `LoadRunPlan` as JSON); standard input when absent.
    #[arg(long)]
    plan: Option<PathBuf>,
    /// Where to write the report (a `LoadReport` as JSON); standard output when absent.
    #[arg(long)]
    report: Option<PathBuf>,
}

/// Parse the process arguments, run the load, and return the exit code.
pub fn entrypoint() -> ExitCode {
    let command_line = StagingLoadCommandLine::parse();
    match run_command_line(&command_line) {
        Ok(()) => ExitCode::from(EXIT_REPORT_WRITTEN),
        Err(error) => {
            eprintln!("staging-load: {error}");
            ExitCode::from(EXIT_LOAD_NOT_RUN)
        }
    }
}

/// Read the plan, run it, and write the report where the command line says.
fn run_command_line(command_line: &StagingLoadCommandLine) -> Result<()> {
    let plan = decode_plan(&read_plan(command_line)?)?;
    let report = encode_report(&run(&plan)?)?;
    match &command_line.report {
        Some(path) => std::fs::write(path, format!("{report}\n")).map_err(|error| {
            Error::ReportFileUnwritable {
                path: path.clone(),
                error,
            }
        }),
        None => {
            let mut output = io::stdout().lock();
            writeln!(output, "{report}")
                .and_then(|()| output.flush())
                .map_err(|error| Error::ReportOutputUnwritable { error })
        }
    }
}

/// The plan's text, from the named file or from standard input.
fn read_plan(command_line: &StagingLoadCommandLine) -> Result<String> {
    match &command_line.plan {
        Some(path) => std::fs::read_to_string(path).map_err(|error| Error::PlanFileUnreadable {
            path: path.clone(),
            error,
        }),
        None => {
            let mut text = String::new();
            io::stdin()
                .read_to_string(&mut text)
                .map_err(|error| Error::PlanInputUnreadable { error })?;
            Ok(text)
        }
    }
}

#[cfg(test)]
#[path = "tests/command_line_tests.rs"]
mod tests;
