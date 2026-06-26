# T-068.5 — Mod equip from loadout-export JSON

**Ticket:** T-068 · **Slice:** T-068.5  
**Status:** Spec ready — code pending  
**Executor:** claude-code (**enfusion-mcp required** for compile/reload/play verify)  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md)

---

## In one sentence

Mod reads `$profile:TBD_LoadoutTest.json` and equips exact ResourceNames on test spawn.

---

## Problem

Downloaded loadout JSON has no in-game consumer.

---

## Goal

1. Enfusion component or spawn hook reads JSON matching `loadout-export.schema.json`.
2. On test spawn (dev scenario): apply primary/uniform/vest/helmet via exact `ResourceName` APIs — no alias layer.
3. Log each equip success/failure to console.
4. Document profile path in mod README / this spec.

---

## Out of scope

- Mission `json_payload` compiler path (T-068.11)
- Compat validation

---

## Locked decisions

| Decision | Choice |
|----------|--------|
| Phase 1 input file | `$profile:TBD_LoadoutTest.json` (manual copy from web download) |
| Identity | Full ResourceName strings only |

---

## Tasks

1. `TBD_LoadoutEquipComponent.c` or extend spawn manager in `apps/mod/tbd-framework/`
2. JSON parse + equip calls
3. Dev scenario test entity

---

## Verify

```bash
# Preflight — Claude Code runs bootstrap (auto-launches Workbench if :5775 closed)
bash scripts/mod/tbd-dev-bootstrap.sh
bash scripts/mod/mcp-call.sh wb_connect '{}'

# Copy T-068.4 download first:
cp /path/to/loadout-export.json "$HOME/.local-test-profile/TBD_LoadoutTest.json"
# or $profile path documented in scripts/mod/

bash scripts/mod/mcp-call.sh wb_reload '{"scope":"scripts"}'
bash scripts/mod/mcp-call.sh wb_play '{}'
sleep 5
bash scripts/mod/mcp-wb-logs.sh | grep -E '\[TBD\].*Loadout|Loadout equip' | tail -20
bash scripts/mod/mcp-call.sh wb_stop '{}'
```

---

## Verification gate (mandatory)

### Preconditions

- T-068.4 download JSON validates (paste same file hash or contents in verify block).
- Profile file path **exactly** documented in paste (no ambiguity).

### Acceptance criteria

| ID | Check | Pass condition |
|----|-------|----------------|
| A1 | File read | Console log contains `Loadout`/`TBD_LoadoutTest` loaded — **no** parse error |
| A2 | Primary | Log line equipping **primary** ResourceName from JSON — **OK** or explicit success |
| A3 | Uniform | Log line for **uniform** — success |
| A4 | Vest | Log line for **vest** — success |
| A5 | Helmet | Log line for **helmet** — success |
| A6 | No alias | Logged strings contain `{GUID}` form — **not** `kit:` aliases |
| A7 | Visual | In-game entity shows expected kit (screenshot or short video timestamp) |

**FAIL if:** any slot logs `FAILED` / `unknown` / skipped without documented reason.

### Verify paste (required)

Paste **20+ lines** of console.log around spawn + profile path + SHA256 of JSON file:

```bash
sha256sum "$PROFILE/TBD_LoadoutTest.json"
```

---

## Depends on / Unblocks

- **Depends on:** T-068.0.1, T-068.4
- **Unblocks:** T-068.6

---

## Documentation sync (Cursor)

After verify paste: link mod script path in T-068.6 checklist.

---

## Workbench checklist

Map 1:1 to **§Verification gate** A1–A7 — all PASS before paste.

---
