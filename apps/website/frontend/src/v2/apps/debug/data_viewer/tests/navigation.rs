use super::*;
#[test]
fn equipment_viewer_dataset_switch_drops_dataset_specific_documents() {
    for tab in ["document", "selection"] {
        let current = Navigation::parse(&format!(
            "tab={tab}&document=publication_receipt&pointer=/excluded_diagnostic_dependencies&resource=scope&catalog_capability=attachment&section=relationships&kind=native_type_match&cursor=100"
        ));
        let changed = current.changed(&[("dataset", "diagnostic")]);
        assert_eq!(changed.tab(), "overview");
        assert_eq!(changed.section(), "data");
        for key in ["document", "pointer", "kind", "cursor"] {
            assert_eq!(changed.get(key), "");
        }
        assert_eq!(changed.get("resource"), "scope");
        assert_eq!(changed.get("catalog_capability"), "attachment");
    }
}
#[test]
fn equipment_viewer_dataset_identity_separates_catalog_and_inspection_requests() {
    let current = Navigation::parse(
        "dataset=diagnostic&generation=pinned&resource=scope&node=old&catalog_capability=attachment&q=scope",
    );
    assert!(
        current
            .catalog_request("pinned", false)
            .contains("dataset=diagnostic")
    );
    assert!(
        current
            .inspection_request("resource-cards", "pinned", &[])
            .contains("dataset=diagnostic")
    );
    let changed = current.changed(&[("dataset", "gameplay")]);
    assert_eq!(changed.generation(), "latest");
    assert_eq!(changed.get("node"), "");
    assert_eq!(changed.get("resource"), "scope");
    assert_eq!(changed.get("catalog_capability"), "attachment");
    assert!(
        changed
            .inspection_request("resource-cards", "new", &[])
            .contains("dataset=gameplay")
    );
}
#[test]
fn equipment_viewer_locations_preserve_opaque_identity_and_back_context() {
    let n = Navigation::parse(
        "resource=guid%3A1&node=parent%2Fnode%5B2%5D%23x&property=a%26b&generation=old&section=source",
    );
    assert_eq!(n.get("node"), "parent/node[2]#x");
    let href = n.href(&[("property", "different / name")]);
    assert_eq!(
        Navigation::parse(href.split_once('?').unwrap().1).get("property"),
        "different / name"
    );
    assert_eq!(n.generation(), "old");
    assert_eq!(
        Navigation::parse(n.resource_link("name:{exact}").split_once('?').unwrap().1).get("node"),
        ""
    );
    assert_eq!(n.get("property"), "a&b");
}
#[test]
fn equipment_viewer_anonymous_route_is_full_window() {
    assert!(crate::router::role_may_enter("/debug/data-viewer", None));
    assert!(crate::router::full_bleed("/debug/data-viewer"));
    assert!(crate::router::chromeless("/debug/data-viewer"));
}

#[test]
fn equipment_viewer_new_generation_resets_installations_but_preserves_resource() {
    let n = Navigation::parse("resource=guid%3A1&node=shared%2F2&property=mass&relation=2&field=3");
    let result = n.after_publication("a", "b").unwrap();
    let result = Navigation::parse(result.split_once('?').unwrap().1);
    assert_eq!(result.get("resource"), "guid:1");
    for key in ["node", "property", "relation", "field"] {
        assert_eq!(result.get(key), "");
    }
    assert!(
        n.changed(&[("generation", "a")])
            .after_publication("a", "b")
            .is_none()
    );
    assert!(n.after_publication("a", "a").is_none());
}

#[test]
fn equipment_viewer_source_navigation_clears_stale_value_ranges_and_search() {
    let n = Navigation::parse(
        "resource=a&node=one&property=old&pointer=%2F100&value_cursor=200&property_q=old",
    );
    let next = n.changed(&[("resource", "b"), ("node", "two"), ("property", "new")]);
    assert_eq!(next.get("node"), "two");
    assert_eq!(next.get("property"), "new");
    for key in ["pointer", "value_cursor", "property_q"] {
        assert_eq!(next.get(key), "");
    }
}

#[test]
fn equipment_viewer_catalog_filters_survive_resource_and_source_navigation() {
    let n = Navigation::parse(
        "tab=resources&capability=attachment&q=scope&domain=equipment&resource_cursor=50",
    );
    let a = Navigation::parse(n.resource_link("scope-a").split_once('?').unwrap().1);
    let b = Navigation::parse(
        a.href(&[("node", "opaque/container"), ("property", "FOV")])
            .split_once('?')
            .unwrap()
            .1,
    );
    for location in [a, b] {
        assert_eq!(location.get("catalog_capability"), "attachment");
        assert_eq!(location.get("q"), "scope");
        assert_eq!(location.get("resource_cursor"), "50");
        assert!(
            location
                .catalog_request("generation", false)
                .contains("capability=attachment")
        );
        assert!(
            !location
                .inspection_request("resource-cards", "generation", &[])
                .contains("capability")
        );
    }
}
