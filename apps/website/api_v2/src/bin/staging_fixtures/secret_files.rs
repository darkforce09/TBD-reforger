//! Private files for the tool's secrets: the per-instance layout of machine credentials, exclusive
//! creation with mode 600, reading a staged secret back, and promoting it over the live file.
//!
//! **Role:** the only code of the tool that touches secret files.
//!
//! **Position:** `fleet_provisioning` writes the live credential files of every instance and
//! `credential_rotation` stages and promotes one; both check the layout here before they write
//! anything to the database. `load_population::account_file` checks the load account file's
//! directory and path here before a seeding writes any account, then writes the file of refresh
//! tokens through [`write_new_secret_file`]; that file lives where `--account-file` names, outside
//! the layout below.
//!
//! **Signals & state:** none; each call works on the paths it is given.
//!
//! **Invariants:** a secret file is created only where no file, link or directory exists, with
//! `O_CREAT | O_EXCL` and mode 600 set on the open descriptor, and synced before the call returns;
//! a secrets directory is a real directory with no group or other permission bits, and one this
//! module creates is mode 700; a secret never appears in a message.
//!
//! The layout under the secrets root, the fleet root the deploy uses:
//!
//! ```text
//! <secrets root>/instance-<n>/secrets/host-agent-credential           the host agent's credential
//! <secrets root>/instance-<n>/secrets/mod-runtime-credential          the game runtime's credential
//! <secrets root>/instance-<n>/secrets/<credential file>.staged        a rotation's new credential
//! ```

use std::fs::{self, DirBuilder, File, OpenOptions, Permissions};
use std::io::{ErrorKind, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use website_api::server_infrastructure::models::machine_credential::ExecutorKind;

use crate::tool_failure::ToolFailure;

/// The mode of a secret file: read and write for the owner only.
const SECRET_FILE_MODE: u32 = 0o600;
/// The mode of a directory this module creates: the owner only.
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
/// Group and other permission bits.
const SHARED_PERMISSIONS: u32 = 0o077;
/// A machine credential is 101 bytes; anything past this is not one.
const SECRET_FILE_MAX_BYTES: u64 = 4096;

/// Where each instance's credential files live under one secrets root.
#[derive(Debug, Clone)]
pub(crate) struct SecretFileLayout {
    root: PathBuf,
}

impl SecretFileLayout {
    /// The layout under `root`, which must be absolute so the files land where the deploy reads
    /// them whatever the working directory.
    pub(crate) fn new(root: String) -> Result<Self, ToolFailure> {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err(ToolFailure::refused(format!(
                "--secrets-root must be an absolute path, not {}",
                root.display()
            )));
        }
        Ok(Self { root })
    }

    /// `<root>/instance-<n>/secrets`.
    pub(crate) fn secrets_directory(&self, instance: u32) -> PathBuf {
        self.root
            .join(format!("instance-{instance}"))
            .join("secrets")
    }

    /// The live credential file of `executor` on instance `instance`.
    pub(crate) fn credential_file(&self, instance: u32, executor: ExecutorKind) -> PathBuf {
        self.secrets_directory(instance)
            .join(credential_file_name(executor))
    }

    /// The staged credential file a rotation writes beside the live one.
    pub(crate) fn staged_credential_file(&self, instance: u32, executor: ExecutorKind) -> PathBuf {
        self.secrets_directory(instance)
            .join(format!("{}.staged", credential_file_name(executor)))
    }
}

/// The file name of `executor`'s credential.
fn credential_file_name(executor: ExecutorKind) -> &'static str {
    match executor {
        ExecutorKind::HostAgent => "host-agent-credential",
        ExecutorKind::ModRuntime => "mod-runtime-credential",
    }
}

/// What stands at a secrets directory's path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectoryState {
    /// Nothing: a writing run creates it with mode 700.
    Absent,
    /// A real directory no other user can enter or read.
    Private,
}

