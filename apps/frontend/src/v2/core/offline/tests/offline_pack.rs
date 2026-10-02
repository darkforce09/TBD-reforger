use super::*;
use crate::v2::core::offline::offline_manifest::target_for;

#[test]
fn the_offline_pack_route_is_the_mortar_calculator_and_below() {
    assert!(is_offline_pack_route("/tools/mortar"));
    assert!(is_offline_pack_route("/tools/mortar/"));
    assert!(is_offline_pack_route("/tools/mortar/saved"));
    assert!(!is_offline_pack_route("/tools/mortars"));
    assert!(!is_offline_pack_route("/tools"));
    assert!(!is_offline_pack_route("/"));
}

#[test]
fn pack_progress_follows_declared_bytes_and_reads_100_only_when_every_file_is_stored() {
    let mut progress = PackProgress::new(3, 1_000);
    assert_eq!(progress.percent(), 0);
    progress.record_stored(Some(500));
    assert_eq!(progress.percent(), 50);
    progress.record_stored(Some(500));
    assert_eq!(progress.percent(), 99, "a file is still missing");
    progress.record_stored(None);
    assert_eq!(progress.percent(), 100);
    progress.record_stored(Some(9_999));
    assert_eq!(progress.stored_files, 3);
    assert_eq!(progress.stored_bytes, 1_000);
}

#[test]
fn pack_progress_counts_files_when_no_size_is_declared_and_an_empty_download_is_done() {
    let mut progress = PackProgress::new(4, 0);
    progress.record_stored(None);
    assert_eq!(progress.percent(), 25);
    assert_eq!(PackProgress::new(0, 0).percent(), 100);
}

const ORIGIN: &str = "https://tbd.example";

fn target(url: &str) -> OfflineTarget {
    target_for(ORIGIN, url, None).expect("a pack file")
}

fn icon_font_stylesheet() -> OfflineTarget {
    target("https://fonts.googleapis.com/css2?family=Material+Symbols+Outlined:opsz,wght,FILL,GRAD@24,400,0,0")
}

fn icon_font_file() -> OfflineTarget {
    target("https://fonts.gstatic.com/s/materialsymbolsoutlined/v1/a.woff2")
}

fn essential_files() -> Vec<OfflineTarget> {
    vec![
        target("/frontend-0123abcd.js"),
        target("/api/v1/ballistics-catalogs"),
        target("/api/v1/ballistics-catalogs/m252/versions/1"),
        target("/map-assets/everon/manifest.json"),
        target("/map-assets/everon/tiles/map/index.json"),
    ]
}

fn failed(targets: &[OfflineTarget]) -> FailedFiles {
    let mut failed = FailedFiles::default();
    for target in targets {
        failed.record(target);
    }
    failed
}

#[test]
fn final_outcome_is_ready_with_missing_optional_files_when_only_the_icon_font_failed() {
    let outcome = final_outcome(true, failed(&[icon_font_stylesheet(), icon_font_file()]));
    assert_eq!(outcome.state, OfflineState::Ready);
    assert_eq!(outcome.optional_files, OptionalFiles::Missing);
    assert_eq!(outcome.optional_files.attribute_value(), Some("missing"));
    let incomplete = final_outcome(false, failed(&[icon_font_file()]));
    assert_eq!(incomplete.state, OfflineState::Incomplete);
    assert_eq!(incomplete.optional_files, OptionalFiles::Missing);
}

#[test]
fn final_outcome_is_failed_whenever_an_essential_file_failed() {
    for essential in essential_files() {
        assert!(!essential.is_optional(), "{}", essential.key);
        let alone = final_outcome(true, failed(std::slice::from_ref(&essential)));
        assert_eq!(alone.state, OfflineState::Failed, "{}", essential.key);
        assert_eq!(alone.optional_files, OptionalFiles::Complete);
        let with_font = final_outcome(true, failed(&[essential.clone(), icon_font_file()]));
        assert_eq!(with_font.state, OfflineState::Failed, "{}", essential.key);
        assert_eq!(with_font.optional_files, OptionalFiles::Missing);
        assert_eq!(
            final_outcome(false, failed(&[essential])).state,
            OfflineState::Failed
        );
    }
}

