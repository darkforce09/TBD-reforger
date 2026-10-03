//! The basemap mode's report spellings and mode codes are fixed.

use super::BasemapMode;

#[test]
fn the_report_spells_each_mode() {
    assert_eq!(BasemapMode::Unified.as_str(), "unified");
    assert_eq!(BasemapMode::Pyramid.as_str(), "pyramid");
    assert_eq!(BasemapMode::Single.as_str(), "single-bitmap");
    assert_eq!(BasemapMode::Hillshade.as_str(), "hillshade");
}

#[test]
fn the_mode_codes_name_each_mode_and_unknown_codes_are_unified() {
    assert_eq!(BasemapMode::from_u32(0), BasemapMode::Unified);
    assert_eq!(BasemapMode::from_u32(1), BasemapMode::Pyramid);
    assert_eq!(BasemapMode::from_u32(2), BasemapMode::Single);
    assert_eq!(BasemapMode::from_u32(3), BasemapMode::Hillshade);
    assert_eq!(BasemapMode::from_u32(4), BasemapMode::Unified);
    assert_eq!(BasemapMode::from_u32(u32::MAX), BasemapMode::Unified);
}
