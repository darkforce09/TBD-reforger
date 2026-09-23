use std::os::unix::fs::symlink;

use super::*;

/// The names in `directory`, sorted.
fn entries(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn the_file_takes_the_new_contents_and_keeps_its_mode() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("server.json");
    fs::write(&path, "old").unwrap();
    fs::set_permissions(&path, Permissions::from_mode(0o640)).unwrap();
    replace_atomically(&path, b"new").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "new");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & PERMISSION_BITS,
        0o640
    );
    assert_eq!(
        entries(directory.path()),
        vec!["server.json"],
        "no new file left behind"
    );
}

#[test]
fn a_symbolic_link_keeps_naming_the_replaced_file() {
    let directory = tempfile::tempdir().unwrap();
    fs::create_dir(directory.path().join("configs")).unwrap();
    let real = directory.path().join("configs/server.json");
    fs::write(&real, "old").unwrap();
    let link = directory.path().join("server.json");
    symlink(&real, &link).unwrap();
    replace_atomically(&link, b"new").unwrap();
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_to_string(&real).unwrap(), "new");
    assert_eq!(
        entries(&directory.path().join("configs")),
        vec!["server.json"]
    );
}

#[test]
fn a_failed_rename_leaves_no_new_file_behind() {
    // A directory cannot be replaced by a file: the new file is written, then the rename fails.
    let directory = tempfile::tempdir().unwrap();
    let occupied = directory.path().join("server.json");
    fs::create_dir(&occupied).unwrap();
    fs::write(occupied.join("inside"), "kept").unwrap();
    assert!(replace_atomically(&occupied, b"new").is_err());
    assert_eq!(entries(directory.path()), vec!["server.json"]);
    assert_eq!(fs::read_to_string(occupied.join("inside")).unwrap(), "kept");
}

#[test]
fn a_missing_file_is_an_error_and_nothing_is_created() {
    let directory = tempfile::tempdir().unwrap();
    assert!(replace_atomically(&directory.path().join("server.json"), b"new").is_err());
    assert!(entries(directory.path()).is_empty());
}
