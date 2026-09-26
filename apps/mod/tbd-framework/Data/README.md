# Framework data files

The JSON data the framework [mod](/documentation_v2/glossary/g_to_m.md#mod) ships: the alias spawn
registry that turns the aliases a [mission](/documentation_v2/glossary/g_to_m.md#mission) names into
prefabs, and the template of the backend config a dedicated server reads from its profile.

## Contents

```text
apps/mod/tbd-framework/Data/
├── backend.example.json  the template of the profile's `TBD_BackendConfig.json`
└── registry.json         the alias spawn registry: 354 aliases, each mapped to a vanilla prefab
```

## How it works

`registry.json` travels inside the addon. `TBD_Registry.Load` reads it as
`$TBD_Framework:Data/registry.json` and falls back to `$profile:TBD_Registry.json` only when the
addon copy is missing; it maps each entry's `alias` to its `guid` and logs
`[TBD] Registry loaded (<n> aliases).`. `TBD_Registry.Resolve` then answers the spawn and preview
code for every alias a mission names: slot kits, entities, vehicles, objective targets and trigger
effects. The entries are 289 `prop:`, 45 `comp:`, 15 `kit:`, 4 `preset:` and 1 `veh:`. An alias it
lacks logs `[TBD] Unknown registry alias:` and resolves to nothing.

`backend.example.json` is never read in place: `cargo xtask setup server-profile` copies it to
`<profile>/profile/TBD_BackendConfig.json`, where `TBD_BackendConfig` reads it as
`$profile:TBD_BackendConfig.json`.

| Key | Example value | Meaning |
|---|---|---|
| `backendUrl` | `http://127.0.0.1:8080` | the API the server calls |
| `serverToken` | `replace-with-SERVICE_TOKEN-value` | the `X-Service-Token` of the link confirmation and match result ingest routes |
| `machineCredential` | a placeholder | this server's `mod_runtime` [machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential); anything not starting `tbdm_` counts as not configured |

The mission and its [event](/documentation_v2/glossary/a_to_f.md#event) are not configured here: the
server runs the mission deployed to it on the platform.

## Format

- File type: UTF-8 JSON. `registry.json` follows
  [registry.schema.json](/contracts_v2/definitions/registry.schema.json): `registryVersion`,
  `generatedAt`, `modset` (the vanilla addon `58D0FB3206B6F859`) and `entries`, each with `alias`
  (`<kind>:<name>`), `guid` (a full `{GUID}Prefabs/….et` resource name), `displayName` and, on two
  entries, a `footprint` the web map draws. `backend.example.json` holds the three string keys of
  `TBD_BackendConfigStruct`.
- Naming: the mod reads these two file names by path, so they do not change.
- Adding an alias: add an entry to `registry.json` by hand; a `kit:` or `preset:` entry also goes
  into `contracts_v2/rules/kit-aliases.json`, and a `prop:` or `comp:` entry must match the
  Mission Creator's Objects palette. Run `cargo xtask schema validate` and
  `cargo xtask verify object-registry-aliases` afterwards.

## Referenced by

- `TBD_Registry` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/TBD_Registry.c` loads
  `registry.json` by path; `TBD_BackendConfig` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/TBD_BackendConfig.c` reads the profile copy of
  `backend.example.json`.
- `cargo xtask setup server-profile` copies both files into the profile, `registry.json` as
  `TBD_Registry.json` (`tools_v2/xtask/src/commands/setup/server_profile.rs`);
  `cargo xtask mod test-mission` and `cargo xtask mod world-boot` copy `registry.json` the same
  way.
- `cargo xtask schema validate` reads `registry.json` to check every golden mission's kit aliases
  and that `contracts_v2/rules/kit-aliases.json` mirrors its kit rows
  (`tools_v2/xtask/src/verifications/schemas/checks/mission_validation.rs`).
- `cargo xtask verify object-registry-aliases` checks the Objects palette's `prop:` and `comp:`
  aliases against it (`tools_v2/xtask/src/verifications/registry/object_registry_aliases.rs`), and
  `cargo xtask verify no-crf-leak` scans it for upstream-only prefab GUIDs.
- The Mission Creator embeds `registry.json` at compile time to know which object aliases the mod
  can spawn (`apps/website/frontend/src/v2/apps/editor/arsenal/asset_catalog.rs`).

## Boundaries

- Depends on: the vanilla prefabs the registry's resource names point to.
- Used by: the framework scripts, the xtask commands and gates, and the Mission Creator, as listed
  above.
- Rules: an alias a mission or the Objects palette uses stays in `registry.json` with the same
  prefab; `contracts_v2/rules/kit-aliases.json` mirrors its kit rows (`cargo xtask schema
  validate`); `backend.example.json` holds placeholders only, never a real token or credential.

## Related documentation

- [Mod design](/documentation_v2/mod/tbd-framework/mod_design.md) — the mission-as-data contract
  the registry serves.
