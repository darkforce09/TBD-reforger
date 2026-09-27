use super::*;

/// `list_users` must SELECT the live `users.total_deployments` column. A literal
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

/// Absent values serve page 1 of 20; a `per_page` above 100 is served as 100.
///
/// RED: drop the clamp — `per_page=500` reads 500 rows.
#[test]
fn personnel_page_window_defaults_and_clamps() {
    assert_eq!(page_window(None, None).expect("defaults"), (1, 20));
    assert_eq!(page_window(Some(3), Some(50)).expect("explicit"), (3, 50));
    assert_eq!(page_window(Some(1), Some(100)).expect("ceiling"), (1, 100));
    assert_eq!(page_window(Some(2), Some(101)).expect("clamped"), (2, 100));
    assert_eq!(
        page_window(Some(1), Some(i64::MAX)).expect("clamped"),
        (1, 100)
    );
}

/// A `page` or `per_page` below 1 answers 400 instead of falling back to a default.
///
/// RED: filter the values to the default the way offset paging does — `page=0` answers 200.
#[test]
fn personnel_page_window_refuses_values_below_one() {
    for (page, per_page) in [
        (Some(0), None),
        (Some(-1), None),
        (None, Some(0)),
        (None, Some(-20)),
        (Some(i64::MIN), Some(i64::MIN)),
    ] {
        let error = page_window(page, per_page).expect_err("below one");
        assert_eq!(
            error.status,
            axum::http::StatusCode::BAD_REQUEST,
            "{page:?} {per_page:?}"
        );
    }
}

/// The offset of a page is `(page - 1) * per_page`, saturating for pages too far out to address.
///
/// RED: compute `page * per_page` — page 1 skips its own rows.
#[test]
fn personnel_page_offset_starts_at_zero_and_saturates() {
    assert_eq!(page_offset(1, 20), 0);
    assert_eq!(page_offset(2, 20), 20);
    assert_eq!(page_offset(5, 100), 400);
    assert_eq!(page_offset(i64::MAX, 100), i64::MAX);
}

/// The search pattern matches the text literally: `%`, `_` and `\` are escaped.
///
/// RED: bind `%{q}%` unescaped — `q=_` matches every member with a non-empty name.
#[test]
fn personnel_search_pattern_matches_literally() {
    assert_eq!(contains_pattern("Target Z"), "%Target Z%");
    assert_eq!(contains_pattern("50%_off"), "%50\\%\\_off%");
    assert_eq!(contains_pattern("a\\b"), "%a\\\\b%");
}

fn decode(uri: &str) -> Result<PersonnelRosterQuery, ApiError> {
    roster_query(Query::try_from_uri(&uri.parse().expect("uri")))
}

/// A query value that is not an integer answers 400 in the `{error, details?}` envelope, not
/// axum's plain-text rejection.
///
/// RED: take `Query<PersonnelRosterQuery>` directly — the rejection body is plain text.
#[tokio::test]
async fn personnel_query_that_does_not_decode_answers_the_error_envelope() {
    use axum::response::IntoResponse;
    for uri in [
        "/admin/users?page=abc",
        "/admin/users?per_page=1.5",
        "/admin/users?page=",
        "/admin/users?page=99999999999999999999",
    ] {
        let error = decode(uri).expect_err(uri);
        assert_eq!(error.status, axum::http::StatusCode::BAD_REQUEST, "{uri}");
        let response = error.into_response();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let envelope: serde_json::Value = serde_json::from_slice(&body).expect("json envelope");
        let keys: Vec<&str> = envelope
            .as_object()
            .expect("object")
            .keys()
            .map(String::as_str)
            .collect();
        assert!(
            envelope["error"].is_string() && keys.iter().all(|k| matches!(*k, "error" | "details")),
            "{uri}: {envelope}"
        );
    }
}

/// A well-formed query decodes every value; absent values stay absent for the defaults.
#[test]
fn personnel_query_decodes_search_and_paging() {
    let query = decode("/admin/users?q=Target%20Z&page=2&per_page=5").expect("decodes");
    assert_eq!(query.q.as_deref(), Some("Target Z"));
    assert_eq!((query.page, query.per_page), (Some(2), Some(5)));
    let empty = decode("/admin/users").expect("decodes");
    assert_eq!((empty.q, empty.page, empty.per_page), (None, None, None));
}
