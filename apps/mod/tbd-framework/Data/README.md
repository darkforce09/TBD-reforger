# Data

Static mod data files, default lookup tables, and example configuration templates.

### Roles & Responsibilities
- `registry.json`: Canonical alias registry resolving semantic mission equipment and vehicle keys (`kit:*`, `vehicle:*`) to vanilla prefab GUIDs.
- `backend.example.json`: Template configuration file documenting parameters for `$profile:TBD_BackendConfig.json`: `backendUrl`, `serverToken` (shared `X-Service-Token` of link confirmation and match results) and `machineCredential` (this server's `mod_runtime` machine credential, `tbdm_...`; the placeholder does not start with `tbdm_`, so the mod reports it as not configured). The mission and its event are not configured: the server runs the mission deployed to it on the platform.

### Call Flow & Contracts
`registry.json` is packaged with the addon (`$TBD_Framework:Data/registry.json`) and loaded statically by `Core/TBD_Registry.c` at mission initialization.
