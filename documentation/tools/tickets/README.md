**Status:** live

# Ticket crates documentation

The documents on the ticket crates in `tools/tickets/`, the libraries that own the
[ticket](/documentation/glossary/n_to_z.md#ticket) registry in `.ai/tickets/`: the typed ticket
and its store (`ticket_model`), run receipts and estimates (`ticket_metrics`), the
[wave](/documentation/glossary/n_to_z.md#wave) lock (`ticket_wave_lock`), and the operations,
validation and sync outputs (`ticket_registry`). Developers and AI agents read them below the
crates' code READMEs, which say what each module declares.

## Contents

```text
documentation/tools/tickets/
└── token_estimate_factor.md  the tokens-per-line factor, its one measurement and the excluded paths
```

## How it works

The crates' code READMEs are exact about the modules: the
[ticket crates README](/tools/tickets/README.md) for the four crates and the order they depend
in, the [registry crate README](/tools/tickets/ticket_registry/README.md) for the flow from a
`cargo xtask ticket` verb to the store, the
[registry source README](/tools/tickets/ticket_registry/src/README.md) for its layers (`registry`,
`ops`, `validation`, `sync`, `verbs`) and the
[estimates README](/tools/tickets/ticket_metrics/src/estimates/README.md) for the estimate
sources and the cohort ladder. The documents here hold what code cannot: the measurement and the
reasons behind a constant. The [token estimate factor](/documentation/tools/tickets/token_estimate_factor.md)
is the document of record for `TOKENS_PER_LOC`; its path is compiled into `ticket_model` as
`TOKEN_ESTIMATE_FACTOR_DOC` in `tools/tickets/ticket_model/src/repository.rs`, so it never moves
without that constant.

The registry's rules for authors (statuses, ids, plans, commands) belong to the
[ticket registry README](/.ai/tickets/README.md) and the
[ticket identifiers standard](/documentation/standards/ticket_identifiers.md); the tooling's
crate boundaries to the [tooling architecture](/documentation/tools/tooling_architecture.md).

## Code

- [Ticket crates](/tools/tickets/) — the crates these documents cover.
- [Token estimates](/tools/tickets/ticket_metrics/src/estimates/) — the estimator the factor
  document governs.

## Boundaries

- Depends on: the crates' code and the `.ai/tickets/` tree, which every claim is checked against;
  the feature doc template; the ticket registry for open work.
- Used by: the `ticket_metrics` and `ticket_registry` READMEs and the estimates README, which link the factor document; the
  estimate check in `tools/tickets/ticket_metrics/src/estimates/verification.rs`, whose refusal
  message names it; and `factor_constant_is_pinned_in_the_doc`, which reads it.
- Rules: `token_estimate_factor.md` keeps its path and quotes `TOKENS_PER_LOC = <value>`, the words
  "pending calibration" and the three excluded prefixes ".ai/", "docs/TICKET_" and "Cargo.lock"
  verbatim (`cargo test -p ticket_metrics estimate_provenance`).

## Related documentation

- [Factory waves](/documentation/runbooks/factory_waves/README.md) — the ship, stamp-sha and
  repack order that writes estimates during a wave.
- [Ticketboard documentation](/documentation/apps/ticketboard/README.md) — the viewer that shows
  measured and estimated tokens apart.
