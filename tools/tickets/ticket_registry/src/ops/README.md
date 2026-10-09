# Ticket operations

The typed mutations of the [ticket](/documentation/glossary/n_to_z.md#ticket) corpus: minting,
removing, ordering, status changes, readiness, shipping and the landing-commit stamp. Each
operation computes the whole corpus after the change, validates it, and only then replaces the
in-memory corpus; it never touches a file itself.

## Contents

```text
tools/tickets/ticket_registry/src/ops/
├── creation.rs     `add`, `add_child`: mint the next parent or child id, promoting work to program
├── fields.rs       `OpOutcome`, `VALID_STATUS_NAMES` and the field accessors both ticket kinds share
├── mod.rs          the module tree; re-exports the operations and `OpOutcome`
├── ordering.rs     `remove`, `reorder` and `advance_slice`, and the append order
├── readiness.rs    `mark_ready` and `default_plan_path`: the spec, plan, dependency and body gates
├── transitions.rs  `set_status`, `ship` and `stamp_sha`, and `commit`, which every operation ends in
└── validation.rs   `validate_post_image`: what a candidate corpus must pass before commit
```

## How it works

```text
caller (crate::verbs)                       ops::<operation>(&mut Corpus, …, now_utc)
  Corpus::load  ──────────────────────────►  validate the injected clock (RFC 3339 UTC)
                                             clone the tickets, apply the change
                                             commit: validate_post_image(pre, post, changed, made_live)
                                               ├─ refused: Err(text), corpus untouched
                                               └─ held: corpus = post, Ok(OpOutcome { changed, deleted })
  Corpus::write_back(changed) ◄────────────
  Corpus::delete_files(deleted)
```

`validate_post_image` re-renders and re-parses every changed ticket and requires a round trip to
the same value, then refuses what the operation would introduce: a work summary over the word cap
(unless `migration_legacy` is set), an empty or debt title, a missing `main_goal` on changed live
work, a changed parent that is past `idea` without an order, a new duplicate live order, a ticket made live without `owns` or with a component but no surface, a
child id not shaped `<parent>.<n>` on a changed program, and, across the whole corpus, a duplicate
or dangling `children` entry. Rules that the existing corpus already breaks bind only on what the
operation changes, so an operation never refuses because of a ticket it did not touch.

| Operation | Does |
|---|---|
| `add` | mints the highest parent number plus one as an `idea` work ticket with scope `repo`/`docs`, a class from `classify_work`, `created_at` stamped |
| `add_child` | mints the next dotted child under a program; a work parent refuses unless `promote` turns it into a program in the same operation |
| `remove` | deletes a work ticket and scrubs it from its program's `children`; a program only with `force`, which deletes every descendant |
| `reorder` | sets the order to the anchor's plus one and moves an `idea` to `queued` |
| `set_status` | builds the target `Status`, refusing what it lacks; `cancelled` stamps `completed_at`; `shipped`, `deferred` and `cancelled` give an order-less parent the append order |
| `mark_ready` | needs a spec and a plan file that exist, no open dependency, an order, and the ready-tier body fields |
| `ship` | needs `created_at` and the full work body, stamps `completed_at`, and clears the ticket as any program's `active` slice |
| `stamp_sha` | writes a 7–40 hex landing commit to a shipped ticket's `shipped_at`; the same commit again changes nothing, a different one refuses |

## Boundaries

- Depends on: `ticket_model` — `store::Corpus`, the model (`Status`, `Ticket`, the word caps and
  debt predicates) and `parse_ticket_toml` and `render_ticket_toml` for the round-trip check;
  `time_source::validate_rfc3339_utc` for the injected clock.
- Used by: the mutation commands in `tools/tickets/ticket_registry/src/verbs/` (`mutations.rs`,
  `readiness.rs`, `status.rs`, `shipping.rs`), which load the corpus, call one operation, write
  back the outcome and re-sync. No caller outside the crate calls an operation directly.
- Rules:
  - an operation never writes a corpus its own checks refuse, and a refusal leaves the corpus
    untouched;
  - the clock is injected, so the same inputs give the same corpus;
  - an existing collision is never refused on behalf of an operation that did not cause it;
  - no status change leaves `ticket check` red for lack of an order.

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket fields and statuses these operations
  change.
