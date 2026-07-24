# T-068.15.2 — Arsenal capacity + cargo UI + seed · verify log

**Date:** 2026-07-24 · **Executor:** Fable 5 / Claude Code · **Branch:** `main` ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t068_15_2_arsenal_cargo_ui.md` ·
**Baseline:** tag **T-068.15.1** @ `85acbb13`

## Result

**PASS** (one explicit operator visual remains — see §Operator). Arsenal shows per-container
capacity (registry-only), edits `SlotLoadoutV2.cargo[]`, and seeds `character_default_cargo`
defaults at place / apply-kit / Arsenal-open under a strict idempotency rule.

## What shipped

### Domain (`arsenal_rules.rs`, pure + native-tested)
- `CargoRow {container, item, qty}` (loadout-export v2 row shape) + `cargo_from_loadout` /
  `cargo_rows_json` round-trip (malformed rows drop; `qty ≥ 1`).
- `cargo_container_from_evidence`: `TargetStorage=` first path segment → `pants` / `jacket` /
  `vest` / `backpack` (`Back…` prefix); unknown → skipped, never guessed (spike lock).
- `cargo_defaults_by_character`: built from the **raw** compat rows (the `CompatGraph`
  deliberately drops evidence/qty), aggregated by (container, item), qty summed, deterministic
  order.
- `seed_cargo`: eligible **only when the loadout has no `cargo` key** (or no loadout — a minimal
  V2 shell with all-null wear is created); a present key (`[…]`, `[]`, `null`) is user state and
  is never reseeded.
- `cargo_budget` + `CargoBudget::over()`: warn-only Σ(weight/volume × qty) vs the garment's
  `max_weight_kg` / `max_volume_cm3`; absent capacity never warns (never invented).

### Persistence (`arsenal.rs`)
- `picks_to_loadout(picks, names, cargo: Option<&[CargoRow]>)`: `Some(rows)` re-emits the key on
  every persist (the commit fires per pick change — dropping it would wipe seeded rows);
  `Some(&[])` keeps the cleared-list marker; `None` stays key-less so a later seed can fire
  (protects the compat-outage race). All-empty rule now counts cargo.
- ArsenalTab: open-time seed (`editor_ops::seed_slot_cargo`) for pre-existing slots; `cargo` +
  `cargo_present` signals; capacity badge (top row: `max N kg · W×H grid`, absent parts simply
  don't render — PASGT shows nothing, by design); per-container cargo editor (rows with qty
  stepper + remove, add-picker over kinds `magazine/ammo/gear_item/gear_throwable/gear_explosive`,
  warn-only budget line, red when over); `armoredVest` garment backs the `vest` container key.
- Download button now carries `cargo[]` under the same key-presence rule.

### Wiring (`editor_ops.rs`, `mission_editor.rs`)
- `mission_editor` compat fetch → `cargo_defaults_by_character(raw rows)` →
  `editor_ops::set_cargo_defaults` (thread_local beside `OPS_CTX` — `set_ctx`'s 12-arg
  signature untouched).
- `place_at`: seeds the freshly placed character inside the same doc borrow ⇒ **one undo step
  with the place** (`payload.asset_id` captured before the move).
- `orbat_apply_faction`: post-pass over `slots_json` seeding every eligible slot (cargo-key-absent
  only ⇒ user edits and library-carried `cargo[]` are preserved) ⇒ one undo step with the apply.
- `seed_slot_cargo(id)`: Arsenal-open path with its own history tail; returns the seeded JSON so
  the first render needs no re-read.

## Automated gates (all exit 0, pipefail)

```bash
make schema-validate            # PASS (no schema change this slice — regression only)
cargo test -p website-frontend  # 94 passed (+5 new: container mapping, aggregation,
                                #   seed idempotency, cargo round-trip+budget, key-presence)
make ci-local-leptos            # fmt + clippy(wasm32, -D warnings) + tests + trunk release PASS
make leptos-gates               # gate doctor + editor smokes + frozen V-suite (headless Chrome)
```

## Live half of the manual gate (real stack, curl — API on :8080, dev DB)

- `GET /registry`: 1257 rows carry `cargo_grid_*`; `Jacket US BDU` serves
  `{"max_weight_kg":5.0,"cargo_grid_w":4,"cargo_grid_h":4}` — the badge's exact inputs.
- `GET /registry/compat?edge_type=character_default_cargo`: 5919 rows, 4140 with `qty>1`,
  **Σqty = 16223** (conservation, live).
- `Character_US_Rifleman` defaults sample (seed input): `MorphineInjection_01 ×2 →
  TargetStorage=Pants/…` (≥1 medical ✓), STANAG mags in `Vest/…` pouch paths (≥1 mag ✓),
  flashlights → jacket/vest — container mapping covers each observed evidence shape.

## Operator (explicit, not silently skipped)

The Claude-in-Chrome extension is not connected in this session, so the in-browser
click-through (badge/panel rendering, place-seed visual, undo feel) could not be screen-driven.
Everything the click-through would assert is proven at the layer below (native tests + live API
+ headless editor gates); please eyeball on next dev-login: vest → capacity badge; place US
rifleman → Cargo panel pre-filled (morphine in pants, mags in vest); clear list → reopen stays
empty; qty edit → single undo step.

## Known limitations (explicit)

- Hand-added ORBAT slots (`orbat_add_slot`) carry no `assetId` → nothing to seed until a
  character is placed/applied (expected; same rule as loadout display).
- Cargo add-picker offers the flat eligible-kind list (no per-container compat narrowing) —
  budget warnings are the guard; matches the "list minimum" spec scope.

## Ready for Cursor

Registry/status/doc sync per this log. Tag: **T-068.15.2**.
