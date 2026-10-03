# Ticket crates

The libraries of the [ticket](/documentation/glossary/n_to_z.md#ticket) registry in `.ai/tickets/`:
the typed ticket and its canonical TOML encoding, the run receipts and token estimates, the
[wave](/documentation/glossary/n_to_z.md#wave) lock, and the registry operations, checks and
derived files. The `ticket` and `wave` command groups of xtask, its platform wave driver and the
[ticketboard](/documentation/glossary/n_to_z.md#ticketboard) in `apps/ticketboard/` call them; each
crate holds one layer, and a crate depends only on the crates below it.

## Contents

```text
tools/tickets/
├── ticket_metrics/     `ticket_metrics`: slice-run receipts under `.ai/tickets/metrics/` and token estimates under `.ai/tickets/estimates/`
├── ticket_model/       `ticket_model`: the typed ticket, `TicketId`, the TOML encoding, the corpus store, the scope vocabulary, the ticket paths, the commit-subject miner
├── ticket_registry/    `ticket_registry`: the registry view, the typed operations, `ticket check`, `ticket sync` and the bodies of the `ticket` verbs
├── ticket_wave_lock/   `ticket_wave_lock`: the wave lock compiler, renderer, reader and checker
└── ticketboard_model/  `ticketboard_model`: the ticketboard's headless half — corpus, projections, filters, wave lanes, metrics, documents, ticket commands, application state
```

## How it works

```text
ticket_registry (tier 4) ───┬─▶ ticket_metrics (tier 3) ───▶ ticket_model (tier 2) ──▶ tools/foundation, time_source, newtype_ids
ticketboard_model (tier 4) ─┴─▶ ticket_wave_lock (tier 3) ─┘
```

`ticket_model` is the substrate: every ticket file loads into a typed `Ticket` and renders back byte
for byte, and every ticket id is a `TicketId`, which serialises as its bare string. The two tier-3
crates read the corpus through it: `ticket_metrics` keeps the measured and estimated token
accounting, `ticket_wave_lock` packs the open tickets into file-disjoint waves and checks the
committed lock. `ticket_registry` composes them into the operations the `ticket` verbs run, the
`ticket check` rules and the files `ticket sync` regenerates. None of the crates starts an agent,
removes a worktree or decides an exit code: xtask performs those side effects and turns a refusal
into exit code 1.

## Getting started

Run these from the repository root:

```bash
cargo test -p ticket_model -p ticket_metrics -p ticket_wave_lock -p ticket_registry
cargo xtask ticket check --strict  # the full check of the committed tickets
cargo xtask wave check             # the wave lock against the ticket files
```

## Boundaries

- Depends on: the `tools/foundation` crates and, among `crates/foundation`, `time_source`,
  `content_digest` and `newtype_ids` (the test
  `ticket_crates_depend_only_on_foundations_and_lower_ticket_crates` in
  `tools/checks/repository_checks/src/tests/tooling_dependency_boundaries.rs`).
- Used by: `xtask` and `ticketboard`; `ticketboard_model` is the ticketboard's own headless half,
  used by `apps/ticketboard` alone.

## Related documentation

- [Ticket registry feature docs](/documentation/tools/tickets/README.md) — the token estimate
  factor behind the estimates.
