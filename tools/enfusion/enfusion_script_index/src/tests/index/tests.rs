use super::*;

/// The `sha256` column of the files table is the lowercase hex SHA-256 of the file's bytes, and
/// an unreadable file leaves the column empty.
#[test]
fn the_files_table_digest_is_the_lowercase_hex_sha256_of_the_bytes() {
    let folder = std::env::temp_dir().join(format!("enf-index-digest-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("Sample.c");
    std::fs::write(&file, b"abc").unwrap();
    assert_eq!(
        sha256_file(&file),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(sha256_file(&folder.join("Missing.c")), "");
    std::fs::remove_dir_all(&folder).unwrap();
}
