//! Attributes modal transform field copy tests.

use crate::v2::core::test_support::class_r_scrub::{live_source, only_body};

#[test]
fn stale_dem_manual_hint_is_gone() {
    let code = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let stale = ["Z is manual until terrain elevation ", "(DEM) ships"].concat();
    assert!(
        !code.contains(&stale),
        "F-23: the stale 'Z is manual until DEM ships' hint must be replaced (DEM has shipped)"
    );
}

#[test]
fn coordinate_fields_suffix_metres() {
    let code = live_source(super::ATTRIBUTES_MODAL_SOURCE);
    let body = only_body(&code, "fn transform_tab(");
    let m_suffix = body.matches("Some(\"m\")").count();
    assert!(
        m_suffix >= 3,
        "F-13: X/Y/Z must each pass Some(\"m\") as the unit suffix (found {m_suffix})"
    );
    assert!(
        body.contains("Some(\"\u{b0}\")"),
        "F-13: Rotation must still carry its ° suffix"
    );
}
