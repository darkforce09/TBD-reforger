# Ticket registry source

The `ticket_registry` library: the JSON registry view of the
[ticket](/documentation/glossary/n_to_z.md#ticket) files, the typed operations that change them,
the files `ticket sync` derives from them, the `ticket check` rules, the corpus pins and the body
of every `cargo xtask ticket` verb.

## Contents

```text
tools/tickets/ticket_registry/src/
├── corpus_pins.rs  `load` of `.ai/tickets/corpus-pins.toml`: never-minted ids, the game-mod programme ticket, pinned gap rows (`EditorGapRowId`)
├── error.rs        `Error` and `Result`; `Error::Refused` is a message xtask prints bare before exit 1
├── lib.rs          the crate root: module header, `mod` lines and the re-exports
├── ops/            the typed operations over `ticket_model::Corpus`, each validated as a whole corpus before it commits
├── prelude.rs      `Registry`, `load_registry`, `cmd_sync`, `cmd_check`, `Error`, `OpOutcome` and `Result` for glob import
├── registry/       the JSON registry view, the ticket file storage, the typed projection, shipping status, status history
├── sync/           `ticket sync`: `queue.json`, the roadmap next-work markers, the gap-analysis ticket column
├── tests/          unit tests for the corpus pins
├── validation/     `ticket check` and the preflight every writing verb runs
└── verbs/          the bodies of the `cargo xtask ticket` verbs
```

## How it works

```text
.ai/tickets/T-*.toml ──Corpus::load──► typed corpus (parents and children) ──► ops ──► write_back / delete_files
        │                                                                                   │
        └──registry::load_registry──► parents-only JSON value ──► verbs, sync, validation ◄─┘ reload
```

- `registry/` builds the JSON value. Typed files (they carry `kind =`) project through
  `registry/typed_projection.rs`, which orders the parent rows by `order` then id and gives each
  program a `slice_plan` read from its children's files; an untyped folder loads through
  `registry/ticket_file_storage/`. A folder with neither refuses instead of answering an empty
  registry. The whole-tree writer `save_registry` refuses a typed tree.
- `ops/` holds every mutation as a function over an in-memory `ticket_model::Corpus`: it builds
  the candidate corpus, validates it with the post-image gate, and only then swaps it in and
  reports the changed and deleted ids in an `OpOutcome`. A refusal is the exact text the command
  prints. The clock is an RFC 3339 UTC argument, so the same inputs give the same corpus.
- `verbs/` runs a writing verb in a fixed order: the `ticket check` preflight
  (`validation::require_check_ok`), one operation, `Corpus::write_back` and `delete_files`, a
  reload of the registry value from disk, then `sync::cmd_sync` and, for status changes, a wave
  lock repack through `ticket_wave_lock`. `stamp-sha` alone skips the preflight and the sync. A
  verb never starts an agent or removes a worktree: `cmd_run` takes the executor as a callback and
  `cleanup_targets` only resolves paths.
- `sync/` writes `queue.json`, the roadmap block between its markers and the gap-analysis ticket
  column; an absent document is skipped, and no write collapses a block to a bare heading.
- `validation/` collects every rule into one ordered list of findings: the JSON schema, the row
  rules, scope, body caps, the ship gate, readiness, debt pins, references, the wave lock, the run
  metrics and the scope vocabulary. A rule that cannot load its input reports the load error.
- `corpus_pins.rs` is fail-closed: a missing or malformed pins file is an error, never an empty
  table.
- `error.rs`: `Error::Context` prints its own line and keeps the cause as its source;
  `Error::Refused` carries the whole output of a refusal, which xtask prints with no prefix before
  it exits 1.

## Boundaries

- Depends on: the crates the package manifest one folder up names; `git` for the
  past-revision reads, the fossil path guard and the commit-subject mining.
- Used by: `tools/xtask/`.
- Rules:
  - the operations are the only writer of ticket files, and each writes only the files it names
    (`mutators_never_reach_the_value_writer_pin` in
    `registry/tests/typed_projection/typed_projection_tests.rs`);
  - the sync after a write reads the files the write produced
    (`ship_regenerates_queue_from_post_state_reload_pin` in
    `verbs/tests/command_mutation_tests.rs`);
  - the committed corpus pins load with both tables populated (`tests/corpus_pins_tests.rs`);
  - `lib.rs` holds only the module header, `mod` lines and re-exports
    (`cargo xtask verify crate-anatomy`).

## Related documentation

- [Ticket registry files](/.ai/tickets/README.md) — the ticket files, their schema and the
  derived files.
