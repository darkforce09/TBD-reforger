use super::registry_session;
use crate::core::dto::RegistryItem;
use crate::editor::arsenal::arsenal_rules::{CompatFeed, CompatStatus};
use std::collections::HashMap;

fn sample_item(resource_name: &str) -> RegistryItem {
    RegistryItem {
        id: "00000000-0000-0000-0000-000000000001".to_string(),
        modpack_id: "test".to_string(),
        resource_name: resource_name.to_string(),
        display_name: resource_name.to_string(),
        category: "NATO/Rifleman".to_string(),
        icon_url: None,
        kind: "character".to_string(),
        r#abstract: None,
        arsenal_type: None,
        weight_kg: None,
        volume_cm3: None,
        max_weight_kg: None,
        max_volume_cm3: None,
        cargo_grid_w: None,
        cargo_grid_h: None,
        addon: None,
        variant_of: None,
        sort_order: 0,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

/// Cold session → both network paths are still required (first open pays once).
#[test]
fn cold_session_must_fetch_both() {
    registry_session::clear_for_test();
    assert!(
        registry_session::must_fetch_registry(),
        "cold session must still GET /registry once"
    );
    assert!(
        registry_session::must_fetch_compat(),
        "cold session must still GET /registry/compat once"
    );
}

/// After a successful fetch is stored, a remount must NOT plan another network round-trip
/// — this is the load-bearing T-245 contract (no re-pay on every editor open).
#[test]
fn warm_session_skips_both_unpaginated_fetches() {
    registry_session::clear_for_test();
    let items = vec![sample_item("Prefab.Character.Test")];
    registry_session::store_registry(items.clone());
    let feed = CompatFeed {
        status: CompatStatus::Ready,
        graph: Default::default(),
    };
    registry_session::store_compat(feed, HashMap::new());

    assert!(
        !registry_session::must_fetch_registry(),
        "warm session must skip GET /registry"
    );
    assert!(
        !registry_session::must_fetch_compat(),
        "warm session must skip GET /registry/compat"
    );
    let hit = registry_session::cached_registry().expect("registry session hit");
    assert_eq!(hit.len(), 1);
    assert_eq!(hit[0].resource_name, "Prefab.Character.Test");
    let (feed_hit, _) = registry_session::cached_compat().expect("compat session hit");
    assert!(
        matches!(feed_hit.status, CompatStatus::Ready),
        "cached compat feed must stay Ready"
    );
}

/// Mount source must consult the session gate before calling the cold fetch helpers.
/// Guards against a future "helpful" revert to the always-spawn_local dual fetch.
#[test]
fn mount_source_gates_unpaginated_fetches_on_session_cache() {
    let src = include_str!("../mission_editor.rs");
    assert!(
        src.contains("registry_session::must_fetch_registry()"),
        "mount path must gate GET /registry on must_fetch_registry()"
    );
    assert!(
        src.contains("registry_session::must_fetch_compat()"),
        "mount path must gate GET /registry/compat on must_fetch_compat()"
    );
    assert!(
        src.contains("registry_session::store_registry"),
        "successful /registry response must populate the session cache"
    );
    assert!(
        src.contains("registry_session::store_compat"),
        "successful /registry/compat response must populate the session cache"
    );
}
