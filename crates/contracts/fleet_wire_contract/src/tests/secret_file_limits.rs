//! The secret-file limits agree with each other and with the credential they hold.

use super::*;
use crate::machine_credential_format::{
    CREDENTIAL_ID_HEX_DIGITS, CREDENTIAL_RANDOM_HEX_DIGITS, MACHINE_CREDENTIAL_PREFIX,
};

#[test]
fn limits_hold_their_values() {
    assert_eq!(SECRET_FILE_MAX_BYTES, 4096);
    assert_eq!(SECRET_FILE_MODE, 0o600);
    assert_eq!(SHARED_PERMISSION_BITS, 0o077);
}

#[test]
fn a_created_secret_file_passes_the_permission_check() {
    assert_eq!(SECRET_FILE_MODE & SHARED_PERMISSION_BITS, 0);
}

#[test]
fn a_machine_credential_fits_a_secret_file() {
    let credential_bytes = MACHINE_CREDENTIAL_PREFIX.len()
        + CREDENTIAL_ID_HEX_DIGITS
        + 1
        + CREDENTIAL_RANDOM_HEX_DIGITS;
    assert_eq!(credential_bytes, 102);
    assert!(u64::try_from(credential_bytes).unwrap() < SECRET_FILE_MAX_BYTES);
}
