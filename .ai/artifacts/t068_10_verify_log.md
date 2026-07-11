# T-068.10 verify log — Smart Loadout Forge UI + per-slot loadout in the mission doc

**Date:** 2026-07-11 · **Executor:** Claude Code (Fable 5) · **Branch:** `main` @ repo root ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_10_smart_forge_ui.md` ·
**Baseline:** T-068.9 @ `d41418e5`

## Result

**PASS** — the Attributes → Arsenal tab is the first consumer of the T-068.9 registry compat
worker (`initRegistryCompat` / `itemsFor` feeds; canEquip/canAttach families enforced through
the same index), invalid picks are blocked in the UI and the `loadout-export.json` download is
disabled while any pick is incompatible, and — per operator direction in-session — **each slot's
loadout now persists in the mission document itself**: it rides Save Version (`editor.slots[]`),
the mission Export JSON (slot `loadout` block + `orbat[].slots[].loadout` display summary),
IndexedDB, undo, and copy/paste. 19/19 headless browser gates PASS against the live stack.

## Scope note (operator-directed, supersedes the handoff fence)

The handoff scoped this slice to the loadout-file download only. The operator, reviewing the
plan, directed: *"the loadouts for each slot and the actual schema, like the JSON export,
shouldn't just be a JSON loadout export, but rather it should be in the entire mission … we can
have both. So you can export the JSON or loadout JSON file, but when we save the mission or
export the mission, it includes the loadout for each slot."* This pulls the mission-embed
forward from T-068.11; the remaining T-068.11 delta is now: hydrate/compile were already
whole-slot (nothing to do), the orbat display summary shipped here — Cursor should re-scope the
T-068.11 registry row accordingly. No compiler *envelope* change was needed: `editor.slots`
already carries whole slot objects.

## What shipped

### Doc core (Rust owns doc policy — D5)
- `crates/map-engine-core/src/doc/store.rs`: new `update_slot_loadout(id, Option<String>)` —
  one txn = one undo step; JSON → `Any` via the existing `json_str_to_any` (hydrate machinery);
  `None`/empty removes the key. `paste_slots` gains a parallel `loadouts: Vec<String>`
  (`''` = absent, same convention as tag/assetId). `add_slot` unchanged (new slots unforged).
  Legacy `loadoutId` (shared-template ref) untouched.
- `crates/map-engine-wasm/src/lib.rs`: delegate shims for both.
- Rust tests: `update_slot_loadout_roundtrips_and_clears` (set → `slots_json` parse → undo →
  clear → missing-slot no-op) + `paste_slots_copies_loadout`. `cargo test -p map-engine-core
  --all-features`: **149 passed** (+2).

### TS mirror (byte-parity with wasm)
- `state/schema.ts`: `SlotLoadout` (six nullable resource_name slots + optional `summary`);
  `Slot.loadout?`; `ClipboardSlot.loadout?`.
- `state/ydoc.ts`: `buildSlot` optional `loadout` param (omit-when-absent, matching wasm);
  new `updateSlotLoadout` (O(k) `_patchSlots` mirror; delete key on clear); `pasteSlots`
  threads the JSON array + hands the clip loadout to `buildSlot`. Barrel exports
  `updateSlotLoadout` + `SlotLoadout`.
- `MissionCreatorPage.tsx` Ctrl+C snapshot copies `loadout`.
- Parity gate `ydoc.okPatch.test.ts`: `toClip` carries loadout + new case
  `updateSlotLoadout — set / paste-copies / clear` asserting `pickMapSnapshot(store)` ≡
  `snapshotFromShadow(wasm)` after every mutation. **13/13**.
- Free rides verified by exploration (no code needed): Zustand mirror (`slots_json` parse),
  `compile.ts` `editor.slots: Object.values(slotsById)`, Save Version blob, export envelope
  (payload verbatim), IDB v3 whole-blob (`encode_state`), `hydrate` (verbatim `load_rows`),
  backend `mission-editor-payload.schema.json` (slots intentionally unconstrained → no 400),
  Rust flatten `SlotIn` + orbat `Sl` (serde without `deny_unknown_fields` → ignored).

### Contract (loadout-export v1, additive)
- `packages/tbd-schema/schema/loadout-export.schema.json`: optional `gear.optic` +
  `gear.magazine` (`required` unchanged; `additionalProperties:false` retained;
  `loadoutVersion` stays `"1"`). `registry/loadout-export.sample.json` untouched and still
  valid. `make schema-codegen` regenerated `types/contract/loadoutExport.ts`
  (`optic?/magazine?`) + `apps/website/src/contract/generated/loadout.rs`.
- Mod reader unaffected — no mod edits: `TBD_LoadoutEquipComponent` struct-maps known members
  via `JsonLoadContext.ReadValue` (unknown keys ignored) and its `loadoutVersion != "1"` guard
  still passes. optic/magazine equip on the NPC/player is T-068.12 scope.
- `loadout/loadoutExport.ts`: `LoadoutGear` gains `optic`/`magazine` (always emitted, null when
  empty); new `slotLoadoutToGear` projection (drops `summary`).

### Smart Forge UI (`features/mission-creator/loadout/` — one module, adjustability-first)
- `arsenalRules.ts` (pure, node-env testable): declarative **`LOADOUT_ROWS`** config —
  Primary/Optic/Magazine/Uniform/Vest/Helmet; a row is `{key, label, source}` where source is
  `kind` (catalog ∩ canEquip) or `edge` (`itemsFor(picks[dependsOn], family)`). Adding a future
  row (backpack/handgun/launcher) = one config line + one `SlotLoadout` field — options,
  validation, persistence, export all data-drive off the config. `resolveRowAllowed` /
  `buildRowOptions` / `validateLoadout` / `picksToLoadout` / `loadoutToPicks` /
  `buildLoadoutSummary`. No `ammo_in_mag` anywhere (empty in committed data — not invented).
- `useArsenalValidation.ts` (bridge hook): `initRegistryCompat(modpackId)` → character
  `itemsFor(assetId, 'character_default_loadout')` → status `ready`; edge feeds re-fetch on
  dependency change with stale-guards; all setState in async continuations (lint-clean),
  StrictMode-safe via the client's single-flight init.
- `ArsenalTab.tsx` (thin view, extracted from `AttributesModal.tsx` — modal itself untouched
  beyond the import + `md` prop): renders the row config through the shared `SelectField`;
  Aegis `Badge variant="error"` chip per invalid row; download button disabled while invalid;
  status chips `Compat active` / `No compat data for this character` / `Compat unavailable —
  full catalog`. **Picks write through `updateSlotLoadout`** — no ephemeral tab state; undo
  works per pick; reopening the modal (or reloading) shows the persisted kit.
- `useMissionEditor.ts`: `terminateRegistryWorker()` beside `terminateCompiler()` on editor
  unmount (T-066 lifecycle).
- `compile.ts`: `orbat[].slots[].loadout = slot.loadout?.summary ?? ''` (both sync + progress
  paths) — replaces the Phase-6 placeholder.
- Vitest `arsenalRules.test.ts`: **16 cases** — filter/degrade/stranded option building, the
  validation matrix, pick↔loadout round-trip, summary builder.

## Decisions (explicit)

1. **Block-before-download** (spec preference): stale invalid picks are *kept* and flagged
   (`— incompatible` option suffix + error chip) rather than silently stripped or auto-cleared;
   the loadout-file download is disabled until fixed (plus a guard toast). Nothing is ever
   silently dropped from the export.
2. **Mission Save/Export is never blocked** by loadout validity — the doc is lossless editor
   state (an in-progress kit must survive Save Version); validation gates only the
   `loadout-export.json` handoff. Chips make invalid state visible in the editor.
3. **Per-kind degrade** (data-driven): the T-150 export links **clothing kinds only** to
   characters (`character_default_loadout` kind histogram: gear_uniform 1505 · gear_helmet 628 ·
   gear_vest 401 · other 115 · gear_backpack 97 — **zero `gear_primary`**). A hard canEquip
   filter would collapse the Primary row to "None" for every character, so a kind whose
   compat intersection is empty falls back to the full catalog (`resolveRowAllowed → null`).
   The moment a future export ships weapon equip edges, filtering tightens automatically.
   Weapon-level truth stays enforced through the optic/magazine edge feeds.
4. **Degrade tiers:** (a) init failure / dead route → `Compat unavailable` chip + one-shot
   toast + Phase-1 full-catalog pickers, **no** optic/magazine rows, download enabled;
   (b) worker up but zero equip edges for this character → full kind lists, edge rows still
   live. Both proven in the browser gates.
5. **`canEquip` semantics caveat** (T-068.9): compatibility = "appears in that character's
   default loadout" — the only equip evidence T-150 exports.
6. **`Slot.loadoutId` untouched** — remains the future shared-template reference; the new
   embedded `Slot.loadout` is per-slot picks.
7. **`useMissionEditor.ts` crossed the SIZE-1 warn line** (604 > 600, was ~597): +7 lines from
   the teardown import/effect. Warning tier only (0 violations); the file was already the #1
   hotspot pre-slice. Noted for a future split rather than dodged with formatting tricks.

## Automated gates (all exit 0)

```bash
make registry-import      # idempotent re-run: items/compat inserted=0 updated=0 pruned=0
make rust-ci              # fmt + clippy -D warnings (backend + engine crates) + build
                          #   + map-engine-core --all-features: 149 passed (+2 loadout tests)
                          #   + backend test-it: 74 passed
