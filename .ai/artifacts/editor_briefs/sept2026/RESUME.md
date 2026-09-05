# September 2026 factory run — RESUME HERE (command center = Claude Code)

Operator ask (2026-09-04): "get everything done that needs to be done" — the map storage spec, the
master audit, all idea tickets. Plan approved: `~/.claude/plans/spin-up-20x-agents-breezy-gadget.md`
(3 agents per wave, waves back to back, plain-language wave list). Operator decisions: Reforger only ·
mod `.c` scripts agent-editable, gate `cargo xtask mod compile`, in-game checks on the eye-pass list ·
rkyv · auto-continue waves · deferred on operator word: T-137, RadioManagerEntity world edit, in-editor
"Play scenario" · all six "frozen scope" tickets approved 2026-09-05 · **one wave at a time** (token cap).

## State at last save (2026-09-05 evening)
- Registry: 150 dispatchable tickets ready, packed 3 per wave (`wave.lock`, 52 waves). Programs T-935
  (map storage) … T-941 (mod lifecycle), T-942 (chore, shipped), T-943 (push guard), T-944, T-945.
- Wave 248 = T-940.5 (DB pool config), T-940.6 (audit LISTEN/NOTIFY), T-311 (leaderboard tie-break):
  ALL THREE LANDED + SHIPPED + sha-stamped (8fc66b589, f5f7abc10, 7094bc2b0). Trunk fixes landed:
  36f65687e (gitignore/.mjs/hub header), 021a711eb (registry), 21cf11b32 (xtask clippy),
  2b509127b (T-311 test moved to tests/ for the T-542 pin), 9730d812d (ledger).
- Wave 248 close is PENDING: full wave gate was at step 10/30 all PASS when the machine restarted.
- Worktrees T-940.5 / T-940.6 / T-311 still exist under `.ai/artifacts/worktrees/` (drop at close).

## Close wave 248 (do in this order)
```bash
cd /run/media/system/Disk_2/Projects/TBD-Reforger
export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target CARGO_BUILD_JOBS=4
cargo xtask db up; cargo xtask mk rust-api &   # and `cargo xtask mk leptos-debug &`
cargo xtask platform preflight                 # PASS
podman exec tbd_reforger_db psql -U tbd -d postgres -c "DROP DATABASE IF EXISTS tbd_wave248_cold WITH (FORCE);" -c "CREATE DATABASE tbd_wave248_cold OWNER tbd;"
BASE=$(git rev-list --extended-regexp --grep='^wave [0-9]+ CLOSED' -1 HEAD)   # = 1d3253ca8
# run DETACHED (the Claude Code memory watchdog kills background builds): 
setsid nohup bash -c "TBD_GATE_DB='postgres://tbd:tbd@localhost:5434/tbd_wave248_cold?sslmode=disable' TBD_GATE_WAVE=248 TBD_GATE_BASE_CONFIRM=$BASE cargo xtask platform wave gate; echo GATE_EXIT=\$?" > /tmp/gate-w248.log 2>&1 &
# expect GATE: PASS. Then the adversarial verifier (brief: wave248/VERIFY.md, model opus) on merged main.
# Triage (BLOCKER fix in-wave via completion agent; MAJOR that can lose authored work fix; else file queued).
cargo xtask platform wave verified $(git rev-parse HEAD)
cargo xtask platform wave wave --close --summary "wave 248: DB pool config, audit LISTEN/NOTIFY, leaderboard tie-break; GATE PASS"
# ledger row in docs/platform/FACTORY_RUN_2026-09.md; eye-pass rows already in docs/platform/EYE_PASS_2026-09.md
for t in T-940.5 T-940.6 T-311; do cargo run -q -p xtask -- platform slice-worktree -- drop $t; done
git push origin main            # plain push works here (git-lfs on host); `platform wave push` deadlocks (T-943)
```

## Next wave (249) — the loop
`cargo xtask platform wave status` → 3 tickets → `cargo run -q -p xtask -- platform slice-worktree -- new T-xxx` each →
brief = `SLICE_BRIEF_TEMPLATE.md` filled from the ticket (`cargo xtask ticket show T-xxx`, its plan in
docs/plans/, its spec prompt block) + sibling owns → 3 Agent-tool slice agents in parallel (each with its
own `*_it` test DB via `TBD_IT_BASE_DB`, e.g. `tbd_slice_t305_it`) → on report: reject-table, three checks
(`git log main..slice/T-x`, `git diff --stat`, worktree status empty) → `cargo xtask platform wave land --bookkeeping T-x`
→ `cargo xtask ticket ship T-x` → `cargo xtask ticket stamp-sha T-x <land sha>` → after all three: gate (detached)
→ verifier → verified → close → push → next. Registry edits: `cargo test -p xtask` after every edit.

## Traps learned this run
- `platform wave push` deadlocks on LFS-heavy ranges (T-943): use `git push origin main`.
- Slice gates run in the worktree branch: merge `main` into the slice first when trunk fixes landed.
- `--slice` gates skip `test api` / T-542 pin: DB-backed tests must live in `tests/*.rs` via `common`.
- `active_slice` is not an on-disk ticket key (T-945); `cargo test -p xtask` pins registry facts (T-212 deps).
- Test DB names must match the T-381 allow-list (`*_it`, `*_cold`, `tbd_gate*`); `db test-it` uses `TBD_IT_BASE_DB`.
- Claude Code kills background commands when `free` memory is low: run gates via `setsid nohup`, poll the log.
- Rate limits cut agents; resume the same agent id with SendMessage (context intact) — never a twin.
