//! Guards on the personnel roster: the route helpers, the roster passes and the sanction paths.

use super::{
    admin_user_ban_path, admin_user_warnings_path, apply_roster_filter, apply_roster_sort,
    classify_ban_reason, reason_confirm_enabled, roles_sync_success_message,
    roles_sync_updated_count, BanReason, FilterMode, SortMode, ADMIN_ROLES_SYNC_PATH,
};
use crate::v2::core::api::dto::AdminUserRow;
use crate::v2::core::auth::Role;

fn production_src() -> String {
    crate::v2::core::test_support::pins::personnel_source()
}

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
fn dossier_deployments_binds_total_deployments_not_em_dash() {
    // The dossier used to print a fixed dash here while the column existed. A conditional
    // that substituted a dash for zero still contained the binding needle and went green, so
    // this requires the exact call with no conditional, and forbids any dash between the label
    // and the next sibling.
    let production = production_src();

    // Exact live bind — assembled so this test's own source cannot satisfy it.
    let exact = format!(
        "{}{}",
        r#"stat("Deployments", "#, "u.total_deployments.to_string())"
    );
    assert!(
        production.contains(&exact),
        "dossier Deployments must be exactly `{exact}` (no conditional / no em-dash)"
    );

    let start = production
        .find(r#"stat("Deployments""#)
        .expect("Deployments stat call present");
    let region = &production[start..];
    let end = region
        .find("Current Rank")
        .expect("Current Rank sibling after Deployments");
    let deployments_region = &region[..end];
    assert!(
        !deployments_region.contains('—'),
        "Deployments/stat region must not contain em-dash (zero is a real count; \
         conditional zero→— is a fail)"
    );
    assert!(
        !deployments_region.contains("total_deployments == 0"),
        "Deployments/stat region must not conditionalize on total_deployments == 0"
    );
}

#[test]
fn admin_roles_sync_path_matches_live_api_route() {
    // This screen is the only caller of the route, so the check reads the live router source:
    // a constant compared against itself would stay green while examining nothing.
    let src = crate::v2::core::test_support::fixtures::api_route_source();
    let src: &str = &src;
    let live_registration = format!(r#".route("{ADMIN_ROLES_SYNC_PATH}""#);
    assert!(
        src.contains(&live_registration),
        "the api_v2 domain route tables must register {live_registration}, …); \
         Personnel posts ADMIN_ROLES_SYNC_PATH"
    );
    assert_eq!(ADMIN_ROLES_SYNC_PATH, "/admin/roles/sync");
}

#[test]
fn admin_ban_and_warnings_paths_match_live_api_routes() {
    // The path helpers must track the router, and a constant compared against itself would
    // stay green forever. These registrations span lines, so the path string is matched.
    let src = crate::v2::core::test_support::fixtures::api_route_source();
    let src: &str = &src;
    assert!(
        src.contains(r#""/admin/users/{discordId}/ban""#),
        "the api_v2 domain route tables must register ban/unban on /admin/users/{{discordId}}/ban"
    );
    assert!(
        src.contains(r#""/admin/users/{discordId}/warnings""#),
        "the api_v2 domain route tables must register warnings on /admin/users/{{discordId}}/warnings"
    );
    assert!(
        src.contains("unban_user"),
        "the api_v2 ban route must wire DELETE to unban_user"
    );
    assert_eq!(admin_user_ban_path("42"), "/admin/users/42/ban");
    assert_eq!(admin_user_warnings_path("42"), "/admin/users/42/warnings");
}

#[test]
fn issue_warning_is_not_a_mock_toast() {
    // Issuing a warning once reported a fake success while the endpoint behind it worked. The
    // phrases are assembled at runtime so the scan cannot go green off this test's own literals.
    let production = production_src();
    let mock_toast = format!("{}{}", "Warning issued ", "(mock)");
    let real_toast = format!("{}{}", "toasts.success(", r#""Warning issued")"#);
    assert!(
        !production.contains(&mock_toast),
        "Issue Warning must not toast a mock success (perturbation: reintroduce the mock toast)"
    );
    assert!(
        production.contains("admin_user_warnings_path")
            && production.contains(&format!("{}{}", "api_post_ok", "(store, &path, body)")),
        "Issue Warning must POST via admin_user_warnings_path + api_post_ok"
    );
    assert!(
        production.contains("personnel-warn-reason")
            && production.contains("personnel-warn-confirm"),
        "warning reason must be collected via a driveable Dialog (T-342)"
    );
    assert!(
        production.contains(&real_toast),
        "success toast after a real POST must say Warning issued"
    );
}

#[test]
fn ban_and_warn_use_dialog_not_window_prompt() {
    // The browser's own prompt cannot be driven by the automated gate, so the dialog and its
    // test identifiers are the path that has to exist. Only live call sites are pinned; the
    // documentation may still name the retired one.
    let production = production_src();
    assert!(
        !production.contains("prompt_with_message")
            && !production.contains("win.prompt")
            && !production.contains("Prompt::"),
        "personnel must not call web_sys prompt APIs"
    );
    for needle in [
        "data-testid=\"personnel-ban-reason\"",
        "data-testid=\"personnel-ban-confirm\"",
        "data-testid=\"personnel-warn-reason\"",
        "data-testid=\"personnel-warn-confirm\"",
        "reason_confirm_enabled",
    ] {
        assert!(
            production.contains(needle),
            "missing Dialog drive surface: {needle}"
        );
    }
    assert!(
        production.contains("<Dialog")
            && production.contains("title=\"Ban personnel?\"")
            && production.contains("title=\"Issue warning?\""),
        "ban and warn must each open a Dialog"
    );
}

#[test]
fn role_patch_surfaces_api_error_message() {
    // The role editor used to discard the refusal body and show a flat sentence instead.
    let production = production_src();
    let flat = format!("{}{}", r#"toasts.error("Failed to update role")"#, "");
    assert!(
        !production.contains(&flat),
        "role PATCH must not toast a flat Failed to update role (discarded server message)"
    );
    assert!(
        production.contains("api_error_message") && production.contains("Failed to update role"),
        "role PATCH Err must use api_error_message(..., \"Failed to update role\")"
    );
}

#[test]
fn unban_control_deletes_ban_when_banned() {
    // Bans were once irreversible from this screen. The needles are assembled at runtime so the
    // scan cannot go green off this assertion's own string literals.
    let production = production_src();
    let delete_call = format!("{}{}", "api_delete", "(store, &path)");
    let unban_label = format!("{}{}", "Unban ", "Personnel");
    let unban_testid = format!("{}{}", "personnel-", "unban");
    assert!(
        production.contains(&delete_call),
        "Unban must DELETE via api_delete (perturbation: drop api_delete call)"
    );
    assert!(
        production.contains("admin_user_ban_path") && production.contains(&unban_label),
        "banned dossier must expose Unban Personnel on the ban path"
    );
    assert!(
        production.contains(&unban_testid),
        "unban control needs a stable testid"
    );
}

#[test]
fn sort_and_filter_are_not_toast_stubs() {
    let production = production_src();
    // Assembled at runtime so the assertion's own literals cannot satisfy the scan.
    let sort_stub = format!("{}{}", "Sort options ", "coming soon");
    let filter_stub = format!("{}{}", "Filter options ", "coming soon");
    assert!(
        !production.contains(&sort_stub) && !production.contains(&filter_stub),
        "Sort/Filter must not toast stub copy"
    );
    assert!(
        production.contains("apply_roster_sort") && production.contains("apply_roster_filter"),
        "header Sort/Filter must drive apply_roster_sort / apply_roster_filter"
    );
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
fn roles_sync_success_message_names_the_count() {
    assert_eq!(
        roles_sync_success_message(0),
        "Discord roles resynced (0 user(s) updated)"
    );
    assert_eq!(
        roles_sync_success_message(3),
        "Discord roles resynced (3 user(s) updated)"
    );
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
