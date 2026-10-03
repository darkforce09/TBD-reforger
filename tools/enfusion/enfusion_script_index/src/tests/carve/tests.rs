use super::*;

/// The carved file name's digest is the first four bytes of the SHA-256 of the carved run, so a
/// blob carved again from the same pak lands on the same name.
#[test]
fn the_carved_name_digest_is_the_first_four_sha256_bytes() {
    assert_eq!(sha8(b"abc"), "ba7816bf");
    assert_eq!(sha8(b""), "e3b0c442");
}
