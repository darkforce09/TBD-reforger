# Wave 243 — slice reports and command-centre verdicts

## T-675.2 — ACCEPTED (2026-09-06)
Commits: `858cbd35c`, `67911241b`. Worktree clean. Diff symmetric across both mod trees
(43 / 751 / 7 lines, framework and export), new `TBD_MissionVehicleStruct.c` byte-identical in both,
zero bytes > 0x7F in either tree. `hcargo xtask mod compile` = `OK: compiled clean`, exit 0.

**SLICE GATE: FAIL, and the failure is the one the brief predicted and forbade the agent to fix.**
Exactly one red row (`schema`); `cargo check`, `wasm32`, `fmt`, `clippy` and all 15 pin gates PASS.
The red is `UNREAD_WIRE_FIELDS`: `vehicles` 2 identifiers (baseline 0), `seats` 12 (baseline 8).
`JsonLoadContext` binds by field name, so those identifiers ARE the contract — no reader can avoid them.
**Command centre clears this after T-936.1 releases `packages/tbd-schema/schema/mission.schema.json`**
(retire the two rows at `xtask/src/schema_gates.rs:2484-2495` + drop the "no reader on any shipped
build" wording; precedent `0b9c05c8b`).

Two engineering calls worth keeping:
- `MoveInVehicle` was WRONG for these bodies — it accepts the request and the character then *walks* to
  the door, and slot bodies have AI disabled at spawn, so crew would have stood beside the vehicle all
  event while the log said "seated". Replaced with `GetInVehicle(..., forceTeleport=true, ...)`.
- `GetVehicleIn` answers **null in the requesting frame** even for an accepted force-teleport seat, so an
  in-frame verdict would have been a dead mechanism reading as a live one (rule 17). Seats are now queued
  and re-asked 1000 ms later against the specific vehicle the seat named.

Double-spawn guard: `uid` exact match first (one-shot claim), then an `alias|x|z` fingerprint for the
residual hole flatten documents (blank editor id => neither row carries a uid), refusing to positionally
claim a twin carrying a *different* non-empty uid. Proved non-vacuous by forcing `ClaimTwin` to return
null and watching an independent `QueryEntitiesByAABB` world census go red with 2 duplicate vehicles.

### FILE AFTER THE WAVE — the lockstep still has a hole, reproduced
`mirror_lockstep` walks the EXPORT tree only, so a NEW script committed to `tbd-framework` with no export
twin is invisible: `mv` the export copy away and the gate says `OK: compiled clean`, **exit 0**. The
reverse direction is correctly red (T-946.24 fixed that one). This slice's own new file fell straight into
the hole — both copies are committed, so the shipped state is right, but the gate would not have caught a
one-sided add. File as a child of T-946 (id space is exhausted at T-999 — T-946.1).

### Pre-existing, NOT this slice
`mod world-boot --mission=bridgehead-at-levie` -> `FAIL validator warnings rose: 3 > baseline 0`. The
baseline file's own T-609 block already records `msn_8f3a2c 0, boots 3 environment + layers(1 alias) +
orbat.roles.radio` — exactly what fired. No validator was touched.

### manual_checklist (8 items) — carried to the wave's human checklist
1. Live server: crew visibly SITTING in the vehicle at mission start, not standing beside it, each in the
   station the crew panel authored (driver at the wheel, gunner behind the gun).
2. A player deploying onto a crewed slot takes control of the body INSIDE the vehicle and can drive/gun
   immediately, not teleported out and not losing the seat.
3. Exactly one vehicle model at the authored position — no doubled/z-fighting pair.
4. A `commander` seat on a prefab with no turret compartment logs `has no compartment of the type
   role='commander' needs -- NOT seated` and the rest of the crew still seats.
5. Two same-role seats without `index` (e.g. two `cargo`): fall-forward line appears, both crew land in
   DIFFERENT cargo stations.
6. `copilot` on a helicopter takes the SECOND pilot station, not the pilot's.
7. A crewman who gets out and back in mid-round is unaffected (reader runs once at load).
8. Crew survives the safestart -> live transition still seated.

---

## T-702 — ACCEPTED (2026-09-06), with a declared owns widening
Commits: `c9bf3eb31`, `17b25b7a0`, `0df3d0ade`, `ecb83a81c`. Worktree clean. **SLICE GATE: PASS**
(`gate verdict PASS @ ecb83a81cbd4`). numstat: zones_panel.rs 512/2, entity.rs 98/16, store.rs 115/0.

### The widening — accepted
`crates/map-engine-core/src/doc/store.rs` is outside the ticket's owns. The agent declared it loudly
rather than hiding it, but did NOT stop first as the brief's clause demands. Accepted anyway, because:
- **No sibling holds it.** T-936.1 owns `map-engine-core/src/mission/*`, not `doc/*`.
- **Purely additive: 115 added, 0 removed.** Verified: `add_polygon_zone` now has a one-line body
  delegating to the new `add_polygon_zone_labelled`, and git counted it additive because the old body
  was reused verbatim as the new function's. There is exactly ONE implementation; they cannot drift.
- **The acceptance is unreachable without it.** `capture_timeout_millis = 0` makes every core mutator
  its own undo step, so `add_polygon_zone` + `set_zone_label` is structurally TWO Ctrl+Z presses. A
  labelled zone in one undo step needs both keys under one `begin()`, and `begin()` is private.
  House precedent: `move_entities_and_vehicles` (T-425), `update_slots_attr_batch` (T-649).
The verifier should attack this hardest: it is the one place this wave left its lane.

### It defeated the salvage rather than inheriting it
`4e4eefd7` claimed "same borrow scope => same undo step (no second transaction)". False — measured
`undo_depth() == 2`. The author's single Ctrl+Z would have stripped the name and left an anonymous zone
standing. That is the brief's flagged trap, already shipped once. Also rejected: an inline `"boundary"`
spelling behind a comment claiming it was schema-resolved; a dead `DrawTarget::Trigger` arm (rule 17);
source pins grepping raw `include_str!` text, which the file's own T-759 note forbids.

### Rule 18 honoured concretely
`terrain_rect_ring(terrain, bounds)` returns `None` unless `bounds == compile::terrain_bounds(terrain)`,
so an Everon rect handed to Arland is refused rather than authored. The golden re-derives nothing: it
compiles a zone-free mission through the real `flatten_to_mod_document` for BOTH shipped terrains
(everon 12800**2, arland 4096**2) and demands the authored ring equal the `z_bounds` ring the compile
itself produced. Eight perturbations, all production-side; TWO of them found vacuity in the slice's own
tests, which were then rewritten and re-perturbed.

### TWO CORRECTIONS TO MY OWN BRIEF — fix before the next wave
1. **`trunk serve` is bound to the MAIN checkout, not to slice worktrees.** The wasm served on :3000 is
   byte-identical to `apps/website/frontend/dist/` in the main checkout and does NOT contain this
   slice's new string. So "drive the live editor to verify your button" is NOT available to a worktree
   slice, and the T-702 brief said it was. Either drop that invitation or give the slice its own server.
2. **`verify file-length` reports 11 SIZE-3 violations on `79b4da61f`, not 10.** The known-issues number
   carried from wave 242 is one low. Set: `world/prefab.rs`, `world/water.rs`, `tbd-tickets/src/ops.rs`,
   `forest_smooth.rs`, `backfill_stamps.rs`, `check.rs`, `estimate_tokens.rs`, `gate_mod_compile.rs`,
   `wave/base.rs`, `wave/land.rs`, `wave_lock.rs`. None are files this slice touched.

### FILE AFTER THE WAVE — latent defect found, correctly not fixed
`entity.rs:3297` -> `store.rs:2434`: `terrain_bounds_of` returns `[minX, minY, maxX, maxY]` but its
element names said `[x0, y0, width, height]`, and `place_composition(..., b[2], b[3])` passes them to
parameters named `width`/`height`. Correct TODAY only because every terrain `compile::terrain_bounds`
knows is anchored at (0, 0). A non-zero-anchored terrain would silently mis-stamp every composition.
The comment was corrected; the call was not touched (store.rs composition placement is out of scope).

### manual_checklist — carried to the wave's human checklist
- Everon mission, Zones panel, press "Whole-terrain zone": one row named "Play Area", corners exactly on
  the map edge, Attributes panel already open below the list with that zone selected.
- Ctrl+Z ONCE: the zone disappears completely (not merely renamed). Ctrl+Y once: it returns WITH its name.
- Repeat on Arland: the ring hugs 4096 m, not 12800.
- Press the button twice on one mission: a second zone `z2` is authored (no dedupe). Confirm that is
  acceptable product behaviour or file a follow-up for an "already has a play area" guard.
- Save + Export: the compiled document carries exactly one `boundary` zone — the authored one — with no
  synthesised `z_bounds` beside it.

---

## T-936.1 — ACCEPTED (2026-09-06), with ONE thing the command centre must finish
7 commits (`3b586a45b` … `9377a848c`). **SLICE GATE: PASS** (`gate verdict PASS @ 9377a848c2e3`).
11 files, all inside owns, `files_outside_owns = []`. Tracked tree clean. Evaluator twins
byte-identical and pure ASCII in both trees. Perturbation proved the gate READS the new `.c`:
`TBD_WinConditionEvaluator.c:147: Can't find variable 'TBD_ThisSymbolDoesNotExist'`.

### THE CARD IS REGISTERED BUT NOT MOUNTED — a dead mechanism, and it is MINE to close
`win_conditions_card` is built, registered in `panels/mod.rs` and fully tested, but its one-line mount
belongs beside `settings_modal::render_flow_section` and `panels/settings_modal.rs` is OUTSIDE the
ticket's owns. The agent correctly refused to widen and said so plainly. But the acceptance "the Win
conditions card round-trips add, edit and undo" is not reachable by a human until it is mounted, and an
unmounted card is exactly the rule-17 shape (a mechanism that cannot fire). **Command centre mounts it
after the merge** — one line, `settings_modal.rs` is unowned by any slice this wave. It is NOT a
deferral: it lands in this wave.

Also noted by the agent for whoever next opens the file: `env.rs::CARRIED_ENV_KEYS` should gain a
`winConditions` row; the reader chain is restated as `WIN_CONDITIONS_READERS` in the card meanwhile.

### Deviations — all four accepted, all measured
1. **The `mode` enum is SEVEN values, not the five the ticket named.** Four committed hand-authored
   goldens carry `points_then_attrition` / `defender_holds_or_attacker_destroys`, and the requirement
   also says "goldens still validate". Proved by perturbation: removing the grandfathered value fails
   `bridgehead-at-levie`, `empty-warning-fields` and `slot-loadout-coverage` with
   `/winConditions/mode "points_then_attrition" is not one of ...`. The EDITOR is still pinned to five
   (`mission-editor-payload.schema.json` + `AUTHORED_MODES`), with a lockstep test reading both enums
   out of the committed bytes. Grandfathering in the compiled contract only is the right split.
2. **`vip` may also carry an optional `extractionZoneId`.** The spec asks the evaluator for "VIP
   extracted => win" but gave `vip` only `vipSlotId`, so that half of the rule had nowhere to extract to.
3. **`mode: timeout` projects onto `flow.timeLimitSeconds` at compile time** rather than starting its own
   runtime timer. Two timers for one deadline is the T-946.19 defect shape; the evaluator starts none and
   warns when `endOn` lacks `time_limit`.
4. **`DIAG_WIN_CONDITIONS` is one rule id for four situations** — `COMPILE_DIAGNOSTIC_RULE_IDS`' own
   reachability test requires every id to fire over one fixture, and a document has exactly one `mode`,
   so four ids would leave three permanently unreachable.

### `mk ci-local-leptos` / `mk leptos-gates` were NOT run by this slice
Both are wave-level per the brief and their `trunk build` races the `trunk serve` on :3000 that rule 13
says to leave up. Covered in-slice by `cargo test -p website-frontend` (1273 passed), wasm32 clippy
(0 findings on its files) and `fmt (changed)`. **The WAVE gate must therefore actually run the leptos
lane — do not skip it.**

### FILE AFTER THE WAVE — the mod gate cannot run in a slice worktree unaided
`mirror_lockstep` fails in EVERY slice worktree with 19 `Scripts/WorkbenchGame/EnfusionMCP/*.c`
"in tbd-export only". Those files are UNTRACKED — `git ls-files` is empty for them and
`apps/mod/.gitignore:28` ignores the framework copy — so they exist only in the main checkout. The agent
had to copy the gitignored directory into its worktree to reach the compile at all. Nothing to fix in
slice code; the lockstep walker cannot see an ignored tree. File as a child of T-946.

### manual_checklist — carried to the wave's human checklist
- After the mount lands: in the browser pick VIP, type a slot uid, tick/untick triggers, Ctrl+Z after
  each — every edit exactly one undo step, and the block survives Save + reload.
- Dedicated server, `mode: vip` + `vipSlotId`: kill the VIP, round ends ONCE
  (`[TBD][Win] vip_down — winner=<side>` exactly once, no further `[Win]` lines).
- Same + `extractionZoneId`: walk the VIP into the zone, one `[TBD][Win] vip_extracted`.
- `mode: extraction`: every living player of one side into the zone => one `[TBD][Win] extraction`;
  a side already wiped out must NOT win it.
- `mode: timeout, timeoutMinutes: N`: the round clock is armed for N*60 s — ONE clock, not two — and
  `[Win] timeout minutes=N` is logged once at LIVE.
- `mode: objective` with an `endOn` declaring no objective trigger: the `[Win]` warning
  "this rule can NEVER end the round" appears.

---

## MERGE PRECONDITION — file-disjointness PROVED
20 changed paths across the three slices, **zero overlaps** (T-675.2 6, T-702 3, T-936.1 11).
