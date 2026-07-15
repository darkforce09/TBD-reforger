# T-161 — Ticket CLI: Python → Rust xtask

**Status:** program hub · **ACTIVE:** **T-161.1** (xtask scaffold + sync/check parity) ·
**Worktree:** `.ai/artifacts/worktrees/TBD-T-161/` @ `t-161-ticket-xtask`  
**Handoff:** [`.ai/artifacts/t161_1_claude_code_handoff.md`](../../.ai/artifacts/t161_1_claude_code_handoff.md)

## In one sentence

Replace the Python ticket pipeline (`scripts/lib/ticket_*.py` + bash `scripts/ticket`) with a
Cargo **`xtask`** binary in the workspace so `./scripts/ticket …` / `make ticket-*` run on Rust —
same CLI, same derived docs, no Python runtime for everyday ticket ops.

## Why

- Ticket tooling is core platform infra (~1.3k LOC in `ticket_registry.py` alone) and already
  sits next to a Rust workspace (`Cargo.toml` members: website + map-engine crates).
- Agents and CI should not depend on a host `python3` for `sync` / `check` / `brief` / `prompt`.
- One language for repo tooling + map-engine matches the T-145 / T-151 / T-159 Rust direction.

## Authority

| Doc | Owns |
|-----|------|
| **This hub** | Slice order, locked decisions, out of scope |
| [`.ai/tickets/README.md`](../../.ai/tickets/README.md) | Operator recipes (update on flip) |
| [`.ai/tickets/schema.json`](../../.ai/tickets/schema.json) | Registry shape (unchanged) |
| Python libs (until deleted) | **Oracle** for parity gates |

## Locked decisions

| ID | Decision |
|----|----------|
| **D1** | New workspace member `xtask/` (binary crate). Invoke via `cargo xtask <args>` and a thin `./scripts/ticket` wrapper that forwards to it after the flip. |
| **D2** | CLI surface stays: `sync`, `check [--strict]`, `brief`, `prompt`, `show`, `next`, `add`, `remove`, `reorder`, `ship`, `mark-ready`, `advance-slice`, `milestone`, `plan-batch`, `list`, `run`, `done`, `clean`, plus queue helpers used by the bash pipeline. |
| **D3** | Derived outputs stay byte-identical in intent: `docs/TICKET_*.md`, `docs/MILESTONES.md`, `.ai/tickets/queue.json`, CLAUDE status markers, ROADMAP next markers, gap_analysis ticket column. Parity gate = run both oracles on the same registry and diff. |
| **D4** | `registry.json` + `schema.json` remain source of truth — Rust validates against the same schema. |
| **D5** | Work on branch **`t-161-ticket-xtask`** in standing worktree **`TBD-T-161`** (parallel with T-159). Merge to `main` when the program ships or per-slice if operator prefers. |
| **D6** | Do **not** port one-shot / dead scripts in this program: `rewrite-doc-links.py`, `rewrite-ticket-paths.py`, `backfill-registry-monorepo.py`, `seed_registry.py` (unless still referenced by Makefile — then thin-wrap or delete with proof). |
| **D7** | Mod MCP Python (`scripts/mod/lib/mcp-*.py`) is **out of scope** (separate tooling). |

## Slice plan

| Slice | Executor | Scope | Status |
|-------|----------|--------|--------|
| **T-161.0** | cursor-docs | Hub + registry + worktree + handoff | **shipped** (this pass) |
| **T-161.1** | claude-code | `xtask` crate scaffold; port **`sync`** + **`check [--strict]`**; parity vs Python | **ready** |
| **T-161.2** | claude-code | Read path: `brief`, `prompt`, `show`, `next`, `list`, `milestone`, `plan-batch`, `sparse-paths`, `gap-round-trip` | queued |
| **T-161.3** | claude-code | Mutators: `add`, `remove`, `reorder`, `ship`, `mark-ready`, `advance-slice` + queue `set-status` / `ready-ids` / `get` / `config` | queued |
| **T-161.4** | claude-code | Flip: `./scripts/ticket` → `cargo xtask`; port `run` / `done` / `clean`; delete Python ticket libs; Makefile still green | queued |
| **T-161.5** | cursor-docs | Doc sync (README, playbook, CLAUDE); mark program shipped | queued |

