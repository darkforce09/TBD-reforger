# Cursor handoff — the factory from wave 244 onward

**The process has not changed. Do NOT re-derive it.** Read, in this order:
1. [`docs/platform/FACTORY_FOR_CURSOR.md`](../../../docs/platform/FACTORY_FOR_CURSOR.md) — the 505-line
   procedural runbook (cold start, staleness check, brief template, accept/reject, merge, gate, verifier,
   close, push). It is current as of 2026-09-04 and still governs.
2. [`.cursor/rules/platform-factory-mode.mdc`](../../../.cursor/rules/platform-factory-mode.mdc) — mode switch.
3. `RESUME.md` beside this file — live state, and the T-935.13 evidence dossier.
4. `CLAUDE.md` HARD GATE — no deferrals without the operator's explicit word.

This file carries **only what changed after 2026-09-04**, which the runbook does not yet know.

---

## 1. WAVE NUMBERS WERE RE-SEATED. Waves 235-243 are closed.
T-946 re-seated the lock's wave labels onto the close-marker ledger, which they had run 13 numbers ahead
of. Anything in `docs/platform/FACTORY_RUN_2026-09.md` calling a wave "248" means **235**. Do not
renumber anything to reconcile them; the ledger is right.

**Wave 244 is next.**

## 2. THE ENFUSION GATE ONLY BECAME REAL AT WAVE 242 — this is the biggest change
Before T-946.23, `cargo xtask mod compile` compiled **tbd-EXPORT's** copy of all 139 shared scripts and
never read `apps/mod/tbd-framework`, the only tree the shipping server loads. Every Enfusion slice this
factory ran before wave 242 was gated over code that does not ship.

Now: addon order is `TBD_Export,TBD_Framework` (framework last = framework wins = framework is what
compiles), and the export mirrors are held by `mirror_lockstep` (`xtask/src/gate_mod_compile.rs:641-711`).

**What this means for every slice that touches a `.c` file:**
- Edit the SAME relative path in **both** `apps/mod/tbd-framework/` and `apps/mod/tbd-export/`.
- The two copies must match after comments are stripped, whitespace collapsed, and 23 `ASCII_FOLD`
  punctuation substitutions applied. **Code and string-literal contents must match exactly** — a
  differing resource GUID inside quotes fails, deliberately.
- The `tbd-export` copy must be **pure ASCII** in every `Scripts/**/*.c`. Write ASCII in both anyway;
  Workbench hard-rejects non-ASCII punctuation.
- Put the export-twin paths in the ticket's `owns`. Tickets written before wave 242 list framework paths
  only and are therefore all **wrong by omission** — widen them.

### A HOLE REMAINS, and wave 243 reproduced it
`mirror_lockstep` walks the **export** tree only, so a **new** script committed to `tbd-framework` with
no export twin is invisible: move the export copy away and the gate says `OK: compiled clean`, exit 0.
The reverse direction is correctly red (T-946.24 fixed that one). Filed as a child of T-946.
Until it is fixed, **manually confirm both twins exist** for every new `.c` file.

### Neither gate compiles Enfusion at all (T-946.17)
`wave gate --slice` and the wave gate have no `mod compile` step. The slice agent must run
`cargo xtask mod compile` itself and paste the verdict, and the command centre must re-run it at wave
level. Do NOT run `cargo test -p xtask` while `mod compile` is running — they share
`~/.local/share/tbd-server-addons` and two T-878 tests fail under contention (T-946.13).

## 3. WAVE 244 = T-935.13 (giant) + T-673 — and T-935.13 needs care
### T-935.13 — the map-storage cutover
**Operator decision 2026-09-06: run it as ONE GIANT WIDENED SLICE.** Owns grows from 5 files to ~20:
the 7 frontend loaders (`world_host.rs`, `occluder_host.rs`, `labels.rs`, `water.rs`, `satellite.rs`,
`dem_load.rs`, `mod.rs`), `crates/map-engine-core/Cargo.toml` and `world/store.rs`, both manifests,
`tools/tbd-tools/src/world/build.rs`, and the gz-JSON consumers (`map_world_los.rs`,
`verify_blas_manifest.rs`, `label_gates.rs`, `schema_gates.rs`,
`map_blueprint/{library,library_cli,world_row}.rs`, `world/gates.rs`, `reclassify.rs`).
**T-985 and T-993 fold in.**

**`dem.raw` STAYS UNFILLED. This is operator-authorized, not an agent deferral** — record it in the
ticket and the commit body. `dem/elevation.dem` cannot be built: its only writer (`aux.rs:1242-1251`)
consumes an ASCII-decimal u16 raster (`aux.rs:1163-1183`) and no `.r16` exists in the repo or the 1.5 G
staging tree. It needs a Workbench GetSurfaceY re-export from the operator.

**Four more blockers the ticket does not name — read `RESUME.md` for the full dossier:**
1. Deleting the gz-JSON breaks the tools that BUILD the binaries (`library_cli.rs:171` is the
   building-archive emitter itself; `build.rs:954,978`; `roads_emit` regenerates the rkyv FROM the gz;
   `labels_emit.rs:218`). Everon could never be regenerated.
2. Three of the ticket's own five verify commands read the deleted files.
   `schema_gates.rs:4044` goes **silently vacuous** (`if catalog.exists()`).
3. Dropping flate2 from the `world` feature deletes `bytes_to_json` (`store.rs:51`) — the shared decode
   behind the gzip-vs-rkyv sniff in four loaders. flate2-removal and sniff-fallback are **mutually
   exclusive**; pick one and say which.
