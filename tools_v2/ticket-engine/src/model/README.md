# Ticket model

The typed [ticket](/documentation_v2/glossary.md#ticket): the two ticket kinds, the status with the
fields each status requires, the four-level scope, and the field limits and predicates that the
operations and `ticket check` share, so each rule has one definition.

## Contents

```text
tools_v2/ticket-engine/src/model/
├── mod.rs      the module tree; re-exports every item, as the crate root does again
├── scope.rs    `Domain`, `ScopeV2`, the value sets, word caps and debt predicates
├── status.rs   `StatusName`, the eight statuses, and `Status`, which carries each status's fields
├── tests/      unit tests for ready statuses, title debt, class triage and spellings
└── tickets.rs  `Ticket`, either a `ProgramTicket` or a `WorkTicket`, with their fields
```

## How it works

A `Ticket` is a `ProgramTicket`, which groups dotted child tickets through `children` and an
`active` slice and carries no scope, or a `WorkTicket`, which carries a `parent`, a `ScopeV2` and
the typed body fields (`context`, `requirement`, `current_state`, `approach`, `verify`,
`acceptance`, `citations`).

`Status` makes illegal combinations unrepresentable: `idea` carries nothing; `queued` requires an
`order`; `ready`, `running` and `review` require an order, a `spec`, a `main_goal` and a non-empty
`acceptance` (`Status::live_ready` builds them); `shipped` carries an optional `shipped_at`, and
`shipped`, `deferred` and `cancelled` an optional order. `StatusName::is_live` is true for
`queued`, `ready`, `running` and `review`.

`ScopeV2` is the flat `[scope]` table: exactly one `domain` from the closed `Domain` enum
(`website`, `mod`, `schema`, `engine`, `repo`), one `layer`, an optional `component` and a
`surface` list. Only the domain is compiled; layers, components and surfaces are words checked
against `.ai/tickets/scope-vocab.toml` when the corpus loads (`crate::vocab`).

`scope.rs` also holds the shared rules:

| Item | Rule |
|---|---|
| `CLASS_VALUES` | `bug`, `feature`, `chore`, `audit`, `docs` |
| `ESTIMATED_VALUES` | the fields an `estimated` list may name: `created_at`, `completed_at`, `shipped_at`, `tokens`, `scope` |
| `is_sha_shaped` | 7 to 40 lowercase hex characters, the shape of `shipped_at` and of estimate commits |
| `SUMMARY_WORD_CAP`, `BODY_LINE_WORD_CAP`, `CITATION_WORD_CAP`, `TITLE_WORD_CAP` | 40, 30, 8 and 10 words, counted with `split_whitespace` |
| `title_is_debt`, `main_goal_is_debt` with `TITLE_DEBT_PIN` and `MAIN_GOAL_DEBT_PIN` | a title equal to the id or over the cap, a live work ticket without `main_goal`; both pins are 0 |
| `empty_ready_tier_fields` | the body fields a ready or later work ticket lacks |
| `classify_work` | a class from title and summary words, `bug` before `audit`, `docs`, `chore`, `feature` |

## Boundaries

- Depends on: `serde` only.
- Used by: the rest of the crate (`encoding`, `store`, `ops`, `validation`, `wave_lock`,
  `metrics`); through the crate root, `apps/ticketboard/` (`StatusName`, `Status`, `Ticket`,
  `ScopeV2`), whose tests hold its class list equal to `CLASS_VALUES`.
- Rules:
  - a ready-class status cannot be built without its fields (`ready_constructor_rejects_empty_goal`);
  - `Domain` stays closed and holds no `frontend`, which is a layer
    (`tools_v2/ticket-engine/tests/fail/mod_frontend.rs`, a compile-fail test); the crate denies
    wildcard matches on its enums (`#![deny(clippy::wildcard_enum_match_arm)]` in
    `tools_v2/ticket-engine/src/lib.rs`), so a new variant breaks every match that ignores it;
  - class triage matches whole words, never substrings (`classify_work_is_token_boundary_and_ordered`).
