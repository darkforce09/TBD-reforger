# T-068.4 — Dumb loadout picker + JSON download

**Ticket:** T-068 · **Slice:** T-068.4  
**Status:** **shipped** @ `a85f16b` (git tag **T-068.4**)  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

**Replace** the Attributes **Arsenal tab stub** with a **working** (basic) loadout picker — four gear dropdowns + download `loadout-export.json`.

---

## Shipped @ T-068.4

**Arsenal tab is live** in [`AttributesModal.tsx`](../../../apps/website/frontend/src/features/mission-creator/layout/AttributesModal.tsx) + [`loadoutExport.ts`](../../../apps/website/frontend/src/features/mission-creator/loadout/loadoutExport.ts):

- Character slots only — four registry gear dropdowns + **Download loadout JSON**
- Non-character selection → empty state (“Loadout applies to placed characters.”)
- Export validates against `loadout-export.schema.json` (jq + `npm run validate`)

| | Before T-068.4 | After (shipped) |
|---|----------------|-----------------|
| Arsenal tab | Stub text + disabled button | **4 live dropdowns** + **Download JSON** (enabled) |
| Loadout export | Impossible | `loadout-export.json` per schema |
| Paper-doll / attachments | N/A | Still **out of scope** until T-068.10 |

---

## Problem

1. **No loadout UI** — stub blocks Phase 1 E2E (T-068.5/T-068.6 need a download path).
2. No way to compose a flat loadout or export JSON for mod equip test.

---

## Goal

1. **Replace `ArsenalTab()` stub** in `AttributesModal.tsx` — remove disabled Forge button and “later phase” copy entirely.
2. **Attributes modal → Arsenal tab** when exactly one slot selected (tab shell already exists; **implement content**).
3. **Character slots only:** if selected slot’s `assetId` is not a registry **`character`** row (lookup via `useRegistry`), show empty state (“Loadout applies to placed characters”) — do not render gear dropdowns for props/vehicles.
4. Four **working** `<select>` dropdowns from `useRegistry()` filtered by `gear_primary`, `gear_uniform`, `gear_vest`, `gear_helmet`.
5. **Download loadout JSON** — `modpackId` from `RegistryResponse.modpack_id` (same as `GET /registry`); schema fields `loadoutVersion`, `modpackId`, `gear`.
6. Optional: sessionStorage for last picks (UX only) — not Y.Doc Phase 1.
7. No attachment/mag/ammo UI; no `canEquip`; no paper-doll (smart Forge = **T-068.10**).

---

## Out of scope

- Smart Forge (T-068.10)
- Compiler mission export (T-068.11)
- Standalone `/loadout-forge` route

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| UI surface | **Replace stub inside** existing Attributes → Arsenal tab |
| Scope | **Functional dumb UI** — not “wire up existing Forge” (there is none) |
| Validation | Schema shape only on download (client-side) |
| E2E file | User saves download; copies to `$profile:TBD_LoadoutTest.json` for mod |

---

## Tasks

1. `AttributesModal.tsx` — **delete stub `ArsenalTab`** (disabled button + placeholder copy); implement real tab body
2. `loadoutExport.ts` — build + download blob helper
3. Wire registry gear filters from `useRegistry` data into four dropdowns
4. Types matching `loadout-export.schema.json`
5. Update `feature_inventory` **ATTR-TAB-004** on ship (stub → working dumb picker)

---

## Verify

```bash
cd apps/website/frontend && npm run build && npm run lint
# Stub must be gone — these strings must NOT appear in AttributesModal Arsenal tab
if rg -q 'Open Loadout Forge \(soon\)|lands in a later phase|arrives with Phase' \
  src/features/mission-creator/layout/AttributesModal.tsx; then
  echo "FAIL: Arsenal stub copy still present"; exit 1
fi
```

---

## Verification gate (mandatory)

### Automated (exit 0)

```bash
cd apps/website/frontend && npm run build && npm run lint
# Stub must be gone — these strings must NOT appear in AttributesModal Arsenal tab
if rg -q 'Open Loadout Forge \(soon\)|lands in a later phase|arrives with Phase' \
  src/features/mission-creator/layout/AttributesModal.tsx; then
  echo "FAIL: Arsenal stub copy still present"; exit 1
fi
```

### Manual + downloaded file

1. Dev stack up; single slot selected; **Attributes → Arsenal** tab visible.
2. Each dropdown lists ≥1 option (proves registry gear kinds wired).
3. Download `loadout-export.json` to `/tmp/loadout-export.json`.

```bash
jq -e '.loadoutVersion == "1"' /tmp/loadout-export.json
jq -e '.modpackId != null and .modpackId != ""' /tmp/loadout-export.json
jq -e '.gear | keys | sort == ["helmet","primary","uniform","vest"]' /tmp/loadout-export.json
# Each non-null gear value is a ResourceName
jq -e '[.gear[] | select(. != null)] | all(test("^\\{[0-9A-F]{16}\\}")?)' /tmp/loadout-export.json
cd packages/tbd-schema && npm run validate
```

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A0 | **Stub removed** | No “Loadout Forge (soon)”, “later phase”, or “arrives with Phase” copy; no disabled-only Arsenal UI |
| A1 | Build/lint | Exit 0 |
| A2 | Arsenal tab | Opens for exactly-one slot; real controls (not placeholder) |
| A3 | Dropdowns | Four selectors **enabled**, each ≥1 option from registry |
| A4 | Download | **Enabled** button saves file; jq structural checks pass |
| A5 | ResourceName | Every non-null gear value matches GUID regex |
| A6 | No worker | No `canEquip` / `registry.worker` in T-068.4 files |
| A7 | Schema package | `npm run validate` exit 0 |

### Verify paste (required)

Build/lint log + jq outputs + pasted `/tmp/loadout-export.json` contents (redact nothing except JWT).

---

## Depends on / Unblocks

- **Depends on:** T-068.2
- **Unblocks:** T-068.5, T-068.6

---

## Documentation sync (Cursor)

**Done @ T-068.4 ship (`a85f16b`):** `mission-editor.md` Element Inventory — Arsenal tab; `feature_inventory` **ATTR-TAB-004** → working; MC/FE/BE ROADMAPs; `eden/attributes.md` + `gap_analysis.md`; program hub shipped table.

---

## Claude Code prompt — T-068.6 (next — human only)

Human runs [`t068_6_phase1_e2e_gate.md`](t068_6_phase1_e2e_gate.md) E1–E12. No code. Sign-off paste → Cursor.
