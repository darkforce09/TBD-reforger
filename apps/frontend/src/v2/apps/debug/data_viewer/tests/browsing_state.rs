use super::*;
#[test]
fn equipment_viewer_resource_memory_keeps_expansions_and_evicts_old_resources() {
    let mut m = ResourceMemory {
        scroll_top: 123.,
        ..Default::default()
    };
    m.expanded.insert(("opaque/id".into(), "Zero".into()));
    save("first".into(), m);
    assert_eq!(restore("first").scroll_top, 123.);
    assert!(restore("first")
        .expanded
        .contains(&("opaque/id".into(), "Zero".into())));
    for i in 0..8 {
        save(format!("next-{i}"), ResourceMemory::default());
    }
    assert_eq!(restore("first").scroll_top, 0.);
}
#[test]
fn equipment_viewer_catalog_position_is_scoped_to_its_query() {
    save_catalog_scroll("attachments".into(), 450);
    assert_eq!(catalog_scroll("attachments"), 450);
    assert_eq!(catalog_scroll("vehicles"), 0);
}
