use super::*;

/// NIST FIPS 180-4 sample vector for SHA-384.
#[test]
fn matches_the_published_abc_vector() {
    assert_eq!(
        sha384_hex(b"abc"),
        "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
         8086072ba1e7cc2358baeca134c825a7"
    );
}

#[test]
fn hashes_whole_file_bytes_including_the_trailing_newline() {
    let dir = std::env::temp_dir().join(format!("content-digest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("m.sql");
    std::fs::write(&path, b"abc").unwrap();
    assert_eq!(
        sha384_hex_of_file(&path).as_deref(),
        Some(sha384_hex(b"abc").as_str())
    );

    // A trailing newline is part of what sqlx hashed; it must change the digest.
    std::fs::write(&path, b"abc\n").unwrap();
    assert_ne!(
        sha384_hex_of_file(&path).as_deref(),
        Some(sha384_hex(b"abc").as_str())
    );

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_unreadable_path_is_none_rather_than_an_empty_digest() {
    assert_eq!(
        sha384_hex_of_file(std::path::Path::new("/no/such/migration.sql")),
        None
    );
}
