# T-175 — Claude Code handoff

**Start on `main` after T-174 @ `bbb99526`.** Do not touch `apps/mod/` or docs/registry.

## Operator word (2026-07-18)

Sat/heatmap/guides look good. Remaining:

- Zoom in/out **stutter** (not fine); pan stutter — forest highlight suspect.  
- Forest mass delay after zoom-out too long.  
- **Tree glyphs sticky on zoom-out** — stay after load; only hide out-of-frame.  
- Contours too dark.  
- Selection/highlight laggy.  
- First load: **slots wrong position**.  
- Palette drag: **no ghost** until mouse-up.  
- Slot move: **~1 px then jump** on release (commit works).  
- **Loading bar / loading screen** so the user knows the machine is still loading (React T-060 overlay missing on Leptos).  
- Also: experience hunt for other optimization / LOD / interaction holes.

## Hottest lead (A1)

`apps/website/frontend/src/world_assets/world_host.rs` — sticky empty glyph upload:

```text
if !trees.is_empty() || pin_settled {
    e.upload_icon_lane(0, &trees, true);
}
```

When LOD clears trees but `pin_settled` is false, the empty buffer is **not** uploaded → GPU keeps the previous tree instances. Matches “glyphs stay when zoomed out.”

## Authority

1. [`docs/platform/t175_mc_interaction_lod_perf.md`](../../docs/platform/t175_mc_interaction_lod_perf.md)  
2. This handoff  

## Matrix quick map

| ID | Area |
|----|------|
| A1 | Tree unpack / GPU clear |
| A2 | Forest settle latency |
| A3 | Contour contrast (`CONTOUR_RGBA`) |
| A4–A5 | Pan/zoom stutter |
| B1 | Cold-load slot bind |
| B2 | Place ghost |
| B3 | Drag preview (`set_drag`) |
| B4 | Selection tint lag |
| B5 | Boot loading bar/screen (hydrate + map readiness) |
| C* | Mandatory hunt |

## Return

Tag **T-175** @ sha · inventory + verify · A/B/H PASS · Cursor list · ASK if blocked.
