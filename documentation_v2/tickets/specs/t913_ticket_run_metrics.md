# T-913 — Ticket run metrics

Program hub: lifecycle stamps on the ticket (`created_at` / `completed_at`); per-run observations in `.ai/tickets/metrics/<id>/<ts>-<sha>.json` including **required** `tokens_consumed`; `cargo xtask platform slice-run` is the producer.

**Do not write the `STRICT_LEGACY` phrase matching `Track [ABC]\b` in this spec, the ticket title/summary/notes, README, or any commit subject/body that `ticket sync` copies into `docs/TICKET_*.md`.** Scanner: `xtask/src/constants.rs`.

Design contract: approved plan “Metrics (phase 3)”. Depends on **T-912** landed (land already stamps git_sha/outcome and, after T-912.2, runs `wave repack`).

## LIMITS (honest coverage)

In-chat Task dispatch (command-center subagents) is **not** captured. Coverage is the `platform slice-run` / `ticket run` harness and `platform wave land` stamps. Do not claim 100% factory-token accounting. Do not coerce missing usage to `0`.

Factory lands are **strict** (T-913.2): `platform wave land` refuses any landing ticket that has no run file under `.ai/tickets/metrics/<id>/`. The escape hatch for command-center / manual bookkeeping lands is the explicit **`--bookkeeping`** flag on `platform wave land` — it waives the receipt requirement for that invocation, stamps only receipts that already exist, and never fabricates a run file or token counts. Shipped-schema note (supersedes the field list in must-ship 2, per the T-913.2 dispatch prompt): required keys are `id`, `agent`, `started`, `tokens_consumed`; `finished`, `outcome` and `git_sha` are optional stamps that `platform wave land` writes; `ended`/`elapsed_sec` are not stored — elapsed is derived as `finished − started` at query time by `ticket metrics`.

## Slices

`parallel_ok` is **false**. Both slices own xtask ticket writers and `.ai/tickets/`. Serial waves only. T-913.1 depends on T-912.2 landed; T-913.2 depends on T-913.1 landed.

| Id | Status at mint | What |
|----|----------------|------|
| T-913.1 | queued | `created_at` / `completed_at` on the ticket (RFC 3339 UTC). Deliberate `ALLOWED_NEW` widen. Writers: `ticket add` / `ticket ship` / `cmd_done` / `set-status cancelled`. No backfill. `shipped_at` stays a SHA. |
| T-913.2 | queued | Per-run JSON files + `platform slice-run` producer + land stamps + `ticket metrics --by agent`. Schema and writer in the **same commit**. |

## T-913.1 must-ship

1. Optional `created_at` / `completed_at` on the ticket file (RFC 3339 UTC). Malformed = parse error, not `unwrap_or(now)`.
2. Deliberate commit that **says so**: `ALLOWED_NEW = {created_at, completed_at}`. Test: every on-disk TOML key ∈ mapped encoding-C keys ∪ `ALLOWED_NEW`. Widen `.ai/tickets/schema.json` + `TicketFile` + Work/Program types in **that same commit**. Do not weaken the old 27-key mapped set.
3. `ticket add` writes `created_at`. `ticket ship` / `cmd_done` / `set-status cancelled` write `completed_at`.
4. No guessed backfill on existing shipped tickets. `shipped_at` remains a commit SHA.

### T-913.1 proofs (paste stdout)

1. `ticket add` writes RFC 3339 `created_at`; ship/cancel writes `completed_at`.
2. Malformed timestamp → parse error.
3. Plant `created_at` without widening `ALLOWED_NEW` / schema → check red; the widen commit is what makes it green.
4. Existing shipped tickets still lack `created_at` (no backfill).
5. `cargo xtask ticket check --strict` prints `check OK`.
6. `cargo xtask platform wave gate --slice T-913.1` prints `GATE: PASS`.

## T-913.2 must-ship

