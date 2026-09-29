//! The flags after a `staging-fixtures` subcommand name, taken one by one by the parsers.
//!
//! **Role:** splits the command line into `--flag value` and `--switch` entries and hands each to
//! the parser that asks for it; whatever nobody asks for is refused.
//!
//! **Position:** built in `main.rs` from the arguments after the subcommand name; the common
//! options and then the subcommand's parser take their flags, and [`ArgumentList::finish`] refuses
//! the rest before any guard connects to the database.
//!
//! **Signals & state:** the entries not yet taken.
//!
//! **Invariants:** every token belongs to exactly one entry; a token starting with `--` names a
//! flag and the token after it is its value unless it starts with `--` too; a flag given twice, a
//! token before the first flag, and a flag nobody takes are refused. Argument values never hold
//! secrets: no flag of the tool takes one.

use std::str::FromStr;

use crate::tool_failure::ToolFailure;

/// One flag of the command line, with its value when a value followed it.
#[derive(Debug)]
struct ArgumentEntry {
    flag: String,
    value: Option<String>,
}

/// The flags of one command line that no parser has taken yet.
#[derive(Debug)]
pub(crate) struct ArgumentList {
    entries: Vec<ArgumentEntry>,
}

impl ArgumentList {
    /// Split `tokens` into entries, refusing a stray value and a repeated flag.
    pub(crate) fn parse(tokens: &[String]) -> Result<Self, ToolFailure> {
        let mut entries: Vec<ArgumentEntry> = Vec::new();
        let mut tokens = tokens.iter().peekable();
        while let Some(token) = tokens.next() {
            if !token.starts_with("--") {
                return Err(ToolFailure::refused(format!(
                    "unexpected argument `{token}`: every argument is a --flag or a flag's value"
                )));
            }
            if entries.iter().any(|entry| entry.flag == *token) {
                return Err(ToolFailure::refused(format!("{token} is given twice")));
            }
            let value = tokens.next_if(|next| !next.starts_with("--")).cloned();
            entries.push(ArgumentEntry {
                flag: token.clone(),
                value,
            });
        }
        Ok(Self { entries })
    }

    /// Whether `flag` is present; a switch takes no value.
    pub(crate) fn switch(&mut self, flag: &str) -> Result<bool, ToolFailure> {
        match self.take(flag) {
            None => Ok(false),
            Some(ArgumentEntry { value: None, .. }) => Ok(true),
            Some(ArgumentEntry { value: Some(_), .. }) => Err(ToolFailure::refused(format!(
                "{flag} is a switch and takes no value"
            ))),
        }
    }

    /// The value of `flag`, or `None` when the flag is absent.
    pub(crate) fn optional(&mut self, flag: &str) -> Result<Option<String>, ToolFailure> {
        match self.take(flag) {
            None => Ok(None),
            Some(ArgumentEntry {
                value: Some(value), ..
            }) => Ok(Some(value)),
            Some(ArgumentEntry { value: None, .. }) => {
                Err(ToolFailure::refused(format!("{flag} needs a value")))
            }
        }
    }

    /// The value of `flag`, which must be present.
    pub(crate) fn required(&mut self, flag: &str) -> Result<String, ToolFailure> {
        self.optional(flag)?
            .ok_or_else(|| ToolFailure::refused(format!("{flag} is required")))
    }

    /// The value of `flag` parsed as `T`, or `None` when the flag is absent; `expected` names the
    /// accepted form in the refusal.
    pub(crate) fn optional_parsed<T: FromStr>(
        &mut self,
        flag: &str,
        expected: &str,
    ) -> Result<Option<T>, ToolFailure> {
        self.optional(flag)?
            .map(|value| {
                value.parse::<T>().map_err(|_| {
                    ToolFailure::refused(format!("{flag} takes {expected}, not `{value}`"))
                })
            })
            .transpose()
    }

    /// The value of `flag` parsed as `T`, which must be present.
    pub(crate) fn required_parsed<T: FromStr>(
        &mut self,
        flag: &str,
        expected: &str,
    ) -> Result<T, ToolFailure> {
        self.optional_parsed(flag, expected)?
            .ok_or_else(|| ToolFailure::refused(format!("{flag} is required")))
    }

    /// Refuse every flag no parser took.
    pub(crate) fn finish(self) -> Result<(), ToolFailure> {
        let unknown: Vec<&str> = self
            .entries
            .iter()
            .map(|entry| entry.flag.as_str())
            .collect();
        if unknown.is_empty() {
            Ok(())
        } else {
            Err(ToolFailure::refused(format!(
                "this subcommand takes no {}",
                unknown.join(", ")
            )))
        }
    }

    fn take(&mut self, flag: &str) -> Option<ArgumentEntry> {
        let index = self.entries.iter().position(|entry| entry.flag == flag)?;
        Some(self.entries.remove(index))
    }
}

#[cfg(test)]
#[path = "tests/argument_list.rs"]
mod tests;
