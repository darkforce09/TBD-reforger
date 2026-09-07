# Wave 254 adversarial verify

> Renamed 2026-09-08: this file was `wave255/VERIFY.md` and titled "Wave 255". Both numbers were the
> LOCK ROW, which ran one ahead of the ledger label through waves 253-255. The ledger label for this
> wave is **254** (`425478f87`). Directory and title now both track the ledger.

Base `aac282ff1` (wave 253 CLOSED). Verify ran read-only against `b134c72f4`; the fixes below landed
after it. Host cargo through `/home/Samuel/.cache/tbd-bin/hcargo`,
`CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`.

**The first wave dispatched five-wide.** `wave.lock` was repacked with `TBD_MAX_CONCURRENT=5` before
dispatch, because the lock records its own `max_concurrent` and every incidental repack (`ticket ship`
runs one per id) inherits it — dispatching five against a lock that says three would let a mid-wave
repack reshape the wave being gated, the failure T-946 documented for wave 236. Membership is lock row
255 exactly, no custom pack.

| Ticket | Merge | Agent |
|---|---|---|
| T-936.7 | `30f09d8a444d0094b3e41137fb0648b8313c4726` | tactical graphics: schema, wire, canvas |
| T-937.3 | `f73c0032089faac4c2ff75d0c22901d823d801ee` | side-key memo behind a doc observer |
| T-190   | `e27fd905cdabe43838734218758cd6101f9e9cf9` | two-tab writer role and read-merge-write |
| T-938.5 | `e914301d12e28f86c6a6b982f4c2ef8798d0fa60` | viewsheds sliced across frames |
| T-938.6 | `7b67376f9ab9e9e5a16ac68bb9b18f8412148e2e` | wasm memory budget and mip floor |

Plus `a3e63856c` — **T-946.64, the slice gate's first test step.** Wave 253 shipped two
deterministically-failing frontend tests that only the wave gate caught, because `gate --slice` ran
`cargo check`, wasm32, fmt, clippy, schema, catalogue drift, two `db_migrate` steps and the
`VERIFY_STEPS` loop, and **no test of any kind**. Proven before dispatch by perturbation: a broken
frontend test gave `test (frontend, changed) FAIL` → `SLICE GATE: FAIL`; fixed, `PASS` → `SLICE GATE:
PASS`. It then ran in all five slice gates.

## Fixed in-wave

### 1. BLOCKER — T-936.7's lane is never bound on the only path rows can arrive by

`upload_tactical_graphics` (`state/history.rs:623`) had two callers: `after_doc_change` (`:456`) and
`refresh_tactical_lane` (`:660`). It was **not** called from `rebind_engine_from_doc` (`:344-385`),
which binds squad links, vehicles, markers and comments and is the entry point for the IDB restore
(`mission_editor.rs:2230`, `:2376`), the server hydrate and conflict resolution (`hydrate.rs:1044`)
and T-190's peer merge (`persist.rs:939`). `after_doc_change` is private and reached only from undo,
redo and `after_local_edit` — i.e. from an **edit**.

Since `begin_tactical_draw` has no caller (T-946.69), a hydrated payload is currently the **only** way
rows exist at all. So the slice's headline acceptance — "the canvas draws all four kinds" — failed on
100% of live openings. Pick still read the document (`live_tactical_graphics`), so the graphic was
invisible **and** clickable and deletable: exactly the "what is drawn and what a click can find are
one set" invariant the slice's own header cites. The comment at `history.rs:468` claimed the lane
"can never go stale after an undo/redo/**restore**" while sitting on the edit path only; corrected to
say which half each site is.