#[test]
fn final_outcome_is_ready_with_complete_optional_files_when_nothing_failed() {
    let outcome = final_outcome(true, FailedFiles::default());
    assert_eq!(outcome.state, OfflineState::Ready);
    assert_eq!(outcome.optional_files, OptionalFiles::Complete);
    assert_eq!(outcome.optional_files.attribute_value(), Some("complete"));
    assert_eq!(
        final_outcome(false, FailedFiles::default()).state,
        OfflineState::Incomplete
    );
}

#[test]
fn an_unreadable_icon_font_stylesheet_counts_as_a_missing_optional_file() {
    let mut failed = FailedFiles::default();
    failed.record_unlisted_optional_files();
    let outcome = final_outcome(true, failed);
    assert_eq!(outcome.state, OfflineState::Ready);
    assert_eq!(outcome.optional_files, OptionalFiles::Missing);
}

#[test]
fn ensure_offline_pack_starts_once_per_page() {
    assert!(!DOWNLOAD_STARTED.with(|started| started.get()));
    ensure_offline_pack();
    assert!(DOWNLOAD_STARTED.with(|started| started.get()));
    ensure_offline_pack();
    assert!(DOWNLOAD_STARTED.with(|started| started.get()));
}

#[test]
fn a_refresh_the_server_could_not_answer_keeps_a_cached_essential_file_and_the_pack_ready() {
    let mut kept = FailedFiles::default();
    for essential in essential_files() {
        kept.record_refresh_failure(&essential, true, true);
    }
    assert_eq!(kept.essential, 0);
    assert_eq!(kept.kept_saved_copies, essential_files().len());
    let outcome = final_outcome(true, kept);
    assert_eq!(outcome.state, OfflineState::Ready);
    assert!(outcome.kept_saved_copy);
    assert_eq!(outcome.optional_files, OptionalFiles::Complete);
    let incomplete = final_outcome(false, kept);
    assert_eq!(incomplete.state, OfflineState::Incomplete);
    assert!(incomplete.kept_saved_copy);
}

#[test]
fn a_refresh_failure_is_a_failure_when_the_saved_copy_is_missing_or_the_server_refused() {
    for essential in essential_files() {
        for (server_unavailable, saved_copy_cached) in
            [(true, false), (false, true), (false, false)]
        {
            let mut failed = FailedFiles::default();
            failed.record_refresh_failure(&essential, server_unavailable, saved_copy_cached);
            assert_eq!(failed.essential, 1, "{}", essential.key);
            assert_eq!(failed.kept_saved_copies, 0);
            let outcome = final_outcome(true, failed);
            assert_eq!(outcome.state, OfflineState::Failed, "{}", essential.key);
            assert!(!outcome.kept_saved_copy);
        }
    }
}

#[test]
fn an_optional_file_is_never_kept_as_a_saved_copy() {
    let mut failed = FailedFiles::default();
    failed.record_refresh_failure(&icon_font_file(), true, true);
    assert_eq!(failed.optional, 1);
    assert_eq!(failed.kept_saved_copies, 0);
    let outcome = final_outcome(true, failed);
    assert_eq!(outcome.state, OfflineState::Ready);
    assert_eq!(outcome.optional_files, OptionalFiles::Missing);
    assert!(!outcome.kept_saved_copy);
}

#[test]
fn a_listing_read_from_its_saved_copy_keeps_the_pack_ready_and_flags_the_refresh() {
    let mut failed = FailedFiles::default();
    failed.record_kept_saved_copy();
    let outcome = final_outcome(true, failed);
    assert_eq!(outcome.state, OfflineState::Ready);
    assert!(outcome.kept_saved_copy);
    assert!(!final_outcome(true, FailedFiles::default()).kept_saved_copy);
}
