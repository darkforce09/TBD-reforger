//! The FIPS 180-4 sample vectors for both algorithms: the empty message, "abc" and the 448-bit
//! message.

use super::*;

const EMPTY: &[u8] = b"";
const ABC: &[u8] = b"abc";
const FOUR_HUNDRED_FORTY_EIGHT_BITS: &[u8] =
    b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";

#[test]
fn sha256_matches_the_published_vectors() {
    assert_eq!(
        sha256_hex(EMPTY),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(ABC),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(FOUR_HUNDRED_FORTY_EIGHT_BITS),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn sha384_matches_the_published_vectors() {
    assert_eq!(
        sha384_hex(EMPTY),
        "38b060a751ac96384cd9327eb1b1e36a21fdb71114be0743\
         4c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b"
    );
    assert_eq!(
        sha384_hex(ABC),
        "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded163\
         1a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
    );
    assert_eq!(
        sha384_hex(FOUR_HUNDRED_FORTY_EIGHT_BITS),
        "3391fdddfc8dc7393707a65b1b4709397cf8b1d162af05ab\
         fe8f450de5f36bc6b0455a8520bc4e6f5fe95b1fe3c8452b"
    );
}

#[test]
fn digests_are_lowercase_hex_of_the_full_width() {
    for (digest, width) in [(sha256_hex(ABC), 64), (sha384_hex(ABC), 96)] {
        assert_eq!(digest.len(), width);
        assert!(
            digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        );
    }
}