make wasm                 # merged pkg rebuilt; map_engine_wasm_bg.wasm = 4,152,125 B
                          #   (T-068.9 baseline 4,071,877 B; +80,248 — paste_slots param +
                          #   update_slot_loadout + json plumbing)
make schema-codegen       # regenerates exactly the two loadout contract files (committed)
cd packages/tbd-schema && npm run validate        # All contracts valid (sample untouched)
cd apps/website/frontend && npm test              # 42 files / 316 passed (+16 rules, +1 parity)
npm run build                                     # clean (1.5 s)
npm run lint              # 1 pre-existing error: router.tsx react-refresh (T-150 precedent)
npm run format:check      # 24 pre-existing drift files — none from this slice
make verify-citations     # front-end @contract/@model resolve; @route match; GO-7 skipped (Rust)
make verify-coding-standards  # 4 SIZE-1 warnings (3 pre-existing + useMissionEditor.ts 604 —
                              #   decision 7), 0 violations; no-select-star clean
```

## Manual M-gates — automated headless browser (19/19 PASS)

Real stack (`make api` + `make web`), dev DB after `make registry-import`, dev-login
`mission_maker`, playwright-core driving the ms-playwright chromium; character under test
`KS Phantom` (`{6387A8477B677BF2}…/KS_Phantom.et`, 14 equip edges), weapons
`Rifle_M16A2_M203` (12 optics / 7 mags) vs `Rifle_AK74_base` (disjoint optics). Harness +
artifacts (screenshots `m1-blocked.png`, `m2-restored.png`, `m-degrade.png`, downloaded
`loadout-export.json`, `mission-export.json`, `results.json`) in the session scratchpad
`m-gates-out/`; script `m-gates.mjs` (drop via synthetic `DragEvent` with the real
`application/x-tbd-asset` payload).

| Gate | Evidence |
|------|----------|
| M3-chip | `Compat active` badge rendered on Arsenal open |
| M3-worker | dedicated worker `registry.worker.ts` live in page targets |
| M3-fetch | `GET /api/v1/registry/compat?modpack=00000000-…-000000000001` → **200** on first open |
| M3-idb | `tbd-registry-compat` IndexedDB present |
| M1-uniform-filtered | Uniform: **7** options (None + KS Phantom's 6-piece kit) vs **100** uniforms in the registry |
| M1-primary-full | Primary: **126** options — per-kind degrade (no weapon equip edges exist; decision 3) |
| M1-optic-locked | Optic/Magazine rows read "Pick a primary first" until a weapon is picked |
| M1-optic-feed | M16A2 M203 → exactly **12** optics (oracle: 12 `optic_on_weapon` edges) |
| M1-mag-feed | M16A2 M203 → exactly **7** magazines (oracle: 7 `mag_in_weapon` edges) |
| M1-blocked | swap Primary → AK74: **2** `Not compatible with the selected primary` chips; download **disabled** |
| M1-stranded-visible | stranded pick listed as `Collim AP2k — incompatible` (no silent blank) |
| M2-file | download re-enabled after fix; `loadout-export.json` carries primary+optic+magazine; **ajv-valid** against the updated schema; modpackId = current |
| M2-save | Save Version → `POST /missions/:id/versions` **201** |
| M2-mission-embed | mission Export JSON: `payload.editor.slots[].loadout` present with `summary: "Rifle M16A2 M203 · Collim AP2k · Magazine 556x45 STANAG 30rnd Base"` |
| M2-orbat-summary | `payload.orbat[0].slots[0].loadout` = same display summary (compile wiring) |
| M2-reload-restore | full page reload → Arsenal shows the persisted primary + optic from the doc |
| M3-etag-304 | compat re-request after reload → **304** (ETag + IDB warm start) |
| M-degrade | fresh profile + compat route dead: `Compat unavailable` chip + toast, Primary 126 / Uniform **101** (full), **no** optic row |
| M-degrade-export | download stays enabled in degrade mode (Phase 1 behavior) |

## Known limitations (explicit, not silent)

- Weapon-level `canEquip` filtering waits on data: no `gear_primary` rows in the
  `character_default_loadout` family (T-150 export scope) — decision 3 documents the fallback.
- `ammo_in_mag` / `ammo_in_vehicle_weapon` remain empty (T-150 OPEN) — no ammo row shipped.
- The mod consumes `gear.primary/uniform/vest/helmet` only until T-068.12; optic/magazine ride
  the file (schema-optional) and the compiled mission for that slice to pick up.
- Edge-feed revalidation is strict-while-loading: for ~a worker round-trip after a weapon swap
  the stale optic/mag flag as invalid until the fresh `itemsFor` lands (blocked > wrongly-valid).
- Vitest remains node-env pure-module testing (no jsdom/@testing-library in the repo) — the
  React surfaces are covered by the 19 headless browser gates instead.

## Ready for Cursor

- Doc-sync: CLAUDE §Status bullet, arsenal hub row (T-068.10 shipped), ticket registry
  `T-068.10 → shipped`, `advance-slice`.
- **T-068.11 re-scope:** mission-embed + orbat summary shipped here (operator direction);
  remaining T-068.11 value = decide the compiled *mod* document loadout block (flatten output)
  for T-068.12 player equip.
- **T-146** Asset Browser wiring still unblocked (unchanged).
