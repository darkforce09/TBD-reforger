# T-161.1 — xtask scaffold + sync/check parity

**Ticket:** T-161.1 · **Program:** [T-161 hub](t161_ticket_xtask_program.md) · **Status:** ready  
**Executor:** claude-code · **Worktree:** `.ai/artifacts/worktrees/TBD-T-161` (`t-161-ticket-xtask`)  
**Handoff:** [`.ai/artifacts/t161_1_claude_code_handoff.md`](../../.ai/artifacts/t161_1_claude_code_handoff.md)

## Goal

Stand up a Cargo **`xtask`** workspace member and port **`ticket sync`** + **`ticket check [--strict]`**
so Rust reproduces the Python oracle’s derived outputs and validation.

## Scope

### In

1. **`xtask/`** binary crate — workspace member in root `Cargo.toml`.
2. CLI: `cargo xtask ticket sync` and `cargo xtask ticket check [--strict]`
   (subcommand nesting is fine; aliases OK if documented in verify log).
3. Port of:
   - Registry load/save
   - JSON Schema validation (`.ai/tickets/schema.json`)
   - All `generate_*` markdown / `queue.json` writers used by `cmd_sync`
   - CLAUDE.md status-marker inject + ROADMAP next-marker inject
   - `gap_analysis` ticket-column sync (same behavior as Python)
   - `check` / `check --strict` (including legacy-ID scan rules)
4. Verify log at `.ai/artifacts/t161_1_verify_log.md`

### Out

- `brief` / `prompt` / mutators / `run` / deleting Python (T-161.2–.4)
- Changing registry schema or ticket IDs
- Mod MCP Python

## Locked (slice)

| ID | Decision |
|----|----------|
| **S1** | Python remains the default `./scripts/ticket` path until T-161.4. |
| **S2** | Parity gate is mandatory: after both syncs, listed derived files must match. |
| **S3** | Prefer `serde` / `serde_json`; **no** JSON Schema crate (Python `load_schema` unused). |
| **S4** | Do not invent a second registry format. |

## Derived outputs (must match)

| Path |
|------|
| `docs/TICKET_REGISTRY.md` |
| `docs/TICKET_LEAD.md` |
| `docs/TICKET_DEV_QUEUE.md` |
| `docs/TICKET_BRAINSTORM.md` |
| `docs/TICKET_MOD_QUEUE.md` |
| `docs/MILESTONES.md` |
| `.ai/tickets/queue.json` |
| `CLAUDE.md` status markers (`<!-- ticket-sync:status:* -->`) |
| `docs/specs/Mission_Creator_Architecture/ROADMAP.md` next markers (if present) |
| `docs/specs/Mission_Creator_Architecture/eden/gap_analysis.md` ticket column |

## Verify / Rebuild

```bash
# From worktree root
cargo build -p xtask
cargo clippy -p xtask -- -D warnings

# Baseline (Python oracle) — stash copies
./scripts/ticket sync
mkdir -p /tmp/t161-py && cp docs/TICKET_*.md docs/MILESTONES.md .ai/tickets/queue.json /tmp/t161-py/
# also copy CLAUDE.md + gap_analysis if check depends on them

# Rust sync (overwrites same paths)
cargo xtask ticket sync

# Diff (must be empty)
diff -u /tmp/t161-py/TICKET_LEAD.md docs/TICKET_LEAD.md
# …repeat for each derived file; or a small shell loop in the verify log

cargo xtask ticket check
cargo xtask ticket check --strict
```

Restore working tree cleanliness: if Rust sync matches Python, `git status` on those
derived files should be clean relative to the post-Python sync state (no spurious churn).

## Acceptance

| ID | Criterion |
|----|-----------|
| A1 | `xtask` in workspace; `cargo build -p xtask` green |
| A2 | `cargo xtask ticket sync` regenerates all listed outputs |
| A3 | Diff vs Python sync is empty for all listed paths |
| A4′ | `check` / `--strict` exit + sorted ERROR set match Python (not exit 0) |
| A5 | Tag **T-161.1** on the worktree branch |

## Claude Code prompt — T-161.1 (copy-paste)

See hub [`t161_ticket_xtask_program.md`](t161_ticket_xtask_program.md) §Claude Code prompt — T-161.1
(same block; keep in sync if this slice drifts).
