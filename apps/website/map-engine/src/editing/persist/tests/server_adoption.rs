//! Role: what an adopt writes into the document, and what it refuses to write.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: documents built in the test body; nothing shared between tests.
//! Invariants: the row's fields reach the document on the path that actually runs, the payload's
//! own title survives a stale row, a blank row is never written, and the host's post-edit tail runs
//! exactly once per adopt that touched a document.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use super::*;
use crate::data::store::MissionDocCore;

fn handle() -> DocHandle {
    Rc::new(RefCell::new(Some(MissionDocCore::new())))
}

fn seeded_handle() -> DocHandle {
    let doc = handle();
    {
        let guard = doc.borrow();
        let core = guard.as_ref().expect("a document");
        core.set_origin_init(true);
        core.seed_random(4, 12_800.0, 12_800.0, 7);
        core.set_origin_init(false);
    }
    doc
}

fn full_row() -> RowMeta {
    RowMeta {
        title: "Row Title".to_string(),
        terrain: "everon".to_string(),
        time_of_day: "06:00:00".to_string(),
        weather: "Clear".to_string(),
        briefing: "The library blurb".to_string(),
    }
}

fn payload_with_title(title: &str) -> String {
    format!(r#"{{"title":"{title}","editor":{{"slots":[]}}}}"#)
}

/// The document's metadata, as the store projects it.
fn meta(doc: &DocHandle) -> serde_json::Value {
    let guard = doc.borrow();
    let core = guard.as_ref().expect("a document");
    serde_json::from_str::<serde_json::Value>(&core.small_maps_json())
        .expect("the small maps are JSON")["meta"]
        .clone()
}

#[test]
fn a_blank_row_carries_nothing_worth_writing() {
    assert!(RowMeta::default().is_empty());
    assert!(
        RowMeta {
            time_of_day: "06:00:00".to_string(),
            weather: "Clear".to_string(),
            ..RowMeta::default()
        }
        .is_empty(),
        "an enumeration with a default is not content; writing it alone would replace authored \
         fields with defaults"
    );
    assert!(!full_row().is_empty());
    assert!(
        !RowMeta {
            title: "x".to_string(),
            ..RowMeta::default()
        }
        .is_empty()
    );
    assert!(
        !RowMeta {
            terrain: "everon".to_string(),
            ..RowMeta::default()
        }
        .is_empty()
    );
    assert!(
        !RowMeta {
            briefing: "x".to_string(),
            ..RowMeta::default()
        }
        .is_empty()
    );
}

#[test]
fn the_payload_title_wins_over_a_stale_row_title() {
    assert_eq!(
        prefer_payload_title(r#"{"title":"  Authored Bridgehead  "}"#, "Stale Row Title"),
        "Authored Bridgehead"
    );
    assert_eq!(prefer_payload_title(r#"{"title":"   "}"#, "  Row  "), "Row");
    assert_eq!(
        prefer_payload_title(r#"{"editor":{}}"#, "Row Only"),
        "Row Only"
    );
    assert_eq!(
        prefer_payload_title("not json at all", "Row Only"),
        "Row Only"
    );
}

#[test]
fn a_whitespace_only_payload_title_is_no_title() {
    assert_eq!(
        payload_title_nonblank(r#"{"title":"  Authored  "}"#).as_deref(),
        Some("Authored")
    );
    assert_eq!(payload_title_nonblank(r#"{"title":"  "}"#), None);
    assert_eq!(payload_title_nonblank(r#"{"editor":{}}"#), None);
    assert_eq!(payload_title_nonblank(r#"{"title":42}"#), None);
}

/// The wire the row travels: every field the row carries must arrive in the document, and the title
/// must be the payload's rather than the row's.
#[test]
fn an_adopt_writes_the_whole_row_and_prefers_the_payload_title() {
    let doc = handle();
    let ran = Cell::new(0_u32);
    adopt_payload(
        &doc,
        &payload_with_title("Authored Name"),
        &full_row(),
        Adopt::Init,
        &|| ran.set(ran.get() + 1),
    );
    let m = meta(&doc);
    assert_eq!(m["title"], "Authored Name");
    assert_eq!(m["terrain"], "everon");
    assert_eq!(m["environment"]["time"], "06:00:00");
    assert_eq!(m["environment"]["weather"], "Clear");
    assert_eq!(m["briefing"], "The library blurb");
    assert_eq!(ran.get(), 1, "the host's tail runs exactly once");
}

/// A row field left empty is an ABSENT field: it must not blank what the document already holds.
#[test]
fn an_empty_row_field_does_not_blank_the_document() {
    let doc = handle();
    adopt_payload(
        &doc,
        &payload_with_title("Authored Name"),
        &full_row(),
        Adopt::Init,
        &|| (),
    );
    adopt_payload(
        &doc,
        &payload_with_title("Authored Name"),
        &RowMeta {
            title: "Row Title".to_string(),
            terrain: "everon".to_string(),
            briefing: "Still a blurb".to_string(),
            ..RowMeta::default()
        },
        Adopt::Init,
        &|| (),
    );
    let m = meta(&doc);
    assert_eq!(
        m["environment"]["time"], "06:00:00",
        "an unset row column must leave the document's value alone"
    );
    assert_eq!(m["environment"]["weather"], "Clear");
    assert_eq!(m["briefing"], "Still a blurb");
}

/// A blank row is not written at all — but the payload still replaces the document, and the host's
/// tail still runs.
#[test]
fn a_blank_row_leaves_the_metadata_alone_and_still_runs_the_tail() {
    let doc = handle();
    adopt_payload(
        &doc,
        &payload_with_title("Authored Name"),
        &full_row(),
        Adopt::Init,
        &|| (),
    );
    let ran = Cell::new(0_u32);
    adopt_payload(
        &doc,
        r#"{"editor":{"slots":[]}}"#,
        &RowMeta::default(),
        Adopt::Undoable,
        &|| ran.set(ran.get() + 1),
    );
    let m = meta(&doc);
    assert_eq!(m["terrain"], "everon");
    assert_eq!(m["briefing"], "The library blurb");
    assert_eq!(ran.get(), 1);
}

/// No document to adopt into means nothing happened — including the tail, which would otherwise
/// rebind and re-arm over a document that is not there.
#[test]
fn an_absent_document_takes_no_adopt_and_runs_no_tail() {
    let doc: DocHandle = Rc::new(RefCell::new(None));
    let ran = Cell::new(0_u32);
    adopt_payload(
        &doc,
        &payload_with_title("Authored Name"),
        &full_row(),
        Adopt::Init,
        &|| ran.set(ran.get() + 1),
    );
    assert_eq!(ran.get(), 0);
}

/// The row-only path: a mission whose only server truth is its row. The row arrives whole, and the
/// document's own rows are left exactly where they were.
#[test]
fn the_row_only_path_writes_the_row_and_hydrates_nothing() {
    let doc = seeded_handle();
    apply_row_meta_only(&doc, &full_row());
    let m = meta(&doc);
    assert_eq!(m["title"], "Row Title");
    assert_eq!(m["terrain"], "everon");
    assert_eq!(m["environment"]["time"], "06:00:00");
    assert_eq!(m["briefing"], "The library blurb");
    assert_eq!(
        doc.borrow().as_ref().map(MissionDocCore::slot_count),
        Some(4),
        "the row path must not touch the slots"
    );
}

#[test]
fn the_row_only_path_writes_nothing_for_a_blank_row() {
    let doc = handle();
    apply_row_meta_only(&doc, &full_row());
    apply_row_meta_only(&doc, &RowMeta::default());
    assert_eq!(meta(&doc)["title"], "Row Title");
}

/// An adopt over a cold document must not become an undo step; one over live work must.
#[test]
fn the_mode_alone_decides_whether_an_adopt_can_be_taken_back() {
    let cold = seeded_handle();
    adopt_payload(
        &cold,
        r#"{"editor":{"slots":[]}}"#,
        &full_row(),
        Adopt::Init,
        &|| (),
    );
    assert_eq!(
        cold.borrow().as_ref().map(MissionDocCore::slot_count),
        Some(0),
        "the adopt replaced the document"
    );
    assert!(
        !cold.borrow().as_ref().expect("a document").can_undo(),
        "an initialization adopt leaves nothing on the undo stack"
    );

    let warm = seeded_handle();
    adopt_payload(
        &warm,
        r#"{"editor":{"slots":[]}}"#,
        &RowMeta::default(),
        Adopt::Undoable,
        &|| (),
    );
    assert!(
        warm.borrow().as_ref().expect("a document").can_undo(),
        "a conflict adopt is one undo step"
    );
}
