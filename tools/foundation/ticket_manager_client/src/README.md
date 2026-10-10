# Ticket manager client source

The `ttm` command line as a value, its calls, the JSON documents they return and the ticket
reference shapes the tools accept.

## Contents

```text
tools/foundation/ticket_manager_client/src/
├── error.rs              `Error` (did not run, refused, contract broken), `NOT_FOUND_KIND` and `Result`
├── lib.rs                the crate root: module header, `mod` lines and the re-exports
├── prelude.rs            the client, the run record, the reference shapes and the wave plan for glob import
├── ticket_commands.rs    `version`, `show`, `resolve`, `list`, `next`, `brief`, `check`, `record_run`, `land`, `unland`, `ship`, `set_status`, `metrics`
├── ticket_documents.rs   the `ttm.version`, `ttm.ticket`, `ttm.resolve`, `ttm.list`, `ttm.next`, `ttm.check`, `ttm.receipt`, `ttm.land`, `ttm.unland` and `ttm.metrics` documents
├── ticket_manager.rs     `TicketManager`: the binary and project, the JSON and text runs, `parse_document`
├── ticket_references.rs  `TicketSlug`, `LegacyTicketNumber`, `ReceiptName`, `is_ticket_reference`, `parent_slice`
├── wave_commands.rs      `wave_show`, `wave_repack`, `wave_check`, `wave_close`, `wave_history`, `wave_collisions`
├── wave_documents.rs     the `ttm.wave`, `ttm.wave-repack`, `ttm.wave-check`, `ttm.wave-close`, `ttm.wave-history` and `ttm.collisions` documents; the wave plan's questions
└── tests/                unit tests and the hand-written JSON documents under `fixtures/`
```

## How it works

- `ticket_manager`: every JSON call runs `<binary> --json --project <project> <args…>`. A
  non-zero exit is a refusal, read from the `{"error": {"kind", "message", "candidates"}}`
  document when stdout holds one, else `exit <code>` with stderr; `check` and `wave check` accept
  exit 1 with their own document. `parse_document` accepts one JSON object whose `format` equals
  the type's `TicketManagerDocument::FORMAT`.
- `ticket_documents` and `wave_documents`: each type declares only the fields a caller reads and
  ignores the rest, because the contract adds fields freely within a format version.
  `WavePlan::open_wave` is the first open wave holding a ticket that is neither `shipped` nor
  `cancelled`; `WavePlan::row` finds a ticket by slug or legacy number in the open and
  pending-close waves; an unknown ticket is never complete.
- `ticket_references`: `is_ticket_reference` accepts a legacy number (`T-674.1`) or a slug
  (lowercase letters, digits, `-` and `.`, no empty or dash-edged dot segment), so an accepted
  reference never leaves the folder it is joined onto; `parent_slice` maps a reference of three or
  more dot segments to its first two.

## Boundaries

- Depends on: `process_runner::Run`, `verification_core::NotRun`, `newtype_ids::string_id`,
  `serde`, `serde_json`, `thiserror`.
- Used by: the crate root's re-exports.
- Rules: the contract shapes are pinned by `tests/ticket_documents_tests.rs` over the fixtures,
  the reference shapes by `tests/ticket_references_tests.rs`; `tests/live_ticket_manager_tests.rs`
  runs only when `TBD_TTM_BIN` is set.
