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

- 2026-09-06 **WAVE 239 CLOSED** (`b0257e946`) — T-935.10 satellite TBDS v2 with an rkyv index
  (v1 still reads), T-149 forest ring smoothing, T-935.3 chunk `.bin` ingest by header check plus
  cast. Batch-shipped, closed from its own pending entry.
- 2026-09-06 Wave 239 verifier, and it found the thing the brief asked it to hunt. **T-994: the
  forest smoothing turns a SIMPLE polygon set into a SELF-CROSSING one** — 0 crossings in, 38 out
  on the committed everon corpus (37 sibling pairs, 1 self-crossing ring). The tracer legitimately
  emits pinch vertices (121 of 1022 rings repeat one), and at a pinch the two passes carry OPPOSITE
  normals, so any non-zero area-restoring offset separates them across each other. The rail never
  binds (0.19 of a mean edge against a 0.5 cap) so `offset_capped` says nothing, and a bowtie
  cancels its own area exactly — the slice's only instrument is area, so it is structurally blind
  to this. FILED, not fixed: pinning one ring's pinches does not address the sibling half, and
  **T-995** says the output is unrendered anyway (`world_host` filters out every `kind: "forest"`
  region, and all 36 everon regions are forest), so T-149's acceptance cannot be observed as
  shipped and T-988's 5.7x payload is fetched and discarded. Also filed: T-996 (the v2 satellite
  verifier cross-checks less than v1, and v2 is now the default), T-997 (two "committed everon"
  pins never open the bundle), T-998 (container version read as u16 by the SPA, u32 by the tool).
  Fixed in `ac6a8f5ba`: `world_host` discarded `ChunkBinError` through `is_ok()`, so a tile served
  under the wrong id failed closed and silently and became a permanently empty chunk — the verifier
  proved the diagnostic unreachable by grepping the gate's own wasm for its text, zero hits.
- 2026-09-06 A slice left `scratch_old_v2.txt` in the MAIN checkout (rule 8: probes go in /tmp).
  Untracked, unreferenced, and it would have blocked the close. Moved to the session scratchpad
  rather than deleted — it was not the command center's to destroy.

- 2026-09-06 **WAVE 240 CLOSED** (`52a038a77`) — T-935.6 the road network to rkyv (with the
  gzip-vs-rkyv magic sniff every later archive fetch reuses), T-935.9 water vectors plus a TBDB
  bathymetry pyramid and a placement-guard mask, T-674.1 slot identity and squad leader on the wire
  at schema 1.3. Full gate PASS, 31 steps. Closed from a RESERVED pending entry — see below.
- 2026-09-06 The wave could not be closed at all when its gate finished, and the reason is worth
  the paragraph. `carry_emptied` freezes a pending `[[emptied]]` entry only when ONE repack sees a
  whole open wave landed, and `ticket ship` repacks after EVERY id. This wave's batch broke mid-way
  on T-674.1's missing `created_at`, the per-id path finished it, and by the last ship the label 240
  had already been re-issued to T-935.11 / T-675.1 / T-676. `--close --tickets` then asked for 240,
  which a live open wave held, and the collision guard refused — correctly: oracle 2 reads the plan
  at the marker's PARENT and would have seen wave 240 assigned to unshipped tickets, refusing every
  later gate. That is exactly the shape that cost wave 236 its marker (`35328a7b1`, disavowed).
  Fixed rather than forced: `cargo xtask wave repack --reserve "<ids>"` (T-946.12, `6e72b0b4d`)
  freezes an operator-named set of SHIPPED ids as a pending target, so the label the ceremony claims
  is one no open wave holds. The repack stays the only writer of the lock; the vouching is for
  MEMBERSHIP only and every id is re-checked shipped against the tree. The entry is self-sustaining
  under later repacks, which matters because `ticket ship` runs one on every id.
