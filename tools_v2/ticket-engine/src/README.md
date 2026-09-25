# Ticket engine source

The `ticket_engine` library: the typed [ticket](/documentation_v2/glossary.md#ticket) model and its
TOML encoding, the corpus store and the operations that change it, the checks, the derived files
(`queue.json`, the roadmap block, the gap-analysis column, the [wave](/documentation_v2/glossary.md#wave)
lock), the run metrics, and the one module that spells every repository path the ticket domain
touches.

## Contents

```text
tools_v2/ticket-engine/src/
├── cli/            the body of every `cargo xtask ticket` subcommand
├── corpus_pins.rs  `load` of `.ai/tickets/corpus-pins.toml`: never-minted ids and pinned gap rows
├── encoding.rs     `TicketFile`, `parse_ticket_toml`, `render_ticket_toml`: the canonical ticket TOML
├── lib.rs          the crate root: the module list and the re-exports of the model and entry points
├── metrics/        run receipts, their check and summary, and the token estimates
├── model/          the typed ticket: kinds, statuses, scope, word caps and shared predicates
├── ops/            the typed mutations, each validated as a whole corpus before it is committed
├── registry/       the registry as a parents-only JSON value, its field helpers and the platform readers
├── repository.rs   every repository path the ticket domain reads or writes, and checkout-root discovery
├── store.rs        `Corpus`: every ticket file loaded, and surgical per-file writes and deletes
├── sync/           `ticket sync`: `queue.json`, the roadmap next-work block and the gap-analysis column
├── tests/          unit tests for encoding, the store, timestamps, the vocabulary and the layout
├── timestamp.rs    `validate_rfc3339_utc` and `now_utc_rfc3339`: the one timestamp rule
├── validation/     `ticket check` and the preflight every mutation runs
├── vocab.rs        `ScopeVocab`: the scope word list the corpus load checks each work ticket against
└── wave_lock/      the wave lock compiler, reader and checker, and `slice-collisions`
```

## How it works

Two views of the ticket files serve two kinds of command:

```text
.ai/tickets/T-*.toml ──store::Corpus::load──► typed corpus (parents and children) ──► ops ──► write_back
        │                                                                              │
        └──registry::load_registry──► parents-only JSON value ──► queries, sync, check ◄┘ reload
```

- `encoding.rs` maps a file onto `TicketFile` and into the typed `Ticket`, and renders keys in one
  canonical order, so a load and a write reproduce a file byte for byte. A malformed timestamp or
  an illegal field value refuses the parse.
- `Corpus::load` reads every `T-*.toml`, refuses the whole load on the first file that fails to
  parse or whose id differs from its file name, and checks each work ticket's scope against
  `ScopeVocab`. `write_back` writes only the ids it is given: render, re-parse, write a
  dot-prefixed temporary file, rename it over the target. `delete_files` deletes only the ids it
  is given.
- Mutations run `validation`'s preflight, one `ops` operation over the corpus, the write, a
  reload of the registry value, then `sync` and, for status changes, a wave lock repack.
- Timestamps everywhere are RFC 3339 in UTC, written with `Z` or `+00:00` and an upper-case `T`
  (`timestamp.rs`); a malformed stamp is a load error, never replaced with the current time.
- `repository.rs` is one of the three modules allowed to spell a repository path; its
  `documentation` submodule holds every document the crate names (`TREE_DIR`, `SPECS_DIR`,
  `PLANS_DIR`, `PLAN_TEMPLATE`, `ROADMAP`, `GAP_ANALYSIS`, `TOKEN_ESTIMATE_FACTOR_DOC`, the scan
  roots and exemptions, `ARCHIVED_WAVE_PLANS`, `RETIRED_QUEUE_VIEW_PREFIX`), and
  `SPARSE_CHECKOUT_SETS` names the folders a sparse checkout needs per ticket target (`website`,
  `mod`, `shared`, `root`). `find_repo_root` walks up from the working directory to the folder
  that holds `.ai/tickets/ROOT`.

## Public surface

- The crate root re-exports the model (`Ticket`, `WorkTicket`, `ProgramTicket`, `Status`,
  `StatusName`, `ScopeV2`, `Domain`, the caps and predicates), `TicketFile`,
  `parse_ticket_toml`, `render_ticket_toml`, `Corpus`, `OpOutcome`, `ScopeVocab`,
  `validate_rfc3339_utc` and `now_utc_rfc3339`; the ticketboard reads tickets through these.
- `repository`: the paths `tools_v2/xtask/src/core/repository_layout.rs` re-exports and the
  ticketboard joins onto its checkout root, and `find_repo_root`, which
  `tools_v2/xtask/src/core/repository_root.rs` calls.
- `cli`, `sync`, `validation`, `wave_lock`, `metrics`, `registry` and `corpus_pins`: the entry
  points of the xtask `ticket`, `wave`, `platform` and `mod` command groups; each folder's README
  names its callers.

## Boundaries

- Depends on: the crates in `tools_v2/ticket-engine/Cargo.toml` and no workspace crate; `git` for
  history reads (commit subjects, past revisions, wave-close commits, the fossil path guard).
- Used by: `tools_v2/xtask/` and `apps/ticketboard/`.
- Rules:
  - every ticket file loads and renders back byte for byte (`corpus_roundtrip_real_tree_byte_identical`
    in `tests/store/corpus_storage_tests.rs`, and the property tests in
    `tests/proptest_roundtrip_tests.rs`);
  - every committed location in `repository.rs` exists, and every `documentation` item is
    classified (`every_required_documentation_location_exists_in_the_checkout` and
    `every_documentation_item_is_classified` in `tests/repository_layout_tests.rs`);
  - no production source outside the layout modules spells a `.ai/` or `documentation_v2/` path
    (`only_a_layout_module_spells_a_repository_path` in
    `tools_v2/xtask/src/tests/tooling_prose_rules.rs`).

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, their schema and the derived files.
