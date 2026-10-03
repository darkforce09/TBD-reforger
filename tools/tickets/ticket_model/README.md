# Ticket model

The `ticket_model` crate: the typed [ticket](/documentation/glossary/n_to_z.md#ticket), its
canonical TOML encoding, the corpus store over `.ai/tickets/T-*.toml`, the scope vocabulary, the
repository paths only the ticket domain names, and the commit-subject miner. Every other ticket
crate, xtask and the [ticketboard](/documentation/glossary/n_to_z.md#ticketboard) read and write
tickets through it.

## Contents

```text
tools/tickets/ticket_model/
├── Cargo.toml  the `ticket_model` library package: `repository_layout`, `time_source`, `process_runner`, `newtype_ids`, layout tier 2
├── src/        the model, the ticket id, the encoding, the store, the vocabulary, the paths and the miner
└── tests/      the compile-fail test that keeps the scope `Domain` enum closed
```

## How it works

```text
.ai/tickets/scope-vocab.toml ──► ScopeVocab::load ─┐
.ai/tickets/T-*.toml ──► parse_ticket_toml ────────┴──► Corpus::load ──► Corpus { tickets: BTreeMap<TicketId, Ticket> }
                          (TicketFile ──► Ticket)                               │
                                                     caller changes tickets ◄───┘
Corpus::write_back(ids) ──► render_ticket_toml ──► re-parse, must equal ──► .<id>.toml.tmp ──► rename over <id>.toml
git log ──► commit_subjects::mine_subjects ──► BTreeMap<TicketId, Vec<SubjectCommit>>
```

A ticket is a `ProgramTicket`, which groups dotted child tickets, or a `WorkTicket`, which carries
a `[scope]` table and the typed body fields. `Status` carries the fields each status requires, so
a `ready` ticket without a spec, a `main_goal` or acceptance criteria cannot be built. A
`TicketId` holds every ticket, child and parent id; it serialises as its bare string, so the
ticket files, `queue.json` and `wave.lock` carry the plain id.

`TicketFile` is the file shape: the status flattened into `status` and `order`, every key of
either kind, in the canonical key order. A parse checks shape and values; `Corpus::load` adds
the scope check against the vocabulary and refuses the whole load on the first bad file, naming
it. `Corpus::write_back` writes only the ids it is given, each through a re-parse check and an
atomic rename, and `Corpus::delete_files` deletes only the ids it is given. `src/README.md`
describes each module.

## Getting started

Run from the repository root:

```bash
cargo test -p ticket_model   # unit tests and the compile-fail test; some read the live .ai/tickets/ tree and git history
cargo xtask ticket check     # loads the whole corpus through Corpus::load and runs the registry checks
```

The store and miner tests load the checkout's own `.ai/tickets/` and run `git log`, so they run
inside a checkout with its history.

## Configuration

No feature and no environment variable. The crate reads `.ai/tickets/T-*.toml` and
`.ai/tickets/scope-vocab.toml` under the checkout root it is given, and runs `git log` in that
checkout.

## Public surface

- At the crate root: the model (`Ticket`, `ProgramTicket`, `WorkTicket`, `Status`, `StatusName`,
  `ScopeV2`, `Domain`), `TicketId`, the encoding (`TicketFile`, `parse_ticket_toml`,
  `render_ticket_toml`), `Corpus`, `ScopeVocab`, the value sets and word caps (`CLASS_VALUES`,
  `ESTIMATED_VALUES`, `SUMMARY_WORD_CAP`, `BODY_LINE_WORD_CAP`, `CITATION_WORD_CAP`,
  `TITLE_WORD_CAP`), the debt pins (`TITLE_DEBT_PIN`, `MAIN_GOAL_DEBT_PIN`), the predicates
  (`is_sha_shaped`, `title_is_debt`, `main_goal_is_debt`, `empty_ready_tier_fields`,
  `classify_work`), `Error`, `Result` and `error_chain_text`.
- `store`: `Corpus` and `ticket_id_order_key`, the one ordering of ticket ids.
- `vocab`: `ScopeVocab`.
- `repository`: `handoff_doc`, `SPARSE_CHECKOUT_SETS` and the `documentation` paths.
- `commit_subjects`: `mine_subjects`, `subject_ids`, `to_utc_z` and `SubjectCommit`.
- `prelude`: the model types, `TicketId`, `Corpus`, `ScopeVocab` and the encoding functions.

## Boundaries

- Depends on: `repository_layout` (the registry paths and the documentation root),
  `time_source` (the RFC 3339 UTC check), `process_runner` (`git log`), `newtype_ids` (the
  `TicketId` declaration), `serde`, `toml` with `preserve_order`, `time`, `regex` and
  `thiserror`.
- Used by: `ticket_metrics`, `ticket_wave_lock` and `ticket_registry` in `tools/tickets/`;
  `xtask` (`TicketId`, `StatusName`, `error_chain_text`); the ticketboard in `apps/ticketboard/`
  (the model types, `parse_ticket_toml`, `CLASS_VALUES`, `ESTIMATED_VALUES`).
- Rules: tier 2 of `tools/tickets`, depending only on foundation crates
  (`ticket_crates_depend_only_on_foundations_and_lower_ticket_crates` in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`; `cargo xtask verify crate-tiers`);
  every live ticket file renders back byte for byte and loads fail-closed (`src/tests/store/`);
  `Domain` stays closed (`tests/trybuild.rs`); `src/repository.rs` is the crate's only file
  that spells a repository path (`only_a_layout_module_spells_a_repository_path` in
  `tools/checks/repository_checks/src/tests/tooling_prose_rules.rs`).

## Related documentation

- [Ticket crates](/tools/tickets/README.md) — the four ticket crates and their tiers.
- [Ticket registry](/.ai/tickets/README.md) — the ticket files, statuses and commands this crate
  models.
- [Ticket crates documentation](/documentation/tools/tickets/README.md) — the measurements and
  reasons behind the ticket crates' constants.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — how the tooling crates
  fit together.
