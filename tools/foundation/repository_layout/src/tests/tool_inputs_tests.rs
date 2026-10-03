use super::*;
use crate::find_repository_root;

/// The data folder and both files are committed, so they exist in a real checkout.
#[test]
fn the_staging_load_inputs_exist_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    assert!(root.join(STAGING_LOAD_DATA_DIR).is_dir());
    for file in [STAGING_LOAD_WORKLOAD, STAGING_LOAD_POPULATION] {
        assert!(root.join(file).is_file(), "not a file: {file}");
    }
}

/// Both files lie inside the data folder.
#[test]
fn the_staging_load_files_lie_under_the_data_folder() {
    for file in [STAGING_LOAD_WORKLOAD, STAGING_LOAD_POPULATION] {
        assert!(
            file.starts_with(&format!("{STAGING_LOAD_DATA_DIR}/")),
            "{file}"
        );
    }
}
