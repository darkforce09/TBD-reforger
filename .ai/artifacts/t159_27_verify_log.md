# T-159.27 — Arsenal loadout tab — verify log

**Slice:** T-159.27 (finish program stream 4 — the Arsenal Forge; fills the T-159.26 disabled
Attributes Arsenal stub).
**Worktree:** `.ai/artifacts/worktrees/TBD-T-159` · branch `t-159-leptos-ui` · **base** T-159.29 tip.
**Executor:** claude-code (solo session). **Result: PASS.**

## Goal

Port the React `ArsenalTab.tsx` + `arsenalRules.ts` "dumb Forge" (T-068.4 essence) into Leptos so a
placed slot's loadout is editable in the Attributes modal, persisted on the doc as the canonical
`SlotLoadoutV2` — the same shape the mod equip + compiler read. Replaces the .26 stub.

## What shipped

- **`arsenal.rs`** (new, 422 LOC incl. tests) — `ArsenalTab` component: one `<select>` per gear row,
  sourced from the flat `/registry` filtered by item `kind`, committed via `editor_ops::set_loadout`
  (one undo step per pick). The 12 `kind`-sourced rows match React `LOADOUT_ROWS` **exactly** (keys,
  labels, kinds, order); the 2 compat `edge` rows (optic/magazine feeds) + paper-doll + weight +
  Faction Manager fold forward to a Smart-Forge follow-on (the doc contract is unchanged, so they add
  rows/panels without altering what's persisted).
- **Faithful `picksToLoadout` / `loadoutToPicks` port** — canonical `SlotLoadoutV2`
  (`version: 2`, `wear{}` map with all 8 keys present-or-null, `weapons[]` with
  `slotIndex`/`slotType`, primary carries `optic`/`magazine`/`attachments: []`). Two parity gaps
  closed against a naïve port so a React-authored loadout never regresses on a Leptos re-save:
  1. **`summary`** — `buildLoadoutSummary` (display names of primary/optic/magazine/launcher, ` · `).
  2. **optic/magazine sticky pass-through** — `loadoutToPicks` captures the Smart-Forge sub-fields off
     `weapons[0]` so the dumb Forge re-emits them instead of dropping them. (`attachments` is `[]` in
     React too — no array pass-through needed.)
- **`editor_ops::read_loadout` / `set_loadout`** — read the slot loadout from `slots_json`;
  `update_slot_loadout` + `after_local_edit` (one undo step). `attributes.rs` mounts `ArsenalTab` in
  the Arsenal tab (index 3) with `slot_id` + the re-read `loadout_json` + the `registry_items` signal;
  `mission_editor.rs` populates `registry_items` from the existing `/registry` fetch.
- **`MissionEnv` relocation (native-compile fix)** — `eden_chrome.rs`'s **native** view-shell fallback
  referenced `crate::editor_ops::MissionEnv`, but `editor_ops` is `#[cfg(wasm32)]`-only, so the native
  `cargo test` harness **never compiled** (latent since .26 — the `.29` CI `cargo test` step would have
  gone red on first run). `MissionEnv` (pure data) moved to the always-compiled `dto.rs`, re-exported
  from `editor_ops` for wasm callers; `eden_chrome` native fallback now uses `crate::dto::MissionEnv`.
  **Net: the CI `cargo test` step goes red → green.**

## Gates

| Gate | Result |
|------|--------|
| `cargo check -p website-leptos --target wasm32-unknown-unknown` | clean (0 errors) |
| native lib check (`cargo check -p website-leptos`) | clean (0 warnings in touched files) |
| `cargo clippy … --target wasm32-unknown-unknown` | **12 = baseline** (stash-diff: 0 new) |
| `cargo test -p website-leptos` (native) | **46 passed** (42 prior + 4 new arsenal parity); newly compiling |
| `cargo fmt -p website-leptos --check` | clean |
| `trunk build --release` | ✅ success · wasm **7,153,883 B** |
| 15 editor smokes (incl. new arsenal) | **15/15 PASS** |

### Native clippy note (non-gating, honest)

Making the native harness compile for the first time surfaces **19** pre-existing latent native-shell
clippy warnings (`router`/`nav`/`mission_editor`/`leaderboards`/`personnel`/`mortar`/`layout`/
`event_manager` — the .25 suite pages' non-wasm branches). **None are in the .27-touched files.** The
CI clippy gate is **wasm32** (12, unchanged) and does not `-D warnings` on native, so these do not gate
CI; recorded here as a native-shell-hygiene follow-on now that native compiles.

## Arsenal smoke (`smoke_arsenal_editor.mjs`, Class R — 9/9)

Registry served from the committed `GET__registry.json` golden (the R-api fixture, 4 `gear_primary`
rows) via CDP `Fetch.fulfillRequest`, so the app runs its **real** `api_get::<RegistryResponse>` path:

| Check | Proves |
|-------|--------|
| `r1_open` | dbl-click seed slot → Attributes opens; Arsenal tab clicked |
| `r2_registryFetched` / `r2_selectsRendered` | 1 registry hit → 12 gear `<select>`s render (not "Loading catalog…") |
| `r3_m16Listed` | the Primary select lists the golden's M16A2; native `change` picks it |
| `r4_version2` / `r4_weaponSlot` / `r4_weaponIsPick` | `compile_save_json().editor.slots[].loadout` = canonical `SlotLoadoutV2` (`version 2`, `weapons[0]` slotIndex 0 / slotType primary / weapon = the pick) |
| `r5_oneUndoStep` / `r5_undoClears` | the pick is one undo step; real Ctrl+Z clears the loadout back to absent |

### Native parity tests (`arsenal::tests`, 4)

`all_empty_picks_clear_the_field` · `canonical_v2_shape_matches_react` (slotIndex/slotType/attachments/
8-key wear/summary) · `round_trips_through_the_doc_field` · `optic_magazine_survive_a_dumb_forge_resave`
(the pass-through regression guard + summary resolution).

## Folded forward (recorded, not lost)

Smart Forge: compat-worker optic/magazine edge rows, the clickable paper-doll (`arsenalDollModel`),
weight + validation, and the Faction Manager dialog. The persisted contract (`SlotLoadoutV2`) is
identical, so these are additive — no doc migration when they land.

## Next

All non-destructive T-159 finish-program streams (.24–.29 + .27) are complete and green. The remaining
work is **operator-gated + destructive**: the default SPA flip (prod `.env` + OAuth origin + staging
soak + real 142 MB save) and the React deletion (purge `apps/website/frontend/`, npm CI job, re-home
the `_wasm` parity oracles → `cargo test`, freeze the V DOM/PNG oracle). See `t159_29_verify_log.md`
§HELD and the hub doc.
