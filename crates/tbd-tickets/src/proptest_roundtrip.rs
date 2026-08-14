#![cfg(test)]

use crate::{RepoLayer, Scope, Status, Ticket, WorkTicket, parse_ticket_toml, render_ticket_toml};
use proptest::prelude::*;

/// Valid RFC 3339 UTC stamps under the T-913.1 rule (`Z` or `+00:00`, uppercase `T`).
/// Day capped at 28 so every generated date exists.
fn rfc3339_utc() -> impl Strategy<Value = String> {
    (
        2000i32..=2099,
        1u8..=12,
        1u8..=28,
        0u8..=23,
        0u8..=59,
        0u8..=59,
        prop::bool::ANY,
    )
        .prop_map(|(y, mo, d, h, mi, s, zulu)| {
            let suffix = if zulu { "Z" } else { "+00:00" };
            format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}{suffix}")
        })
}

proptest! {
    #[test]
    fn parse_render_work_queued_roundtrip(
        title in "[A-Za-z0-9 _-]{1,64}",
        summary in "[A-Za-z0-9 .,'_-]{0,200}",
        order in 1i64..50_000,
        created_at in prop::option::of(rfc3339_utc()),
        completed_at in prop::option::of(rfc3339_utc()),
    ) {
        let t = Ticket::Work(WorkTicket {
            id: "T-905".into(),
            title,
            summary,
            status: Status::Queued { order },
            executor: Some("claude-code".into()),
            notes: None,
            spec: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope: Scope::Repo {
                layers: vec![RepoLayer::Ci],
            },
            user_story: None,
            acceptance: vec![],
            shipped_at: None,
            priority: None,
            created_at,
            completed_at,
            owns: vec![],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        let back = parse_ticket_toml(&s).unwrap();
        prop_assert_eq!(t, back);
    }
}
