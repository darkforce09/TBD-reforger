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

- 2026-09-06 **WAVE 236 CLOSED** (`35328a7b1`): T-305 pak entry offsets are absolute, T-298 the
  tbd-tools density lane runs in CI, T-943 the push guard no longer deadlocks. Gate PASS twice —
  once at the landing tree and again after the harness fixes below, because code changed between
  them. Closed with `--tickets`, since the wave's own ids had already left the lock.
- 2026-09-06 Wave 236 verifier: two MAJORs against the COMMAND CENTER's own T-946 work, both fixed
  in `49569e0ff`. (1) `ticket ship`'s repack hook re-shaped a wave that was being run: it repacks
  with no environment and `max_concurrent()` defaults to 8, so a 3-wide wave 236 came back holding
  eight different tickets and had nothing left to close. `compile` now inherits the previous lock's
  width unless `TBD_MAX_CONCURRENT` asks otherwise. (2) The `--no-repack` waiver swallowed
  `missing_lock_error`, which carries the same "run `cargo xtask wave repack`" phrase and is
  returned ALONE ahead of every other lock check — so `ship --no-repack` could write ticket status
  with no plan on disk. Excluded by identity. A third lesson rode along: the first width test used
  `set_var("TBD_MAX_CONCURRENT")` and made a sibling fail one run in three, so `compile_with_cap`
  exists and no test mutates that variable.
- 2026-09-06 Wave 236 findings filed: T-954 check-attr answers are never counted against the paths
  fed; T-955 the git-lfs probe misses the operator's Homebrew install, so T-943's normal-push branch
  never runs through the factory PATH; T-956 `ci-local` claims to mirror ci.yml and no longer does;
  **T-957 `apps/mod/vanilla_reference` is 2,483 rotated files from the pre-T-305 reader and the
  committed enf-index TSVs were built over them** — filed, not fixed, because re-extraction wipes
  and rewrites a committed artifact tree; T-958 a stale test count in a comment.

- 2026-09-06 **WAVE 236 CLOSED** (`4f2d4598f`) — T-300 the run target and its build-provenance
  stamp, T-935.1 the `world::binary` formats, T-277 the map catalogue classified (27.3% fallback to
  2.35%). The FIRST close of this run that needed neither `--tickets` nor `TBD_GATE_BASE_CONFIRM`:
  the wave was batch-shipped (`ship --no-repack` ×3 then ONE repack), so it froze its whole set as
  a pending entry with its own reserved label, and oracle 2 corroborated from that entry.
- 2026-09-06 The FIRST wave-236 marker (`35328a7b1`) was DISAVOWED (`36f462d3f`). Its three tickets
  had shipped one at a time, no pending entry formed, the repack handed the freed label 236 to the
  next batch, and `--close --tickets` wrote the marker over a label the plan still called open.
  Oracle 2 reads the lock at the marker's PARENT and refused every later gate. The label was wrong;
  the work was not — T-305, T-298 and T-943 stayed shipped and stamped, and their span was re-gated
  from wave 235's close. A close marker carries no diff, so `git revert` produces nothing to commit
  and the disavowal trailer had to be written by hand. Guarded now: `--close --tickets` refuses a
  label the lock still calls open.
- 2026-09-06 Wave 237 verifier: FOUR MAJORs, three fixed in-wave. (1) `chunk_container.rs` computed
  every length with unchecked `as usize *`, and `usize` is 32-bit on wasm32 — THE LOADER'S OWN
  TARGET — so a header claiming 2^27 instances matched an EMPTY payload there while erroring on
  x86_64. Proven on a real wasm32 build under node. (2) `reclassify` preserved `needsReview`
  wholesale, so after T-277 classified 420 of 443 the artifact still published
  `needsReview.prefabTypes = 443` — the ticket's own headline number — and `verify type-inventory`
  is shape-only. (3) `byKind.road.segments` and `byRoadClass` were literally hardcoded to 0 and
  `{}`: roads export as prefab-less `RoadEntity` rows that classification never sees, so T-277's
  requirement 2 was unreachable from its owned file. New `build::road_census` derives both from the
  committed `roads.json.gz` — 887 segments, five classes — and corrected five places claiming 888,
  one of them a live browser assertion. (4) FILED as T-963: the run stamp describes a directory
  rather than each binary, and ignores a dirty tree; the run lane has no caller yet, so it blocks
  T-959 instead. Minors/nits T-964…T-969.

