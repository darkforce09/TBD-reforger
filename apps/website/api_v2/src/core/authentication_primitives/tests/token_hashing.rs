use super::*;

#[test]
fn numeric_code_is_zero_padded_digits() {
    for _ in 0..100 {
        let c = numeric_code(6);
        assert_eq!(c.len(), 6);
        assert!(c.chars().all(|ch| ch.is_ascii_digit()));
    }
}

#[test]
fn constant_time_equal_rejects_unequal_content_and_length() {
    assert!(constant_time_equal("abc", "abc"));
    assert!(!constant_time_equal("abc", "abd"));
    assert!(!constant_time_equal("abc", "ab")); // unequal length -> false
    assert!(!constant_time_equal("", "x"));
    assert!(constant_time_equal("", ""));
}

#[test]
fn hash_token_is_sha256_hex() {
    assert_eq!(
        hash_token(""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        hash_token("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn random_token_hex_length_and_uniqueness() {
    assert_eq!(random_token(16).len(), 32);
    assert_eq!(random_token(32).len(), 64);
    assert_ne!(random_token(16), random_token(16));
}
