//! Source pins for the roster read: the compile gate it must go through, and the trimming of
//! the `arma_id` seating key.

const SRC: &str = include_str!("../roster_ingest.rs");

/// Production source with the sibling-test declaration cut off the end.
fn production() -> &'static str {
    SRC.split("#[cfg(test)]")
        .next()
        .expect("roster_ingest.rs must declare its sibling tests")
}

/// The live roster must load the Save phys catalog and compile through the catalogued gate.
/// The empty-catalog `flatten_to_mod_document` seats over-capacity versions.
///
/// RED: swap back to the no-arg flatten, or drop the catalog load.
#[test]
fn roster_loads_cargo_phys_catalog() {
    let production = production();
    let start = production
        .find("pub async fn ingest_event_roster(")
        .expect("ingest_event_roster must exist");
    let after = &production[start..];
    let body = after
        .split("\n/// Phys attrs for the cargo-capacity walk")
        .next()
        .expect("ingest_event_roster must precede CargoPhysRow");
    assert!(
        body.contains("load_cargo_phys_catalog"),
        "roster must load registry phys into the catalog; got:\n{body}"
    );
    assert!(
        body.contains("flatten_to_mod_document_with_catalog("),
        "roster must call the catalogued compile gate; got:\n{body}"
    );
    // Isolate so a with_catalog import alone cannot false-green a no-arg call.
    let stripped = body.replace("flatten_to_mod_document_with_catalog", "");
    assert!(
        !stripped.contains("flatten_to_mod_document("),
        "roster must not call the empty-catalog no-arg flatten; got:\n{body}"
    );
}

/// `ingest_event_roster` must filter AND emit via `btrim(u.arma_id)`.
///
/// RED: restore `SELECT … u.arma_id` + `u.arma_id <> ''` → the asserts fail.
#[test]
fn ingest_event_roster_btrims_arma_id_filter_and_select() {
    let handler = production()
        .split("pub async fn ingest_event_roster")
        .nth(1)
        .expect("ingest_event_roster")
        .split("\npub async fn ")
        .next()
        .expect("handler body until next pub async fn");
    let collapsed: String = handler.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        collapsed.contains("btrim(u.arma_id)"),
        "roster must btrim(u.arma_id) — whitespace-only rows must not seat"
    );
    assert!(
        collapsed.contains("btrim(u.arma_id) <> ''"),
        "roster WHERE must use btrim nonempty, not raw <> ''"
    );
    assert!(
        !collapsed.contains("SELECT os.squad, os.slot_index, u.arma_id"),
        "roster must SELECT btrim(u.arma_id), not raw u.arma_id (emit seating key trimmed)"
    );
    assert!(
        !collapsed.contains("AND u.arma_id <> ''"),
        "roster must not keep the untrimmed <> '' guard"
    );
}
