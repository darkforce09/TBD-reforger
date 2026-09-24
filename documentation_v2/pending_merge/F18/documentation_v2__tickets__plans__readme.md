# Ticket Implementation Plans (`tickets/plans/`)

This directory houses the 206+ four-section ticket implementation plans referenced by `.ai/tickets/*.toml` ticket records via `plan = "..."`.

## Canonical Plan Template (`TEMPLATE.md`)
Every implementation plan is enforced by the ready-gate in `tools_v2/ticket-engine/src/ops.rs` and must strictly define four mandatory sections:
1. `## 1. Context & Invariants`: Preconditions, non-negotiable architectural boundaries, and problem definition.
2. `## 2. Approach`: Implementation sequence, structural refactorings, and file-by-file changes.
3. `## 3. Risks & Edge Cases`: Potential regression vectors, concurrency hazards, and mitigation strategies.
4. `## 4. Verification & Gate Evidence`: Exact automated test commands, CDP gates, and deterministic assertions.