4. A slice worktree CANNOT run the `water` or `unified v2` emitters — they read gitignored `staging/`.

**T-981 is REFUTED — do not act on it.** `descriptor.rs:405-409` never censuses a blocking row; the
1,322 blocking prefabs keep their JSON descriptor lane with full `instances`, pinned by
`archive_emit.rs:568`. `prefabs/descriptors/` must stay.

### T-673 — marker style reader
Its own `notes`/`context` are **stale and wrong**: they claim `$defs/marker` is closed and the executor
is `workbench`. T-706 shipped the widening (`9228a458`); `mission.schema.json:1176-1298` declares 11 keys
and `golden-missions/schema-1_3-wire-fields.json:227-257` already carries all six attributes. It is real,
dispatchable claude-code work.
- The six fields must be declared on `TBD_MissionMarkerStruct`, which lives in **`TBD_MissionLoader.c`**
  (`JsonLoadContext` binds by member name — no other home). Add it to owns. It is free in wave 244
  because T-675.2 landed in 243.
- It also needs **`TBD_MarkerController.c`**: the wire is four positional `array<>` RPC parameters and
  six more columns change both signatures.
- Design gap to decide: `color` is `#rrggbb` but `SetColorEntry` takes an **enum index** into
  `SCR_EScenarioFrameworkMarkerCustomColor` (`TBD_MarkerClient.c:335`). No direct home for a hex string.
- `size`'s `UNREAD_WIRE_FIELDS` row is owned by **T-681**, not T-673 — a cross-ticket collision.

## 4. CHECK `depends_on` BEFORE COMPOSING A WAVE — the lock does not
Wave 243's third ticket, T-678, was **not dispatchable**: `depends_on = ["T-677"]` and T-677 is `ready`,
not shipped. The lock had packed it anyway. Compute, for every candidate: deps all `shipped`, AND owns
file-disjoint from its wave siblings **including the export twins**. T-936.1 was substituted.

## 5. THE `UNREAD_WIRE_FIELDS` RETIREMENT IS COMMAND-CENTRE WORK, NOT THE SLICE'S
Any slice that lands an Enfusion reader for a 1.3 wire field WILL fail the `schema` gate row, because
`JsonLoadContext` binds by field name so the identifiers ARE the contract. That is expected. The slice
must NOT edit `xtask/src/schema_gates.rs` or `packages/tbd-schema`. After the merge, the command centre:
- **deletes** the row whose baseline was 0 (its whole purpose was "no reader yet"), and
- **re-pins** a row with a non-zero baseline to the new count rather than deleting it (it still guards
  against a further unnoticed reader),
- drops the field's "NOTHING reads it on any shipped build" wording in `mission.schema.json`, replacing
  it with a "READ SINCE T-xxx (date) by <path>" sentence — the `editorTriggers` description
  (`mission.schema.json`, T-676) is the model.
Precedent commit: `0b9c05c8b`. Then `cargo test -p xtask` (never while `mod compile` runs).

## 6. TICKET ID SPACE IS EXHAUSTED — operator decision still open (T-946.1)
Top-level ids run out at T-999. Every new finding is filed as a child of **T-946**, now 26 deep. A
hand-filed ticket must be added to its parent's `children` list or `cargo test -p xtask` goes red on
`no_ticket_lost_set_equality` (`ticket check` stays green without it — that is why the rule is
"cargo test after every registry edit", not "ticket check").

## 7. TRAPS MEASURED THIS RUN THAT THE RUNBOOK DOES NOT CARRY
- **`trunk serve` is bound to the MAIN checkout, not to slice worktrees.** The wasm on :3000 is
  byte-identical to main's `dist/`. Do NOT tell a worktree slice to "drive the live editor to verify" —
  it cannot. Either give the slice its own server or have it prove the DOM structurally and put the
  visual confirmation on the human checklist.
- **`cargo xtask verify file-length` has 11 pre-existing SIZE-3 violations, not 10.**
- **If a rate limit kills the agents, they CANNOT be resumed** unless your harness has a
  send-message-to-agent tool. Claude Code's did not. Their uncommitted work is unreviewed: revert it
  surgically (`git checkout --` on named files, `rm` on named untracked files) and respawn.
  **NEVER `git stash` and NEVER `git clean -fd`** — both eat LFS pointer files.
- Tell every slice agent to **commit after each self-contained step**, so an outage leaves reviewed work
  instead of a revert.
- A slice worktree's `packages/map-assets` payloads are **LFS pointers**, so ~7 xtask `map_blueprint::*`
  / `map_world_los::*` tests fail there with `bad magic [118, 101, 114, 115]` ("vers"), and
  `dem::peaks::tests::everon_peaks_max_above_350` fails on the pointer PNG. Environmental. In a worktree
  run `xtask schema validate`, not `xtask ci schema-validate`.
- **Restart the dev API if preflight says it is STALE.** Preflight compares its start time against the
  API source mtime and says "Restart it or verifications lie." It is right.

## 8. CLOSING A WAVE WHOSE MEMBERSHIP CHANGED
`wave.lock` is compiled from the tickets by `cargo xtask wave repack` — the only legal writer. If you run
a wave whose membership differs from the lock (as wave 243 did), close with
`cargo xtask platform wave wave --close --tickets <ids>`. `wave repack --reserve "<ids>"` freezes shipped
ids as the pending close target when the wave dissolved id by id and left no `[[emptied]]` entry.
`platform wave status` will disagree with reality until the close; that is expected, not drift.
