# T-068.11 — Compiler loadout export + ticket ship

**Ticket:** T-068 · **Slice:** T-068.11  
**Status:** Spec ready — Phase 2  
**Executor:** claude-code  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Embed resolved loadouts in mission `json_payload` export superset; complete T-068 code path.

---

## Problem

Mission save/export does not carry loadout gear for game/mod consumption beyond manual profile file.

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
| Ship gate | Last code slice before `ticket done` |
| Identity | ResourceName strings in compiled payload |

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

**Last code slice — after PASS, human merges and runs `./scripts/ticket done T-068`.**

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
| A4 | Ticket done | Only after A1–A3: `./scripts/ticket done T-068` + git tag **T-068** |

---

## Depends on / Unblocks

- **Depends on:** T-068.10
- **Unblocks:** `./scripts/ticket done T-068`; T-069+

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
