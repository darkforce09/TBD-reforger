# Spawn / equip determinism program

**Ticket:** T-274 (record / Makefile / docs — this hub)  
**Gate script:** [`scripts/mod/tbd-spawn-determinism.sh`](../../scripts/mod/tbd-spawn-determinism.sh)  
**Verify log (on disk):** [`.ai/artifacts/spawn_determinism_verify_log.md`](../../.ai/artifacts/spawn_determinism_verify_log.md)  
**Related MCP tooling:** [`MCP_TOOLING.md`](MCP_TOOLING.md)

This program asserts that Workbench play → spawn → equip produces the **same
player-visible outcome** across N fresh Workbench processes. It was implemented
and measured on `main` **without a ticket id on the commits**; T-274 only wires
Makefile targets + this hub. It does **not** re-run Workbench or invent new
PASS/FAIL results.

---

## How to run

**Prerequisite (hard):** a live **Arma Reforger Workbench** with the Net API
listening (default `:5775`, override `ENFUSION_WORKBENCH_PORT`). There is **no
headless / CI path** — `make ci-local` and `wave.sh gate` do **not** invoke this
gate.

```bash
# Fail-fast: is Workbench Net API up? (exit 2 + how-to if not; seconds, not minutes)
make mod-spawn-determinism-preflight

# Full gate (preflight first, then N Workbench restart + play cycles)
make mod-spawn-determinism              # RUNS=5 default
make mod-spawn-determinism RUNS=3

# Direct script (same semantics)
bash scripts/mod/tbd-spawn-determinism.sh --preflight
bash scripts/mod/tbd-spawn-determinism.sh 5 worlds/TBD_Dev_POC.ent
```

| Env | Default | Meaning |
|-----|---------|---------|
| `ENFUSION_WORKBENCH_PORT` | `5775` | Workbench Net API port |
| `TBD_DET_TIMEOUT` | `120` | Seconds to wait for `[TBD][Audit]` per run |
| `TBD_DET_KEEP` | `0` | `1` keeps `/tmp/tbd-spawn-det.*` snapshots on PASS |

Offline MCP (no Workbench): `make mcp-selftest`. Live connect smoke only:
`make mcp-smoke`.

---

## What it asserts

Per run (after a **full Workbench restart** — loader statics survive same-process
re-play, measured in the T-068.12 / determinism verify log):

1. `[TBD][Audit]` census line present; `characters == bodies`
2. `[TBD][Slots] materialized [1-9]…` present
3. Zero `path=vanilla-fallthrough`
4. Zero `SCRIPT (E)` / `Virtual Machine Exception`
5. Faction churn (`has switched from faction`) ≤ 3 total; **none after** the audit line
6. No duplicate `bound player N` lines; no `[TBD][Loadout]…(FAILED|not worn)`
7. Normalized outcome digest (volatile IDs/positions stripped; gear collapsed to
   `GEAR-ENSURED`) is **byte-identical** across all N runs

Cross-run: `sha256` of each run's normalized log must equal run 1.

---

## Evidence already on disk / git (do not invent)

| Artifact | Location / pin |
|----------|----------------|
| Gate script | `scripts/mod/tbd-spawn-determinism.sh` (introduced `7a5ab1e3`, amended `f4b25440`, `a18eeb2e`) |
| Verify log | `.ai/artifacts/spawn_determinism_verify_log.md` (298 lines tip; started 128 lines @ `7a5ab1e3`, appended @ `a18eeb2e`) |
| Program ship commit | `7a5ab1e3` — *Spawn determinism program: single authority, event equip, reaper, uid thread* (2026-07-24) |
| Materialization follow-up | `f4b25440` — *Slot-body materialization…* (2026-07-24) |
| Possess / respawn follow-up | `a18eeb2e` — *Slot-body deploy via vanilla possess request…* (2026-07-25) |
| Baseline named in verify log | `0be53e16` (T-068.12) |

**Recorded Workbench result (from the verify log, not re-measured here):**

> DETERMINISM PASS — 5/5 runs byte-identical (outcome digest `3e31fd8cf7c6`, 18
> canonical lines); census `characters=1 players=1` every run; zero
> `vanilla-fallthrough`; zero `[TBD]` errors; exactly one `spawn requested`;
> every gear item `GEAR-ENSURED`; roster `settled=failed` deterministically
> pre-LOBBY.

That PASS is historical evidence from the 2026-07-24 program run documented in
the verify log. T-274 does not claim a fresh Workbench run.

---

## Why Workbench restart (not same-process re-play)

`TBD_MissionLoader` / `TBD_RosterLoader` statics survive play sessions inside one
Workbench process. A same-process `wb_play` → `wb_stop` → `wb_play` does **not**
re-exercise fetch/settle. The harness `pkill`s Workbench and relaunches via Steam
between runs (with a game-dead boot probe + up to 3 cycles). See the verify log
§Determinism gate and §Ops findings.

---

## Explicit non-goals (this ticket)

- No registry / status sync (command center owns that after land)
- No new Enfusion / compiler / schema changes (siblings: T-237 xtask, T-275 clamps)
- Not wired into `ci-local` or automated wave gates (Workbench cannot run headless)
