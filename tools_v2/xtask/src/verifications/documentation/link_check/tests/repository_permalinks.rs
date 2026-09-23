use super::*;

const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

fn blob(id: &str, size: usize) -> BlobObject {
    BlobObject {
        id: id.to_string(),
        size,
    }
}

#[test]
fn a_permalink_names_a_full_commit_and_a_decoded_path() {
    let url = format!("{PERMALINK_BASE}{COMMIT}/docs/My%20Plan.md?plain=1#L4-L9");
    assert_eq!(
        read_repository_url(&url),
        Some(Some(Permalink {
            commit: COMMIT.to_string(),
            path: "docs/My Plan.md".to_string(),
            plain_view: true,
            fragment: Some("L4-L9".to_string()),
        }))
    );
    let sha256 = "ab".repeat(32);
    let long = format!("{PERMALINK_BASE}{sha256}/apps/");
    assert_eq!(
        read_repository_url(&long)
            .flatten()
            .map(|found| (found.commit.len(), found.path)),
        Some((64, "apps".to_string()))
    );
}

#[test]
fn the_repository_is_matched_without_regard_to_case_scheme_or_www() {
    let shouted_base = PERMALINK_BASE.to_uppercase().replace("/BLOB/", "/blob/");
    let upper = format!("{shouted_base}{}/README.md", COMMIT.to_uppercase());
    let found = read_repository_url(&upper).flatten().expect("a permalink");
    assert_eq!(found.commit, COMMIT, "the commit id reads lowercase");
    let insecure = PERMALINK_BASE.replace("https://", "http://www.");
    assert!(matches!(
        read_repository_url(&format!("{insecure}{COMMIT}/README.md")),
        Some(Some(_))
    ));
}

#[test]
fn another_page_of_the_repository_is_no_permalink() {
    let home = PERMALINK_BASE.trim_end_matches("/blob/");
    for page in [
        home.to_string(),
        format!("{home}/"),
        format!("{home}.git"),
        format!("{home}/issues/3"),
        format!("{home}/tree/{COMMIT}/apps"),
        format!("{PERMALINK_BASE}main/README.md"),
        format!("{PERMALINK_BASE}{}/README.md", &COMMIT[..12]),
    ] {
        assert_eq!(read_repository_url(&page), Some(None), "{page}");
    }
}

#[test]
fn a_url_of_another_repository_or_host_is_not_this_repository() {
    let home = PERMALINK_BASE.trim_end_matches("/blob/");
    for other in [
        format!("{home}-fork/blob/{COMMIT}/x.md"),
        format!("{home}x/blob/{COMMIT}/x.md"),
        "https://example.com/darkforce09".to_string(),
        "mailto:someone@example.com".to_string(),
    ] {
        assert_eq!(read_repository_url(&other), None, "{other}");
    }
}

#[test]
fn a_lookup_answer_is_a_blob_another_object_or_unknown() {
    let answers = format!(
        "{COMMIT} blob 120\n{COMMIT} tree 64\n{COMMIT}:gone.md missing\n{COMMIT}:x ambiguous\n"
    );
    assert_eq!(
        parse_lookup(&answers, 4),
        Ok(vec![
            ObjectLookup::Blob(blob(COMMIT, 120)),
            ObjectLookup::NotBlob {
                kind: "tree".to_string()
            },
            ObjectLookup::Unknown {
                answer: "missing".to_string()
            },
            ObjectLookup::Unknown {
                answer: "ambiguous".to_string()
            },
        ])
    );
}

#[test]
fn a_lookup_answer_in_an_unexpected_shape_is_refused() {
    assert!(parse_lookup(&format!("{COMMIT} blob 1\n"), 2).is_err());
    assert!(parse_lookup("garbage\n", 1).is_err());
    assert!(parse_lookup(&format!("{COMMIT} blob many\n"), 1).is_err());
}

#[test]
fn blob_contents_split_by_their_sizes() {
    let first = "line one\nline two\n";
    let second = "é\n";
    let stdout = format!(
        "{COMMIT} blob {}\n{first}\nfeedface blob {}\n{second}\n",
        first.len(),
        second.len()
    );
    let blobs = [blob(COMMIT, first.len()), blob("feedface", second.len())];
    assert_eq!(
        parse_contents(&stdout, &blobs),
        Ok(vec![first.to_string(), second.to_string()])
    );
}

#[test]
fn a_blob_that_decoded_longer_than_its_size_ends_at_the_next_header() {
    // Two invalid bytes decode as two three-byte replacement characters.
    let decoded = "a\u{FFFD}\u{FFFD}\nb";
    let stdout = format!("{COMMIT} blob 5\n{decoded}\nfeedface blob 2\nok\n");
    let blobs = [blob(COMMIT, 5), blob("feedface", 2)];
    assert_eq!(
        parse_contents(&stdout, &blobs),
        Ok(vec![decoded.to_string(), "ok".to_string()])
    );
    let last = format!("{COMMIT} blob 5\n{decoded}\n");
    assert_eq!(
        parse_contents(&last, &blobs[..1]),
        Ok(vec![decoded.to_string()])
    );
}

#[test]
fn contents_that_do_not_answer_the_request_are_refused() {
    let blobs = [blob(COMMIT, 3)];
    assert!(parse_contents("", &blobs).is_err());
    assert!(parse_contents("feedface blob 3\nabc\n", &blobs).is_err());
}

#[test]
fn a_permalink_names_its_object_as_commit_colon_path() {
    let permalink = Permalink {
        commit: COMMIT.to_string(),
        path: "apps/README.md".to_string(),
        plain_view: false,
        fragment: None,
    };
    assert_eq!(permalink.object_name(), format!("{COMMIT}:apps/README.md"));
}

#[test]
fn a_failed_or_misshapen_batch_did_not_run() {
    match batch_problem(LOOKUP_ARGUMENTS, 128, "fatal: not a git repository") {
        NotRun::ToolError {
            tool,
            status,
            stderr,
        } => {
            assert_eq!(tool, "git cat-file --batch-check");
            assert_eq!(status, 128);
            assert_eq!(stderr, "fatal: not a git repository");
        }
        other => panic!("a tool error, not {other:?}"),
    }
}

#[test]
fn the_real_history_answers_a_lookup_in_one_batch() {
    let root = crate::core::repository_root::test_repo_root();
    let objects = GitObjects::new(&root);
    let names = vec![
        "HEAD:Cargo.toml".to_string(),
        "HEAD:tools_v2".to_string(),
        "HEAD:no-such-file-anywhere.md".to_string(),
    ];
    let found = objects.lookup(&names).expect("git answers");
    assert!(matches!(found[0], ObjectLookup::Blob(_)));
    assert_eq!(
        found[1],
        ObjectLookup::NotBlob {
            kind: "tree".to_string()
        }
    );
    assert_eq!(
        found[2],
        ObjectLookup::Unknown {
            answer: "missing".to_string()
        }
    );
    let ObjectLookup::Blob(manifest) = &found[0] else {
        unreachable!()
    };
    let texts = objects
        .contents(std::slice::from_ref(manifest))
        .expect("git answers");
    assert!(texts[0].contains("[workspace]"));
}
