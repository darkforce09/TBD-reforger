# Factory run ledger — September 2026 (map storage · audit remediation · idea backlog)

Operator ask (2026-09-04): "spin up agents in parallel and get everything done that needs to be done" —
the Map Data Storage & Binary Architecture spec, the Master Architectural Audit, all idea tickets.
Cadence: 3 slice agents per wave, waves run back to back; eye-pass checklist accumulates in
[`EYE_PASS_2026-09.md`](EYE_PASS_2026-09.md). Operator decisions: Reforger only · mod scripts
agent-editable (gate `cargo xtask mod compile`) · rkyv · deferred: T-137, RadioManagerEntity world
edit, in-editor play button.

## Waves

| Wave | Close marker | Tickets | Gate | Verifier | Outcome |
|---|---|---|---|---|---|
| 0 | — | T-942 (path sweep, two chore commits `4c07ce55b`, `33e675e84`), push `340bc3e2b..33e675e84` | preflight PASS after clean tree | — | done 2026-09-04 |

## Incidents

- 2026-09-04 `cargo xtask platform wave push` deadlocked in `git check-attr` on a 1,691-file LFS
  commit; killed, pushed with plain `git push origin main` (git-lfs 3.7.1 on host). Filed T-943.
- 2026-09-05 First slice gate of the run (T-311) exposed three trunk defects on `main`, none in
  any slice's owns: (1) `apps/website/api/.gitignore` `missions/` also ignored
  `handlers/missions/mod.rs` — main did not build from a clean checkout; (2) two tracked `.mjs`
  tools tripped `verify no-python`, so every slice gate was red; (3) schema gate 10 read a missing
  `active_slice` on T-090. Fixed by the command center in `36f65687e` (harness, not app code).
- 2026-09-05 Rate limits (claude.ai session cap) cut the planning agents three times and two slice
  agents once; resumed with context intact each time. Planning wave ran sequentially for that
  reason; slice waves stay at 3.
