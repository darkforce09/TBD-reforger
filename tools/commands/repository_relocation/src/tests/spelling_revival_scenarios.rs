//! `--verify` over a retired `path` spelling that a later manifest puts back in use: a later
//! `path` row whose `to` is the retired `from` or a path below it makes the spelling legal again at
//! or below that `to`, for the earlier row's spelling scan and its still-tracked check; a `to` one
//! folder above the retired spelling and a manifest ordered before the retiring one revive nothing,
//! and a still-later manifest that retires the spelling again judges it with its own row.

use std::path::PathBuf;

use super::file_treatment::manifests_folder;
use super::fixture_repository::FixtureRepository;
use super::repository_files::RepositorySnapshot;
use super::retired_spellings::{ManifestToJudge, judge_manifests};
use super::single_pass_verification_scenarios::{composed_manifests, rendered};
use super::verify;

const HEADER: &str = "kind\tfrom\tto\tscope\n";

/// The retiring move: `x/old` leaves for `x/new`.
const MOVE_AWAY: &str = "path\tx/old\tx/new\t\n";

/// The reviving move: `x/new` comes back to `x/old`.
const MOVE_BACK: &str = "path\tx/new\tx/old\t\n";

fn manifest_path(name: &str) -> String {
    format!("{}/{name}", manifests_folder())
}

/// Write the stage manifest `name` holding `rows`.
fn write_manifest(repo: &FixtureRepository, name: &str, rows: &str) {
    repo.write(&manifest_path(name), &format!("{HEADER}{rows}"));
}

/// The checkout as the move back leaves it: `x/old/file.txt` tracked and spelled by a live file.
fn write_moved_back_tree(repo: &FixtureRepository) {
    repo.write("x/old/file.txt", "kept\n")
        .write(
            "README.md",
            "See `x/old/file.txt` and [the folder](/x/old).\n",
        )
        .track();
}

/// Each stage manifest's file name with its notes and verdicts rendered, oldest first.
fn judgements(repo: &FixtureRepository) -> Vec<(String, Vec<String>)> {
    let snapshot = RepositorySnapshot::load(repo.root()).expect("list the fixture checkout");
    let manifests = composed_manifests(repo.root());
    let judged: Vec<ManifestToJudge<'_>> = manifests
        .iter()
        .map(|(label, rows, later)| ManifestToJudge {
            label,
            rows,
            run_manifest: None,
            later,
        })
        .collect();
    let names = manifests.iter().map(|(label, _, _)| {
        label
            .rsplit('/')
            .next()
            .expect("a manifest label")
            .to_string()
    });
    names
        .zip(
            judge_manifests(&snapshot, &judged)
                .into_iter()
                .map(rendered),
        )
        .collect()
}

/// The rendered lines of the manifest named `name`.
fn lines_of<'a>(judged: &'a [(String, Vec<String>)], name: &str) -> &'a [String] {
    judged
        .iter()
        .find(|(manifest, _)| manifest == name)
        .map(|(_, lines)| lines.as_slice())
        .unwrap_or_else(|| panic!("no judgement of {name}: {judged:#?}"))
}

fn failed(lines: &[String]) -> bool {
    lines.iter().any(|line| line.contains("Failed"))
}

fn still_tracked(lines: &[String]) -> bool {
    lines.iter().any(|line| line.contains("is still tracked"))
}

#[test]
fn relocate_verify_passes_a_spelling_a_later_manifest_moved_back() {
    let repo = FixtureRepository::new("revival-move-back");
    write_manifest(&repo, "stage_a_away.tsv", MOVE_AWAY);
    repo.commit("stage a: move x/old away");
    write_manifest(&repo, "stage_b_back.tsv", MOVE_BACK);
    repo.commit("stage b: move it back");
    write_moved_back_tree(&repo);

    assert_eq!(
        verify(repo.root(), None),
        0,
        "the moved-back path is tracked and spelled again"
    );
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(!failed(away), "the earlier row holds: {away:#?}");
    assert!(
        away.iter().any(|line| line.starts_with("note:")
            && line.contains("`x/old` is in use again at or below `x/old`")
            && line.contains("stage_b_back.tsv line 2")),
        "the revival leaves a note naming the later row: {away:#?}"
    );
    let named = PathBuf::from(repo.root()).join(manifest_path("stage_a_away.tsv"));
    assert_eq!(
        verify(repo.root(), Some(&named)),
        0,
        "a named committed manifest composes the later revival"
    );
}

#[test]
fn relocate_verify_lets_an_uncommitted_manifest_revive_a_spelling() {
    let repo = FixtureRepository::new("revival-uncommitted");
    write_manifest(&repo, "stage_b_away.tsv", MOVE_AWAY);
    repo.commit("stage b: move x/old away");
    // Sorts first by name, but no commit added it, so it comes after every committed manifest.
    write_manifest(&repo, "stage_a_back.tsv", MOVE_BACK);
    write_moved_back_tree(&repo);

    assert_eq!(
        verify(repo.root(), None),
        0,
        "a manifest not committed yet is the latest, and its move back revives the spelling"
    );
}

#[test]
fn relocate_verify_fails_a_retired_spelling_no_later_manifest_revives() {
    let repo = FixtureRepository::new("revival-none");
    write_manifest(&repo, "stage_a_away.tsv", MOVE_AWAY);
    repo.commit("stage a: move x/old away");
    write_moved_back_tree(&repo);

    assert_eq!(verify(repo.root(), None), 1, "no manifest revives x/old");
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(
        failed(away) && still_tracked(away) && away.iter().any(|l| l.contains("README.md:1")),
        "the spellings and the tracked folder are findings of the retiring row: {away:#?}"
    );

    write_manifest(&repo, "stage_b_unrelated.tsv", "path\ty/one\ty/two\t\n");
    repo.track();
    assert_eq!(
        verify(repo.root(), None),
        1,
        "a later manifest whose `to` lies elsewhere revives nothing"
    );
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(failed(away) && still_tracked(away), "{away:#?}");
    assert!(
        !away.iter().any(|line| line.starts_with("note:")),
        "an unrelated move leaves no revival note: {away:#?}"
    );

    std::fs::remove_file(repo.root().join("README.md")).expect("drop the spelling");
    repo.track();
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(
        failed(away) && still_tracked(away),
        "a tracked x/old alone fails the still-tracked check: {away:#?}"
    );
}

