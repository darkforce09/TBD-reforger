//! The incremental hasher agrees with the one-shot digest and frames fields unambiguously.

use super::*;
use crate::sha256_hex;

fn scratch_folder(name: &str) -> std::path::PathBuf {
    let folder = std::env::temp_dir().join(format!(
        "content-digest-hasher-{name}-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&folder).unwrap();
    folder
}

#[test]
fn an_unfed_hasher_is_the_digest_of_the_empty_message() {
    assert_eq!(Sha256Hasher::new().finalize_hex(), sha256_hex(b""));
}

#[test]
fn incremental_updates_equal_the_one_shot_digest() {
    let message = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
    for split in 0..=message.len() {
        let mut hasher = Sha256Hasher::new();
        hasher.update(&message[..split]);
        hasher.update(&message[split..]);
        assert_eq!(
            hasher.finalize_hex(),
            sha256_hex(message),
            "split at {split}"
        );
    }
}

#[test]
fn a_framed_field_hashes_as_its_little_endian_length_then_its_bytes() {
    let mut framed = Sha256Hasher::new();
    framed.update_length_framed(b"abc");
    let mut expected = 3_u64.to_le_bytes().to_vec();
    expected.extend_from_slice(b"abc");
    assert_eq!(framed.finalize_hex(), sha256_hex(&expected));
}

#[test]
fn framing_separates_field_sequences_that_concatenate_alike() {
    let mut joined = Sha256Hasher::new();
    joined.update_length_framed(b"ab");
    joined.update_length_framed(b"c");
    let mut split = Sha256Hasher::new();
    split.update_length_framed(b"a");
    split.update_length_framed(b"bc");
    assert_ne!(joined.finalize_hex(), split.finalize_hex());
}

#[test]
fn a_framed_file_hashes_like_its_bytes_framed_in_memory() {
    let folder = scratch_folder("file");
    let path = folder.join("input.bin");
    let bytes: Vec<u8> = (0..200_000_u32).map(|value| (value % 251) as u8).collect();
    std::fs::write(&path, &bytes).unwrap();

    let mut from_file = Sha256Hasher::new();
    from_file.update_file_length_framed(&path).unwrap();
    let mut from_memory = Sha256Hasher::new();
    from_memory.update_length_framed(&bytes);
    assert_eq!(from_file.finalize_hex(), from_memory.finalize_hex());

    let _ = std::fs::remove_dir_all(&folder);
}

#[test]
fn an_unreadable_file_is_an_error_rather_than_a_digest() {
    let mut hasher = Sha256Hasher::new();
    let error = hasher
        .update_file_length_framed(std::path::Path::new("/no/such/fingerprint/input"))
        .unwrap_err();
    assert!(matches!(error, Error::FileUnreadable { .. }), "{error}");
}
