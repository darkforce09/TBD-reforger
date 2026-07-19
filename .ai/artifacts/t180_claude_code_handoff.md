# T-180 — Claude Code / Grok handoff (ORBAT + Eden placement)

**Hub:** [`docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md`](../../docs/specs/Mission_Creator_Architecture/t180_orbat_eden_program.md)  
**Pins:** [`docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md`](../../docs/specs/Mission_Creator_Architecture/t180_class_r_pins.md)  
**Active:** **T-180.3** — [`t180_3_map_side_tint.md`](../../docs/specs/Mission_Creator_Architecture/t180_3_map_side_tint.md)  
**CWD:** repo root · **Branch:** `main`

## Shipped

| Slice | SHA / tag | Verify |
|-------|-----------|--------|
| **T-180.1** | `aeb51209` / T-180.1 | [`t180_1_verify_log.md`](t180_1_verify_log.md) |
| **T-180.2** | `83557768` / T-180.2 | [`t180_2_verify_log.md`](t180_2_verify_log.md) |

## Now — T-180.3

Map slot rings tinted by faction **key** (side). Exact RGBA in pins:

- BLUFOR `[173,198,255,255]`
- OPFOR `[248,113,113,255]`
- INDFOR `[34,197,94,255]`
- Selected stays `[250,204,21,255]`

Copy prompt from `t180_3_map_side_tint.md`.

## Order

```text
.1 ✓ .2 ✓ → .3 (NOW) → .4 → .5 → .6 → .7 → .8 → .9
```

## Do not

Docs/registry · squad leader lines (.4) · CSS-only fake tint

## Return

SHA + tag `T-180.3` · `.ai/artifacts/t180_3_verify_log.md` · Ready for T-180.4
