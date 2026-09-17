/// T-807 — the Transform-tab copy debts: the stale DEM hint (F-23) and the coordinate unit suffix
/// (F-13). Both are about user-visible strings, so these pin on `live_source` (string literals
/// KEPT — `live_code` would blank the very copy under test and make the pin hollow, the T-759 class).
use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};

/// F-23 — DEM shipped, so the "Z is manual until terrain elevation (DEM) ships" hint is stale.
/// The old promise must be gone from the whole live source.
#[test]
fn stale_dem_manual_hint_is_gone() {
    let code = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    // Concat so this test's own literal cannot self-match.
    let stale = ["Z is manual until terrain elevation ", "(DEM) ships"].concat();
    assert!(
        !code.contains(&stale),
        "F-23: the stale 'Z is manual until DEM ships' hint must be replaced (DEM has shipped)"
    );
}

/// F-13 — the X/Y/Z coordinate fields carry a metre unit suffix (Rotation already carried `°`).
/// Pinned to `transform_tab`'s body so it is the coordinate fields, not a stray literal.
#[test]
fn coordinate_fields_suffix_metres() {
    let code = live_source(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/v2/apps/editor/ui/inspector/attributes_modal.rs"
    )));
    let body = only_body(&code, "fn transform_tab(");
    // Three coordinate fields, each `Some("m")`; Rotation keeps its own `Some("°")`.
    let m_suffix = body.matches("Some(\"m\")").count();
    assert!(
        m_suffix >= 3,
        "F-13: X/Y/Z must each pass Some(\"m\") as the unit suffix (found {m_suffix})"
    );
    // The bare `None` suffix on a coordinate field is exactly the pre-fix state.
    assert!(
        body.contains("Some(\"\u{b0}\")"),
        "F-13: Rotation must still carry its ° suffix"
    );
}
