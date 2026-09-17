# Ticket Engine (`tools_v2/ticket-engine`)

## Phase-one implementation

The typed ticket library is live with its existing source layout and dependencies (`serde`, `time`, `toml`). Ticket checking, CLI orchestration, view synchronization, metrics, and wave lock compilation remain in `../xtask`. The consolidation and test decomposition below are future targets.

## Target architecture

Complete, self-contained ticket domain database, schema validation, view synchronization, and wave lockfile compiler for the `.ai/tickets/` registry.

Consolidates the typed data models and store from `crates/tbd-tickets` with the ticket validation (`check.rs`), command operations (`cmds.rs`), markdown synchronizer (`sync.rs`), and lockfile compiler (`wave_lock.rs`) previously stranded in `xtask`.

---

## 1. Directory Structure & Architecture

```text
tools_v2/ticket-engine/
├── Cargo.toml                           <-- serde, time, toml, jsonschema
├── src/
│   ├── lib.rs                           <-- Public API interface & re-exports
│   ├── core/                            <-- Domain types, timestamps, vocabulary, TOML encoding
│   ├── store/                           <-- Corpus repository store & transactional file I/O
│   ├── operations/                      <-- Transactional ticket mutation state machine
│   ├── validation/                      <-- (Relocated from xtask check.rs) Complete schema & gate checks
│   ├── sync/                            <-- (Relocated from xtask sync.rs) Markdown queue views & queue.json
│   ├── wave_lock/                       <-- (Relocated from xtask wave_lock.rs) Concurrency DAG & wave.lock
│   ├── metrics/                         <-- (Relocated from xtask metrics.rs) Receipts & token estimation
│   └── cli/                             <-- (Relocated from xtask cmds.rs) High-level command runners
└── tests/
    ├── proptest_roundtrip.rs            <-- Lossless render(parse(t)) == t property verification
    ├── ticket_check_tests.rs            <-- Extracted from check.rs
    ├── ticket_cmds_tests.rs             <-- Extracted from cmds.rs
    └── wave_lock_tests.rs               <-- Extracted from wave_lock.rs
```

---

## 2. Architectural Responsibilities

| Submodule | Source Files / Origin | Functional Responsibilities | Invariants Enforced |
|---|---|---|---|
| **`core/`** | `tbd-tickets/src/lib.rs`, `timestamp.rs`, `vocab.rs`, `encoding.rs` | Strongly typed representations of tickets (`Domain`, `ScopeV2`, `Status`, `Ticket`), canonical-order TOML, and strict UTC timestamps. | • UTC RFC 3339 timestamps only (zero naive time).<br>• Word caps: title ≤ 10, summary ≤ 40, body lines ≤ 30. |
| **`store/`** | `tbd-tickets/src/store.rs` | Loads `.ai/tickets/T-*.toml` files into in-memory `Corpus`, manages referential integrity, coordinates atomic disk writes. | • Atomic temp-file write and rename.<br>• Never writes malformed TOML to disk. |
| **`operations/`** | `tbd-tickets/src/ops.rs` | Pure transactional state transitions: `ship`, `mark_ready`, `set_status`, `add`, `remove`, `reorder`, `stamp_sha`. | • Injects clock for deterministic time.<br>• Validates post-images before committing. |
| **`validation/`** | `xtask/src/check.rs` (1,264 LOC prod) | Audits all tickets against `schema.json`, validates work classes, ownership (`owns`), title debt pins, and ship gates. | • Zero debt drift beyond ratified pins.<br>• All ready tickets must satisfy readiness criteria. |
| **`sync/`** | `xtask/src/sync.rs` (521 LOC) | Generates `TICKET_REGISTRY.md`, `TICKET_LEAD.md`, `TICKET_DEV_QUEUE.md`, and `.ai/tickets/queue.json`. | • Idempotent generation.<br>• Refuses to emit empty files. |
| **`wave_lock/`** | `xtask/src/wave_lock.rs` (1,142 LOC prod) | Compiles `.ai/tickets/wave.lock` from ticket graph, schedules file-disjoint slices, verifies drift. | • `repack` is the only legal writer of `wave.lock`.<br>• `check` fails on any graph or hash drift. |
| **`metrics/`** | `xtask/src/metrics.rs`, `estimate_tokens.rs` | Analyzes execution receipts, calculates elapsed duration, estimates token usage. | • RFC 3339 timestamp arithmetic only. |
| **`cli/`** | `xtask/src/cmds.rs` (1,343 LOC prod) | Executes high-level operations invoked by `cargo xtask ticket ...` and `apps/ticketboard`. | • Clean exit code mapping. |

---

## 3. Invariants

1. **Zero Monorepo Dependencies**: `ticket-engine` depends only on general Rust crates (`serde`, `time`, `toml`, `jsonschema`).
2. **Single Authority for Tickets**: No code in `xtask` or `apps/ticketboard` directly touches raw ticket TOML bytes; all mutations route through `ticket-engine`.
3. **No Inline Test Modules**: All test suites are extracted into sibling files under `tests/`.
