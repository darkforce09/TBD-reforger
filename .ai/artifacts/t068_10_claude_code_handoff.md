# T-068.10 — Claude Code handoff (Smart Arsenal / Forge)

**Spec:** [`t068_10_smart_forge_ui.md`](../../docs/specs/Mission_Creator_Architecture/t068_10_smart_forge_ui.md)
· **Baseline:** T-068.9 @ `d41418e5` · **CWD:** repo root `main`.

## Simple version

Make the Arsenal tab ask the worker “does this fit?” before allowing the pick / export.

## APIs to use

`initRegistryCompat` · `canAttach` · `canEquip` · `itemsFor` —
`features/mission-creator/registry/registryCompatClient.ts`

## Do not

Docs/registry · compiler · map browser · invent edges.

## Return

SHA + tag **T-068.10** · verify log.
