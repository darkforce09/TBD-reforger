# Ticket command services

Owns ticket queries, briefs, prompts, mutations, shipping, batch selection, and configuration lookup. Mutations use `Corpus` and `ops`; registry projections remain read-only. Batch execution receives a callback from the host. `cleanup_targets` resolves paths and branches without deleting them. Agent invocation and cleanup side effects belong to xtask.
