//! The machine credential check accepts exactly `tbdm_<32 hex>_<64 hex>` in lower case.

use super::*;

const VALID: &str = "tbdm_0123456789abcdef0123456789abcdef_\
     0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

#[test]
fn a_well_formed_credential_passes() {
    assert_eq!(VALID.len(), 102);
    assert_eq!(check_machine_credential_format(VALID), Ok(()));
    assert!(VALID.starts_with(MACHINE_CREDENTIAL_PREFIX));
}

#[test]
fn a_wrong_prefix_is_refused() {
    let wrong_prefix = VALID.replacen("tbdm_", "tbdx_", 1);
    let no_prefix = VALID.trim_start_matches("tbdm_").to_owned();
    let with_scheme = format!("Bearer {VALID}");
    for invalid in [wrong_prefix, no_prefix, with_scheme] {
        assert_eq!(
            check_machine_credential_format(&invalid),
            Err(Error::MalformedMachineCredential),
            "{invalid:?}"
        );
    }
}

#[test]
fn wrong_lengths_are_refused() {
    let one_digit_more = format!("{VALID}0");
    let one_digit_less = VALID[..VALID.len() - 1].to_owned();
    let short_id = VALID.replacen("tbdm_0", "tbdm_", 1);
    let long_id = VALID.replacen("tbdm_", "tbdm_0", 1);
    for invalid in [
        String::new(),
        "tbdm_".to_owned(),
        "tbdm__".to_owned(),
        one_digit_more,
        one_digit_less,
        short_id,
        long_id,
    ] {
        assert!(
            check_machine_credential_format(&invalid).is_err(),
            "{invalid:?}"
        );
    }
}

#[test]
fn upper_case_hex_and_other_characters_are_refused() {
    let upper_case_all = VALID.to_uppercase();
    let upper_case_id = VALID.replacen("abcdef_", "ABCDEF_", 1);
    let upper_case_random = format!("{}F", &VALID[..VALID.len() - 1]);
    let hyphenated_id = VALID.replacen("01234567", "0123-567", 1);
    let surrounded = format!(" {VALID}\n");
    for invalid in [
        upper_case_all,
        upper_case_id,
        upper_case_random,
        hyphenated_id,
        surrounded,
    ] {
        assert!(
            check_machine_credential_format(&invalid).is_err(),
            "{invalid:?}"
        );
    }
}

#[test]
fn the_refusal_names_the_format_and_never_the_secret() {
    let message = Error::MalformedMachineCredential.to_string();
    assert_eq!(
        message,
        "a machine credential reads tbdm_<32 lowercase hex digits>_<64 lowercase hex digits>"
    );
    assert_eq!(CREDENTIAL_ID_HEX_DIGITS, 32);
    assert_eq!(CREDENTIAL_RANDOM_HEX_DIGITS, 64);
}
