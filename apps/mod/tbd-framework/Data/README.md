# Data

Static mod data files, default lookup tables, and example configuration templates.

### Roles & Responsibilities
- `registry.json`: Canonical alias registry resolving semantic mission equipment and vehicle keys (`kit:*`, `vehicle:*`) to vanilla prefab GUIDs.
- `backend.example.json`: Template configuration file documenting parameters for `$profile:TBD_BackendConfig.json`.

### Call Flow & Contracts
`registry.json` is packaged with the addon (`$TBD_Framework:Data/registry.json`) and loaded statically by `Core/TBD_Registry.c` at mission initialization.
