# T-161.1 — Claude Code handoff

**Slice:** T-161.1 · **Branch:** `t-161-ticket-xtask` · **CWD:**
`.ai/artifacts/worktrees/TBD-T-161`

## Context

Platform ticket CLI is Python (`scripts/lib/ticket_registry.py` ~1344 LOC +
`ticket_queue.py` + bash `scripts/ticket`). Program **T-161** ports it to a
Cargo **`xtask`** crate. This slice is **scaffold + sync/check only** — leave
Python as the `./scripts/ticket` default until **T-161.4**.

T-160 is unrelated (GpuTimer idea) — do not reuse that id.

## Preflight

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-161
git checkout t-161-ticket-xtask
git status -sb
./scripts/ticket brief T-161
```

## File map (expected)

| Path | Action |
|------|--------|
| `Cargo.toml` (workspace) | Add `xtask` member |
| `xtask/Cargo.toml` | New binary crate |
| `xtask/src/main.rs` (+ modules) | `ticket sync` / `ticket check` |
| `scripts/lib/ticket_registry.py` | **Read-only oracle** — do not delete |
| `.ai/artifacts/t161_1_verify_log.md` | Write verify evidence |

## Execution phases

1. Scaffold `xtask` + clap (or similar) with `ticket sync|check`.
2. Port load/validate/generate/inject from `ticket_registry.py` (`cmd_sync`, `check`).
3. Parity loop: Python sync → snapshot → Rust sync → `diff -u` until empty.
4. `check` + `check --strict` green.
5. Commit + tag **T-161.1**; write verify log.

## Gotchas

- Marker inject in `CLAUDE.md` / ROADMAP must preserve surrounding file content.
- `FORBIDDEN_PHANTOM_IDS` + `STRICT_LEGACY` regexes must behave the same under `--strict`.
- `gap_analysis` column sync is easy to miss — include it in the diff set.
- Workspace already has `apps/website` + `crates/map-engine-*` — do not break
  `cargo build --workspace` / wasm members.

## Return contract

- Tag **T-161.1** + commit SHA
- Verify log with A1–A5 and diff proof
- Note any intentional formatting delta (prefer none)
- Ready for Cursor: advance-slice + T-161.2 spec when operator asks
