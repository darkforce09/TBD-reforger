//! The text of a machine credential secret: `tbdm_<credential id>_<randomness>`.
//!
//! **Role:** the credential's prefix, the lengths of its two hex parts and the format check a
//! reader of a credential applies before presenting it.
//! **Position:** the API issues secrets with [`MACHINE_CREDENTIAL_PREFIX`] and the two lengths;
//! the host agent refuses a credential file whose text fails
//! [`check_machine_credential_format`]; `xtask` recognises a configured credential by the prefix.
//! **Signals & state:** none; constants and a pure function.
//! **Invariants:** a well-formed credential is the prefix, 32 lowercase hex digits (the
//! credential id), one `_`, and 64 lowercase hex digits; nothing else passes the check.
//!
//! @contract machine-credential.schema.json#/properties/secret

use crate::error::{Error, Result};

/// The prefix of every machine credential secret.
pub const MACHINE_CREDENTIAL_PREFIX: &str = "tbdm_";

/// Hex digits of the credential id part: a UUID in its simple spelling.
pub const CREDENTIAL_ID_HEX_DIGITS: usize = 32;

/// Hex digits of the random part.
pub const CREDENTIAL_RANDOM_HEX_DIGITS: usize = 64;

/// Accept `tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>` and refuse anything else,
/// upper-case hex, a scheme such as `Bearer ` and surrounding whitespace included.
pub fn check_machine_credential_format(secret: &str) -> Result<()> {
    let lowercase_hex = |part: &str, digits: usize| {
        part.len() == digits
            && part
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    };
    let (id, random) = secret
        .strip_prefix(MACHINE_CREDENTIAL_PREFIX)
        .and_then(|rest| rest.split_once('_'))
        .ok_or(Error::MalformedMachineCredential)?;
    if lowercase_hex(id, CREDENTIAL_ID_HEX_DIGITS)
        && lowercase_hex(random, CREDENTIAL_RANDOM_HEX_DIGITS)
    {
        Ok(())
    } else {
        Err(Error::MalformedMachineCredential)
    }
}

#[cfg(test)]
#[path = "tests/machine_credential_format.rs"]
mod tests;
