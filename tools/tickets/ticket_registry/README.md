# Ticket registry

The `ticket_registry` crate: the top layer of the [ticket](/documentation/glossary/n_to_z.md#ticket)
crates. It projects the ticket files in `.ai/tickets/` into one JSON registry value, changes them
only through typed operations over `ticket_model::Corpus`, regenerates the files derived from them
(`queue.json`, the roadmap's next-work block, the gap-analysis ticket column), runs
`ticket check`, and holds the body of every `cargo xtask ticket` verb. Side effects that leave the
ticket domain, such as starting an agent or removing a worktree, stay in xtask, which supplies
them as callbacks or performs them on the paths this crate resolves.

## Contents

```text
tools/tickets/ticket_registry/
├── Cargo.toml  the `ticket_registry` library package: the three lower ticket crates, the tooling foundations, layout tier 4
└── src/        the registry view, the typed operations, the sync, the checks and the verb bodies
```

## How it works

```text
cargo xtask ticket <verb> ──► load_registry (JSON view) ──► verbs::cmd_<verb>
                                                              │
     read verbs ─────────────────────────────────────────────┤ print from the view
     writing verbs ──► validation::require_check_ok ──► ops::<operation>(&mut Corpus)
                         (red check: refuse, write nothing)       │
                                                              Corpus::write_back / delete_files
                                                              ──► reload the view ──► sync::cmd_sync
                                                              ──► ticket_wave_lock repack (status writes)
```

Two views of the same files serve two kinds of work. The registry value
(`registry::load_registry`) holds the parent tickets as JSON rows, each program carrying a
`slice_plan` built from its children's files; the read verbs, `ticket sync` and `ticket check`
consume it. The typed corpus (`ticket_model::Corpus`) holds every file, children included; the
typed operations in `src/ops/` are the only writer, and each computes the whole corpus after its
change, validates it, and reports exactly which files to write and delete. A writing verb reloads
the registry value from disk before it syncs, so the derived files always come from the state the
write produced. The `src/` README describes each layer.

A refusal the command prints as it stands (an operation's own refusal, a red `ticket check`, an
unknown ticket) is `Error::Refused`; xtask prints its message bare, with no prefix, and exits 1.

## Getting started

Run these from the repository root:

```bash
cargo test -p ticket_registry      # every unit test; several read the committed .ai/tickets tree
cargo xtask ticket check --strict  # the full check of the committed tickets, as CI runs it
cargo xtask ticket sync            # regenerate queue.json, the roadmap block and the gap column
cargo xtask ticket --help          # every ticket verb
```

The tests need a checkout with the committed `.ai/tickets/` tree, the documents it names and git
history.

## Configuration

- No feature and no environment variable.
- The files under `.ai/tickets/` that configure the rules, all required: `schema.json` (the
  ticket schema `ticket check` validates against), `scope-vocab.toml` (the scope words, whose
  shape `ticket check` validates) and `corpus-pins.toml` (never-minted ids, the game-mod
  programme ticket, pinned gap rows); their paths come from `repository_layout`.
- `queue.json`'s `batch_size` (10), `concurrency` (3), `worktree_base`
  (`.ai/artifacts/worktrees`) and `git_base` (`main`), read by `ticket config`, `ticket run`,
  `ticket ready-ids` and `ticket clean`; `ticket sync` rewrites the file with these defaults.

## Public surface

- At the crate root: `load_registry`, `OpOutcome`, `Error` and `Result`.
- `registry`: the `Registry` value, its row helpers, `write_json_ascii`, `ShippingStatus`,
  `status_map_at_rev`, `read_ticket_title` and the untyped ticket file storage.
- `ops`: `add`, `add_child`, `remove`, `reorder`, `advance_slice`, `set_status`, `ship`,
  `stamp_sha`, `mark_ready`, `default_plan_path` and `VALID_STATUS_NAMES`.
- `sync`: `cmd_sync`, `generate_queue_json`, `refuse_empty_write` and `gap_analysis`.
- `validation`: `check`, `cmd_check`, `require_check_ok`, `require_check_ok_deferring_repack`,
  `validate_registry_schema`, `constants` and `vocabulary`.
- `verbs`: one `cmd_*` function per `ticket` verb, `cleanup_targets` with `CleanupTargets`,
  `ExecutorResult`, `stamp_sha_with_inputs`, `require_ticket`, `unknown_ticket` and
  `prompt::extract_prompt`.
- `corpus_pins`: `load`, `CorpusPins` and `EditorGapRowId`.
- `prelude`: `Registry`, `load_registry`, `cmd_sync`, `cmd_check`, `Error`, `OpOutcome` and
  `Result`.
- No binary: every command runs through `cargo xtask`.

## Boundaries

- Depends on: `ticket_model` (the typed ticket, `TicketId`, the corpus store, the commit-subject
  miner, the ticket paths), `ticket_metrics` (receipts and estimates), `ticket_wave_lock` (the
  lock check and repack), `repository_layout` (the shared locations), `time_source` (the RFC 3339
  UTC clock), `process_runner` (to run `git`) and `newtype_ids` (`EditorGapRowId`); `jsonschema`,
  `regex`, `serde`, `serde_json` and `toml` (both with `preserve_order`), `thiserror` and
  `walkdir`; `git` on `PATH`.
- Used by: `tools/xtask/` — the `ticket` command group and `registry-get`, the platform wave
  driver and slice runner, the mod wave driver and the schema group's `refuse_empty_write` — and
  nothing else.
- Rules:
  - tier 4 of `tools/tickets`, depending only on the tooling foundations and the lower ticket
    crates (`ticket_crates_depend_only_on_foundations_and_lower_ticket_crates` in
    `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`), and ticket logic lives here rather
    than in xtask, whose `ticket` adapters delegate to it (`ticket_implementations_have_one_owner`,
    same file);
  - a mutation refuses on a red `ticket check` and writes nothing, and no verb reaches the
    whole-tree writer (`require_check_ok_blocks_invalid_registry`,
    `mutators_never_reach_the_value_writer_pin`);
  - the crate never starts an agent and never deletes a worktree or branch
    (`dry_run_does_not_call_executor`,
    `cleanup_resolution_preserves_defaults_and_performs_no_deletion`);
  - production files stay at or under 500 lines and test files at or under 1,000, with tests in
    separate `tests/` files (`cargo xtask verify file-length`).

## Related documentation

- [Ticket crates](/tools/tickets/README.md) — the four ticket crates and how they depend on each
  other.
- [Ticket registry files](/.ai/tickets/README.md) — the ticket files, their schema and the
  derived files.
- [Ticket command group](/tools/xtask/src/commands/ticket/README.md) — every `ticket` verb, its
  flags and exit codes.
- [Taking a ticket from idea to shipped](/documentation/runbooks/ticket_run_pipeline.md) — the
  ticket lifecycle these verbs implement.
- [Ticket identifiers](/documentation/standards/ticket_identifiers.md) — how ticket ids are
  formed and cited.
- [Ticket documentation](/documentation/tools/tickets/README.md) — the deeper documents on the
  ticket crates.
- [Tooling architecture](/documentation/tools/tooling_architecture.md) — the dependency rules
  this crate lives under.
