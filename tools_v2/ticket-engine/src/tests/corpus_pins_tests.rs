use super::*;

/// The committed pins load, and both tables are populated: an empty table would silently retire
/// the rule it feeds, so the shapes are asserted rather than only the parse.
#[test]
fn committed_pins_load_with_both_tables_populated() {
    let root = crate::repository::find_repo_root().expect("repository root");
    let pins = load(&root).expect("committed corpus pins parse");
    assert!(!pins.never_minted.is_empty(), "never_minted is empty");
    assert!(
        !pins.gap_implementations.is_empty(),
        "gap_implementations is empty"
    );
    for programme in [
        &pins.game_mod_programme_ticket,
        &pins.map_terrain_programme_ticket,
    ] {
        assert!(
            root.join(crate::repository::TICKETS_DIR)
                .join(format!("{programme}.toml"))
                .is_file(),
            "{programme} is pinned as a programme and has no ticket file"
        );
    }
    for id in &pins.never_minted {
        assert!(
            !root
                .join(crate::repository::TICKETS_DIR)
                .join(format!("{id}.toml"))
                .exists(),
            "{id} is pinned as never minted and has a ticket file"
        );
    }
}

/// Lookups answer by exact id and refuse everything else.
#[test]
fn lookups_answer_by_exact_id() {
    let pins: CorpusPins = toml::from_str(
        "game_mod_programme_ticket = \"X-100\"\nmap_terrain_programme_ticket = \"X-200\"\n\
         never_minted = [\"X-001\"]\n\
         [gap_implementations]\n\"GAP-001\" = \"X-002\"\n",
    )
    .expect("fixture parses");
    assert!(pins.is_never_minted("X-001"));
    assert!(!pins.is_never_minted("X-0010"));
    assert_eq!(pins.gap_implementation("GAP-001"), Some("X-002"));
    assert_eq!(pins.gap_implementation("GAP-002"), None);
}

/// A missing file is an error naming the path, never an empty table.
#[test]
fn a_missing_file_is_an_error_naming_the_path() {
    let empty = std::env::temp_dir().join(format!("corpus-pins-absent-{}", std::process::id()));
    std::fs::create_dir_all(&empty).expect("scratch dir");
    let error = load(&empty).expect_err("absent pins must fail");
    assert!(
        format!("{error:#}").contains(crate::repository::CORPUS_PINS),
        "{error:#}"
    );
}

/// An unknown key is an error: a renamed table must not read as an absent one.
#[test]
fn an_unknown_key_is_an_error() {
    let error = toml::from_str::<CorpusPins>(
        "game_mod_programme_ticket = \"X-100\"\nmap_terrain_programme_ticket = \"X-200\"\n\
         never_minted = []\nstray = 1\n\
         [gap_implementations]\n",
    )
    .expect_err("unknown keys must fail");
    assert!(error.to_string().contains("stray"), "{error}");
}
