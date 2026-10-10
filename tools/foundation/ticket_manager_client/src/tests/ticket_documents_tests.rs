//! The JSON contract read back: hand-written documents in the shapes `ttm --json` prints parse
//! into the typed values, a wrong format tag or a non-JSON answer is a contract break, and an
//! error document is a refusal that keeps its kind.

use crate::ticket_documents::{CheckDocument, ReceiptDocument, TicketDocument, VersionDocument};
use crate::wave_documents::{WaveCheckDocument, WaveHistoryDocument, WavePlan};
use crate::{Error, TicketManager, parse_document};

const VERSION: &str = include_str!("fixtures/version.json");
const SHOW_PROGRAMME: &str = include_str!("fixtures/show_programme.json");
const WAVE_SHOW: &str = include_str!("fixtures/wave_show.json");
const WAVE_HISTORY: &str = include_str!("fixtures/wave_history.json");
const WAVE_CHECK_FAILING: &str = include_str!("fixtures/wave_check_failing.json");
const RECORD_RUN: &str = include_str!("fixtures/record_run.json");
const ERROR_NOT_FOUND: &str = include_str!("fixtures/error_not_found.json");
const ERROR_AMBIGUOUS: &str = include_str!("fixtures/error_ambiguous.json");

#[test]
fn a_ticket_document_parses_with_its_children_and_legacy_number() {
    let ticket: TicketDocument = parse_document("ttm show T-181", SHOW_PROGRAMME).expect("parses");
    assert_eq!(ticket.slug, "game-mod");
    assert_eq!(ticket.display_reference(), "T-181");
    assert_eq!(ticket.children.len(), 2);
    assert_eq!(ticket.children[1].slug, "game-mod.briefing-map");
    assert_eq!(
        ticket.active_slice.as_ref().map(|s| s.as_str()),
        Some("game-mod.briefing-map")
    );
    assert!(ticket.has_spec);
    assert_eq!(ticket.wave, Some(0));
}

#[test]
fn the_wave_plan_answers_the_open_wave_rows_and_completion() {
    let plan: WavePlan = parse_document("ttm wave show", WAVE_SHOW).expect("parses");
    assert_eq!(plan.open_wave().map(|wave| wave.n), Some(249));
    assert_eq!(plan.open_counts(), (3, 2));
    // A slug and a legacy number in any case name the same row.
    assert!(plan.is_complete("slot-identity.flatten-emit"));
    assert!(plan.is_complete("t-674.1"));
    assert!(!plan.is_complete("slot-identity.wire-ids"));
    // A cancelled ticket of a pending-close wave counts as complete.
    assert!(plan.is_complete("T-640"));
    // An unknown ticket and one the plan does not list are never complete.
    assert!(!plan.is_complete("retired-ticket"));
    assert!(!plan.is_complete("never-planned"));
    assert_eq!(plan.emptied.first().map(|wave| wave.n), Some(248));
}

#[test]
fn a_plan_whose_every_open_wave_is_complete_has_no_open_wave() {
    let mut plan: WavePlan = parse_document("ttm wave show", WAVE_SHOW).expect("parses");
    plan.waves.truncate(1);
    for row in &mut plan.waves[0].tickets {
        row.status = Some("shipped".to_string());
    }
    assert!(plan.open_wave().is_none());
}

#[test]
fn the_remaining_documents_parse() {
    let version: VersionDocument = parse_document("ttm version", VERSION).expect("version");
    assert_eq!(version.json_version, 1);
    let history: WaveHistoryDocument =
        parse_document("ttm wave history 248", WAVE_HISTORY).expect("history");
    assert_eq!(history.state, "closed");
    assert_eq!(history.members[1].status_as_of, None);
    let check: WaveCheckDocument =
        parse_document("ttm wave check", WAVE_CHECK_FAILING).expect("wave check");
    assert!(!check.ok);
    assert_eq!(check.findings.len(), 1);
    let receipt: ReceiptDocument = parse_document("ttm record-run", RECORD_RUN).expect("receipt");
    assert_eq!(receipt.receipt_id, "run-17");
}

#[test]
fn a_document_of_another_format_breaks_the_contract() {
    let refused = parse_document::<CheckDocument>("ttm check", WAVE_CHECK_FAILING);
    assert!(
        matches!(refused, Err(Error::Contract { .. })),
        "{refused:?}"
    );
    let untagged = parse_document::<CheckDocument>("ttm check", r#"{"ok":true}"#);
    assert!(
        matches!(untagged, Err(Error::Contract { .. })),
        "{untagged:?}"
    );
}

#[test]
fn output_that_is_not_one_json_object_breaks_the_contract() {
    for stdout in [
        "",
        "warning: something\n{\"format\":\"ttm.version/1\"}",
        "[1, 2]",
    ] {
        let parsed = parse_document::<VersionDocument>("ttm version", stdout);
        assert!(
            matches!(parsed, Err(Error::Contract { .. })),
            "{stdout:?}: {parsed:?}"
        );
    }
}

#[test]
fn an_error_document_is_a_refusal_that_keeps_its_kind_and_candidates() {
    let missing =
        parse_document::<TicketDocument>("ttm show T-999", ERROR_NOT_FOUND).expect_err("refused");
    assert!(missing.is_not_found());
    match parse_document::<TicketDocument>("ttm show edb4e4e", ERROR_AMBIGUOUS) {
        Err(Error::Refused {
            kind, candidates, ..
        }) => {
            assert_eq!(kind, "ambiguous");
            assert_eq!(candidates.len(), 2);
        }
        other => panic!("expected an ambiguous refusal, got {other:?}"),
    }
}

#[test]
fn a_missing_binary_did_not_run_and_is_not_a_refusal() {
    let manager = TicketManager::new("/nonexistent/ticket-manager-binary", "reforger");
    let outcome = manager.version();
    assert!(matches!(outcome, Err(Error::NotRun { .. })), "{outcome:?}");
}

#[test]
fn every_call_names_the_project() {
    let manager = TicketManager::new("ttm", "reforger");
    assert_eq!(
        manager.text_command(&["wave", "collisions"]),
        ["ttm", "--project", "reforger", "wave", "collisions"]
    );
}
