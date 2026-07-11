# T-068.11 — Claude Code handoff (Compiled loadout block)

**Spec:** [`t068_11_compiler_loadout_export.md`](../../docs/specs/Mission_Creator_Architecture/t068_11_compiler_loadout_export.md)
· **Baseline:** T-068.10 @ `3bc0bd24` · **CWD:** repo root `main`.

## Simple version

Editor already saves loadouts on slots. This slice only puts that gear onto the **compiled
mod JSON** (`/compiled`) so the game can read it in **T-068.12**.

## Do not

Redo Arsenal / editor.slots loadout · mod equip · docs/registry.

## Return

SHA + tag **T-068.11** · sample compiled slot with gear · ready for T-068.12.
