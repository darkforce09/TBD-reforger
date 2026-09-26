# Ticket engine

The `ticket-engine` crate: everything about the [ticket](/documentation_v2/glossary/n_to_z.md#ticket)
registry in `.ai/tickets/` that is not a process side effect. It stores, validates and changes the
ticket files, regenerates the files derived from them, compiles the
[wave](/documentation_v2/glossary/n_to_z.md#wave) lock, and keeps the run metrics. `cargo xtask` mounts it
as the `ticket` and `wave` command groups and the platform wave driver, and the
[ticketboard](/documentation_v2/glossary/n_to_z.md#ticketboard) reads tickets through it.

## Contents

```text
tools_v2/ticket-engine/
├── .gitignore  keeps a local `wip/` folder out of git
├── Cargo.toml  the `ticket-engine` library package, with no workspace dependency
├── src/        the library: model, store, operations, commands, checks, sync, wave lock, metrics
└── tests/      the closed-`Domain` compile-fail test and run receipt fixtures shared with xtask
```

## How it works

The crate reads and writes the files under `.ai/tickets/` and nothing else in the checkout, apart
from the two documents `ticket sync` writes into and git history. Every function takes the
checkout root as an argument; `repository::find_repo_root` finds it for a running command by
walking up to `.ai/tickets/ROOT`, so a command run inside a linked worktree reads that worktree's
files.

```text
cargo xtask ticket <verb> ──► cli::cmd_<verb> ──► validation preflight ──► ops over store::Corpus
                                                                        ──► write_back, sync, wave_lock repack
cargo xtask wave <verb>   ──► wave_lock::cmd_repack / cmd_check / collisions::run
cargo xtask platform …    ──► wave_lock, registry, metrics, history rules
apps/ticketboard          ──► model types, parse_ticket_toml, repository paths
```

The typed operations are the only writer of ticket files, and each writes only the files it names;
the wave lock has one writer, the repack. The crate never starts an agent and never deletes a
worktree or branch: `cli::cmd_run` takes the executor as a callback and `cli::cleanup_targets`
only resolves paths, and xtask performs both. The `src/` README describes the layers.

## Getting started

Run these from the repository root:

```bash
cargo test -p ticket-engine        # every unit test, the property tests and the compile-fail test
cargo xtask ticket check --strict  # the full check of the committed tickets, as CI runs it
cargo xtask wave check             # the wave lock against the ticket files
cargo xtask ticket --help          # every ticket subcommand
```

The tests need a git checkout with full history: several read the live `.ai/tickets/` tree and git
log. Neither the CI workflow nor the platform wave gate runs `cargo test -p ticket-engine`; run it
by hand after a change here.

## Configuration

- `TBD_MAX_CONCURRENT` (optional): the most tickets one wave may hold. Unset, a repack keeps the
  width the committed lock records, and a lock with none uses 8
  (`src/wave_lock/compiler.rs`, `src/wave_lock/ticket_views.rs`).
- The files under `.ai/tickets/` that configure the rules, all required: `schema.json` (the
  ticket schema), `scope-vocab.toml` (the scope words), `corpus-pins.toml` (never-minted ids, the
  game-mod program, pinned gap rows), `metrics.schema.json` and `estimates.schema.json`; their
  paths are in `src/repository.rs`.
- `queue.json`'s `batch_size` (10), `concurrency` (3), `worktree_base`
  (`.ai/artifacts/worktrees`) and `git_base` (`main`), read by `ticket config`, `ticket run` and
  `ticket clean`; `ticket sync` rewrites the file with these defaults.

## Public surface

- The library `ticket_engine`: the model, `Corpus`, `parse_ticket_toml` and `render_ticket_toml`
  at the crate root, and the modules `cli`, `registry`, `sync`, `validation`, `wave_lock`,
  `metrics`, `corpus_pins` and `repository`; the [source README](/tools_v2/ticket-engine/src/README.md)
  lists who calls each.
- No binary: every command runs through `cargo xtask`.

## Boundaries

- Depends on: `anyhow`, `regex`, `serde`, `serde_json`, `toml`, `time`, `walkdir` and
  `jsonschema`, and no workspace crate; `git` on `PATH`.
- Used by: `tools_v2/xtask/` (the `ticket`, `wave`, `platform`, `mod`, `fetch` and `schema`
  command groups and `src/core/`) and `apps/ticketboard/`, both by path dependency.
- Rules:
  - the crate depends on no workspace crate (`foundational_engines_have_no_workspace_dependencies`),
    and ticket logic lives here rather than in xtask, whose `ticket` and `wave` adapters must
    delegate to it (`ticket_implementations_have_one_owner`), both in
    `tools_v2/xtask/src/tests/tooling_dependency_boundaries.rs`;
  - production files stay under 500 lines and test files under 1,000, with tests in separate
    `tests/` files (`tooling_source_files_stay_below_their_structural_limits`,
    `tooling_test_modules_live_in_separate_files`, same file);
  - every tracked file here is held to the prose rules of
    `tools_v2/xtask/src/tests/tooling_prose_rules.rs`: no ticket ids, no history words, no
    retired names, no script file names, and every `.rs` named exists;
  - `Domain` stays a closed enum without `frontend` (`tests/trybuild.rs`).

## Related documentation

- [Ticket registry](/.ai/tickets/README.md) — the ticket files, their schema and the derived files.
- [Ticket command group](/tools_v2/xtask/src/commands/ticket/README.md) — the `ticket`
  subcommands.
- [Wave lock command group](/tools_v2/xtask/src/commands/wave/README.md) — `wave repack`,
  `wave check` and `slice-collisions`.
- [Ticket identifiers](/documentation_v2/standards/ticket_identifiers.md) — how ticket ids are
  formed and cited.
- [Factory waves](/documentation_v2/runbooks/factory_waves/README.md) — the ship, stamp and repack
  order during a wave.
- [Taking a ticket from idea to shipped](/documentation_v2/runbooks/ticket_run_pipeline.md) — the
  ticket lifecycle these commands implement, and the canonical form of a hand-edited ticket file.
- [Token estimate factor](/documentation_v2/tools_v2/ticket-engine/token_estimate_factor.md) — the
  measurement behind the token estimates.
- [Ticket engine documentation](/documentation_v2/tools_v2/ticket-engine/README.md) — the index of
  the deeper documents on this crate.
- [Tooling architecture](/documentation_v2/tools_v2/tooling_architecture.md) — the dependency rules
  this crate lives under.
