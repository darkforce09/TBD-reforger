# Archive — master index

**Status:** archived (reference only)  
**Audience:** historians, design archaeology, agents needing context  
**Authority:** Live code and living docs supersede everything here  
**Updated:** 2026-06-20

Preserved material from the design phase. **Do not implement from these files.**

## Visual references (HTML)

| Location | Contents | Live alternative |
|----------|----------|------------------|
| [`Design_Docs/macOS_Blueprints/`](../../Design_Docs/macOS_Blueprints/) | 24 page blueprint folders (`code.html`, `screen.png`) | [`frontend/src/pages/`](../../frontend/src/pages/) |
| [`frontend/src/stitch-exports/`](../../frontend/src/stitch-exports/) | 21 Stitch export folders | [`frontend/src/pages/`](../../frontend/src/pages/) + [`features/`](../../frontend/src/features/) |
| [`Design_Docs/Mission_Creator_Mock_Up/`](../../Design_Docs/Mission_Creator_Mock_Up/) | Mission creator product mockups | [`features/mission-creator/`](../../frontend/src/features/mission-creator/) |

## Design prose (partially stale)

| Doc | Status | Live alternative |
|-----|--------|------------------|
| [`docs/platform/context_handoff.md`](../platform/context_handoff.md) | §3 data models pre-T-008; §4 UI blueprints useful | `internal/models/` + live pages |
| [`docs/backend/architecture.md`](../backend/architecture.md) | Target schema — verify vs models | [`internal/models/`](../../internal/models/) |
| [`Design_Docs/Mission_Creator_Mock_Up/mission_creator_design.md`](../../Design_Docs/Mission_Creator_Mock_Up/mission_creator_design.md) | Product vision / JSON contract philosophy | MC `05` Decisions log + `04` Eden UX spec |
| [`docs/platform/registration_flow.md`](../platform/registration_flow.md) | Design doc — **implemented** | Live handlers + Event Hub UI |

## Entry format

Each archived item: **Status: archived** · **When to use:** historical reference · **Live alternative:** linked above.

## Related

- [Frontend master](../frontend/README.md) — links stitch + blueprints
- [Platform doc hub](../README.md)
