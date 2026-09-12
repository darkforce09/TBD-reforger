# Core

Foundational, zero-dependency utilities, structured logging, and asset alias resolution.

### Roles & Responsibilities
- `TBD_Log.c`: Static structured logger outputting standard greppable `[TBD][<channel>]` events across all mod systems.
- `TBD_Registry.c`: Static asset resolver translating mission semantic alias strings (`kit:*`, `vehicle:*`) into Enfusion prefab resource paths via `Data/registry.json`.
- `TBD_RegistryPocComponent.c`: Development harness component spawning registered aliases in Workbench for visual verification.

### Call Flow & Contracts
Pure static utilities consumed across all domains without circular dependencies.
