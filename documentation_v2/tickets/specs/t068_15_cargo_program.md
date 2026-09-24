# T-068.15 — Arsenal cargo capacity + default contents program

**Status:** **COMPLETE** (through T-068.12) · Fable 5 @ 2026-07-24  
**Authority:** [`t068_virtual_arsenal_program.md`](t068_virtual_arsenal_program.md) ·
[`.ai/tickets/registry.json`](../../../.ai/tickets/registry.json)  
**Spike:** [`.ai/artifacts/t068_cargo_capacity_spike.md`](../../../.ai/artifacts/t068_cargo_capacity_spike.md)

---

## In one sentence

Per-garment **max weight** + **cargo grid W×H** + **default cargo** auto-exported from
Workbench, shown/edited in Arsenal, compiled onto `/compiled`, and equipped on the joining
player (including `InsertItem` cargo).

## Slice ladder (shipped)

| Slice | Title | Tag / SHA | Verify |
|-------|-------|-----------|--------|
| **T-068.15.1** | Export capacity + default cargo | **T-068.15.1** @ `85acbb13` | [log](../../../.ai/artifacts/t068_15_1_verify_log.md) |
| **T-068.15.2** | Arsenal capacity + cargo UI + seed | **T-068.15.2** @ `4fb156b7` | [log](../../../.ai/artifacts/t068_15_2_verify_log.md) |
| **T-068.11** | Compiled loadout incl. cargo | **T-068.11** @ `c66494c6` | [log](../../../.ai/artifacts/t068_11_verify_log.md) |
| **T-068.12** | Player equip + InsertItem cargo | **T-068.12** @ `0be53e16` | [log](../../../.ai/artifacts/t068_12_verify_log.md) |

```text
T-068.15.1 export  →  T-068.15.2 Arsenal UI  →  T-068.11 compile  →  T-068.12 equip
         ✓                    ✓                     ✓                    ✓
```

## Headline results

- **Export:** 1857 items / 1257 grids / 20,908 edges (16,223 cargo); qty conservation;
  jacket 4×4 / trousers 4×3 after Universal-storage two-pass fix (flashlight shadow).
- **Arsenal:** capacity badge + cargo editor + seed; leptos-gates 25/25.
- **Compile:** `slot.loadout {gear, cargo[]}` on `/compiled` (no schemaVersion bump).
- **Equip:** live WB — M16/BDU/PASGT worn-verified; morphine ×2/2 pants; STANAG ×3/3 fallback.

## Operator residual

From [T-068.12 verify](../../../.ai/artifacts/t068_12_verify_log.md): **M2** dressed-player
screenshot; optional **M4** NPC-toggle. Not ticket blockers for advancing to **T-068.13**.

## Next

**T-068.13** LOBBY slot picker → **T-068.14** Phase-2 E2E → then `ticket done T-068`.
