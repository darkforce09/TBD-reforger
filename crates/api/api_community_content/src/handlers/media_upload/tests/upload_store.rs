use std::path::PathBuf;

use super::*;

/// A fresh directory under the system temporary directory, removed when dropped.
struct ScratchDirectory(PathBuf);

impl ScratchDirectory {
    fn new() -> Self {
        Self(std::env::temp_dir().join(format!("upload-store-{}", Uuid::new_v4())))
    }
}

impl Drop for ScratchDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn entries(directory: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(directory)
        .expect("read directory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn stores_the_whole_file_under_its_name_and_leaves_no_staging_file() {
    let scratch = ScratchDirectory::new();
    let directory = scratch.0.join("uploads");
    let bytes: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();

    store_upload(&directory, "picture.png", &bytes)
        .await
        .expect("store succeeds and creates the missing directory");

    assert_eq!(entries(&directory), vec!["picture.png".to_owned()]);
    assert_eq!(
        std::fs::read(directory.join("picture.png")).expect("read stored file"),
        bytes
    );
}

#[tokio::test]
async fn a_failed_rename_answers_the_io_error_and_leaves_no_staging_file() {
    let scratch = ScratchDirectory::new();
    let directory = scratch.0.clone();
    std::fs::create_dir_all(directory.join("taken.png")).expect("directory in the way");

    let error = store_upload(&directory, "taken.png", b"bytes")
        .await
        .expect_err("renaming a file over a directory fails");

    assert_ne!(error.kind(), io::ErrorKind::NotFound, "{error}");
    assert_eq!(entries(&directory), vec!["taken.png".to_owned()]);
    assert!(directory.join("taken.png").is_dir());
}