Advance after each code slice: `./scripts/ticket advance-slice T-161` (Python oracle until flip).

## Inventory (Python → Rust)

| Current | Role | Target slice |
|---------|------|--------------|
| `scripts/ticket` | Bash front-door + `run`/`done`/`clean` | .1 forward via xtask where possible; .4 owns flip |
| `scripts/lib/ticket_registry.py` | sync / check / brief / prompt / mutators (~1344 LOC) | .1–.3 |
| `scripts/lib/ticket_queue.py` | queue list / ready-ids / status | .2–.3 |
| `scripts/lib/extract_claude_prompt.py` | Spec fenced-prompt extractor | .2 |
| `scripts/lib/seed_registry.py` | Legacy seed helper | D6 — delete or leave unused |
| Makefile `ticket-*` targets | Call `./scripts/ticket` | .4 verify only |

## Out of scope

- Changing ticket **schema** or inventing a new registry format
- Rewriting map-assets Node scripts
- Mod Workbench MCP Python helpers
- Leptos / map-engine application code (T-159 / T-151)

## Verify (program-level)

After **T-161.4**:

```bash
./scripts/ticket sync
./scripts/ticket check --strict
./scripts/ticket brief T-161
./scripts/ticket prompt T-161 --slice T-161.1   # or active slice
make ticket-sync ticket-check
# no remaining scripts/lib/ticket_*.py (or allowlisted stub that errors pointing to xtask)
```

## Claude Code prompt — T-161.1 (copy-paste)

Authority: this hub + [`t161_1_xtask_scaffold_sync.md`](t161_1_xtask_scaffold_sync.md) + handoff.
**Do not edit docs/registry** (except generated outputs via `sync`).

```
Read CLAUDE.md first.

Implement **T-161.1** — xtask crate + ticket sync/check parity.

═══ PREFLIGHT ═══
  CWD: .ai/artifacts/worktrees/TBD-T-161
  git status -sb && git branch --show-current   # expect t-161-ticket-xtask
  ./scripts/ticket brief T-161

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t161_1_claude_code_handoff.md
  2. docs/platform/t161_1_xtask_scaffold_sync.md
  3. docs/platform/t161_ticket_xtask_program.md  (§Locked)
  4. scripts/lib/ticket_registry.py  (cmd_sync, check, generate_*)
  5. .ai/tickets/schema.json

═══ PROBLEM ═══
  Ticket sync/check are Python-only. Add workspace xtask and port sync + check so
  Rust can regenerate the same derived docs and validate the registry (incl. --strict).

═══ LOCKED ═══
  - D1: crate path xtask/ (workspace member)
  - D3: sync outputs match Python oracle (diff gate)
  - D4: schema.json validation
  - D5: work only on t-161-ticket-xtask worktree
  - D6/D7: no one-shot / MCP Python ports
  - Keep ./scripts/ticket calling Python until T-161.4 — add cargo xtask ticket sync|check alongside

═══ DO ═══
  1. Add xtask binary crate to Cargo workspace (clap or equivalent)
  2. Implement `cargo xtask ticket sync` and `cargo xtask ticket check [--strict]`
  3. Parity: Python sync → snapshot; Rust sync → same paths; diff must be empty (or document intentional whitespace-only if unavoidable — prefer empty)
  4. `cargo xtask ticket check` and `check --strict` exit 0 on current registry
  5. Tag **T-161.1** · commit prefix **T-161.1:**

═══ DO NOT ═══
  - Edit docs/** (except via sync generators), registry.json by hand for “cleanup”
  - Delete Python yet (T-161.4)
  - Port brief/prompt/mutators/run (later slices)
  - Touch apps/website frontend or map-engine behavior

═══ VERIFY (all exit 0) ═══
  cd .ai/artifacts/worktrees/TBD-T-161
  cargo xtask ticket check
  cargo xtask ticket check --strict
  # parity recipe in slice spec §Verify
  cargo build -p xtask
  cargo clippy -p xtask -- -D warnings

═══ RETURN ═══
  - Commit SHA + tag T-161.1
  - .ai/artifacts/t161_1_verify_log.md
  - Diff proof (Python vs Rust sync)
  - Ready for Cursor doc sync / advance-slice
```
