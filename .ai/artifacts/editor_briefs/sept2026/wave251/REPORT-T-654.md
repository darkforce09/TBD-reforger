# REPORT T-654 — Variant-gated document subtrees

## pwd_branch

`/run/media/system/Disk_2/Projects/TBD-Reforger/.ai/artifacts/worktrees/T-654` · `slice/T-654`

## defect_verified_on_main

Verified on this worktree at `87bd9d9b0` (slice base, pre-change): `rg -n variant` over both `TBD_MissionLoader.c` copies was empty. T-706 wire (`variants[]` / `variantId`) had no load-time reader; every subtree spawned.

## changes

- Typed `variantId` on zone / orbat group / entity structs in `TBD_MissionLoader.c`.
- Typed `variants[]` registry (`TBD_MissionVariantStruct`). `default` is an Enforce keyword, so default flags are walked off the raw JSON.
- Active set: `$profile:TBD_VariantConfig.json` `activeVariants[]` (separate file; key presence walked because JsonLoadContext allocates absent arrays) else `default: true` rows.
- Filter in `ParseMissionJson` after typed parse, before `TBD_MissionValidator.Run`. Absent `variants` key is inert (no log). Empty registry is not inert.
- slots[] / vehicles[] gates via `TBD_VariantGateSkeletonStruct` (those structs are not in owns).
- Excluded vehicle drops its entities[] twin (uid, else `alias|x|z`) and crew `slotId`s collected by a raw-JSON walker (does not name the typed `seats` member).
- `GetActiveVariantIds()` published for readers that still parse `GetRawJson()`.

## perturbation

**red_output VERBATIM** (`cargo xtask mod compile` after inserting `TBD_T654_PERTURB_DOES_NOT_EXIST();` next to `ApplyVariantFilter(data);`):

```
FAIL: Enfusion compile errors
------------------------------------------------------------
Scripts/Game/TBD/Backend/TBD_MissionLoader.c:1920: Undefined function 'TBD_MissionLoader.TBD_T654_PERTURB_DOES_NOT_EXIST'
------------------------------------------------------------
1 error(s) in TBD sources, 11 cascaded into vanilla.
```

**restored_green:** call removed, both twins `touch`ed, `cargo xtask mod compile`:

```
OK: compiled clean
    Module: Game; loaded 5755x files; 11442x classes
    Compiling Game scripts took: 844.309000 ms
    0 warning(s) in TBD sources
```

## gate_verdict_tail

Expected UNREAD FAIL on `variants` (baseline 0). Did not edit `schema_gates.rs`. `seats` unread pin stays 12.

```
  schema                   FAIL
      schema: height-labels SKIP in this tree — packages/map-assets/everon/dem/everon-dem-16bit.png is not a materialized PNG
              (LFS pointer or missing). On main with a real DEM this sub-gate RUNS; do not
              treat a worktree skip as 'red on main' or chase `xtask ci lfs-dem` for that.

      ── schema validate (rc 1) ──
        PASS  type-inventory-pending-everon.json
        PASS  map-assets/everon/objects/type-inventory.json
      TBD_MissionValidator unconsumed-key warnings (T-250):
        PASS  TBD_MissionValidator.c (5 unconsumed-key warnings wired; entities retired T-254/T-437)

      1 validation failure(s).
      schema: FAILED validate  (9 sub-gates run; context-skipped: height-labels — DEM not materialized here)
  …
  gate verdict FAIL @ b5356a52a73b recorded: .ai/artifacts/verdicts/T-654.json
SLICE GATE: FAIL
```

Unread line (`cargo xtask schema validate`):

```
'variants' now has 9 mod identifier(s) (baseline 0) — if T-654 landed the reader, DROP the field's "no reader on any shipped build" wording in mission.schema.json and remove/repin its UNREAD_WIRE_FIELDS row; if this is an unrelated change to the pre-existing 'clean' identifier, re-pin the baseline here on purpose
```

## mod_compile_verdict

PASS — `OK: compiled clean` · 5755 files · 11442 classes · 0 TBD warnings (post-restore and post-seats-pin fix).

## files_outside_owns

[]

## found_not_fixed

- `objectives[]` and `editorTriggers[]` are not on `TBD_MissionDocumentStruct`. Their readers (`TBD_ObjectiveRegistry` / `TBD_TriggerRuntime`) re-parse `GetRawJson()` in files this slice does not own, so they are not stripped at load. `GetActiveVariantIds()` is the hook. TriggerRuntime still starts in no-selection mode until `set_variant`.
- Kept vehicles whose crew *slot* was itself variant-excluded still carry that `slotId` on the typed crew plan. Clearing it requires naming `seats`, which bumps T-675's unread pin (baseline 12). Crew of an *excluded vehicle* is dropped from `slots[]`. Spawn may log dangling crew on the inverse case.

## deviations

- Salvage `e596b35b` only patched tbd-framework; both twins are required. Reused salvage design (active set, raw `default` walker, slot skeleton, inert-unless-key) and extended it for today's `vehicles[]` reader plus dependent refs.
- Did not land salvage wholesale (pre-vehicles document, no tbd-export, typed `.seats`).
- Crew slotIds come from a raw-JSON walker instead of `veh.seats` so `seats` stays at unread baseline 12.

## commits

- `bdf7e2864cfcd5db7724b020aad31cc39d4e1d81` T-654: evaluate variant predicates before mission spawn
- `b5356a52a73bd429b57e556d9cbf66429c4e10f8` T-654: drop excluded-vehicle crew without naming seats

## manual_checklist

- Boot a 1.3 mission with `variants[]` and a night-only subtree: it must be absent under the day default / present under night.
- `$profile:TBD_VariantConfig.json` `{ "activeVariants": ["night"] }` replaces document defaults; missing file uses `default: true`.
- Dangling `variantId` logs `[TBD][Variants] EXCLUDING … dangling reference`.
- Excluded vehicle: no world twin, no crew slots in the lobby/spawn list.
- Mission with no `variants` key boots unchanged (no `[TBD][Variants]` line).

## twins_confirmed

Both owns paths edited. tbd-export is the ASCII fold of tbd-framework (pre-change fold was byte-identical; post-change export regenerated the same way). `mod compile` lockstep + ASCII check passed (0 TBD warnings).
