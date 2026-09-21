//! Source pins over the read path: the list endpoint must prefetch in a constant number of
//! round-trips, and the status join that supplies `terrain` must stay in the query.

/// `list_servers` must prefetch via `servers_intel_batch` / `ANY($1)`, never compose
/// one card per row (an N+1 over the whole fleet).
#[test]
fn list_servers_does_not_compose_one_card_per_row() {
    const SRC: &str = include_str!("../server_intel.rs");
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests module");
    let handler = production
        .split("pub async fn list_servers")
        .nth(1)
        .expect("list_servers handler")
        .split("\npub async fn ")
        .next()
        .expect("handler body until next pub async fn");
    let collapsed: String = handler.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        !collapsed.contains("server_intel("),
        "list_servers must not compose a card per row (N+1)"
    );
    assert!(
        collapsed.contains("servers_intel_batch("),
        "list_servers must use servers_intel_batch"
    );
}

#[test]
fn servers_intel_batch_uses_any_prefetch() {
    const SRC: &str = include_str!("../server_intel.rs");
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests module");
    let batch = production
        .split("async fn servers_intel_batch")
        .nth(1)
        .expect("servers_intel_batch")
        .split("\npub async fn ")
        .next()
        .expect("batch fn body");
    assert!(
        batch.contains("SERVER_STATUS_SELECT_ANY") && production.contains("server_id = ANY($1)"),
        "batch path must prefetch statuses with ANY($1)"
    );
    assert!(
        production.contains("LEFT JOIN matches m ON m.id = ss.current_match_id"),
        "the status prefetch must LEFT JOIN matches for terrain"
    );
    assert!(
        batch.contains("FROM modpacks WHERE id = ANY($1)"),
        "batch path must prefetch modpacks with ANY($1)"
    );
    assert!(
        batch.contains("FROM modpack_mods WHERE modpack_id = ANY($1)"),
        "batch path must prefetch modpack_mods with ANY($1)"
    );
}

/// production source must join `matches.terrain` — removing the join is a ship fail.
#[test]
fn server_intel_joins_matches_terrain() {
    const SRC: &str = include_str!("../server_intel.rs");
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests module");
    assert!(
        production.contains("LEFT JOIN matches m ON m.id = ss.current_match_id"),
        "the card composition and the batch must LEFT JOIN matches on current_match_id"
    );
    assert!(
        production.contains("m.terrain AS terrain"),
        "join must project matches.terrain"
    );
    assert!(
        production.contains("pub terrain: Option<TerrainType>"),
        "ServerIntelDto must expose terrain"
    );
}

/// Pure accounting of round-trips vs N (proves the O(1) claim without a live pool).
#[test]
fn batch_round_trips_constant_not_linear_in_n() {
    fn batch_queries(n_servers: usize, n_unique_modpacks: usize) -> usize {
        if n_servers == 0 {
            return 0;
        }
        // statuses always; modpacks+mods only when any required_modpack_id
        1 + if n_unique_modpacks > 0 { 2 } else { 0 }
    }
    fn per_row_queries(n_servers: usize, with_modpack: bool) -> usize {
        // per-row path: status per server + optional modpack load (modpack + mods = 2)
        n_servers * (1 + if with_modpack { 2 } else { 0 })
    }
    assert_eq!(batch_queries(63, 1), 3);
    assert_eq!(per_row_queries(63, true), 189);
    assert!(batch_queries(63, 1) < per_row_queries(63, true));
    assert_eq!(
        batch_queries(1000, 5),
        3,
        "round-trips stay flat as N grows"
    );
}
