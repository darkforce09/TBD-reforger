//! Captured-response round trips for the asset registry and faction rosters.

use super::*;

#[test]
fn registry_envelope() {
    assert_golden::<RegistryResponse>(golden!("GET__registry.json"), &[]);
}

/// The compatibility response was a typed shape with no fixture behind it, so nothing checked its
/// edge shape, its cache tag or its modpack identity. This capture is truncated to two live rows so
/// the corpus stays small while every named edge field is still populated.
#[test]
fn registry_compat_envelope() {
    const G: &str = golden!("GET__registry__compat.json");
    assert_golden::<RegistryCompatResponse>(G, &[]);
    let body: RegistryCompatResponse = serde_json::from_str(G).unwrap();
    assert!(
        body.data.len() >= 2,
        "compat golden must carry ≥2 edges so qty/evidence/timestamps are exercised"
    );
    assert!(
        body.data.iter().all(|e| !e.id.is_empty()
            && !e.from_node.is_empty()
            && !e.to_node.is_empty()
            && !e.edge_type.is_empty()
            && e.qty >= 1),
        "each edge must round-trip populated named fields"
    );
    assert!(
        !body.etag.is_empty() && !body.modpack_id.is_empty() && !body.modpack_version.is_empty(),
        "cache-identity fields must be present"
    );
}

/// Typed as the faction manager reads it, which is not even the same envelope shape an untyped
/// page body would have asserted.
#[test]
fn factions_envelope() {
    assert_golden::<FactionListResponse>(golden!("GET__factions.json"), &[]);
}