/// Inspect the directory that holds or will hold secrets, refusing a link, a non-directory and a
/// directory with group or other permission bits.
pub(crate) fn inspect_secrets_directory(path: &Path) -> Result<DirectoryState, ToolFailure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(DirectoryState::Absent),
        Err(error) => return Err(io_failure("cannot inspect", path, &error)),
    };
    if metadata.file_type().is_symlink() {
        return Err(ToolFailure::refused(format!(
            "{} is a symbolic link; a secrets directory is a real directory",
            path.display()
        )));
    }
    if !metadata.is_dir() {
        return Err(ToolFailure::refused(format!(
            "{} exists and is not a directory",
            path.display()
        )));
    }
    let mode = metadata.permissions().mode() & 0o777;
    if mode & SHARED_PERMISSIONS != 0 {
        return Err(ToolFailure::refused(format!(
            "{} is mode {mode:o}; a secrets directory admits its owner only (mode 700)",
            path.display()
        )));
    }
    Ok(DirectoryState::Private)
}

/// Refuse when anything, a dangling link included, stands at `path`: a secret file is never
/// overwritten.
pub(crate) fn require_absent(path: &Path) -> Result<(), ToolFailure> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(ToolFailure::refused(format!(
            "{} already exists; the tool never overwrites a secret file",
            path.display()
        ))),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_failure("cannot inspect", path, &error)),
    }
}

/// Create a secrets directory [`inspect_secrets_directory`] found absent, with every missing
/// ancestor, each mode 700.
pub(crate) fn create_private_directory(path: &Path) -> Result<(), ToolFailure> {
    DirBuilder::new()
        .recursive(true)
        .mode(PRIVATE_DIRECTORY_MODE)
        .create(path)
        .and_then(|()| fs::set_permissions(path, Permissions::from_mode(PRIVATE_DIRECTORY_MODE)))
        .map_err(|error| io_failure("cannot create", path, &error))
}

/// Create `path` exclusively with mode 600, write `secret` and sync it. A write that fails after
/// the create removes the partial file.
pub(crate) fn write_new_secret_file(path: &Path, secret: &str) -> Result<(), ToolFailure> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(SECRET_FILE_MODE)
        .open(path)
        .map_err(|error| io_failure("cannot create", path, &error))?;
    let written = file
        .set_permissions(Permissions::from_mode(SECRET_FILE_MODE))
        .and_then(|()| file.write_all(secret.as_bytes()))
        .and_then(|()| file.sync_all());
    if let Err(error) = written {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(io_failure("cannot write", path, &error));
    }
    Ok(())
}

/// Read the secret in a private regular file, without its surrounding whitespace; `None` when
/// nothing stands at `path`.
pub(crate) fn read_private_secret_file(path: &Path) -> Result<Option<String>, ToolFailure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(io_failure("cannot inspect", path, &error)),
    };
    let mode = metadata.permissions().mode() & 0o777;
    if !metadata.is_file() || mode & SHARED_PERMISSIONS != 0 {
        return Err(ToolFailure::refused(format!(
            "{} is not a private regular file (mode 600)",
            path.display()
        )));
    }
    if metadata.len() > SECRET_FILE_MAX_BYTES {
        return Err(ToolFailure::refused(format!(
            "{} is larger than a secret file",
            path.display()
        )));
    }
    let bytes = fs::read(path).map_err(|error| io_failure("cannot read", path, &error))?;
    let text = String::from_utf8(bytes).map_err(|_| {
        ToolFailure::refused(format!("{} does not hold UTF-8 text", path.display()))
    })?;
    Ok(Some(text.trim().to_owned()))
}

/// Replace `live` with `staged` in one rename and sync their directory, so the live path holds
/// either the old secret or the new one at every instant.
pub(crate) fn promote_staged_file(staged: &Path, live: &Path) -> Result<(), ToolFailure> {
    fs::rename(staged, live).map_err(|error| io_failure("cannot rename", staged, &error))?;
    match live.parent() {
        Some(directory) => sync_directory(directory),
        None => Ok(()),
    }
}

/// Sync a directory so the entries created or renamed in it survive a crash.
pub(crate) fn sync_directory(path: &Path) -> Result<(), ToolFailure> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_failure("cannot sync", path, &error))
}

/// Remove the files a failed run wrote, returning the ones that could not be removed.
pub(crate) fn remove_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter(|path| fs::remove_file(path).is_err())
        .cloned()
        .collect()
}

fn io_failure(action: &str, path: &Path, error: &std::io::Error) -> ToolFailure {
    ToolFailure::failed(format!("{action} {}: {error}", path.display()))
}