- 2026-09-06 Wave 240 verifier: 11 findings, 4 MAJOR. Two fixed in `cf69afda2` — (1) `water.rs`
  `assemble` accepted a suffix LONGER than the level it claimed, so a mask built from a longer tail
  read DRY over open water with no error; (2) `chunk_container.rs` `level_dims` shifted a `u32` by
  the level with no width check, a panic in wasm at level 32. Filed, not fixed: T-946.9 (the water
  mask carries no world extent, so a stale `worldBounds` gives confident wrong answers instead of
  `Unknown`), T-946.10 (the Range helper parses only the TOTAL from `content-range`, never the
  START, so a server answering a different range is accepted), T-946.11 (the 1.3 flag latches where
  the value is computed, not where it SERIALISES — two factions whose names slug to one key
  overwrite each other and the version can say 1.3 with no key on the wire), T-946.12 (the close
  label above).
- 2026-09-06 The verifier read `e5f46e4fc` as Enfusion script, which no gate could compile (T-946.3),
  and found it CORRECT: the `SCHEMA_1_3` constant matches its three siblings in declaration, access
  and type; returning true is right because 1.3 makes `slots[]` mandatory exactly as 1.2 does; no
  other site refuses 1.3; and the two copies stay structurally identical, differing only in string
  punctuation, each obeying its own tree's ASCII rule. It also confirmed the re-aimed API test
  (`26f7a785a`) and the hand-stamped `created_at` on T-674.1 against the file's first commit.

- 2026-09-06 **WAVE 241 CLOSED** (`cd37d1a78`) — T-675.1 the authored vehicle roster onto top-level
  `vehicles[]` at schema 1.3, T-935.11 the prefab catalogue / forest regions / type inventory to
  rkyv with archive fetch branches for roads and regions, T-676 the Enfusion trigger runtime (2107
  lines: six conditions, seven effects, a 1 Hz LIVE-only tick). Closed from a RESERVED pending
  entry — the wave dissolved id by id again and `wave repack --reserve` is what repaired it, its
  first use on a live wave.
- 2026-09-06 **The mod compile gate worked again for the first time this run** (T-946.3,
  `2392a2513`). Three tbd-export WorkbenchGame scripts carried 15 em-dashes and failed the gate's
  pre-compile ASCII scan, so `cargo xtask mod compile` had never reached the compiler — which is
  why wave 240's edit to both copies of `TBD_MissionValidator.c` shipped uncompiled. Fifteen
  characters. The gate now reports `OK: compiled clean, 5740 files, 11314 classes`. **The claim
  that both validator copies are inside that count is RETRACTED — see T-946.23 in the wave 242 rows
  below.** Only the tbd-EXPORT copy was: the gate listed the export addon last, the Enfusion VFS
  overlays by path, and the framework copy — the one that ships — was never read.
- 2026-09-06 `wave land` had been aborting after the merge on every wave since the close-marker
  ledger started moving (T-946.14, `a0a762416`): its post-merge gate used pre-merge HEAD as the
  diff anchor, which the gate's own T-602 base check refuses whenever main moved after the wave's
  close marker — a ledger row and the wave's briefs are enough. The merge precedes the gate, so
  the operator was left with a landed slice, no bookkeeping and a refusal naming a base nobody
  chose. The anchor moved; the revert target kept its job and is printed under its own name.
- 2026-09-06 T-676's slice gate went RED on `schema`, and it was red BECAUSE the ticket succeeded:
  T-706's unread-wire-field invariant pins per field that a 1.3 key has no identifier of that
  spelling anywhere in the mod, and `editorTriggers` (2), `activation` (15), `effects` (6) and
  `variantId` (4) all gained readers at once. Rows RETIRED rather than re-pinned — a non-zero
  baseline in that table means a pre-existing UNRELATED identifier and a test enforces exactly that
  wording, so pinning real readers at 4/6/15 would have been a lie the test would have to be
  weakened to accept. The two schema descriptions that asserted no reader were corrected in the
  same commit, and so was `schemaVersion`'s, which had been telling every reader since e5f46e4fc
  that the shipped mod would REJECT a 1.3 document and that flatten still emits 1.1/1.2.
