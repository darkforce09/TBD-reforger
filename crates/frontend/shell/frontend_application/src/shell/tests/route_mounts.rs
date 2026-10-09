//! The app's route table mounts the pages on the paths the route registry declares.
//!
//! The route table (`app_routes.rs`) is the app's own file, so the checks that a page is mounted
//! live with the app rather than with the page.

/// The ballistics catalogs page is mounted on the path its registry entry names.
#[test]
fn the_ballistics_catalogs_page_is_mounted_on_its_route() {
    let routes = include_str!("../../app_routes.rs");
    assert!(
        routes.contains(r#"path!("/admin/ballistics-catalogs") view=BallisticsCatalogsPage"#),
        "app_routes.rs mounts the page"
    );
}