#[test]
fn relocate_verify_judges_a_revived_spelling_retired_again_with_the_later_row() {
    let repo = FixtureRepository::new("revival-retired-again");
    write_manifest(&repo, "stage_a_away.tsv", MOVE_AWAY);
    repo.commit("stage a: move x/old away");
    write_manifest(&repo, "stage_b_back.tsv", MOVE_BACK);
    repo.commit("stage b: move it back");
    write_manifest(&repo, "stage_c_third.tsv", "path\tx/old\tx/third\t\n");
    repo.commit("stage c: move it away again");
    repo.write("x/third/file.txt", "kept\n")
        .write("README.md", "See `x/old/file.txt`.\n")
        .track();

    assert_eq!(verify(repo.root(), None), 1, "x/old is retired again");
    let judged = judgements(&repo);
    let third = lines_of(&judged, "stage_c_third.tsv");
    assert!(
        failed(third) && third.iter().any(|line| line.contains("README.md:1")),
        "the re-retiring row judges the spelling: {third:#?}"
    );
    assert!(!still_tracked(third), "{third:#?}");
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(
        !failed(away),
        "the first row stays freed by the revival: {away:#?}"
    );

    std::fs::remove_file(repo.root().join("README.md")).expect("drop the spelling");
    repo.write("x/old/other.txt", "back again\n").track();
    let judged = judgements(&repo);
    let third = lines_of(&judged, "stage_c_third.tsv");
    assert!(
        failed(third) && still_tracked(third),
        "a tracked x/old fails the re-retiring row's still-tracked check: {third:#?}"
    );
    assert!(!failed(lines_of(&judged, "stage_a_away.tsv")));
}

#[test]
fn relocate_verify_revives_nothing_from_a_manifest_ordered_before_the_retiring_one() {
    let repo = FixtureRepository::new("revival-earlier");
    // Name order would put the move back last; the history puts it first.
    write_manifest(&repo, "stage_z_back.tsv", MOVE_BACK);
    repo.commit("stage z: move x/new to x/old");
    write_manifest(&repo, "stage_a_away.tsv", MOVE_AWAY);
    repo.commit("stage a: move x/old away");
    write_moved_back_tree(&repo);

    assert_eq!(
        verify(repo.root(), None),
        1,
        "a move ordered before the retiring manifest revives nothing"
    );
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(failed(away) && still_tracked(away), "{away:#?}");
    let back = lines_of(&judged, "stage_z_back.tsv");
    assert!(
        back.iter()
            .any(|line| line.contains("`x/new` is in use again")),
        "the later retiring manifest revives the earlier one's `from`: {back:#?}"
    );
}

#[test]
fn relocate_verify_revives_only_the_path_below_a_retired_folder_a_later_row_names() {
    let repo = FixtureRepository::new("revival-below");
    write_manifest(&repo, "stage_a_away.tsv", MOVE_AWAY);
    repo.commit("stage a: move x/old away");
    write_manifest(&repo, "stage_b_into.tsv", "path\ty/sub\tx/old/sub\t\n");
    repo.commit("stage b: move y/sub below the retired folder");
    repo.write("x/old/sub/file.txt", "kept\n")
        .write("README.md", "See `x/old/sub/file.txt`.\n")
        .track();

    assert_eq!(
        verify(repo.root(), None),
        0,
        "the path below the retired folder that the later row names is live again"
    );

    repo.write(
        "README.md",
        "See `x/old/sub.md` and `x/old/subway/a.txt`.\n",
    )
    .track();
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(
        failed(away) && !still_tracked(away) && away.iter().any(|l| l.contains("2 retired")),
        "a spelling beside the revived path is still a finding: {away:#?}"
    );

    repo.write("README.md", "No spelling.\n")
        .write("x/old/other.txt", "outside the revived path\n")
        .track();
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(
        failed(away) && still_tracked(away),
        "a file of x/old outside the revived path keeps x/old tracked: {away:#?}"
    );
}

#[test]
fn relocate_verify_revives_nothing_from_a_later_move_into_a_folder_above() {
    let repo = FixtureRepository::new("revival-above");
    write_manifest(
        &repo,
        "stage_a_away.tsv",
        "path\tx/old/file.txt\tx/kept.txt\t\n",
    );
    repo.commit("stage a: move x/old/file.txt away");
    write_manifest(&repo, "stage_b_above.tsv", "path\ty/old\tx/old\t\n");
    repo.commit("stage b: move a folder to x/old, one folder above the retired file");
    repo.write("x/old/file.txt", "back\n")
        .write("README.md", "See `x/old/file.txt`.\n")
        .track();

    assert_eq!(
        verify(repo.root(), None),
        1,
        "a later `to` above the retired spelling revives nothing"
    );
    let judged = judgements(&repo);
    let away = lines_of(&judged, "stage_a_away.tsv");
    assert!(
        failed(away) && still_tracked(away) && away.iter().any(|l| l.contains("README.md:1")),
        "the spelling and the tracked file stay findings of the retiring row: {away:#?}"
    );
    assert!(
        !away.iter().any(|line| line.starts_with("note:")),
        "no revival note: {away:#?}"
    );
}
