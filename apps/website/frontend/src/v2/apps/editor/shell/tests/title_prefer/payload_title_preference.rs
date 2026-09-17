//! Title Prefer t505 tests tests.

use website_map_engine::editing::persist::server_adoption::{
    payload_title_nonblank, prefer_payload_title,
};

/// The prefer helper must keep the authored title when the row is stale.
///
/// RED: make `prefer_payload_title` always return `row_title.trim()`, or drop the prefer.
#[test]
fn prefer_payload_keeps_authored_over_stale_row() {
    let payload = r#"{"title":"  Authored Bridgehead  ","editor":{}}"#;
    assert_eq!(
        prefer_payload_title(payload, "Stale Library Title"),
        "Authored Bridgehead"
    );
    assert_eq!(
        prefer_payload_title(r#"{"title":"   "}"#, "  Row Title  "),
        "Row Title"
    );
    assert_eq!(
        prefer_payload_title(r#"{"editor":{}}"#, "Row Only"),
        "Row Only"
    );
}

#[test]
fn payload_title_nonblank_trim() {
    assert_eq!(
        payload_title_nonblank(r#"{"title":"  Authored  "}"#).as_deref(),
        Some("Authored")
    );
    assert_eq!(payload_title_nonblank(r#"{"title":"  "}"#), None);
    assert_eq!(payload_title_nonblank(r#"{"editor":{}}"#), None);
}

/// The adopt must route the title through the prefer helper rather than writing the row's.
///
/// RED: pass `&row.title` straight into `apply_row_meta`, or drop `prefer_payload_title` /
/// `payload_title_nonblank` from the adopt body.
///
/// Superseded in strength by `t570_tests`, which observes the title `apply_row_meta` actually
/// received: this one still greps, so it is kept only as a fast, readable first failure.
#[test]
fn adopt_payload_wires_prefer_helper() {
    const SRC: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../map-engine/src/editing/persist/server_adoption.rs"
    ));
    let production = SRC.split("#[cfg(test)]").next().unwrap_or(SRC);
    let adopt = production
        .split("fn adopt_payload(")
        .nth(1)
        .and_then(|s| s.split("\n}\n").next())
        .expect("adopt_payload body");
    assert!(
            adopt.contains("prefer_payload_title(")
                || adopt.contains("payload_title_nonblank(payload_json)"),
            "adopt_payload must prefer via prefer_payload_title / payload_title_nonblank; got:\n{adopt}"
        );
    assert!(
            !adopt.contains("&row.title,"),
            "adopt_payload must not pass &row.title straight into apply_row_meta (stomp); got:\n{adopt}"
        );
}
