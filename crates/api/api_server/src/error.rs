//! Why an `api_server` binary stops.
//!
//! **Role:** [`Error`], every failure the `api-server` and `import-item-registry` binaries end
//! on, and its [`Result`] alias.
//! **Position:** the library's error module; each binary's `main` returns [`Result`], so on a
//! failure the Rust runtime prints `Error: ` and the error's [`Debug`](std::fmt::Debug) rendering
//! to standard error and the process exits 1.
//! **Signals & state:** none; plain data.
//! **Invariants:** every variant but the two command-line refusals forwards the message and the
//! cause chain of the error it wraps unchanged; the `Debug` rendering is the message followed by
//! the cause chain under `Caused by:`, one cause per line, numbered when there are several.

use std::error::Error as _;
use std::fmt;

use api_configuration::configuration::ConfigError;
use api_missions::services::registry_import::ImportError;

/// Why an `api_server` binary stops.
#[derive(thiserror::Error)]
pub enum Error {
    /// A configuration variable was refused at boot.
    #[error(transparent)]
    Configuration(#[from] ConfigError),
    /// The Postgres pool could not be opened.
    #[error(transparent)]
    DatabaseConnection(#[from] sqlx::Error),
    /// A pending migration could not be applied.
    #[error(transparent)]
    Migration(#[from] sqlx::migrate::MigrateError),
    /// A socket or file operation failed: the port bind, the server loop or an envelope read.
    #[error(transparent)]
    InputOutput(#[from] std::io::Error),
    /// A registry envelope could not be imported.
    #[error(transparent)]
    RegistryImport(#[from] ImportError),
    /// The `import-item-registry` command line was refused; carries the reason.
    #[error("{0}")]
    CommandLine(String),
    /// `import-item-registry` found no `DATABASE_URL` in the environment or in `.env`.
    #[error("DATABASE_URL is not set (env or .env)")]
    DatabaseUrlMissing,
}

/// The result of a binary's `main`.
pub type Result<T> = std::result::Result<T, Error>;

/// The message, then each cause on its own indented line under `Caused by:`, numbered from 0
/// when there is more than one; a cause spanning several lines keeps them under its indent.
impl fmt::Debug for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self}")?;
        let Some(first) = self.source() else {
            return Ok(());
        };
        formatter.write_str("\n\nCaused by:")?;
        let numbered = first.source().is_some();
        let mut cause = Some(first);
        let mut index = 0;
        while let Some(error) = cause {
            formatter.write_str("\n")?;
            write_indented(formatter, &error.to_string(), numbered.then_some(index))?;
            cause = error.source();
            index += 1;
        }
        Ok(())
    }
}

/// Writes one cause: its first line after the number (or a four-space indent) and every further
/// line under the same indent.
fn write_indented(
    formatter: &mut fmt::Formatter<'_>,
    text: &str,
    number: Option<usize>,
) -> fmt::Result {
    let continuation = if number.is_some() { "       " } else { "    " };
    for (line_index, line) in text.split('\n').enumerate() {
        if line_index == 0 {
            match number {
                Some(number) => write!(formatter, "{number: >5}: ")?,
                None => formatter.write_str("    ")?,
            }
        } else {
            formatter.write_str("\n")?;
            formatter.write_str(continuation)?;
        }
        formatter.write_str(line)?;
    }
    Ok(())
}