- 2026-09-06 **WAVE 237 CLOSED** (`ad9b22890`) — T-924 the gate-verdict receipt, T-935.5 TBDD
  decode by `cast_slice` (bit-exact over all 625 committed tiles), T-935.2 chunk dual-emit (parity
  over all 315 chunks, 1.2 M instances). Batch-shipped again, closed from its own pending entry
  with no `--tickets` and no `TBD_GATE_BASE_CONFIRM`.
- 2026-09-06 Wave 237 verifier: a **BLOCKER — the gate reported success over code it never
  compiled.** `wasm_changed` and the `trunk build` step both scoped by the path prefix
  `apps/website/frontend/`, but the SPA compiles `map-engine-core` and `map-engine-render` into its
  own wasm binary. This wave rewrote `map-engine-core`'s TBDD decode and made `bytemuck`
  unconditional there, so the gate printed `wasm32 (frontend) PASS` beside `trunk build SKIP
  (frontend untouched this wave)` having compiled none of it — and `Runner::run` discards a passing
  step's output, so the reason never reached the log. Main was not broken (verified by hand, rc 0).
  The scope is now DERIVED by walking `path = "…"` deps out of the frontend's manifest; the re-gate
  ran both steps for real. Fixed in `2a94af3ac`, with the old prefix rule kept as the perturbation.
- 2026-09-06 Same verifier, two MAJORs against T-924 — the guard shipped ONE WAVE EARLIER and
  already had holes. (1) It covered `platform wave land` only; `slice-worktree merge` is the second
  door to main and `mod wave land` calls it, so any slice could still merge ungated. The check moved
  to the chokepoint both share, and five fixtures that had been merging ungated now fail closed.
  (2) A non-canonical id (`slice/T-247-hotfix`, a shape this repo creates) hit `path_for`'s refusal
  and was told to "re-gate" — advice with no exit, since the gate refuses the same shape in the same
  place. Both fixed in `2a94af3ac`. Filed: T-974, T-975. Noted: the guard could not run on its own
  wave (the in-flight `land` binary predated its merge), so wave 238 is its first real exercise.

- 2026-09-06 **WAVE 238 CLOSED** (`4f7a0daa7`) — T-935.4 the raw TBDE DEM (emitted beside the PNG,
  streamed into one `Vec<u16>`), T-935.7 the label archive (one rkyv fetch for towns and road
  names), T-935.8 the building-blueprint archive and occluder boot. All three inherited a
  predecessor killed mid-ticket by the session limit; the successors were told they OWN that code
  and must verify, perturb and gate it themselves, and all three found something in it.
- 2026-09-06 THE SESSION LIMIT KILLED ALL THREE AGENTS MID-TICKET, and one died holding a live
  perturbation — a `core::mem::swap` of two header fields, still applied. Committed work was kept
  (1085 / 980 / 1236 lines); every uncommitted diff was reverted, which removed that break and two
  half-written loaders. `SendMessage` is unavailable in this session, so the originals could not be
  resumed by id and fresh agents inherited the branches instead. The method note that justifies
  the whole approach: T-935.4's own perturbation made `RawDemSink::finish` clone rather than move —
  a real second 81.9 MB allocation, semantically invisible — and ALL TWELVE inherited tests passed
  over it. Only the pin the successor added caught it.
- 2026-09-06 Wave 238 verifier: four MAJORs, two fixed in `47ecabf6f`. (1) `BuildingArchiveBytes`
  was accepted on LAYOUT alone — `access_checked` with no `schema_version` read, while its sibling
  `map_labels_from_bytes` checked it in the same wave. A shifted `blocks` bit would seed `no_block`
  for the wrong pids and those buildings would stop occluding for the session with nothing logged.
  (2) `with_archive_blas` ran AFTER the awaited descriptor fetch, so everything it could add was
  already named by `wanted()` and the rest belonged to descriptors that had failed — no round-trip
  bought, real fetches and 48 MB-budget bytes spent on geometry that could never attach. Removed
  with its state. Also fixed: the emitter's only real-corpus proof was `#[ignore]`d so the WAVE gate
  never ran it (now conditional on the file being a pointer, and it prints `skip-lfs:`), and
  `dem_load.rs` claimed "exactly one allocation" 48 lines above its `f32` grid. Filed: T-983, T-984,
  T-985. And the re-gate caught the COMMAND CENTER formatting a 2021-edition crate with
  `--edition 2024` — the exact trap the slice briefs warn agents about.