1. Path `.ai/tickets/metrics/<id>/<ts>-<sha>.json` — **never** a single jsonl.
2. Schema (required): `id`, `agent`, `started`, `ended`, `elapsed_sec`, `outcome`, `tokens_consumed` `{input,output,cache_read,cache_write,total}`. Optional: `git_sha`, `tokens_consumed.reasoning` (not in `total`).
3. `total == input + output + cache_read + cache_write`. Cursor `reasoningTokens` if present is the optional sibling only.
4. `cargo xtask platform slice-run <id>` is the producer (`ticket run` is an alias). Invoke `agent --output-format json` or `claude --print --output-format json`. Pin JSON keys with a **recorded fixture** in-tree (do not guess at merge). Mine `git show 6ae8069c:xtask/src/slice_run.rs` and `metrics.rs` — re-derive against encoding C.
5. Process exit 0 but no usage object → **fail the run**, write no file, never `tokens_consumed: 0`.
6. `platform wave land` stamps `outcome` + `git_sha` on the run file the harness created, then runs `wave repack` (T-912.2 lifecycle a). Does not invent tokens. Factory land without a preceding run file is red.
7. `ticket metrics --by agent` sums elapsed and `tokens_consumed.total` over real files. Printing `tokens=0` for a missing object is forbidden.
8. Schema and writers land in the **same commit**. Empty metrics dir with no writer is a vacuous green — banned.

### T-913.2 proofs (paste stdout)

1. Fixture CLI JSON with usage → written file has matching `input` and `total` per the sum rule.
2. Fixture CLI JSON with **no** usage object → `slice-run` exit ≠ 0; no metrics file.
3. Two parallel lands T-AAA / T-BBB → two files; neither ticket TOML changes.
4. Two runs in one second → two files (sha or increment suffix).
5. Malformed `started` / missing `id` / missing `tokens_consumed` → `ticket check` red.
6. Three run files (tokens 100+50, 20) → `ticket metrics --by agent` prints elapsed **and** summed totals.
7. LIMITS paragraph present in this spec.
8. `cargo xtask ticket check --strict` prints `check OK`.
9. `cargo xtask platform wave gate --slice T-913.2` prints `GATE: PASS`.

## Illegal in this program

Wave lockfile / TSV delete (T-912). Putting tokens on the ticket TOML. Backfill. Restoring `scratch/track-b-draft`. Claiming in-chat Task tokens are captured. Pushing to origin.

## Claude Code prompt — T-913.1 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-913.1** — Ticket timestamps created_at and completed_at.

═══ PREFLIGHT ═══
  CWD = the T-913.1 worktree. T-912.2 must already be on this main.
  export CARGO_TARGET_DIR=<worktree>/target-ctr
  Do not push. Do not write Track [ABC] in synced prose.

═══ DO ═══
  Add created_at / completed_at (RFC 3339 UTC). ALLOWED_NEW widen in a
  deliberate commit that says so. Writers: ticket add / ship / done / cancel.
  No backfill. shipped_at stays a SHA. Paste proofs. GATE: PASS.

═══ DO NOT ═══
  Implement slice-run or metrics JSON files (T-913.2).
  Coerce bad timestamps to now. Put tokens_consumed on the ticket.
```

## Claude Code prompt — T-913.2 (copy-paste)

```
Read CLAUDE.md first.

Implement **T-913.2** — Per-run metrics files and slice-run producer.

═══ PREFLIGHT ═══
  CWD = the T-913.2 worktree. T-913.1 must already be on this main.
  export CARGO_TARGET_DIR=<worktree>/target-ctr
  UNSET CARGO_TARGET_DIR before wave gate. cargo xtask db up first.
  Mine git show 6ae8069c:xtask/src/{slice_run,metrics}.rs — never restore.

═══ DO ═══
  Per-run JSON under .ai/tickets/metrics/<id>/. tokens_consumed REQUIRED.
  platform slice-run is the producer; missing usage fails closed.
  Land stamps outcome+git_sha then wave repack. ticket metrics --by agent
  sums real files. Schema and writer same commit. LIMITS stay in this spec.
  Paste proofs. GATE: PASS.

═══ DO NOT ═══
  Single jsonl. tokens=0 for missing usage. Capture in-chat Task tokens
  and claim complete coverage. JSON wave.lock. Push.
```