- 2026-09-06 Wave 241 verifier: 13 findings, 2 MAJOR. **T-946.18: one authored vehicle emits TWO
  rows and they had no join key.** T-675.1's own comment called `uid` the key; `ModEntity` had no
  such field and `$defs/entity` is `additionalProperties: false`. The mod already spawns
  `entities[]`, so a roster reader would have spawned every crewed vehicle twice. Fixed, along with
  the roster row silently TRIMMING `inventory` — a value the schema declares, the editor authors
  and the entity twin emits. **T-946.19**: `not_present` read TRUE over a live player whose slot had
  not resolved (a section mid-respawn read as an empty zone, and `not_present` + `end_mission` ends
  the round over them); `repeat` was silently inert for `timer` because a TIMER never falls; the
  heartbeat had no idempotence latch, and two timers on one instance both pass the stale-timer
  defence, halving every authored `timeoutSeconds`. Also fixed: `archives.rs` documented
  `half_extents` as zero-when-absent while the writer spells NaN (zero is a LEGAL half-extent), and
  the `by_kind` census order was pinned only at its ends in both crates. Filed: T-946.20 (trigger
  semantics live only in the code), T-946.21 (a foreign-generation objects archive loads while
  chunks fall back to JSON), T-946.22 (the unread-field gate cannot force stale wording out).
- 2026-09-06 Two gaps the wave could not close itself. **T-935.14** (READY, packed into the next
  wave): `objects/prefabs.rkyv` is emitted, verified and UNREACHABLE — both SPA prefab consumers go
  through `load_prefabs_gz`, which only understands gzipped JSON, so the fetch T-935.11 could not
  add would have handed its bytes to nothing. **T-946.17**: neither the slice gate nor the wave gate
  has a `mod compile` step, so 2107 lines of Enfusion landed with no gate compiling them; the
  command centre ran it by hand, which is the manual step a gate exists to remove.

- 2026-09-06 **WAVE 242 CLOSED** (`54153870d`) — T-935.14 the prefab archive reaching the residency,
  T-935.12 the binary formats made schema-described, golden-gated and LFS-routed, T-674.2 the
  Enfusion reader for slot identity and squad leader. Closed from a RESERVED pending entry again.
- 2026-09-06 **THE MOD COMPILE GATE HAD BEEN COMPILING THE WRONG COPY OF EVERY SHARED SCRIPT**
  (T-946.23, `dbaf92891`). The Enfusion VFS overlays addons BY PATH and the last one wins; the gate
  listed `TBD_Framework,TBD_Export`, so all 139 shared paths compiled from tbd-EXPORT and the
  framework tree — the only one the shipping server loads — went unread. Found by the T-674.2
  agent, which could not explain why its framework-only probe compiled clean. Reproduced before
  acting: an undefined function planted in `tbd-framework/.../TBD_MissionSlotStruct.c` gave
  `OK: compiled clean`, exit 0. **THIS RETRACTS A CLAIM MADE IN THIS LEDGER ONE WAVE EARLIER**: the
  wave-241 row says both copies of `TBD_MissionValidator.c` were inside the compile count. Only the
  export copy was. T-676's `TBD_TriggerRuntime.c` is framework-only and genuinely was compiled.
- 2026-09-06 Wave 242 verifier, and it went straight at that fix: **one BLOCKER of my own making.**
  `mirror_lockstep` walked the EXPORT tree and skipped any path with no framework twin, so moving
  `tbd-framework/.../TBD_Log.c` aside left the gate green with the file count UNCHANGED — export's
  copy stepped into its place and nothing said so. The same defect class the fix was written for,
  reintroduced in the opposite direction. Also MAJOR: the stripper it borrowed finds `//` BEFORE
  blanking string literals, so a divergence after any `"http://…"` was invisible; and it ERASED
  literal contents, so two mirrors could name different resource GUIDs — different UI layouts — and
  pass. And the addon flip did not change only scripts: three shared NON-script paths differ
  byte-for-byte while carrying identical GUIDs, so the flip swapped which body resolves.
  All four fixed in T-946.24: an explicit list of the 13 legitimately export-only scripts (so a
  missing framework file is detectable at all), literals blanked before `//`, literals KEPT and
  ASCII-folded rather than erased, and every shared non-script path compared with a three-entry
  allowlist. The lockstep also grew from `Scripts/Game` to all of `Scripts`, covering the 84 shared
  WorkbenchGame scripts that neither tree compiles.
