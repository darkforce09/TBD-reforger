use super::*;

#[test]
fn database_name_from_url_parses_ascii_path() {
    assert_eq!(
        database_name_from_url("postgres://tbd:tbd@localhost:5434/rust_it?sslmode=disable")
            .as_deref(),
        Some("rust_it")
    );
    assert_eq!(
        database_name_from_url("postgres://tbd:tbd@localhost:5434/tbd_reforger?sslmode=disable")
            .as_deref(),
        Some("tbd_reforger")
    );
    assert!(database_name_from_url("not-a-url").is_none());
    assert!(database_name_from_url("postgres://h/").is_none());
    assert!(database_name_from_url("postgres://h/a/b").is_none());
    assert!(database_name_from_url("postgres://h/weird-name").is_none());
}

#[test]
fn safe_scratch_allow_list_matches_t381() {
    assert!(is_safe_scratch_database_name("rust_it"));
    assert!(is_safe_scratch_database_name("tbd_gate_it"));
    assert!(is_safe_scratch_database_name("tbd_wave6_cold"));
    assert!(is_safe_scratch_database_name("tbd_t350_probe"));
    assert!(is_safe_scratch_database_name("tbd_t230_it"));
    assert!(is_safe_scratch_database_name("tbd_t884_probe"));
    assert!(!is_safe_scratch_database_name("tbd_reforger"));
    assert!(!is_safe_scratch_database_name("postgres"));
    assert!(!is_safe_scratch_database_name(""));
    assert!(!is_safe_scratch_database_name("production"));
}

#[test]
fn count_copy_rows_counts_data_not_headings() {
    let sample = "\
COPY public.t (id) FROM stdin;
1
2
\\.
COPY public.empty (id) FROM stdin;
\\.
";
    assert_eq!(count_copy_rows(sample), 2);
}
