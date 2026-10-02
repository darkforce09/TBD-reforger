//! Edge-family parsing and the cargo-defaults aggregation the slim view serves.

use chrono::{TimeZone, Utc};
use uuid::Uuid;

use super::*;
use crate::missions::models::registry::RegistryCompatEdge;

fn edge(from: &str, to: &str, evidence: &str, qty: i32) -> RegistryCompatEdge {
    RegistryCompatEdge {
        id: Uuid::nil(),
        modpack_id: Uuid::nil(),
        from_node: from.into(),
        to_node: to.into(),
        edge_type: "character_default_cargo".into(),
        evidence: evidence.into(),
        qty,
        created_at: Utc.timestamp_opt(0, 0).unwrap(),
        updated_at: Utc.timestamp_opt(0, 0).unwrap(),
    }
}

#[test]
fn edge_types_split_trim_dedupe() {
    assert!(parse_edge_types(None).is_empty());
    assert_eq!(
        parse_edge_types(Some("optic_on_weapon, mag_in_weapon,optic_on_weapon")),
        vec!["optic_on_weapon".to_string(), "mag_in_weapon".to_string()]
    );
}

#[test]
fn cargo_aggregate_sums_qty_and_maps_containers() {
    let edges = vec![
        edge("mag_a", "char_1", "TargetStorage=Vest/Slot", 2),
        edge("mag_a", "char_1", "TargetStorage=Vest/Slot", 1),
        edge("bandage", "char_1", "TargetStorage=Pants/Pockets", 1),
        edge("ignored", "char_1", "TargetStorage=Helmet/X", 1),
        // Wrong family must not appear.
        RegistryCompatEdge {
            edge_type: "mag_in_weapon".into(),
            ..edge("x", "y", "TargetStorage=Vest/Slot", 1)
        },
    ];
    let map = aggregate_cargo_defaults(&edges);
    let char_1 = map.get("char_1").unwrap().as_array().unwrap();
    assert_eq!(char_1.len(), 2, "helmet evidence skipped; vest+pants kept");
    // BTreeMap order: (container, item) — pants before vest.
    assert_eq!(char_1[0]["container"], "pants");
    assert_eq!(char_1[0]["item"], "bandage");
    assert_eq!(char_1[0]["qty"], 1);
    assert_eq!(char_1[1]["container"], "vest");
    assert_eq!(char_1[1]["item"], "mag_a");
    assert_eq!(char_1[1]["qty"], 3);
}

#[test]
fn cargo_aggregate_is_strictly_smaller_than_raw_edge_walk_input() {
    // Shape pin: N raw cargo edges → fewer aggregated rows (duplicates collapse).
    // Proves the slim view cannot re-expand to the full dump.
    let mut edges = Vec::new();
    for i in 0..100 {
        edges.push(edge(
            &format!("item_{}", i % 10),
            &format!("char_{}", i % 5),
            "TargetStorage=Backpack/Slot",
            1,
        ));
    }
    let map = aggregate_cargo_defaults(&edges);
    let rows: usize = map
        .values()
        .map(|v| v.as_array().map(|a| a.len()).unwrap_or(0))
        .sum();
    assert!(rows < edges.len(), "aggregation must collapse duplicates");
    assert_eq!(map.len(), 5);
    // CRT: each char only pairs with items sharing the same mod-5 residue → 2 items/char.
    assert_eq!(rows, 10);
}