- 2026-09-06 The lockstep found one real drift on its first honest run: the framework and export
  copies of `TBD_RegistryItemsExportPlugin.c` called `TBD_ExportJson.Escape` and
  `TBD_MapExportJson.Escape` — the alias and the class it aliases. Both compile, so nothing had
  ever noticed. Aligned on the alias, which `TBD_MapExportPaths.c:164` documents as existing for
  this exact caller.
- 2026-09-06 Also from the wave 242 report, fixed in the same pass (T-946.24): an `objects.binary`
  path set to `""` passed both the schema and the gate, resolving to the asset directory itself;
  the three archive lanes in `world_host` discarded their own refusals with `is_ok()` and fell back
  to JSON in silence, so a catalogue built for another terrain or carrying a duplicate `prefabId`
  cost a slower boot and said nothing; and `load_count_guard`'s message named tbd-framework
  specifically while measuring the UNION of both addons, so it could never have detected the thing
  it named. Filed, not fixed: **T-946.25** — the prefab lane's parity pin compares the archive
  against an f32-NARROWED copy of the export rather than the committed f64 export, so on the real
  input the two lanes differ by f32 rounding; the code says so and the acceptance does not.

- 2026-09-06 **WAVE 243 CLOSED** (`46d8ea65b`) — T-675.2 the Enfusion vehicles[] roster reader and
  authored crew seating, T-936.1 win conditions end to end (schema enum, core passthrough, editor
  card, evaluator), T-702 the whole-terrain Play Area zone. **Membership differs from the lock:**
  T-935.13 and T-673 were pulled and T-936.1 substituted, so the close ran with `--tickets`.
- 2026-09-06 **T-935.13 CANNOT BE HONOURED AS WRITTEN, and T-981 is REFUTED.** The operator's premise
  was that the building archive drops the 1,322 blocking prefabs. It does not: `descriptor.rs:405-409`
  counts blocking rows and NEVER censuses them, the host inserts census only, and the 1,322 keep their
  JSON descriptor lane with full `instances` — pinned by a whole-corpus test. Measured independently:
  1623 descriptors, 1322 `"blocks": true`, all 1623 carrying `instances`. The real blockers are five
  others the ticket never named, the hardest being that **`dem/elevation.dem` cannot be built at all** —
  its only writer consumes an ASCII-decimal u16 raster that exists nowhere in the repo or the 1.5 G
  staging tree, so it needs a Workbench GetSurfaceY re-export from the operator. Also: deleting the
  gz-JSON breaks the tools that BUILD the binaries (the building-archive emitter itself reads
  `prefabs.json.gz`), three of the ticket's own five verify commands read the files it deletes,
  `schema_gates.rs:4044` goes SILENTLY VACUOUS, and dropping flate2 deletes the gzip-vs-rkyv sniff
  that T-935.11/.14 shipped. Operator decision: run it as ONE GIANT WIDENED slice in wave 244 with
  T-985 and T-993 folded in, landing with `dem.raw` unfilled — operator-authorized, not a deferral.
- 2026-09-06 T-673 was pulled from wave 243 because its six marker fields must be declared on
  `TBD_MissionMarkerStruct`, which lives in `TBD_MissionLoader.c` — a file T-675.2 owned. Its own
  ticket notes are stale twice over: they still claim `$defs/marker` is closed and the executor is
  `workbench`. T-706 shipped the widening (`9228a458`) and a committed golden already carries all six.
