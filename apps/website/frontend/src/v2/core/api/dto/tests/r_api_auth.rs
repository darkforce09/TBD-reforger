//! Captured-response round trips for the session, identity and personnel payloads.

use super::*;

// ── strong-typed bodies (every field asserted) ──
#[test]
fn me() {
    assert_golden::<MeResponse>(golden!("GET__me.json"), &[]);
}

#[test]
fn link_status() {
    assert_golden::<LinkStatus>(golden!("GET__me__link__status.json"), &[]);
}

#[test]
fn admin_users_envelope() {
    assert_golden::<Paginated<AdminUserRow>>(golden!("GET__admin__users.json"), &[]);
}

/// The member list the assignee picker reads. The capture deliberately includes both shapes of the
/// avatar field — one row omits it, another carries a URL — so the present and absent cases are
/// both covered.
#[test]
fn members_envelope() {
    const G: &str = golden!("GET__members.json");
    assert_golden::<DataEnvelope<Member>>(G, &[]);
    let env: DataEnvelope<Member> = serde_json::from_str(G).unwrap();
    assert!(
        env.data.len() >= 2,
        "members golden must cover more than one row"
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
