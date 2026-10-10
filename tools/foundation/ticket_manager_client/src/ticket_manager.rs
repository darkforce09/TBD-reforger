//! The `ttm` command line as a value: which binary, which project, and how one call runs.
//!
//! **Role:** [`TicketManager`] names the binary (`TBD_TTM_BIN`, default `ttm` on `PATH`) and the
//! project (`TBD_TTM_PROJECT`, default `reforger`); every call runs it with `--json --project
//! <project>` and hands stdout to [`parse_document`], which checks the format tag and turns an
//! error document into [`Error::Refused`]. [`TicketManager::text_command`] and
//! [`TicketManager::display_command`] give the human-facing command lines.
//! **Position:** the only place the tools spawn `ttm`; the typed calls live in
//! `ticket_commands.rs` and `wave_commands.rs`.
//! **Signals & state:** none held; each call spawns one child through [`process_runner::Run`],
//! which inherits this process's environment (`TBD_TICKETS_DB` selects another database).
//! **Invariants:** a missing binary, a signal or a deadline is [`Error::NotRun`], never a
//! refusal; a non-zero exit is a refusal unless the command's contract prints its document on a
//! failing verdict (`check`, `wave check`); stdout that is not one JSON object with the expected
//! format tag is [`Error::Contract`], never an empty answer.

use process_runner::Run;
use serde_json::Value;

use crate::error::{Error, Result};
use crate::ticket_documents::TicketManagerDocument;

/// The variable naming the `ttm` binary (a path, or a name looked up on `PATH`).
pub const BINARY_VARIABLE: &str = "TBD_TTM_BIN";
/// The binary run when [`BINARY_VARIABLE`] is unset or empty.
pub const DEFAULT_BINARY: &str = "ttm";
/// The variable naming the ticket manager project the tools act on.
pub const PROJECT_VARIABLE: &str = "TBD_TTM_PROJECT";
/// The project acted on when [`PROJECT_VARIABLE`] is unset or empty.
pub const DEFAULT_PROJECT: &str = "reforger";

/// Whether a call accepts a failing exit code that still carries its document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FailingVerdict {
    /// Any non-zero exit is a refusal.
    Refused,
    /// Exit 1 with the expected document is a verdict the caller reads (`check`, `wave check`).
    CarriesDocument,
}

/// The central ticket manager's command line, bound to one binary and one project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TicketManager {
    program: String,
    project: String,
}

impl TicketManager {
    /// The binary and project the process environment names, or the defaults.
    pub fn from_env() -> TicketManager {
        let read = |variable: &str, default: &str| {
            std::env::var(variable)
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| default.to_string())
        };
        TicketManager::new(
            read(BINARY_VARIABLE, DEFAULT_BINARY),
            read(PROJECT_VARIABLE, DEFAULT_PROJECT),
        )
    }

    /// A ticket manager run from `program` acting on `project`.
    pub fn new(program: impl Into<String>, project: impl Into<String>) -> TicketManager {
        TicketManager {
            program: program.into(),
            project: project.into(),
        }
    }

    /// The binary this client runs.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The project every call names.
    pub fn project(&self) -> &str {
        &self.project
    }

    /// The argv of a human-facing (text output) call: the binary, `--project <project>`, then
    /// `args`.
    pub fn text_command(&self, args: &[&str]) -> Vec<String> {
        let mut argv = vec![
            self.program.clone(),
            "--project".to_string(),
            self.project.clone(),
        ];
        argv.extend(args.iter().map(|arg| arg.to_string()));
        argv
    }

    /// The command line an operator types for `args`, such as `ttm --project reforger wave
    /// repack`.
    pub fn display_command(&self, args: &[&str]) -> String {
        self.text_command(args).join(" ")
    }

    /// Runs `args` with `--json` and parses the document `D` from stdout.
    pub(crate) fn document<D: TicketManagerDocument>(
        &self,
        args: &[String],
        failing: FailingVerdict,
    ) -> Result<D> {
        let command = self.display_owned(args);
        let output = self.spawn(true, args, &command)?;
        let accepts_failure = failing == FailingVerdict::CarriesDocument && output.code == 1;
        if output.code != 0 && !accepts_failure {
            return Err(refusal(
                &command,
                output.code,
                &output.stdout,
                &output.stderr,
            ));
        }
        parse_document(&command, &output.stdout)
    }

    /// Runs `args` without `--json` and returns stdout (the text-only commands such as `brief`).
    pub(crate) fn text(&self, args: &[String]) -> Result<String> {
        let command = self.display_owned(args);
        let output = self.spawn(false, args, &command)?;
        if output.code != 0 {
            return Err(refusal(
                &command,
                output.code,
                &output.stdout,
                &output.stderr,
            ));
        }
        Ok(output.stdout)
    }

    fn display_owned(&self, args: &[String]) -> String {
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        self.display_command(&borrowed)
    }

    fn spawn(&self, json: bool, args: &[String], command: &str) -> Result<process_runner::Output> {
        let mut run = Run::new(&self.program);
        if json {
            run = run.arg("--json");
        }
        run.args(["--project", self.project.as_str()])
            .args(args)
            .output()
            .map_err(|source| Error::NotRun {
                command: command.to_string(),
                source,
            })
    }
}

/// Parses `stdout` of `command` as the document `D`: one JSON object whose `format` is
/// `D::FORMAT`. An `{"error": …}` document is the ticket manager's refusal.
pub fn parse_document<D: TicketManagerDocument>(command: &str, stdout: &str) -> Result<D> {
    let contract = |detail: String| Error::Contract {
        command: command.to_string(),
        detail,
    };
    let value: Value = serde_json::from_str(stdout.trim())
        .map_err(|error| contract(format!("stdout is not one JSON object: {error}")))?;
    if let Some(refused) = error_document(command, &value) {
        return Err(refused);
    }
    match value.get("format").and_then(Value::as_str) {
        Some(format) if format == D::FORMAT => {}
        Some(format) => {
            return Err(contract(format!(
                "expected format {}, got {format}",
                D::FORMAT
            )));
        }
        None => return Err(contract(format!("no format tag; expected {}", D::FORMAT))),
    }
    serde_json::from_value(value)
        .map_err(|error| contract(format!("{} does not parse: {error}", D::FORMAT)))
}

/// The refusal an `{"error": {"kind", "message", "candidates"}}` document carries.
fn error_document(command: &str, value: &Value) -> Option<Error> {
    let body = value.get("error")?;
    let text = |key: &str| {
        body.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let candidates = body
        .get("candidates")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Some(Error::Refused {
        command: command.to_string(),
        kind: text("kind"),
        message: text("message"),
        candidates,
    })
}

/// The refusal of a call that exited `code`: its error document when stdout holds one, else the
/// exit code and stderr.
fn refusal(command: &str, code: i32, stdout: &str, stderr: &str) -> Error {
    if let Ok(value) = serde_json::from_str::<Value>(stdout.trim())
        && let Some(refused) = error_document(command, &value)
    {
        return refused;
    }
    let said = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    Error::Refused {
        command: command.to_string(),
        kind: format!("exit {code}"),
        message: said.to_string(),
        candidates: Vec::new(),
    }
}
