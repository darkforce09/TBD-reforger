# T-068.10.3 — Forge picker UX on clean kinds

**Ticket:** T-068 · **Slice:** T-068.10.3 · **Status:** queued (starts after the T-068.10.2
operator pause) · **Executor:** claude-code ·
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·
**Data contract:** [`t068_10_2_exporter_reclassify.md`](t068_10_2_exporter_reclassify.md) ·
**Taxonomy:** [`.ai/artifacts/ace_arsenal_taxonomy_map.md`](../../../.ai/artifacts/ace_arsenal_taxonomy_map.md)

---

## In one sentence

Rebuild the Arsenal tab pickers on the v3 kinds so they read like ACE Arsenal — per-wear-area
rows, grouped and alphabetized options, abstracts hidden, searchable — while keeping the
T-068.10 compat validation and degrade paths intact.

## Problem

`LOADOUT_ROWS` (`arsenalRules.ts:35-50`) still models ACE Phase-1 (primary/optic/magazine/
uniform/vest/helmet); options render in raw `sort_order` (envelope index) with no grouping,
no search, and abstract `* Base` templates listed as picks.

## Goal

1. **Row set v2** (frozen from census populations; a row ships only when its kind has >0
   non-abstract rows): `jacket (61)`, `pants (33)`, `boots (6)`, `vest (34)`,
   `armored vest (12)`, `helmet (92)`, `backpack (43)`, `primary (84)`, `launcher (18)`,
   `throwable (15)`, `optic`, `magazine` (compat-fed rows unchanged). `handgun (5)` may ship
   behind the same mechanism if trivially free; otherwise named later.
2. Options: `<optgroup>` by `category`, locale-aware alpha sort inside groups,
   `abstract == true` excluded everywhere.
3. Search/filter box over the open picker (T-055 catalog-search pattern).
4. Degrade paths preserved: compat worker down → kind-filtered dumb rows; kind empty → row
   hidden with explanatory note.
5. Editor doc `SlotLoadout` **unchanged** in this slice (v1 fields; jacket writes `uniform`,
   armored-vest row disabled-with-note OR maps to `vest` — decide in implementation notes and
   verify log; full shape lands in T-068.10.4).

## Out of scope

SlotLoadout/loadout-export shape change (T-068.10.4) · cargo/equipment micro-slots · favorites/
loadout library · mod-filter UI (data ships in .10.2; UI later) · paper-doll.

## Verify (gates)

| # | Assertion | Method |
|---|-----------|--------|
| F1 | Per-row option count == API/DB count for that kind with `abstract = false` (numbers inline from fixture) | vitest |
| F2 | Group + sort: rendered option order equals locale-sorted copy | vitest |
| F3 | Abstract exclusion: `Rifle M16A2 base` (and count) absent from primary row | vitest |
| F4 | `Grenade RGD5` + `Smoke M18 Red` in throwable row and NOT in primary row | vitest |
| F5 | build + lint + full vitest suite green | exit 0 |
| F6 | Manual screenshot for operator (grouped pickers + search) | operator pause |

## Acceptance

- [ ] Row set v2 live on v3 kinds; F1–F5 PASS; verify log + tag **T-068.10.3**.
- [ ] PAUSE for operator visual review before T-068.10.4.
