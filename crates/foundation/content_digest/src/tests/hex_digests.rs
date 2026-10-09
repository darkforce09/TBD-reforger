use super::*;

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
