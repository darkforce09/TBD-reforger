use super::*;

use ticket_engine::repository::documentation::{GAP_ANALYSIS, ROADMAP};

/// Fixture mirroring a real dirty registry under [`GIT_ARGS`]: modified, staged,
/// untracked, a rename — plus merged-stream pollution that must not count.
fn fixture() -> String {
    format!(
        "\
warning: unable to access '/home/x/.gitconfig': Permission denied
 M .ai/tickets/T-915.3.toml
M  {ROADMAP}
?? .ai/tickets/T-999.toml
R  gap_analysis.md -> {GAP_ANALYSIS}
A  CLAUDE.md

hint: use git add to stage
"
    )
}

#[test]
fn porcelain_parse_counts_and_lists_entries_only() {
    let porcelain = fixture();
    let entries = parse_porcelain(porcelain.lines());
    assert_eq!(entries.len(), 5);
    assert_eq!(
        entries,
        vec![
            " M .ai/tickets/T-915.3.toml".to_owned(),
            format!("M  {ROADMAP}"),
            "?? .ai/tickets/T-999.toml".to_owned(),
            format!("R  gap_analysis.md -> {GAP_ANALYSIS}"),
            "A  CLAUDE.md".to_owned(),
        ]
    );
}

#[test]
fn chip_states_from_exit() {
    let porcelain = fixture();
    let dirty = chip_from_exit(Some(0), porcelain.lines());
    match &dirty {
        GitChip::Dirty(files) => assert_eq!(files.len(), 5),
        other => panic!("expected Dirty, got {other:?}"),
    }
    assert_eq!(dirty.label(), "5 uncommitted registry file(s)");

    let clean = chip_from_exit(Some(0), std::iter::empty());
    assert_eq!(clean, GitChip::Clean);
    assert_eq!(clean.label(), "clean");

    // Nonzero exit (e.g. "fatal: not a git repository" on stderr): the
    // stream is NOT parsed into a count.
    let unavailable = chip_from_exit(Some(128), ["fatal: not a git repository"]);
    assert_eq!(
        unavailable,
        GitChip::Unavailable("git exited 128".to_owned())
    );
    assert_eq!(unavailable.label(), "git unavailable");

    let killed = chip_from_exit(None, std::iter::empty());
    assert_eq!(killed.label(), "git unavailable");
    assert_eq!(GitChip::Unknown.label(), "git …");
}

#[test]
fn shape_filter_edges() {
    // Too short / wrong separator / non-status alphabet.
    assert!(parse_porcelain(["?? "].into_iter()).is_empty());
    assert!(parse_porcelain(["MM"].into_iter()).is_empty());
    assert!(parse_porcelain(["fatal: nope"].into_iter()).is_empty());
    assert!(parse_porcelain(["-- x"].into_iter()).is_empty());
    // Minimal legal entry.
    assert_eq!(parse_porcelain(["?? x"].into_iter()), vec!["?? x"]);
}
