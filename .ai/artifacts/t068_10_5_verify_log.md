# T-068.10.5 verify log — weapon variant collapse (variant_of)

**Date:** 2026-07-13 · **Executor:** Claude Code (operator-approved Mode C session) ·
**Census contract:** [`t068_10_5_weapon_families.md`](t068_10_5_weapon_families.md) (+ `.json`)
— committed BEFORE the scanner change; per-weapon evidence columns.

## Status

**PASS — all gates green.** 71 concrete weapons = **33 keep** (pickers) + **38 variants**
(`variant_of` set, hidden like abstracts). Primary picker 58 → **21 real weapons**; operator's
named offenders all collapsed (1P29s, GP-25s, PBS-4s, PGO-7, camo skins, Tutorials).

## Rule (census-locked, engine-data only)

VARIANT iff: same family dir (`/Variants/` folds up) + longest strict filename-stem prefix
parent + equal magazine wells + equal attachment-slot-TYPE set + equal base mesh — i.e. the
prefab differs only by pre-mounted attachments / camo materials. Fail-safe: any differing or
unresolvable evidence → KEEP. Engine-truth notes for operator review in the census artifact:
M203-integrated M16s KEEP (Reforger bakes that UGL into the weapon — own mesh + 40mm well;
GP-25 is an attachment item and collapses, matching the operator's read); one `Wrapped` M21
scope model survives on mesh-diff.

## Shipped

- Schema v3.2: optional `variant_of` (resourceName pattern) + validate.mjs strict integrity
  check (must reference an envelope item; self-ref forbidden); sample fixture row; codegen.
- DB migration `0005_registry_variant_of.sql` (nullable text + partial index); Rust model +
  UNNEST import + GET /registry projection; FE `RegistryItem.variant_of`.
- Scanner: `ComputeWeaponVariants()` after DeriveEdges (final kinds) — mirrors the census
  rule with the already-collected `muzzleWells`/`slotAttachTypes` + new leaf-most `meshRef`
  (`MeshObject.Object`, most-derived-first bucket); `variants=N` quality counter; emission.
- Pickers: `rowValues` hides `variant_of` rows exactly like `abstract` (a live variant pick
  never blanks).

## Gates

| # | Assertion | Result |
|---|-----------|--------|
| V | Envelope `variant_of` map == census verdicts, per item, both directions (no keep-list row flagged, no unexpected variants, none outside weapon kinds) | PASS — 38/38 exact, first compile |
| R | .10.2 regression harness (per-item kinds, histograms, pollution, abstracts, totals, edges, addon) on the new export | PASS — all 9 gates, unchanged |
| G7 | Double export byte-identical modulo `generatedAt` | PASS |
| S | `npm run validate` incl. variant integrity (sample 1, workbench 38) | PASS |
| D | Import: 38 rows updated then idempotent; DB variant count 38; visible non-variant primaries **21** (SQL) | PASS |
| F | vitest F-gates: row counts vs envelope (primary 21 / launcher 10 / handgun 2 / throwable 10 / jacket 46 / vest 28 / helmet 68), `Rifle AK74N` present + `Rifle AK74N 1P29` absent + PGO-7 flagged + live-pick-never-blanks; full suite **335/335**; build + tsc clean; lint = pre-existing router.tsx only | PASS |
| T | `make test-it` — all suites green (stale 1880/4012 pins updated to the census-gated 1857/4685 envelope with comment) | PASS |

Compile cycle: session `logs_2026-07-13_01-18-51`, clean compile, `variants=38` on first
run — census → plugin translation exact.
