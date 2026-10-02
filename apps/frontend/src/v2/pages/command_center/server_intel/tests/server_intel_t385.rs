//! The theatre readout must stay wired to the key the route sends.

use crate::v2::core::test_support::fixtures::golden;

/// The committed `/servers` capture must carry `terrain` from the match join. The primary row
/// (the one with a current match) pins `"everon"`; unmatched rows pin explicit JSON `null`, the
/// same encoding `status` uses.
///
/// Removing the key, or softening it to an optional field that is skipped when empty ahead of a
/// live value, makes this RED — which is the whole point.
#[test]
fn servers_golden_carries_terrain_from_match_join() {
    const GOLDEN: &str = golden!("GET__servers.json");
    let v: serde_json::Value = serde_json::from_str(GOLDEN).expect("golden parses");
    let rows = v["data"].as_array().expect("golden has a `data` array");
    assert!(!rows.is_empty(), "golden must carry rows to assert against");

    let primary = &rows[0];
    assert_eq!(
        primary.get("terrain").and_then(|t| t.as_str()),
        Some("everon"),
        "primary server (current_match_id → matches.terrain) must carry terrain \"everon\""
    );

    for (i, r) in rows.iter().enumerate() {
        assert!(
            r.as_object().expect("row object").contains_key("terrain"),
            "GET /servers row {i} must carry an explicit `terrain` key (string or null) — \
             T-385 Class-R. Do not drop it or hide it behind skip_serializing_if."
        );
    }

    // Unmatched rows: explicit null, not absent.
    assert!(
        rows[1].get("terrain").is_some_and(|t| t.is_null()),
        "secondary (no current_match_id) must serialize terrain as null"
    );
    assert!(
        rows[2].get("terrain").is_some_and(|t| t.is_null()),
        "staging (no status) must serialize terrain as null"
    );
}

/// The panel source must read `terrain` again — deleting the readout without removing the route
/// field recreates the hole where the wire proves nothing about the page.
#[test]
fn server_panel_reads_terrain_key() {
    let src = crate::v2::core::test_support::pins::server_intel_source();
    let production = src
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests");
    let panel = production
        .split("fn server_panel")
        .nth(1)
        .expect("server_panel fn")
        .split("\nfn ")
        .next()
        .expect("panel body");
    assert!(
        panel.contains("v_str(&s, \"terrain\")"),
        "server_panel must read the terrain key restored by T-385"
    );
    assert!(
        !panel.contains("Theater Unknown"),
        "do not restore the T-359 permanent placeholder inside server_panel"
    );
}
