**Status:** live

# Ticket engine documentation

The documents on `ticket_engine`, the library that owns the [ticket](/documentation/glossary/n_to_z.md#ticket)
registry in `.ai/tickets/`: its typed storage and operations, validation, sync outputs,
[wave](/documentation/glossary/n_to_z.md#wave) lock and metrics. Developers and AI agents read them
below the crate's code READMEs, which say what each module declares.

## Contents

```text
documentation/tools/ticket_engine/
└── token_estimate_factor.md  the tokens-per-line factor, its one measurement and the excluded paths
```

## How it works

The crate's code READMEs are exact about the modules: the
[crate README](/tools/ticket_engine/README.md) for the flow from a `cargo xtask ticket` verb to
the store, the [source README](/tools/ticket_engine/src/README.md) for the layers (`model`,
`store`, `ops`, `validation`, `sync`, `wave_lock`, `metrics`) and the
[estimates README](/tools/ticket_engine/src/metrics/estimates/README.md) for the estimate
sources and the cohort ladder. The documents here hold what code cannot: the measurement and the
reasons behind a constant. The [token estimate factor](/documentation/tools/ticket_engine/token_estimate_factor.md)
is the document of record for `TOKENS_PER_LOC`; its path is compiled into the crate as
`TOKEN_ESTIMATE_FACTOR_DOC` in `tools/ticket_engine/src/repository.rs`, so it never moves
without that constant.

The registry's rules for authors (statuses, ids, plans, commands) belong to the
[ticket registry README](/.ai/tickets/README.md) and the
[ticket identifiers standard](/documentation/standards/ticket_identifiers.md); the tooling's
crate boundaries to the [tooling architecture](/documentation/tools/tooling_architecture.md).

## Code

- [Ticket engine](/tools/ticket_engine/) — the crate these documents cover.
- [Token estimates](/tools/ticket_engine/src/metrics/estimates/) — the estimator the factor
  document governs.

## Boundaries

- Depends on: the crate's code and the `.ai/tickets/` tree, which every claim is checked against;
  the feature doc template; the ticket registry for open work.
- Used by: the crate README and the estimates README, which link the factor document; the
  estimate check in `tools/ticket_engine/src/metrics/estimates/verification.rs`, whose refusal
  message names it; and `factor_constant_is_pinned_in_the_doc`, which reads it.
- Rules: `token_estimate_factor.md` keeps its path and quotes `TOKENS_PER_LOC = <value>`, the words
  "pending calibration" and the three excluded prefixes ".ai/", "docs/TICKET_" and "Cargo.lock"
  verbatim (`cargo test -p ticket_engine estimate_provenance`).

## Related documentation

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the ship, stamp-sha and
  repack order that writes estimates during a wave.
- [Ticketboard documentation](/documentation/ticketboard/README.md) — the viewer that shows
  measured and estimated tokens apart.
