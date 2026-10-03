use super::*;
use crate::VANILLA_REFERENCE;

/// Every folder lies inside the vanilla lane, so the lane's ignore and refusal rules cover it.
#[test]
fn every_vanilla_lane_folder_lies_inside_the_vanilla_lane() {
    for folder in [
        VANILLA_EXTRACTED_SCRIPTS,
        VANILLA_SCRIPT_API_PAGES,
        VANILLA_SOURCE_PAGES,
        VANILLA_RECONSTRUCTED_SOURCE,
    ] {
        assert!(
            folder.starts_with(&format!("{VANILLA_REFERENCE}/")),
            "{folder} is outside {VANILLA_REFERENCE}"
        );
    }
}
