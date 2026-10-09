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
        API_SERVER_CRATE_DIR,
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

/// The API server crate's manifest names the `api_server` package, and its environment file
/// template sits beside the `.env` the binaries read.
#[test]
fn the_api_server_crate_holds_its_manifest_and_environment_template() {
    let root = find_repository_root().expect("active checkout");
    let manifest = std::fs::read_to_string(root.join(API_SERVER_CRATE_DIR).join("Cargo.toml"))
        .expect("the API server manifest reads");
    assert!(
        manifest.contains("name = \"api_server\""),
        "{API_SERVER_CRATE_DIR} is not the api_server package"
    );
    assert!(
        root.join(format!("{API_SERVER_ENVIRONMENT_FILE}.example"))
            .is_file(),
        "no environment template beside {API_SERVER_ENVIRONMENT_FILE}"
    );
}

/// The API crates lie under the library crates, their SQL folders and the `.env` under the
/// crates that own them.
#[test]
fn the_database_folders_nest_under_the_library_crates() {
    assert!(API_DATABASE_CRATE_DIR.starts_with(&format!("{LIBRARY_CRATES_DIR}/")));
    assert!(API_SERVER_CRATE_DIR.starts_with(&format!("{LIBRARY_CRATES_DIR}/")));
    assert_eq!(
        API_SERVER_ENVIRONMENT_FILE,
        format!("{API_SERVER_CRATE_DIR}/.env")
    );
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
        API_SERVER_CRATE_DIR,
        API_SERVER_ENVIRONMENT_FILE,
        API_DATABASE_CRATE_DIR,
        API_DATABASE_MIGRATIONS_DIR,
        API_DATABASE_SEEDS_DIR,
    ] {
        assert!(!folder.ends_with('/'), "trailing separator: {folder}");
    }
}
