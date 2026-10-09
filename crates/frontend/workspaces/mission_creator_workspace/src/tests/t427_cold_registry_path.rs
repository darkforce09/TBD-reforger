/// DTO: paginated envelope + cargo_defaults view round-trip.
#[test]
fn dto_paginated_registry_and_cargo_defaults_round_trip() {
    let page = serde_json::json!({
        "data": [],
        "etag": "W/\"x\"",
        "modpack_id": "00000000-0000-0000-0000-000000000001",
        "modpack_version": "1",
        "total": 1857,
        "limit": 500,
        "offset": 0
    });
    let r: frontend_api_dtos::RegistryResponse = serde_json::from_value(page).unwrap();
    assert_eq!(r.total, Some(1857));
    assert_eq!(r.limit, Some(500));
    assert_eq!(r.offset, Some(0));

    let cargo = serde_json::json!({
        "view": "cargo_defaults",
        "data": {
            "char_a": [{"container": "vest", "item": "mag", "qty": 2}]
        },
        "etag": "W/\"y\"",
        "modpack_id": "00000000-0000-0000-0000-000000000001",
        "modpack_version": "1",
        "source_edge_count": 16223
    });
    let c: frontend_api_dtos::RegistryCargoDefaultsResponse =
        serde_json::from_value(cargo).unwrap();
    assert_eq!(c.view, "cargo_defaults");
    assert_eq!(c.source_edge_count, Some(16223));
    let rows = c.data.get("char_a").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].container, "vest");
    assert_eq!(rows[0].qty, 2);
    // Slim proof: aggregated row count << raw edge count advertised by the server.
    assert!(
        (rows.len() as i64) < c.source_edge_count.unwrap(),
        "cargo_defaults view must be smaller than the raw edge walk"
    );
}
