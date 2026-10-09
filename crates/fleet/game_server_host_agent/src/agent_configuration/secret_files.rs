//! Reading the two secrets from their files: the machine credential and the RCON password.
//!
//! A secret file must be an absolute path to a regular file that no other user can read or
//! write (no group or other permission bits), at most 4 KiB, holding the secret as UTF-8 text;
//! surrounding whitespace, such as a trailing newline, is ignored. The size and permission limits
//! and the machine credential format are the `fleet_wire_contract` ones the writer of the files
//! applies.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use fleet_wire_contract::machine_credential_format::check_machine_credential_format;
use fleet_wire_contract::secret_file_limits::{SECRET_FILE_MAX_BYTES, SHARED_PERMISSION_BITS};

use super::configuration_error::{ConfigurationError, SecretFileProblem};
use crate::secret_text::SecretText;

/// Arma Reforger requires an RCON password of at least 3 characters without spaces.
const RCON_PASSWORD_MIN_CHARS: usize = 3;
/// The password travels in one login packet.
const RCON_PASSWORD_MAX_BYTES: usize = 256;

pub(super) fn read_secret_file(
    key: &'static str,
    path: &Path,
    validate: fn(&str) -> Result<(), &'static str>,
) -> Result<SecretText, ConfigurationError> {
    let rejected = |problem| ConfigurationError::SecretFileRejected {
        key,
        path: path.to_owned(),
        problem,
    };
    if !path.is_absolute() {
        return Err(rejected(SecretFileProblem::RelativePath));
    }
    let metadata =
        fs::metadata(path).map_err(|error| rejected(SecretFileProblem::Unreadable(error)))?;
    if !metadata.is_file() {
        return Err(rejected(SecretFileProblem::NotRegularFile));
    }
    let mode = metadata.permissions().mode() & 0o777;
    if mode & SHARED_PERMISSION_BITS != 0 {
        return Err(rejected(SecretFileProblem::AccessibleToOthers { mode }));
    }
    if metadata.len() > SECRET_FILE_MAX_BYTES {
        return Err(rejected(SecretFileProblem::TooLarge {
            limit: SECRET_FILE_MAX_BYTES,
        }));
    }
    let bytes = fs::read(path).map_err(|error| rejected(SecretFileProblem::Unreadable(error)))?;
    let text = String::from_utf8(bytes)
        .map_err(|_| rejected(SecretFileProblem::InvalidContent("it is not UTF-8 text")))?;
    let secret = text.trim();
    validate(secret).map_err(|problem| rejected(SecretFileProblem::InvalidContent(problem)))?;
    Ok(SecretText::new(secret))
}

/// `tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>`, checked by
/// [`check_machine_credential_format`].
pub(super) fn machine_credential_format(secret: &str) -> Result<(), &'static str> {
    check_machine_credential_format(secret).map_err(
        |_| "a machine credential reads tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>",
    )
}

/// At least 3 characters, at most 256 bytes, no whitespace or control characters.
pub(super) fn rcon_password_format(secret: &str) -> Result<(), &'static str> {
    let valid = secret.chars().count() >= RCON_PASSWORD_MIN_CHARS
        && secret.len() <= RCON_PASSWORD_MAX_BYTES
        && !secret
            .chars()
            .any(|character| character.is_whitespace() || character.is_control());
    if valid {
        Ok(())
    } else {
        Err("an Arma Reforger RCON password has 3 to 256 bytes and no spaces")
    }
}
