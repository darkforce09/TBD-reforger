# September 2026 factory run — RESUME HERE (command center = Claude Code)

Operator ask (2026-09-04): "get everything done that needs to be done" — the map storage spec, the
master audit, all idea tickets. Plan approved: `~/.claude/plans/spin-up-20x-agents-breezy-gadget.md`
(3 agents per wave, waves back to back, plain-language wave list). Operator decisions: Reforger only ·
mod `.c` scripts agent-editable, gate `cargo xtask mod compile`, in-game checks on the eye-pass list ·
rkyv · auto-continue waves · deferred on operator word: T-137, RadioManagerEntity world edit, in-editor
"Play scenario" · all six "frozen scope" tickets approved 2026-09-05 · **one wave at a time** (token cap).

## THE WAVE NUMBERS CHANGED — read this before anything else
T-946 (2026-09-06) re-seated the lock's wave labels on the close-marker ledger, which they had run
**13 numbers ahead of**. Everything this run called "wave 248" closed as **wave 235**, and the wave
after it as **236**. Older notes in `docs/platform/FACTORY_RUN_2026-09.md` still say 248 — that is the
same wave. Do not try to reconcile them by renumbering anything; the ledger is right now.

## State at last save (2026-09-06)
- **wave 235 CLOSED** `b6a3cfd89` — T-940.5 DB pool config, T-940.6 audit LISTEN/NOTIFY, T-311
  leaderboard tie-break, T-934.1 reorg A1. Gate PASS, verifier clean, pushed.
- **wave 236 CLOSED** `35328a7b1` — T-305 pak offsets are absolute, T-298 tbd-tools density lane in
  CI, T-943 push guard deadlock. Gate PASS twice, verifier's two harness MAJORs fixed, pushed.
- **wave 237 DISPATCHED** — T-300 (shared target dir serves unmerged binaries), T-935.1 (world::binary
  POD + rkyv archives), T-277 (27.4% of the map catalogue unclassified). Briefs at
  `.ai/artifacts/editor_briefs/sept2026/wave237/`, worktrees live, three agents running.
- Open findings filed, none fixed: T-947…T-953 (wave 235 verifier), T-954…T-958 (wave 236 verifier).
  **T-957 is the one to look at**: `apps/mod/vanilla_reference` is 2,483 files rotated by the
  pre-T-305 pak reader, and the committed `enf-index` TSVs were built over them. Re-extraction wipes
  and rewrites a committed artifact tree, so it needs an operator word.

## THE BRIDGE — every cargo command runs on the host
This session runs inside the `claude-desktop` container (glibc 2.36). The shared cache
`/home/Samuel/.cache/tbd-target` is stamped for the HOST toolchain (glibc 2.43); running cargo in the
container against it is the GLIBC_2.xx trap (T-853). Two wrappers exist for this, and every brief
points agents at them:
```
/home/Samuel/.cache/tbd-bin/hcargo <cargo args>      # forwards CARGO_TARGET_DIR, CARGO_BUILD_JOBS,
/home/Samuel/.cache/tbd-bin/hrustfmt <args>          # TEST_DATABASE_URL, TBD_IT_BASE_DB
```
`podman` and `git-lfs` differ by side: `podman` is HOST only (`distrobox-host-exec podman exec
tbd_reforger_db psql …`); `git push` must run IN THE CONTAINER, because the host's git-lfs is a
Homebrew binary absent from the non-interactive PATH (that is T-955).

## The loop, as it actually runs now
```bash
cargo xtask db up                      # host podman
# dev servers, host, detached:  cargo xtask mk rust-api  ·  cargo xtask mk leptos-debug
cargo xtask platform preflight         # PASS (worktree warnings are fine mid-wave)
```
Per wave: `platform wave status` → 3 tickets → `platform slice-worktree -- new T-xxx` each → briefs
from the wave-236/237 files as the template (they carry the bridge block and the traps) → 3 Agent-tool
slice agents **with `model: opus` set explicitly** → on report: reject-table + three checks
(`git log`, `git diff --stat $(git merge-base main HEAD)..HEAD`, worktree clean) →
`platform wave land --bookkeeping` → per id `ticket ship` then commit then `ticket stamp-sha <land sha>`
→ **cold DB + detached full gate** → verifier agent → `platform wave verified $(git rev-parse HEAD)`
→ `wave --close` → ledger row in `docs/platform/FACTORY_RUN_2026-09.md` → drop worktrees →
`git push origin main` (from the container) → next wave.

## Traps learned this run
- **Gates and closes need `TBD_GATE_BASE_CONFIRM=<newest close marker>`** while the lock has no rows
  for the base's own wave. This should stop being needed now that labels track the ledger.
- Run gates **detached** (`setsid nohup … > log 2>&1 &`) and poll the log: Claude Code kills
  background children when free memory is low. A full gate is ~30 min, 31 steps.
- `ticket ship` repacks per id, and its repack **inherits the lock's width** since T-946 — but the
  wave still dissolves id by id, so `wave --close --tickets <ids>` is how you close a wave whose
  membership the lock has lost. `ship --no-repack` + one repack at the end is the batch path.
- `cargo test -p xtask` after EVERY registry edit; `ticket check` must be exit 0 before a close.
- A slice worktree's `packages/map-assets` payloads are LFS POINTERS, so ~7 xtask map tests fail
  there with `bad magic [118, 101, 114, 115]` ("vers"). Environmental, not a finding.
- The shared cache can hand an agent ANOTHER worktree's test binary — that is T-300, in flight now.
- Rate limits: the Fable cap cut all three agents at once on 2026-09-05. `SendMessage` was NOT
  available in that session, so agents could not be resumed by id and had to be respawned; an
  agent's uncommitted work is then unreviewed and should be reverted, not inherited.
