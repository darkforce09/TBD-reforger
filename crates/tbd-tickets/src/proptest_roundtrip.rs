#![cfg(test)]

use crate::{Domain, ScopeV2, Status, Ticket, WorkTicket, parse_ticket_toml, render_ticket_toml};
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

/// v2 flat scopes: always a legal SHAPE (surface only ever rides a component); vocab
/// LEGALITY is out of scope here — the round-trip property is parse/render, which is
/// deliberately shape-strict (see `parse_ticket_toml`).
fn scope_v2() -> impl Strategy<Value = ScopeV2> {
    (
        prop::sample::select(vec![
            Domain::Website,
            Domain::Mod,
            Domain::Schema,
            Domain::Engine,
            Domain::Repo,
        ]),
        "[a-z_]{2,16}",
        prop::option::of("[a-z_]{2,16}"),
        prop::collection::vec("[a-z_]{2,16}", 0..3),
    )
        .prop_map(|(domain, layer, component, surface)| {
            let surface = if component.is_some() { surface } else { vec![] };
            ScopeV2 {
                domain,
                layer,
                component,
                surface,
            }
        })
}

fn class_value() -> impl Strategy<Value = Option<String>> {
    prop::option::of(prop::sample::select(crate::CLASS_VALUES.to_vec()).prop_map(str::to_string))
}

fn line_list() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec("[A-Za-z0-9 .,'_-]{1,40}", 0..3)
}

proptest! {
    #[test]
    fn parse_render_work_queued_roundtrip(
        title in "[A-Za-z0-9 _-]{1,64}",
        summary in "[A-Za-z0-9 .,'_-]{0,200}",
        order in 1i64..50_000,
        created_at in prop::option::of(rfc3339_utc()),
        completed_at in prop::option::of(rfc3339_utc()),
        scope in scope_v2(),
        class in class_value(),
        context in line_list(),
        requirement in line_list(),
        current_state in line_list(),
        approach in line_list(),
        verify in line_list(),
        citations in line_list(),
        estimated_scope in prop::bool::ANY,
        estimate_note in prop::option::of("[A-Za-z0-9 _-]{1,40}"),
        migration_legacy in line_list(),
    ) {
        let t = Ticket::Work(WorkTicket {
            id: "T-905".into(),
            title,
            summary,
            class,
            status: Status::Queued { order },
            executor: Some("claude-code".into()),
            notes: None,
            spec: None,
            plan: None,
            depends_on: vec![],
            unblocks: vec![],
            parent: None,
            scope,
            main_goal: None,
            context,
            requirement,
            current_state,
            approach,
            verify,
            acceptance: vec![],
            citations,
            shipped_at: None,
            priority: None,
            created_at,
            completed_at,
            estimated: if estimated_scope { vec!["scope".into()] } else { vec![] },
            estimate_note,
            migration_legacy,
            owns: vec![],
            pack_last: None,
        });
        let s = render_ticket_toml(&t).unwrap();
        let back = parse_ticket_toml(&s).unwrap();
        prop_assert_eq!(t, back);
    }
}
