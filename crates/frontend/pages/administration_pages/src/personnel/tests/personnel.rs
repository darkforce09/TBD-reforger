//! Guards on the personnel roster: the row decoding, the roles resync answer, the sanction reason
//! and the roster passes.

use super::{
    BanReason, FilterMode, SortMode, apply_roster_filter, apply_roster_sort, classify_ban_reason,
    reason_confirm_enabled, roles_sync_updated_count,
};
use frontend_api_dtos::administration::AdminUserRow;
use frontend_api_dtos::role::Role;

fn row(
    discord_id: &str,
    username: &str,
    role: Role,
    is_banned: bool,
    warnings: i64,
) -> AdminUserRow {
    AdminUserRow {
        discord_id: discord_id.into(),
        username: username.into(),
        discord_handle: username.into(),
        arma_id: None,
        arma_character: String::new(),
        role,
        is_banned,
        warnings,
        total_deployments: 0,
    }
}

#[test]
fn admin_user_row_deserializes_total_deployments() {
    // The wire field must survive the roster-row decode; zero is a real count.
    let u: AdminUserRow = serde_json::from_str(
        r#"{"discord_id":"1","username":"u","discord_handle":"u","arma_id":null,"arma_character":"","role":"enlisted","is_banned":false,"warnings":0,"total_deployments":0}"#,
    )
    .expect("AdminUserRow with total_deployments=0");
    assert_eq!(u.total_deployments, 0);
    let u17: AdminUserRow = serde_json::from_str(
        r#"{"discord_id":"1","username":"u","discord_handle":"u","arma_id":null,"arma_character":"","role":"enlisted","is_banned":false,"warnings":0,"total_deployments":17}"#,
    )
    .expect("AdminUserRow with total_deployments=17");
    assert_eq!(u17.total_deployments, 17);
}

#[test]
fn roles_sync_updated_count_reads_integer() {
    let body = serde_json::json!({ "updated": 12 });
    assert_eq!(roles_sync_updated_count(&body), Ok(12));
}

#[test]
fn roles_sync_updated_count_rejects_missing_or_non_integer() {
    // A vacuous `{}` 2xx must not toast as success — that is how curl-only stayed invisible.
    assert!(roles_sync_updated_count(&serde_json::json!({})).is_err());
    assert!(roles_sync_updated_count(&serde_json::json!({ "updated": "12" })).is_err());
    assert!(roles_sync_updated_count(&serde_json::json!({ "updated": null })).is_err());
}

#[test]
fn cancel_aborts_and_sends_nothing() {
    // Dialog Cancel / dismiss → None. Abort carries no toast: the operator chose not to ban.
    assert_eq!(classify_ban_reason(None), BanReason::Abort);
}

#[test]
fn ok_with_blank_is_refused_before_any_request() {
    // This branch used to post an empty body, which the server correctly refuses with "reason
    // is required". Rejecting means the client never makes the request at all, and the dialog's
    // confirm is disabled while the box is empty.
    assert_eq!(classify_ban_reason(Some("")), BanReason::Reject);
    assert!(!reason_confirm_enabled(""));
}

#[test]
fn whitespace_only_is_refused_too() {
    // The server rejects a whitespace-only reason. Agreeing here keeps the operator from
    // seeing a 400 for input that looked non-empty in the field.
    for blank in ["   ", "\t", "\n", " \t\n "] {
        assert_eq!(
            classify_ban_reason(Some(blank)),
            BanReason::Reject,
            "{blank:?}"
        );
        assert!(!reason_confirm_enabled(blank), "{blank:?}");
    }
}

#[test]
fn a_real_reason_is_sent_trimmed() {
    // The server stores `reason.trim()`, so the client sends the same bytes it will store.
    assert_eq!(
        classify_ban_reason(Some("  Repeated TK after warning  ")),
        BanReason::Send("Repeated TK after warning".to_string())
    );
    assert_eq!(
        classify_ban_reason(Some("Repeated TK after warning")),
        BanReason::Send("Repeated TK after warning".to_string())
    );
    assert!(reason_confirm_enabled("  Repeated TK after warning  "));
}

#[test]
fn filter_active_and_banned() {
    let users = vec![
        row("1", "Alice", Role::Enlisted, false, 0),
        row("2", "Bob", Role::Enlisted, true, 1),
        row("3", "Cara", Role::Leader, false, 0),
    ];
    let active = apply_roster_filter(users.clone(), FilterMode::Active);
    assert_eq!(
        active
            .iter()
            .map(|u| u.username.as_str())
            .collect::<Vec<_>>(),
        vec!["Alice", "Cara"]
    );
    let banned = apply_roster_filter(users, FilterMode::Banned);
    assert_eq!(
        banned
            .iter()
            .map(|u| u.username.as_str())
            .collect::<Vec<_>>(),
        vec!["Bob"]
    );
}

#[test]
fn sort_warnings_desc_and_banned_first() {
    let users = vec![
        row("1", "Alice", Role::Enlisted, false, 0),
        row("2", "Bob", Role::Admin, true, 1),
        row("3", "Cara", Role::Leader, false, 5),
    ];
    let by_warn = apply_roster_sort(users.clone(), SortMode::WarningsDesc);
    assert_eq!(
        by_warn
            .iter()
            .map(|u| u.username.as_str())
            .collect::<Vec<_>>(),
        vec!["Cara", "Bob", "Alice"]
    );
    let banned_first = apply_roster_sort(users, SortMode::BannedFirst);
    assert_eq!(banned_first[0].username, "Bob");
    assert!(banned_first[0].is_banned);
}

#[test]
fn sort_and_filter_modes_cycle() {
    assert_eq!(SortMode::NameAsc.next(), SortMode::WarningsDesc);
    assert_eq!(SortMode::BannedFirst.next(), SortMode::NameAsc);
    assert_eq!(FilterMode::All.next(), FilterMode::Active);
    assert_eq!(FilterMode::Banned.next(), FilterMode::All);
}
