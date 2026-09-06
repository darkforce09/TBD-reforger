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

## State at last save (2026-09-06, late)
- **CLOSED AND PUSHED: waves 235, 236, 237, 238, 239.** 21 tickets shipped. Markers `b6a3cfd89`,
  `35328a7b1` (DISAVOWED, see below), `4f2d4598f`, `ad9b22890`, `4f7a0daa7`, `b0257e946`.
- **Wave 240 is next**: `cargo xtask platform wave status` names it. Briefs go in
  `.ai/artifacts/editor_briefs/sept2026/wave240/`; copy the wave239 ones as the template — they
  carry the bridge block, the corrected gate scope and rules 16/17.
- The first wave-236 marker was disavowed (`36f462d3f`) because its label collided with an open
  wave. `--close --tickets` refuses that now. Do not be alarmed by two `wave 236 CLOSED` subjects.
- **49 findings filed, T-947…T-998.** The ones needing an operator decision, not an agent:
  * **T-957** — `apps/mod/vanilla_reference` is 2,483 files rotated by the pre-T-305 pak reader,
    and the committed `enf-index` TSVs were built over them. Re-extraction rewrites a committed
    artifact tree.
  * **T-981 / T-985** — the building archive cannot carry the 1,322 BLOCKING prefabs (no instance
    records on the wire) and drops the boot hot-set. Both slices took the safe path; T-935.13's
    plan assumes neither limit exists. Something in that plan has to give.
  * **T-993** — the satellite BOOT path still refuses any container but v1. Flip the manifest
    without changing it and the basemap silently drops to the low-res preview.
  * **T-994 / T-995** — forest smoothing emits self-crossing rings (38 on everon), and the editor
    filters out every forest region so none of it renders. Fix the second before the first matters.

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
