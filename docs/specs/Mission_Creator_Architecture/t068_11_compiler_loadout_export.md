# T-068.11 — Compiler loadout export + ticket ship

**Ticket:** T-068 · **Slice:** T-068.11  
**Status:** Spec ready — **paused** (after **T-068.10**; map gate **T-090–T-092** + **T-071.2** first)  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Embed resolved loadouts in mission `json_payload` / compiled export superset so the **mod** can dress **human players** on spawn (**T-068.12**).

---

## Problem

Mission save/export does not carry loadout gear for game/mod consumption beyond manual profile file. **Phase 1** only proved equip on a **dev test NPC**. **Player spawn + slot picker verification** requires structured loadout data in compiled mission JSON (**this slice**) then mod equip (**T-068.12**) and slotting UI (**T-068.13**).

---

## Goal

1. Extend `compiler/compile.ts` — optional `loadouts` / per-slot gear block in editor superset (snake_case API, camelCase export envelope per existing rules).
2. Hydrate on load via `hydrateMissionDoc` if present.
3. Align with backend ORBAT `loadout` string fields where applicable (display summary only; ResourceNames in structured block).
4. FE build/lint + `make test-it` if compiler tests exist.

After human merge: **`./scripts/ticket done T-068`** + git tag **T-068**.

---

## Out of scope

- New API routes
- Map/topo

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Identity | ResourceName strings in compiled per-slot loadout block |
| Mod equip | **T-068.12** — player wear; this slice is **website/compiler data only** |
| Ship gate | **T-068.14** human Phase 2 E2E before `./scripts/ticket done T-068` |

---

## Tasks

1. `compiler/compile.ts` + `exportSchema.ts` updates
2. `hydrateMissionDoc` / types in tactical-map schema
3. Manual save version round-trip test

---

## Verify

```bash
cd apps/website/frontend && npm run build && npm run lint
PATH="$HOME/.local/go/bin:$PATH" make test-it
```

---

## Verification gate (mandatory)

**Website slice — after PASS, advance to T-068.12 (mod player equip). Ticket ships @ T-068.14.**

### Automated (exit 0)

```bash
cd apps/website/frontend && npm run build && npm run lint
PATH="$HOME/.local/go/bin:$PATH" make test-it
```

### Round-trip manual

| ID | Step | Pass condition |
|----|------|----------------|
| R1 | Save Version | POST 201; no compile error |
| R2 | Export JSON | Downloaded mission export contains loadout/editor block per spec |
| R3 | Reload | Reload mission URL → Arsenal/loadout state restored |
| R4 | Diff | Paste `jq` path proof: export contains `resource_name` gear keys |

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | Tests | `make test-it` green |
| A2 | Build | FE build/lint clean |
| A3 | Round-trip | R1–R4 all PASS |
| A4 | Advance | A1–A3 PASS → advance-slice to **T-068.12** (not `ticket done` yet) |

---

## Depends on / Unblocks

- **Depends on:** T-068.10
- **Unblocks:** T-068.12, T-068.13, T-068.14; T-069+ after full T-068 ship

---

## Documentation sync (Cursor)

Full [`AGENT_COMMIT_CHECKLIST.md`](../../website/AGENT_COMMIT_CHECKLIST.md): registry shipped, CLAUDE §Status, MC ROADMAP, feature_inventory, gap_analysis, agent_execution ACTIVE → T-069.

---

## Claude Code prompt — T-068.11

```
Read CLAUDE.md §Status. Active slice: T-068.11.
Implement ONLY docs/specs/Mission_Creator_Architecture/t068_11_compiler_loadout_export.md
Do not edit documentation. Branch: ticket/T-068
Verify: make test-it; FE build/lint; R1–R4 round-trip with jq proof
Return: Verify paste A1–A4 + export JSON snippet (gear block only).
```
