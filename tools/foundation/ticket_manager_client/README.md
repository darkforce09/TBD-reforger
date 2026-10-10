# Ticket manager client

The `ticket_manager_client` crate: the one place the tools talk to the central ticket manager. It
runs the `ttm` command line with `--json --project <project>`, parses the versioned JSON documents
it prints into typed values, and turns a refusal into a typed error. Tickets, run receipts, the
wave plan and the wave-close ledger live in the ticket manager's database; no workspace crate
reads or writes ticket files.

## Contents

```text
tools/foundation/ticket_manager_client/
├── Cargo.toml  the `ticket_manager_client` library package: `process_runner`, `verification_core`, `newtype_ids`, `serde`, `serde_json`, `thiserror`; layout tier 2
└── src/        the command line value, the ticket and wave calls, their JSON documents, the reference shapes and the errors
```

## How it works

```text
TicketManager::from_env()   TBD_TTM_BIN (default `ttm` on PATH), TBD_TTM_PROJECT (default `reforger`)
        │
  .show(reference) / .wave_show() / .land(…) / …
        │  ttm --json --project <project> <command> …      (TBD_TICKETS_DB is inherited)
        ▼
parse_document::<D>(stdout) ── {"error": …} ──► Error::Refused { kind, message, candidates }
        │                    ── not JSON, or another format tag ──► Error::Contract
        ▼
typed document (TicketDocument, WavePlan, WaveHistoryDocument, …)
```

A binary that is missing, or a child that dies on a signal or a deadline, is `Error::NotRun`.
`check` and `wave check` print their document on a failing verdict (exit 1), so a failing check
is a document with `ok` false rather than an error. `brief` prints text. `src/README.md` lists
every call and the document it returns.

## Getting started

Run from the repository root:

```bash
cargo test -p ticket_manager_client                       # the documents, the refusals and the reference shapes
TBD_TTM_BIN=ttm cargo test -p ticket_manager_client       # also asks a real ttm binary for its version
```

## Configuration

No feature. `TBD_TTM_BIN` names the binary (a path, or a name looked up on `PATH`; default
`ttm`); `TBD_TTM_PROJECT` names the project every call acts on (default `reforger`);
`TBD_TICKETS_DB`, read by `ttm` itself, selects another ticket database.

## Public surface

- At the crate root: `TicketManager` (`from_env`, `new`, `program`, `project`, `text_command`,
  `display_command` and the calls listed in `src/README.md`), `parse_document`,
  `TicketManagerDocument`, `RunRecord`, `TokenCounts`; the reference shapes `TicketSlug`,
  `LegacyTicketNumber`, `ReceiptName`, `is_ticket_reference`, `is_legacy_ticket_number` and
  `parent_slice`; `WavePlan`, `WaveRow` and `is_complete_status`; the variable names
  `BINARY_VARIABLE`, `PROJECT_VARIABLE` and their defaults; `Error`, `NOT_FOUND_KIND` and
  `Result`.
- `ticket_documents` and `wave_documents`: every document type.
- `prelude`: `TicketManager`, `RunRecord`, `TokenCounts`, `TicketSlug`, `is_ticket_reference`,
  `parent_slice`, `WavePlan`, `WaveRow`, and the error and result as `TicketManagerError` and
  `TicketManagerResult`.

## Boundaries

- Depends on: `process_runner` (the child process), `verification_core` (`NotRun`),
  `newtype_ids` (the string ids), `serde`, `serde_json` and `thiserror`; at run time on a `ttm`
  binary, never on the ticket manager's source.
- Used by: `platform_execution` (slice runs, the platform wave driver, the preflight) and
  `mod_operations` (the mod wave driver).
- Rules: tier 2 of `tools/foundation` (`cargo xtask verify crate-tiers`); a document is accepted
  only with the format tag its command's contract names; the tests parse hand-written documents
  under `src/tests/fixtures/` and run a real binary only when `TBD_TTM_BIN` is set.

## Related documentation

- [Tooling foundation crates](/tools/foundation/README.md) — the foundation crates and their
  tiers.
- [Running a wave](/documentation/runbooks/factory_waves/running_a_wave.md) — the wave procedure
  the drivers run through this client.
