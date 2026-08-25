/// Source guard: cold open pages registry with limit+offset and never calls the bare dump.
#[test]
fn cold_registry_uses_paginated_path_not_unbounded_dump() {
    let src = include_str!("../mission_editor.rs");
    assert!(
        src.contains("fetch_registry_pages"),
        "cold registry must go through the paginated helper"
    );
    assert!(
        src.contains("/registry?limit={REGISTRY_COLD_PAGE}&offset={offset}"),
        "cold registry URL must carry limit+offset"
    );
    // Bare dump path as an api_get literal — the only remaining "/registry" forms are
    // query-bearing (`?limit=` / `?view=` / `?edge_type=`).
    assert!(
        !src.contains("api_get(auth, \"/registry\")"),
        "must not api_get bare registry dump"
    );
}

/// Source guard: cold compat uses filtered Arsenal edges + cargo_defaults view.
#[test]
fn cold_compat_uses_filtered_edges_and_cargo_defaults_view() {
    let src = include_str!("../mission_editor.rs");
    assert!(
        src.contains("fetch_compat_cold"),
        "cold compat must go through the narrow helper"
    );
    assert!(
        src.contains("optic_on_weapon,mag_in_weapon,attachment_on_weapon"),
        "Arsenal edge_type filter must be pinned"
    );
    assert!(
        src.contains("/registry/compat?view=cargo_defaults"),
        "cargo seeds must come from the aggregated view"
    );
    let walk_fn = format!("{}{}", "cargo_defaults_by_character", "(&");
    assert!(
        !src.contains(&walk_fn),
        "client must not walk raw cargo edges on cold open"
    );
    assert!(
        !src.contains("api_get(auth, \"/registry/compat\")"),
        "must not api_get bare compat dump"
    );
}

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
    let r: crate::core::dto::RegistryResponse = serde_json::from_value(page).unwrap();
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
    let c: crate::core::dto::RegistryCargoDefaultsResponse = serde_json::from_value(cargo).unwrap();
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
