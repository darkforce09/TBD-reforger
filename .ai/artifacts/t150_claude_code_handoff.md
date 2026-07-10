# T-150 — Claude Code handoff (Universal registry + compat export)

**Spec (wins on conflict):**
[`t150_universal_registry_export.md`](../../docs/specs/Mission_Creator_Architecture/t150_universal_registry_export.md)
· **Program hub:**
[`t068_virtual_arsenal_program.md`](../../docs/specs/Mission_Creator_Architecture/t068_virtual_arsenal_program.md)
· **Working tree:** repo root **`main`** — `/var/home/Samuel/Projects/TBD-Reforger`.

## Operator intent

Build the **data foundation** for arsenal + vehicles: every loaded mod’s items and
compatibility edges. **Not** map markers. **Not** Forge UI. Export must scale by loading
more addons in Workbench and re-running — never by editing curated lists.

## LANGUAGE / MOD RULES

- Never guess Enfusion APIs or ResourceNames — MCP `api_search` / asset tools first.
- Production mod path: `apps/mod/tbd-framework/` only.
- No silent deferrals: if a compat family cannot be read from the engine, say so in the
  verify log and leave that family empty — do not invent edges.

## CURRENT STATE

| Piece | Status |
|-------|--------|
| T-068.1 curated export | Shipped (~21 rows) — **replace** |
| registry-items schema | Phase 1 kinds only |
| Compat schema / export | Missing |
| T-069 markers | Parked (not this slice) |

## What you are building

1. Schema v2 (items kinds + compat edges).
2. Universal Workbench export plugin.
3. Committed sample JSON + verify log + tag **T-150**.

## Do not

- Edit docs/registry/CLAUDE (verify log OK).
- Hardcoded allowlists.
- Ingest / UI / map placement.

## Return

SHA + tag · counts · edge histogram · Cursor: T-068.8 sync + T-068.9 next.
