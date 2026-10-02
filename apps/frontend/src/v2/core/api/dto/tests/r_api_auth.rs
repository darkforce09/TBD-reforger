//! Captured-response round trips for the session, identity and personnel payloads.

use super::*;
use crate::v2::core::api::dto::administration::PersonnelPage;

// ── strong-typed bodies (every field asserted) ──
#[test]
fn me() {
    assert_golden::<MeResponse>(golden!("GET__me.json"), &[]);
}

#[test]
fn link_status() {
    assert_golden::<LinkStatus>(golden!("GET__me__link__status.json"), &[]);
}

/// The roster page: every row key is a named field, and `arma_id` stays an explicit `null` for a
/// member who has not linked an Arma identity. The capture covers both arms of that field.
#[test]
fn admin_users_page() {
    const G: &str = golden!("GET__admin__users.json");
    assert_golden::<PersonnelPage>(G, &[]);
    let page: PersonnelPage = serde_json::from_str(G).unwrap();
    assert_eq!((page.page, page.per_page), (1, 20));
    assert_eq!(page.total, page.items.len() as i64);
    assert!(
        page.items.iter().any(|row| row.arma_id.is_none())
            && page.items.iter().any(|row| row.arma_id.is_some()),
        "the roster golden must hold a linked and an unlinked member"
    );
}

/// The member list the assignee picker reads. The capture deliberately includes both shapes of the
/// avatar field — one row omits it, another carries a URL — so the present and absent cases are
/// both covered.
#[test]
fn members_envelope() {
    const G: &str = golden!("GET__members.json");
    assert_golden::<MemberSearchPage>(G, &[]);
    let env: MemberSearchPage = serde_json::from_str(G).unwrap();
    assert!(
        env.data.len() >= 2,
        "members golden must cover more than one row"
    );
    assert_eq!(
        env.total,
        env.data.len() as i64,
        "the seeded directory fits one page, so the total counts exactly the rows"
    );
    assert!(
        env.data.iter().any(|m| m.avatar_url.is_some()),
        "at least one row must carry avatar_url (present case)"
    );
    assert!(
        env.data.iter().any(|m| m.avatar_url.is_none()),
        "at least one row must omit avatar_url (absent case)"
    );
    assert!(
        env.data
            .iter()
            .all(|m| !m.discord_id.is_empty() && !m.username.is_empty()),
        "discord_id and username must round-trip populated"
    );
}

// ── session and identity writes ──
// Their answers carry values the server generates per request; the goldens hold the fixed
// placeholders of the API's golden normalisation table at those fields.

/// The rotated pair as the session store decodes it. `token_type` is claimed: the read requires
/// `Bearer`, and the write emits it again.
#[test]
fn session_refresh() {
    const G: &str = golden!("POST__auth__refresh.json");
    assert_golden::<crate::v2::core::auth::session::RefreshResponse>(G, &[]);
    let pair: crate::v2::core::auth::session::RefreshResponse = serde_json::from_str(G).unwrap();
    assert!(!pair.access_token.is_empty() && !pair.refresh_token.is_empty());
}

/// A pair whose token type is anything but `Bearer`, or that carries none, fails the read.
#[test]
fn session_refresh_refuses_a_token_type_other_than_bearer() {
    let mut pair: Value = serde_json::from_str(golden!("POST__auth__refresh.json")).unwrap();
    pair["token_type"] = json!("Basic");
    let refused =
        serde_json::from_value::<crate::v2::core::auth::session::RefreshResponse>(pair.clone());
    assert!(refused.is_err(), "a Basic token type must fail the read");
    pair.as_object_mut().unwrap().remove("token_type");
    let refused = serde_json::from_value::<crate::v2::core::auth::session::RefreshResponse>(pair);
    assert!(
        refused.is_err(),
        "a pair without its token type must fail the read"
    );
}

#[test]
fn link_code_issued() {
    const G: &str = golden!("POST__me__link.json");
    assert_golden::<LinkCodeResponse>(G, &[]);
    let issued: LinkCodeResponse = serde_json::from_str(G).unwrap();
    assert!(issued.expires_at.is_some(), "an issued code always expires");
}
