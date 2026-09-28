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

/// One faction read by id is the row the viewer's faction list carries, document included.
#[test]
fn faction_read_by_id() {
    const G: &str = golden!("GET__factions__00000000-0000-4000-b100-000000000001.json");
    assert_golden::<UserFaction>(G, &[]);
    let faction: UserFaction = serde_json::from_str(G).unwrap();
    let list: FactionListResponse = serde_json::from_str(golden!("GET__factions.json")).unwrap();
    assert!(
        list.data.iter().any(|row| row == &faction),
        "the faction read by id must equal its row in the list"
    );
}

/// A created and a revised faction; the server stamps their times and the created one's id.
#[test]
fn faction_created_and_revised() {
    const CREATED: &str = golden!("POST__factions.json");
    const REVISED: &str = golden!("PUT__factions__00000000-0000-4000-b100-000000000003.json");
    assert_golden::<UserFaction>(CREATED, &[]);
    assert_golden::<UserFaction>(REVISED, &[]);
    let revised: UserFaction = serde_json::from_str(REVISED).unwrap();
    assert_eq!(revised.id, "00000000-0000-4000-b100-000000000003");
    assert_ne!(revised.created_at, revised.updated_at);
}