- 2026-09-06 **T-678 was in the lock and is not dispatchable** — `depends_on = ["T-677"]`, and T-677 is
  `ready`, not shipped. The lock packs by order and does not check deps. Compute deps AND
  owns-disjointness (including export twins) before composing any wave.
- 2026-09-06 Wave 243 verifier: four real defects, three fixed in the pass. The one that mattered:
  **`mode: timeout` reported a conflict on the ORDINARY case** — `flow.timeLimitSeconds` defaults to
  5400, so authoring the timeout rule and nothing else was told it disagreed with a number nobody
  wrote. The slice's own fixture states "a diagnostic that fires on correct input is noise" and then
  tested only the authored path. A delta verifier over that fix then found **the comment which fixed a
  false comment was itself false** ("never a null check", over a caller that tests both). Filed
  T-946.26-.32, including two more holes in the mod gate one wave after T-946.23/.24 were written for
  exactly this class: `mirror_lockstep` still cannot see a framework-only NEW script, and it fails in
  EVERY slice worktree over 19 untracked EnfusionMCP scripts, making `mod compile` unrunnable there.
- 2026-09-06 `mk leptos-gates` ran post-merge for the first time this run (its `trunk build --release`
  collides with the operator's `trunk serve` over the same `dist/`; stop the server first). Exit 1,
  21 fail / 22 pass — and **byte-identical to the pre-merge run: 97 failing oracle paths, `diff`
  empty.** Pre-existing drifted oracles, no regression from this wave.

- 2026-09-06 **WAVE 244 CLOSED** (`ddc49daaa`) — T-935.13 everon binary cutover (T-985 hot-set + T-993 sat-v2 folded; dem.raw UNFILLED; prefabs/descriptors/ kept; flate2 kept; gz-JSON kept as emitter input) and T-673 Enfusion marker style reader (six MRK attrs; Rpc style packed as xs trailer). Membership differs from the lock: T-677 was not dispatched (operator: two seats). GATE: PASS (base `46d8ea65b`, cold `tbd_wave244_cold_*`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11333 classes). Verifier: no findings. UNREAD: deleted rotationDeg/brush/color/alpha; re-pinned shape 32→34 and size 0→3 (T-681 kept).
- 2026-09-06 **WAVE 245 CLOSED** (`f09955da0`) — T-677 waypoint runtime (AI gate for waypointed LIVE groups), T-682 env reader (fog/wind/viewDistance; editor controls still refused), T-936.2 tasks schema/panel/HUD. GATE: PASS (base `ddc49daaa`, cold `tbd_wave245_cold_missions_it`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11363 classes). Verifier (ee7f4a234): three MAJOR, not BLOCKER — filed T-946.33 (panel unmounted), T-946.34 (spawn gate skips claimed players / no ActivateAI), T-946.35 (flatten drops tasks[] from /compiled). UNREAD: retired fog/wind/viewDistance/waypoints/vehicleUid/speedMode/behaviour; combatMode/formation kept for T-678.
- 2026-09-06 **WAVE 246 CLOSED** (`05ee3f3a6`) — T-678 group AI state (combatMode/behaviour/formation/speedMode on T-677-armed groups), T-680 vehicle lock/fuel/ammo (roster vehicles[]; lock:false indistinguishable), T-684 missionParams Get(symbol) (no lobby). GATE: PASS (base `f09955da0`, cold `tbd_wave246_cold_missions_it`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11388 classes). Verifier (d51a04518 / restamp f4411640f): three MAJOR, not BLOCKER — filed T-946.36 (flatten drops params/group/vehicle state), T-946.37 (lock:false cannot unlock), T-946.38 (entities[]-only vehicles). UNREAD: retired combatMode/formation/lock/fuel/ammo/missionParams; kept size expected 3 and shape expected 34.
- 2026-09-06 **WAVE 247 CLOSED** (`21d384ccd`) — T-681 entity state (health/allowDamage/showModel/size/stamina on spawned entities[]; bool false equals omit; stamina skip-logged), T-936.3 radioPlan pass-through (tasks[] restored on flatten — T-946.35), T-133 timed tasks schedule {startAfterS, windowS}. GATE: PASS (base `05ee3f3a6`, cold `tbd_wave247_cold_missions_it` and close `tbd_wave247_close_cold_missions_it`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11397 classes). Verifier (3bfc58b9d / restamp 35b7a6df6): BLOCKER schedule×flatten 500 fixed in-wave (schema `$defs/task.schedule`). MAJOR leftover: T-946.36 flatten still drops params/group/vehicle/entity state; T-946.33/39 panels unmounted; T-946.40 entity bool-false. UNREAD: retired allowDamage/showModel/stamina/health; re-pinned size expected 14.

- 2026-09-06 **WAVE 248 CLOSED** (`dbd4a3e5c`) — T-685 zone volumes (AGL bounds, attacker/defender counts, startingOwner; ObjectivesComponent apply), T-679 placement scatter (radius/shape; group scatter is a shared offset), T-299 one-faction compile (`factions.minItems` 1, pad deleted). GATE: PASS (base `21d384ccd`, close `tbd_wave248_close_cold_missions_it`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11415 classes). Verifier (`d7b388055`): MAJOR leftover T-946.41 flatten omits scatter; T-946.36 params/group/vehicle/entity; T-946.33/39 panels. Zone volume keys ride `rules` Value clone (T-685 flatten-omit claim refuted). UNREAD: retired six T-685 keys and `placementRadius`/`placementShape`; re-pinned `shape` 34→36; fire-once retargeted to `vehicleClasses` / T-689.

- 2026-09-06 **WAVE 249 CLOSED** (`fc2b957e7`) — T-941.1 safestart shield in LOBBY/BRIEFING, T-310 Arsenal `gear.attachments[]` on primary, T-689 play-area `vehicleClasses` (aircraft exemption). GATE: PASS (base `dbd4a3e5c`, close `tbd_wave249_close_cold_missions_it`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11419 classes). Verifier (`05aca949f`): MAJOR T-946.42 empty `vehicleClasses` confines everyone; leftovers T-946.36/41 flatten, T-946.33/39 panels. UNREAD: retired `vehicleClasses`; fire-once retargeted to `framing` / T-212.

- 2026-09-07 **WAVE 250 CLOSED** (`aec833eff`) — T-705 player gadget flags (map/compass/watch/gps/radio after spawn), T-936.4 weatherTimeline keyframes (schema/panel/runtime; flatten still drops the block), T-291 spectatorPolicy/nightVision/windDirDeg readers (color/radio/layers editor-only). GATE: PASS (base `fc2b957e7`, close `tbd_wave250_close_cold_missions_it`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11434 classes). Verifier (`f3fe55a30`): MAJOR T-946.43 flatten omits slot.gadgets, T-946.44 flatten drops weatherTimeline, T-946.45 weather panel unmounted, T-946.46 unauthored nightVision strips NVG; leftovers T-946.36/41 flatten, T-946.33/39 panels. UNREAD: retired compass/watch/gps; re-pinned gadgets 6→33; fire-once stays framing / T-212.
- 2026-09-07 **WAVE 251 CLOSED** (`884fb3627`) — T-654 variant filter at ParseMissionJson (`$profile:TBD_VariantConfig.json` else default:true; crew of excluded vehicles dropped without naming seats), T-936.5 positional audio emitters and music cues (schema/panel/runtime; flatten still drops the block), T-941.2 claimed holders deploy on LOBBY→BRIEFING (DEPLOY locked while pending; pause Change slot). GATE: PASS (base `aec833eff`, close `tbd_wave251_close_cold_missions_it`: 35 tables / 38 missions / 22 migrations). Wave-level `mod compile` OK (11457 classes). Verifier (`8b1424235`): MAJOR T-946.47 flatten drops audio, T-946.48 GetRawJson objectives/triggers miss the variant filter, T-946.49 audio panel unmounted; leftovers T-946.36/41/43/44 flatten, T-946.33/39/45 panels. UNREAD: retired variants; fire-once stays framing / T-212.
