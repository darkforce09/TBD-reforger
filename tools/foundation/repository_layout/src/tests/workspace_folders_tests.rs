use super::*;
use repository_root::find_repository_root;

/// Every folder is committed, so it exists in a real checkout; the SQL folders hold SQL files.
#[test]
fn the_workspace_folders_exist_in_the_checkout() {
    let root = find_repository_root().expect("active checkout");
    for folder in [
        APPLICATIONS_DIR,
        LIBRARY_CRATES_DIR,
        TOOLS_DIR,
        API_DATABASE_CRATE_DIR,
        API_DATABASE_MIGRATIONS_DIR,
        API_DATABASE_SEEDS_DIR,
    ] {
        assert!(root.join(folder).is_dir(), "not a folder: {folder}");
    }
    for folder in [API_DATABASE_MIGRATIONS_DIR, API_DATABASE_SEEDS_DIR] {
        let sql_files = std::fs::read_dir(root.join(folder))
            .expect("the folder reads")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "sql"))
            .count();
        assert!(sql_files > 0, "{folder} holds no SQL file");
    }
}

/// The API database crate lies under the library crates, and its SQL folders under the crate.
#[test]
fn the_database_folders_nest_under_the_library_crates() {
    assert!(API_DATABASE_CRATE_DIR.starts_with(&format!("{LIBRARY_CRATES_DIR}/")));
    for folder in [API_DATABASE_MIGRATIONS_DIR, API_DATABASE_SEEDS_DIR] {
        assert!(
            folder.starts_with(&format!("{API_DATABASE_CRATE_DIR}/")),
            "{folder}"
        );
    }
    for folder in [
        APPLICATIONS_DIR,
        LIBRARY_CRATES_DIR,
        TOOLS_DIR,
        API_DATABASE_CRATE_DIR,
        API_DATABASE_MIGRATIONS_DIR,
        API_DATABASE_SEEDS_DIR,
    ] {
        assert!(!folder.ends_with('/'), "trailing separator: {folder}");
    }
}
