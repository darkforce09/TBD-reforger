# Configs

Enfusion engine configuration files (`.conf`) defining actions, input contexts, and menu presets.

### Roles & Responsibilities
- `System/chimeraMenus.conf`: Registers TBD menu IDs against their layout resources (`.layout`) and backing menu controller classes.
- `System/ActionContext/`: Action contexts declaring active keybinding layers (e.g. `TBD_SpectatorContext`, `TBD_BrowserContext`).
- `System/Actions/`: Custom input action definitions mapping keyboard/mouse/gamepad bindings to named engine actions (admin menu, spectator camera controls).

### Call Flow & Contracts
Loaded by the Enfusion engine on project startup. Referenced by `UI/Core/` and `Session/` scripts for input detection and menu instantiation.
