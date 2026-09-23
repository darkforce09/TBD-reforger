use super::*;
use crate::test_support::Scratch;

fn work(id: &str, status_lines: &str) -> String {
    format!(
        r#"id = "{id}"
kind = "work"
title = "title {id}"
{status_lines}

[scope]
domain = "repo"
layer = "docs"
"#
    )
}

/// 3 parents + 2 children, plus non-ticket noise that the glob must ignore.
fn write_corpus(root: &Path) -> PathBuf {
    let dir = root.join(ticket_engine::repository::TICKETS_DIR);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("T-1.toml"), work("T-1", "status = \"idea\"")).unwrap();
    fs::write(
        dir.join("T-2.toml"),
        r#"id = "T-2"
kind = "program"
title = "program two"
status = "queued"
order = 10
children = ["T-2.1", "T-2.2"]
"#,
    )
    .unwrap();
    fs::write(
        dir.join("T-2.1.toml"),
        work("T-2.1", "status = \"shipped\"\nshipped_at = \"abc123\""),
    )
    .unwrap();
    fs::write(
        dir.join("T-2.2.toml"),
        work("T-2.2", "status = \"deferred\""),
    )
    .unwrap();
    fs::write(
        dir.join("T-3.toml"),
        work("T-3", "status = \"queued\"\norder = 20"),
    )
    .unwrap();
    // Noise the `T-*.toml` glob must not count:
    fs::write(dir.join("README.md"), "not a ticket").unwrap();
    fs::write(dir.join("schema.json"), "{}").unwrap();
    fs::write(dir.join("T-4.txt"), "wrong extension").unwrap();
    fs::write(dir.join("X-5.toml"), "wrong prefix").unwrap();
    fs::create_dir_all(dir.join("T-6.toml")).unwrap(); // a DIRECTORY named like a ticket
    dir
}

#[test]
fn counts_match_the_scratch_corpus() {
    let s = Scratch::new("counts");
    let dir = write_corpus(s.path());
    let corpus = load_corpus(s.path()).unwrap();
    assert_eq!(
        corpus.counts,
        Counts {
            total: 5,
            parents: 3,
            children: 2
        }
    );
    assert_eq!(
        corpus.counts.parents + corpus.counts.children,
        corpus.counts.total
    );
    assert_eq!(corpus.tickets.len(), corpus.counts.total);
    // Per-file source path is recorded.
    let t1 = corpus
        .tickets
        .iter()
        .find(|t| t.ticket.id() == "T-1")
        .unwrap();
    assert_eq!(t1.path, dir.join("T-1.toml"));
}

#[test]
fn fail_closed_names_the_bad_file() {
    let s = Scratch::new("fail-closed");
    let dir = write_corpus(s.path());
    fs::write(dir.join("T-2.1.toml"), "not = [valid toml").unwrap();
    let err = load_corpus(s.path()).unwrap_err();
    assert_eq!(err.file, dir.join("T-2.1.toml"));
    assert!(!err.error.is_empty(), "verbatim parse error expected");
}

#[test]
fn fail_closed_on_semantic_error_too() {
    let s = Scratch::new("semantic");
    let dir = write_corpus(s.path());
    // Valid TOML, invalid ticket: idea must not carry order.
    fs::write(
        dir.join("T-3.toml"),
        work("T-3", "status = \"idea\"\norder = 7"),
    )
    .unwrap();
    let err = load_corpus(s.path()).unwrap_err();
    assert_eq!(err.file, dir.join("T-3.toml"));
    assert!(
        err.error.contains("idea must not carry order"),
        "{}",
        err.error
    );
}

#[test]
fn missing_tickets_dir_refuses_with_the_path() {
    let s = Scratch::new("no-dir");
    let err = load_corpus(s.path()).unwrap_err();
    assert_eq!(
        err.file,
        s.path().join(ticket_engine::repository::TICKETS_DIR)
    );
    assert!(
        err.error.contains(ticket_engine::repository::TICKETS_DIR),
        "{}",
        err.error
    );
}

#[test]
fn child_id_classification() {
    assert!(!is_child_id("T-915"));
    assert!(is_child_id("T-915.1"));
    assert!(is_child_id("T-915.10"));
}

/// Manual smoke against the LIVE repo corpus (`cargo test -p ticketboard -- --ignored`):
/// proves every real ticket file parses through this exact load path, so the
/// board's first launch cannot hit a surprise refusal. Ignored by default —
/// the normal test run stays hermetic (scratch dirs only).
#[test]
#[ignore = "reads the live repo corpus; run explicitly with -- --ignored"]
fn live_corpus_loads_and_counts_sum() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(root) =
        crate::ticket_registry::services::discovery::walk_up_for_tickets(&manifest_dir)
    else {
        panic!(
            "no {}/ above {}",
            ticket_engine::repository::TICKETS_DIR,
            manifest_dir.display()
        );
    };
    let corpus = load_corpus(&root).unwrap_or_else(|e| panic!("live corpus refused: {e}"));
    assert!(corpus.counts.total > 0);
    assert_eq!(
        corpus.counts.parents + corpus.counts.children,
        corpus.counts.total
    );
    println!(
        "live corpus: {} files = {} parents + {} children",
        corpus.counts.total, corpus.counts.parents, corpus.counts.children
    );
}
