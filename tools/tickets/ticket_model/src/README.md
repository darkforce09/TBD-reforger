# Ticket model source

The typed ticket and the files that hold it: the model and its field rules, the ticket id, the
canonical TOML encoding, the corpus store over `.ai/tickets/T-*.toml`, the scope vocabulary, the
ticket domain's repository paths and the commit-subject miner.

## Contents

```text
tools/tickets/ticket_model/src/
├── commit_subjects.rs  `mine_subjects`, `subject_ids`, `to_utc_z`: which commits name each ticket id, and when they landed
├── encoding.rs         `TicketFile`, the on-disk shape, with `parse_ticket_toml` and `render_ticket_toml`
├── error.rs            `Error` and `Result` of the commit-subject miner, and the crate-private context helper
├── error_chain.rs      `error_chain_text`: an error and every cause beneath it on one line
├── lib.rs              the crate root: module header, `mod` lines and the re-exports
├── model/              the ticket kinds, the status with its fields, the scope, value sets, word caps and predicates
├── prelude.rs          the model types, `TicketId`, `Corpus`, `ScopeVocab` and the encoding functions for glob import
├── repository.rs       the paths only the ticket domain names: handoff documents, sparse-checkout sets, cited documents
├── store.rs            `Corpus`: the fail-closed load, id minting, surgical writes and deletes; `ticket_id_order_key`
├── tests/              unit tests for the encoding, the store, the vocabulary, the miner, the paths and the round trip
├── ticket_id.rs        `TicketId`, the ticket id newtype that serialises as its bare string
└── vocab.rs            `ScopeVocab`: the `.ai/tickets/scope-vocab.toml` tree and the scope legality check
```

## How it works

```text
model/ ◄── ticket_id.rs
   ▲
encoding.rs (TicketFile ⇄ Ticket)      vocab.rs (ScopeVocab)
   ▲                                       ▲
   └────────────── store.rs (Corpus) ──────┘

commit_subjects.rs ──► process_runner::Run("git log") ──► error.rs
repository.rs ──► repository_layout
```

- `ticket_id.rs` declares `TicketId` through `newtype_ids::string_id!`: it compares, orders,
  hashes and serialises as its string. `is_parent` and `parent_number` answer whether an id is
  `T-` and digits only, and which numeral it carries.
- `encoding.rs` maps the flat `status` and `order` keys onto `Status`, checks timestamps
  (`time_source::validate_rfc3339_utc`), the `class` and `estimated` value sets and the
  surface-requires-component rule, and builds a `ProgramTicket` or a `WorkTicket` by `kind`. The
  field order of `TicketFile` is the canonical key order, so a render is byte-identical to a
  canonical file. `main_goal` also reads the `user_story` key, `children` the `slices` key and
  `active` the `active_slice` key; a render writes the first name only.
- `store.rs` loads the vocabulary first, then every `T-*.toml` in file name order, refusing the
  whole load on a parse failure, an id that differs from the file stem or an unknown scope word.
  `write_back` renders and re-parses every requested ticket before writing any, then writes each
  as a dot-prefixed temporary file and renames it over the target. `derive_next_parent_id` and
  `next_child_id` mint ids as the highest numeral plus one, so a freed numeral is never reused.
  `ticket_id_order_key` orders ids by parent numeral, then by the whole id.
- `vocab.rs` reads the vocabulary as tables of tables of string arrays and answers whether a
  scope's domain, layer, component and surfaces are in it; the file's own shape rules are
  `ticket_registry`'s check.
- `commit_subjects.rs` runs one `git log` over `HEAD`, takes the ids each subject names at a word
  boundary, normalises each author date to whole-second UTC `Z`, and returns each id's commits
  oldest first. It is the one module that returns `Error`; the parsers, the store and the
  vocabulary refuse with a one-line `String`.
- `repository.rs` spells each ticket-domain path once; its `documentation` module is everything a
  relocation of the documentation tree rewrites.

## Boundaries

- Depends on: `repository_layout` (`TICKETS_DIR`, `SCOPE_VOCAB`, `ARTIFACTS_DIR`, `QUEUE_JSON`,
  the documentation root), `time_source`, `process_runner`, `newtype_ids`, `serde`, `toml`,
  `time`, `regex`, `thiserror`.
- Used by: `ticket_registry` (the store, the encoding, the predicates, the miner, the paths),
  `ticket_wave_lock` (the store, `ticket_id_order_key`, `ARCHIVED_WAVE_PLANS`), `ticket_metrics`
  (the store, the miner, `is_sha_shaped`, the estimate paths), `xtask` and
  `tools/tickets/ticketboard_desktop/`, all through the crate root and the public modules.
- Rules: `lib.rs` holds only the header, `mod` lines and re-exports; the encoding refuses what
  the model cannot hold, and `write_back` never writes a ticket whose render does not re-parse to
  the same ticket (`tests/store/corpus_storage_tests.rs`, `tests/proptest_roundtrip_tests.rs`);
  `commit_subjects.rs` matches ids only at a boundary (`tests/commit_subjects_tests.rs`).