The repo already keeps this pin for every other lane — `t760_markers_bind_feed`,
`t780_connection_line`, `t819_crewed_render_hide`. T-936.7 added a lane and covered one site.
New `mission_editor_tests/t936_7_tactical_lane_bind.rs` pins **both**, plus a second test asserting
the restore binder still binds all five lanes. **Perturbation:** removing the restore-half call turns
both RED verbatim ("rebind_engine_from_doc must bind the tactical lane — the IDB-restore / hydrate /
conflict-resolution / peer-merge half"); restored, `touch`ed, green.

### 2. BLOCKER — T-946.64 built into a target dir this repo measured producing false PASSes

My own change, and the verify caught it. `frontend_tests_changed` ran through
`ctx.gate_check_target` = `main_root/target-gate-check`, and `main_root` is deliberately the primary
checkout **shared by every worktree** (`wave/mod.rs:240-246`). The wave gate refuses that dir for this
exact command in so many words (`gate.rs:500-508`): *"Two agents (T-193, T-195) independently proved
that with the shared `CARGO_TARGET_DIR`, `cargo test -p website-frontend` runs a stale
`website_frontend-<hash>` test binary built from ANOTHER worktree"* — same package name and version
across worktrees means the same artifact hash — and gives its own step `target-gate-frontend`.

Five slices ran concurrently into one directory. A **test** step reporting another worktree's cached
PASS is worse than no step at all, and the commit message asserted the opposite. Fixed to a private
**per-slice** dir, `target-gate-slice-frontend-<TID>`, so five concurrent gates cannot collide with
each other either.

### 3. BLOCKER — T-946.64's scope silently skipped the file this very wave changed

Also mine. `wasm_scope_touched` is a prefix test over `Cargo.toml` `path =` deps, resolving to
`apps/website/frontend`, `crates/map-engine-core`, `crates/map-engine-render`. But the frontend suite
compiles files from **outside** that graph through `include_str!`:
`packages/tbd-schema/schema/mission.schema.json` (`editor/panels/zones_panel.rs:650`),
`loadout-export.schema.json` (`arsenal/`), `apps/website/api/src/app.rs` (four `pages/` census tests),
`apps/mod/tbd-framework/Data/registry.json` (`arsenal/asset_catalog.rs:40`).

**Wave 255 changed `mission.schema.json`.** A slice whose diff was only that file would print
"frontend untouched", skip, and report PASS over `zone_rule_fields_cover_the_whole_vocabulary` — a
test that compiles that exact file and is documented to fail loudly on a new key.

Fixed by adding the wasm-scope crates' include inputs, **scoped** via a new `include_inputs_under`
rather than taken wholesale from `compiled_include_input_paths()` — wholesale would drag
`apps/website/api/**` into the frontend's scope, which this module's own test forbids.
`compiled_include_input_paths()` is now a one-line call into it, byte-identical for its caller.

Two things the new test found on its way in, both worth keeping: the walk is **cwd-relative**
(`workspace_members`/`rs_files_under` resolve against the process CWD), so passing repo-relative
prefixes returned an empty list from anywhere but the repo root — and an empty list reads as "nothing
in scope", i.e. a silent skip of the step it exists to trigger. Pinned with an absolute root. And the
non-vacuity assertion is what caught it. **Perturbation:** making the walk return an empty set turns
the test RED; restored, `touch`ed, green.

T-946.64 shipped with no test at all (the verify's M10, fair). It has one now.

## Filed, not fixed — T-946.65 … T-946.79

MAJOR: `.65` the DEM forecast leaks on six early-return paths and permanently downgrades the
satellite (`hold`/`set_held` have no coverage); `.66` `merge_before_write` blind-writes on a failed
read or a refused merge, without latching `UNREADABLE`; `.67` `MERGED_WRITES` counts decisions not
merges, and it is T-190's own acceptance instrument; `.68` Delete short-circuits onto a tactical
graphic ahead of a marquee selection; `.69` the whole tactical draw path is unreachable (arming needs
a `panels/` surface or a keybinding that fails two keymap census tests); `.70` a viewshed cap refusal
is indistinguishable from an empty result; `.71` the wash lane is shipped, tested and unreachable;
`.72` `tacticalGraphics` never got its `UNREAD_WIRE_FIELDS` row; `.73` T-946.64 fires on
`map-engine-render` / `map-engine-core doc` changes that `website-frontend` does not link natively;
`.74` the partial disc paints unmarched ground as *proven dead ground* rather than Unknown (the
sibling `WashJob` uses the honest sentinel); `.75` the terrain cap bounds the raster while the march
uses the unclamped radius, so 3.9 billion samples can pass a 5,776-cell cap; `.76` the finished disc
reaches the GPU only as a side effect of the object wash's changed-cell flag; `.77` the terrain
scheduler lane's only test exits at its first `?`.
NIT: `.78` `step_wash` holds the slot borrow across host code; `.79` the conflict modal compares a
relative local time against an absolute server time.

`.74` was tempting to fix in-wave and deliberately was not: the job pre-fills `Hidden` to match the
synchronous path's sentinel, so flipping it to `Unknown` breaks `sliced_viewshed_is_bit_identical_to_
the_sync_path` unless the sync path changes with it. That is a design change, not a one-word fix.

## Clean bills — checked and found sound

- **No tolerance laundering anywhere.** Every removed assertion in the wave is the "unlisted key"
  negative witness being re-pointed from `tacticalGraphics` to `notAnAuthoredBlock`, plus
  `AUTHORED_BLOCKS.len()` 6 → 7. The re-point is strictly stronger: `tacticalGraphics` is the seventh
  and last T-936 block, so the witness now names a key nothing can ever register. No `1e-N` widened.
- **T-937.3's memo invalidation is correct.** `observe_after_transaction` covers mutators, undo/redo,
  `apply_update` and hydrate; install failure **disables** the cross-call memo rather than leaving it
  stale; the callback touches only an `AtomicU64` so it cannot re-enter the `RefCell` `materialize`
  holds. The squad-id keying could not be falsified — side is a pure function of `squad_id` under the
  held read txn and every write that could change it bumps the version.
- **No signature moved across slices.** `materialize(&self) -> SlotSoa` and `after_local_edit()` are
  byte-identical to `875ca3ffd`, so T-190's `persist.rs` caller and `entity.rs`'s 49 `after_local_edit`
  calls were never at risk.
- **Class-R:** `flatten.rs`'s own split probe is intact (first `#[cfg(test)]` at `:4077`, all four
  needles below `:3796`); `overlays.rs` still carries no real `#[cfg(test)]` (its two hits are prose,
  same count as main); `mission_editor.rs`'s boundary is unmoved.
- **T-936.7's structural guard is real.** `every_authored_block_key_reaches_the_wire` walks
  `AUTHORED_BLOCKS` itself and fails by name on an eighth block missing either flatten half — what
  `weatherTimeline` and `audio` never had. The new golden is gated by `schema_gates.rs:2917`.
- **T-938.5's bit-identity claim is sound**, verified against the code: the terrain march checkpoints
  at a whole ray (`max_angle` is born and dies inside one ray in both paths), ray order is preserved,
  and the equality test compares against a *different function* at three batch sizes under a
  monotone fake clock, having first asserted the fixture yields all three visibility classes.
- **T-946.64's failure propagation is sound** — raw rc through `host::capture` → `Runner::run` → gate
  exit 1. No `let _ =`, no `.ok()`.

## Environmental, never a finding

`dem::peaks::tests::everon_peaks_max_above_350` fails in every worktree — `everon-dem-16bit.png` is a
133-byte LFS pointer there. Three of five agents had their report `Write` refused by their harness
("Subagents should return findings as text"); those reports were transcribed by the command centre.
