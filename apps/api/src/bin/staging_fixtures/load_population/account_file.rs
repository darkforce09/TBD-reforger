//! The account file of a load population: every synthetic account's refresh token, in account
//! order, in the format the developer_tools load engine reads.
//!
//! **Role:** checks where the file goes before anything is written, renders the population's
//! tokens, and writes the file through `secret_files`.
//!
//! **Position:** `population_seeding` checks the target in every run and writes the file once
//! every account holds its refresh session; the `staging load` harness hands the file to the load
//! engine (`staging_verification::load_generation::account_rotation` in developer_tools), which
//! reads it once.
//!
//! **Signals & state:** none; the tokens stay in the caller's memory until the file holds them.
//!
//! **Invariants:** the file is `{"accounts":[{"discord_id":"…","refresh_token":"…"},…]}` with
//! account `k` at index `k` and no other key; it is created only where nothing stands, with mode
//! 600, in a directory only its owner can enter (created mode 700 when absent); a token reaches the
//! file and nothing else, never a message or a `Debug` rendering.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::secret_files::{
    DirectoryState, create_private_directory, inspect_secrets_directory, require_absent,
    sync_directory, write_new_secret_file,
};
use crate::tool_failure::ToolFailure;

/// One synthetic account and the refresh token of its session.
pub(super) struct SeededAccount {
    /// The account's Discord id.
    pub(super) discord_id: String,
    /// The refresh token `issue_refresh` returned; a secret.
    pub(super) refresh_token: String,
}

/// The file's document, in the field names the load engine decodes with `deny_unknown_fields`.
#[derive(Serialize)]
struct AccountFileDocument<'a> {
    accounts: Vec<AccountFileEntry<'a>>,
}

#[derive(Serialize)]
struct AccountFileEntry<'a> {
    discord_id: &'a str,
    refresh_token: &'a str,
}

/// Where a seeding writes its account file.
#[derive(Debug, Clone)]
pub(super) struct AccountFileTarget {
    path: PathBuf,
    directory: PathBuf,
}

impl AccountFileTarget {
    /// The file `--account-file` names; its directory is the parent, or the working directory for
    /// a bare file name.
    pub(super) fn new(path: String) -> Result<Self, ToolFailure> {
        let path = PathBuf::from(path);
        if path.file_name().is_none() {
            return Err(ToolFailure::refused(format!(
                "--account-file {} does not name a file",
                path.display()
            )));
        }
        let directory = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_owned(),
            _ => PathBuf::from("."),
        };
        Ok(Self { path, directory })
    }

    /// The file's path, as given.
    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    /// Refuse when anything stands at the path or when the directory admits another user.
    pub(super) fn check(&self) -> Result<DirectoryState, ToolFailure> {
        require_absent(&self.path)?;
        inspect_secrets_directory(&self.directory)
    }

    /// Create the directory when it is absent, then write the file exclusively with mode 600 and
    /// sync the directory.
    pub(super) fn write(&self, accounts: &[SeededAccount]) -> Result<(), ToolFailure> {
        if self.check()? == DirectoryState::Absent {
            create_private_directory(&self.directory)?;
        }
        write_new_secret_file(&self.path, &render_account_file(accounts)?)?;
        sync_directory(&self.directory)
    }
}

/// The account file's contents for `accounts`, in their order.
pub(super) fn render_account_file(accounts: &[SeededAccount]) -> Result<String, ToolFailure> {
    let document = AccountFileDocument {
        accounts: accounts
            .iter()
            .map(|account| AccountFileEntry {
                discord_id: &account.discord_id,
                refresh_token: &account.refresh_token,
            })
            .collect(),
    };
    // serde_json's message names the failing position, never the data.
    serde_json::to_string(&document)
        .map_err(|error| ToolFailure::failed(format!("cannot render the account file: {error}")))
}

#[cfg(test)]
#[path = "tests/account_file.rs"]
mod tests;
