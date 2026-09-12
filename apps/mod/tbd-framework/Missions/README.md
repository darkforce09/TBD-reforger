# Missions

Scenario configuration files (`.conf`) declaring playable game headers for Workbench and dedicated servers.

### Roles & Responsibilities
- `TBD_Dev_POC.conf`: Development scenario header defining player limits (128), scenario metadata, starting subscene (`worlds/TBD_Dev_POC.ent`), and game mode bindings.

### Call Flow & Contracts
Loaded by the Reforger server or Workbench launcher (`-config` or scenario selector) to initialize the world and attach the TBD framework game mode.
