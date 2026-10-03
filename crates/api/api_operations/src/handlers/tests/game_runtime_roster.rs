//! Source pins for the roster read: it reads the deployed artifact's bindings rather than
//! compiling, and it trims the `arma_id` seating key.

const SRC: &str = include_str!("../game_runtime_roster.rs");

/// Production source with the sibling-test declaration cut off the end.
fn production() -> &'static str {
    SRC.split("#[cfg(test)]")
        .next()
        .expect("game_runtime_roster.rs must declare its sibling tests")
}

/// The roster is the deployed artifact's slot bindings: it never compiles a version or pairs
/// slots at read time, so it cannot seat from a version other than the one the runtime loaded.
/// Capacity and document validation happen once, when the artifact compiles.
///
/// RED: reintroduce a compile or a pairing pass in the roster read.
#[test]
fn roster_reads_the_deployed_artifact_bindings_and_never_compiles() {
    let production = production();
    assert!(
        production.contains("FROM mission_deployment_slots b"),
        "the roster must read the deployment's slot bindings"
    );
    assert!(
        production.contains("deployment_in_effect("),
        "the roster must read the deployment the server runs"
    );
    for forbidden in [
        "flatten_to_mod_document",
        "parse_orbat_template",
        "load_cargo_phys_catalog",
    ] {
        assert!(
            !production.contains(forbidden),
            "the roster must not compile or pair at read time: found {forbidden}"
        );
    }
}

/// `event_roster` must filter AND emit via `btrim(u.arma_id)`.
///
/// RED: restore `SELECT … u.arma_id` + `u.arma_id <> ''` → the asserts fail.
#[test]
fn event_roster_btrims_arma_id_filter_and_select() {
    let handler = production()
        .split("pub async fn event_roster")
        .nth(1)
        .expect("event_roster")
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
