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
- 2026-09-05 Wave 248 first full gate on merged main: FAIL on three steps, none in the slices'
  own code paths. `clippy xtask+tbd-tools` — 17 lints in `xtask/map_blueprint/*` untouched since
  2026-08-29 (pre-existing trunk red; completion agent on main). `test xtask+tbd-tools` — two
  registry pins broken by the command center's own bookkeeping (`active_slice` is not an on-disk
  key → T-945 filed; T-257 dropped from T-212's deps → restored) — fixed in `021a711eb`.
  `test api` — T-311's in-crate DB test tripped the T-542 "no raw TEST_DATABASE_URL under src/"
  pin, which `--slice` gates do not run (completion agent moves it to `tests/`). Lesson recorded:
  the command center runs `cargo test -p xtask` after every registry edit before dispatch.
- 2026-09-05 Wave 248 full gate, second run (after the machine restart): 30/31 PASS, red on
  `test xtask+tbd-tools` — `map_world_los::tests::world_parity_world_column_clears_its_floor_when_the_dem_is_present`
  NotFound on a fixture that exists. A cwd race, not a slice defect: wave/land tests
  `set_current_dir` into throwaway roots carrying `.ai/tickets/ROOT` (under their own lock), and
  `gate_setup_client_addons::run_reads_home_env` did the same with no lock; every fixture helper
  walking `find_repo_root()` from the cwd at that instant resolved the throwaway root. 3/4 red in
  the gate's cold `target-gate-tools`, never in isolation, never in the warm cache. Fix (command
  center, harness): `root::built_repo_root()` from `CARGO_MANIFEST_DIR` for the fixture/asset
  helpers, and `run_in(root)` so the HOME test injects its root instead of chdir. 3/3 green after.
  target dir, so the binary a checkout runs may have been compiled from a SIBLING worktree; the
  constant then pointed there. Measured 2026-09-05: five `map_world_los` pins failed
  `parse …/worktrees/T-943/packages/map-assets/…/t_picea_abies_0_canopy.bvh: bad magic
  [118, 101, 114, 115]` — "vers", the head of an LFS pointer. That is the T-742 cross-worktree
  false-binary class arriving through a constant. Replaced in T-946 by `root::test_repo_root()`:
  the cwd walk, resolved under `wave::testcwd`'s lock — the READER half of a contract that module
  already documented for the movers. All ten fixture readers use it (the wave 248 verifier's F1
  was that 9aa2acb23 had converted only two).

- 2026-09-06 Wave 248 could not be closed at all, and three defects had to line up for it (T-946,
  `7f97c043c`). (1) `wave::ledger::Registry::load_repo` went through the phase-2 loader, which
  walks `is_parent_id` only, so every slice id read as unshipped and close refused
  `wave 247 still open: T-934.1` about a ticket whose file says `status = "shipped"`. (2) The lock
  reserved one label per emptied wave and numbered past it, reaching 248, while the close oracle
  accepts only `highest standing claim + 1` = 235 — no label the lock proposed was writable.
  (3) `ticket ship` repacked after every id, so a wave shipped one ticket at a time shrank between
  ships and no repack ever saw its whole set landed; wave 248 left no pending entry at all.
  Fixes: the typed corpus for the close-time view, `carry_emptied` re-seats pending labels on the
  marker ledger (frozen SETS stay frozen), `ticket ship --no-repack` + one repack per wave, and
  `wave --close --tickets <ids>` for a set the lock has already lost. Consequence to remember when
  reading these notes: THE RUN'S WAVE NUMBERS MOVED. What this file calls wave 248 closed as
  **wave 235**; the next open wave is **236**, not 249.

- 2026-09-06 Wave 248 verifier findings filed, none fixed in-wave (no BLOCKER, none can lose
  authored work): T-947 world-parity DEM skip is green with zero assertions; T-948 audit_notify's
  six DB tests pass with no database while T-311 wrote the opposite rule in the same wave; T-949
  the leaderboard paging test cannot fail on its named claim; T-950 the audit SSE stream has no
  client; T-951 the listener registry is never pruned and is keyed on a recyclable address; T-952
  `set_var` mutates a shared 281-test process; T-953 migrations jump 0021→0025.

- 2026-09-06 **WAVE 235 CLOSED** (`b6a3cfd89`), the wave this file has been calling 248. Gate PASS
  32/32 on `d4caed957`; adversarial verifier ran on merged main and found no BLOCKER; findings filed
  as T-947…T-953. Closed with `wave --close --tickets T-934.1,T-940.5,T-940.6,T-311` — the honest
  span since `1d3253ca8`, since the lock had lost the trio's membership to per-id repacks. Both the
  gate and the close needed `TBD_GATE_BASE_CONFIRM`: oracle 2 cannot corroborate a base whose wave
  has no rows in `wave.lock`, which is the price of the pre-T-946 numbering and should stop being
  paid now that labels and the ledger agree.
