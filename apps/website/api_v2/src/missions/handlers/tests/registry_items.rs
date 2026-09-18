//! Bounds behaviour of the shared registry page clamp.

use super::*;

#[test]
fn page_bounds_clamp_and_default() {
    assert_eq!(registry_page_bounds(None, Some(10)), None);
    assert_eq!(registry_page_bounds(Some(500), Some(0)), Some((500, 0)));
    assert_eq!(
        registry_page_bounds(Some(9999), Some(-1)),
        Some((REGISTRY_PAGE_MAX, 0))
    );
    assert_eq!(
        registry_page_bounds(Some(0), None),
        Some((REGISTRY_PAGE_DEFAULT, 0))
    );
}
