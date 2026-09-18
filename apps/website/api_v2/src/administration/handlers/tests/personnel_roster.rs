/// Class-R — `list_users` must SELECT the live `users.total_deployments` column. A literal
/// `0::bigint AS total_deployments` alias would false-green the SPA bind while never reading the
/// denormalized counter.
#[test]
fn list_users_selects_bare_total_deployments_column() {
    const SRC: &str = include_str!("../personnel_roster.rs");
    let production = SRC
        .split("#[cfg(test)]")
        .next()
        .expect("production source before tests module");

    // Forbidden fake: literal-zero alias.
    assert!(
        !production.contains("0::bigint AS total_deployments"),
        "list_users must not fake total_deployments with 0::bigint AS total_deployments"
    );
    assert!(
        !production.contains("0 AS total_deployments"),
        "list_users must not fake total_deployments with 0 AS total_deployments"
    );

    // Required: bare column between the warnings subquery alias and FROM users.
    // Collapse escaped newlines from the QueryBuilder string so the pin is stable.
    let collapsed: String = production
        .chars()
        .filter(|c| *c != '\\')
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        collapsed.contains("AS warnings, total_deployments FROM users"),
        "list_users SELECT must project bare total_deployments (not a literal alias) \
         immediately before FROM users"
    );
}
