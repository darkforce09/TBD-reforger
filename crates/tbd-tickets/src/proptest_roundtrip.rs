#![cfg(test)]

use crate::{RepoLayer, Scope, Status, Ticket, WorkTicket, parse_ticket_toml, render_ticket_toml};
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_render_work_queued_roundtrip(
        title in "[A-Za-z0-9 _-]{1,64}",
        summary in "[A-Za-z0-9 .,'_-]{0,200}",
        order in 1i64..50_000,
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
            owns: vec![],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        let back = parse_ticket_toml(&s).unwrap();
        prop_assert_eq!(t, back);
    }
}
