# T-139 — Plan

## Context
The lobby lists slots without showing their kit. Gear is already replicated in the lobby data (13 fields since
T-182); an icon grid needs only a widget, a layout and a selection hook. Packs after T-941.2 on TBD_LobbyScreen.c.

## Approach
1. New `UI/Lobby/TBD_LoadoutPreview.c`: `Refresh(TBD_MissionSlotStruct)` builds up to 13 icon cells from the
   registry icon path; skips empty fields; logs `[TBD][LoadoutPreview] slot=<n> items=<k>`.
2. New `UI/layouts/TBD_LoadoutPreview.layout` (grid panel, ListRow-style cells).
3. `UI/Lobby/TBD_LobbyScreen.c`: host the widget beside the slot list; call Refresh on selection change.
4. `cargo xtask mod compile`; MANUAL checklist item.

## Risks
- Icon path missing for some registry items → placeholder glyph, never a null widget.
- The 2D mannequin stretch is out of acceptance; icon grid only.

## Verification
- `cargo xtask mod compile` · `cargo xtask platform wave gate --slice T-139`
- Human checklist: select three slots; each shows its own icons; empty fields absent.
