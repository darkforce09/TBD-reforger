//! Source guard over the `view!` markup of the doctrine pages that carry it: the wiki and the
//! vehicle index.
//!
//! The scanner lives in [`frontend_test_support::view_attribute_guard`]; this file points
//! it at the two page folders, located from this file so the scan follows the area wherever it
//! moves.

use frontend_test_support::repository_root::source_file_folder;
use frontend_test_support::view_attribute_guard::assert_view_attributes_are_well_formed;
use std::path::PathBuf;

/// The wiki page's folder: the parent of this `tests/` folder.
fn wiki_folder() -> PathBuf {
    let tests = source_file_folder(env!("CARGO_MANIFEST_DIR"), file!());
    tests
        .parent()
        .expect("the tests folder sits inside the wiki folder")
        .to_path_buf()
}

#[test]
fn view_attributes_in_the_wiki_page_are_braced_where_they_must_be() {
    assert_view_attributes_are_well_formed(env!("CARGO_MANIFEST_DIR"), &[wiki_folder()]);
}

#[test]
fn view_attributes_in_the_vehicles_page_are_braced_where_they_must_be() {
    let vehicles = wiki_folder()
        .parent()
        .expect("the wiki folder sits inside the doctrine area")
        .join("vehicles");
    assert_view_attributes_are_well_formed(env!("CARGO_MANIFEST_DIR"), &[vehicles]);
}
