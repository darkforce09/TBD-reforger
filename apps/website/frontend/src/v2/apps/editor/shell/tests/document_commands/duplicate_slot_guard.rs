//! Document Commands t946 86 duplicate guard tests.

use super::duplicate_slot_id_report;
use crate::v2::core::test_support::class_r_scrub::live_code;

fn live() -> String {
    live_code(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/shell/document_commands.rs"
    )))
}

/// **The guard is CALLED, and before the POST.** `duplicate_slot_ids` shipped in wave 255
/// exported and consumed by nothing but its own test, while `save_now` cleared findings,
/// compiled and POSTed with no duplicate check at all.
///
/// The ORDER is the pin, not just the presence: a check that runs after `api_post` has already
/// been spawned refuses nothing — it merely reports on a save that is already in flight.
#[test]
fn save_now_checks_duplicates_before_it_compiles_or_posts() {
    let src = live();
    let at_save = src
        .find("pub fn save_now(")
        .expect("T-946.86 (.85): save_now must survive");
    let body = &src[at_save..];
    let at_check = body
        .find("live_duplicate_slot_ids()")
        .expect("T-946.86 (.85): save_now must ask for the duplicate slot ids");
    let at_compile = body
        .find("compile_payload(")
        .expect("save_now compiles a payload");
    let at_post = body.find("api_post::").expect("save_now POSTs the version");
    assert!(
        at_check < at_compile && at_check < at_post,
        "T-946.86 (.85): the duplicate check must precede the compile AND the POST — a check \
             after either spends a round trip to learn what was knowable locally"
    );
    assert!(
        body[at_check..at_compile].contains("return"),
        "T-946.86 (.85): a document with duplicate slot ids must be REFUSED, not merely \
             annotated on the way to the server"
    );
}

/// The guard reads the LIVE DOC through `duplicate_slot_ids`, not a second private twin. The
/// upload path already has one of those (`check_duplicate_slot_ids_in_payload` in
/// `library/mission_library.rs`) and the two disagree; a third would make it worse.
#[test]
fn the_check_routes_through_the_shared_operation() {
    let src = live();
    assert!(
        src.contains("slot_ids::duplicate_slot_ids(core)"),
        "T-946.86 (.85): the doc-side guard must call the shared `duplicate_slot_ids`, not a \
             locally re-implemented scan"
    );
}

/// The refusal NAMES the callsign and the id. "This mission has duplicate slot ids" is not
/// actionable; "squad 1-1 lists slot s1 twice" is — and naming both is what the ticket's
/// acceptance asks for.
#[test]
fn the_refusal_names_the_callsign_and_the_id() {
    let (head, rows) = duplicate_slot_id_report(&[
        ("1-1".to_string(), "s1".to_string()),
        ("2-4".to_string(), "s9".to_string()),
    ]);
    assert!(
        head.contains("refused") && head.contains('2'),
        "the headline must say the save was refused and how many problems there are: {head}"
    );
    assert_eq!(rows.len(), 2, "one line per duplicate");
    assert!(
        rows[0].contains("1-1") && rows[0].contains("s1"),
        "PERTURB: a line naming only one of the two is not actionable: {}",
        rows[0]
    );
    assert!(
        rows[1].contains("2-4") && rows[1].contains("s9"),
        "every pair gets its own line: {}",
        rows[1]
    );
}

/// One duplicate reads as singular. A refusal that says "1 duplicate slot ids" is the kind of
/// seam that makes an operator distrust the rest of the message.
#[test]
fn the_headline_pluralizes() {
    let (head, _) = duplicate_slot_id_report(&[("1-1".to_string(), "s1".to_string())]);
    assert!(head.contains("1 duplicate slot id"), "{head}");
    assert!(!head.contains("slot ids"), "{head}");
}
