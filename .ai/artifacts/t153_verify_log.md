# T-153 verify log — Faction Library

**Date:** 2026-07-13 · **Executor:** Claude Code (Mode C session) ·
**Spec:** `docs/specs/Mission_Creator_Architecture/t153_faction_library.md`

## Status

**PASS.** Backend + frontend shipped; operator visual at the Mode C pause.

## Gates

| # | Assertion | Result |
|---|-----------|--------|
| S | `npm run validate` — faction-library golden (real census GUIDs incl. the 6B2+Lifchik two-vest role loadout) | PASS |
| C | `make schema-codegen` TS+Rust projections; drift clean on re-run | PASS |
| B1–B8 | `tests/factions.rs`: enlisted 403 (list+create) · schema-invalid 400 with `faction-library.schema.json` details · 201 create from golden with side/name projected · duplicate name 409 · list = own rows only (house shape) · foreign row 404 + never listed · full-doc update flips side/name · delete 204 → 404 | PASS — `make test-it` 22 suites ok, 0 failed (fresh DB runs migration 0006) |
| F1 | buildFactionTree: fixed side order + Aegis accents; faction → Roles/Vehicles nesting | PASS (vitest) |
| F2 | Role leaf payloads: assetId + tag + SlotLoadout v2 + factionRef; loadout-less role → no loadout key | PASS |
| F3 | Vehicle leaves listed, badge T-070, NO payload (drag preventDefault) | PASS |
| F4 | **Structural vanilla purge**: tree ids derive only from library rows; every payload carries factionRef; empty library → empty tree + Manager CTA (registry characters cannot appear — the builder never enumerates registry kinds) | PASS |
| F5 | Full frontend suite **340/340** · build clean · `tsc --noEmit` clean · lint = pre-existing `router.tsx` only | PASS |
| L | Live smoke on the restarted API: `GET /factions` `[]` → `POST` golden **201** → list total 1 (OPFOR · Soviet Army 1980s · roles 2) | PASS |

## Ops notes

- The dev API binary is named `api` (not `reforger-backend`) — kill by `ss -tlnp` PID when
  restarting after handler changes; Rust does not hot-reload (T-060 note, hit twice this
  program).
- Drop path: `ensureFaction(md, side, name)` matches doc factions by key+name and mints on
  first drop; `ensureSquadFor` gives each library faction its own squad ("1st Squad").
  First drop per faction is a structural multi-step undo (documented addSlot behavior).

## Operator visual checklist (pause)

1. Hard-refresh the editor tab. Factions palette shows **Manage** + empty-state CTA
   (vanilla dump gone).
2. Faction Manager: create "US Army 1980s" (BLUFOR), add a role (character + loadout via
   the 14-row panel — variant-collapsed weapon list), add a vehicle; Save.
3. Palette: BLUFOR (blue flag) → US Army 1980s → Roles/Vehicles; drag the role onto the
   map → slot lands with tag + loadout; ORBAT outliner shows the faction with its squad.
4. Vehicles listed with T-070 badge, not draggable.
