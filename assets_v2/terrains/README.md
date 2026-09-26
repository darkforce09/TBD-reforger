# Built-in terrains

The terrain datasets that ship in the repository, one folder per island, and the registry that
lists them. The [API](/documentation_v2/glossary/a_to_f.md#api) serves this folder at `/map-assets`, the
map engine streams a terrain from it into the
[Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator), and the developer tools and
gates read it from disk.

## Contents

```text
assets_v2/terrains/
├── arland/                Arland, registered but not exported: its stub manifest only
├── everon/                Everon, fully exported: every map asset the platform draws for it
└── terrain-registry.json  the registry of every terrain: id, bounds, manifest path, export phases
```

## How it works

The API mounts this folder at `/map-assets` from `MAP_ASSETS_DIR`, whose default
`../../../assets_v2/terrains` resolves here from the API's working directory
`apps/website/api_v2/` (`apps/website/api_v2/src/core/http_router.rs`). The mount sits below the
rate limiter, so streaming a terrain spends no request tokens, and in development the app's Trunk
server proxies `/map-assets` to the API (`apps/website/frontend/Trunk.toml`).

```text
tbd-export plugins in Workbench ──raw exports──▶ developer tools ──write──▶ <terrain>/
                                                                           │
browser: mission's terrain id ──▶ /map-assets/<terrain>/manifest.json ◀────┘ API mount
                                        │
                                        └──▶ every other file, by the paths the manifest names
```

The browser reaches a terrain by the id its [mission](/documentation_v2/glossary/g_to_m.md#mission)
names: the Mission Creator boots `/map-assets/<terrain>/manifest.json` directly
(`apps/website/map-engine/src/streaming/host/bootstrap.rs`) and never reads the registry. Inside a
terrain the manifest names every asset, so a dataset may ship a subset and the map engine loads
what the manifest lists; the few names the readers fix themselves are listed in the
[Everon README](/assets_v2/terrains/everon/README.md). Coordinates are world metres from `0, 0`,
and heights come from the game engine's own surface probe (`GetSurfaceY`), which stays the
authority for spawn heights.

The registry is for tools. Its entries:

| Terrain | World bounds (m) | Status | Phases shipped |
|---|---|---|---|
| `everon` | `0, 0, 12800, 12800` | `active` | `P1_buildings` to `P5_props` |
| `arland` | `0, 0, 4096, 4096` | `queued` | none |

`world phase-gate`, the first step of `cargo xtask map export-terrain`, refuses a phase above the
entry's `importPhaseMax`; `world validate-exports` walks every entry and skips a terrain whose
manifest is missing or has no `objects` block; `world verify-phase` reads the entry's bounds. No
check branches on `status`, which records how far a terrain's export has come.

## Format

- Encoding: `terrain-registry.json` is plain UTF-8 JSON, a plain git blob, `schemaVersion`
  `1.0.0` and `terrains[]`, each with `terrainId` (the folder name), `displayName`,
  `worldBoundsM` `[minX, minY, maxX, maxY]`, `manifestPath` relative to this folder, the Workbench
  world `workbenchWorld`, `exportProfile`, `status` (`active` or `queued`), `importPhaseMax` and
  `importPhaseShipped`. The terrain folders hold JSON plus binaries that Git LFS stores
  (`.gitattributes`: the `assets_v2/terrains/**` rules for `.png`, `.r16`, `.tbd-sat`, `.bin`,
  `.rkyv`, `.dem`, `.tbd-bath` and `prefabs/blas/*.bvh`), except the density tiles, which the last
  rule keeps as plain blobs. Each terrain's `tiles/` pyramids are local build output, gitignored
  (`.gitignore`: `assets_v2/terrains/**/tiles/`).
- Schema: `contracts_v2/definitions/terrain-registry.schema.json` for the registry and
  `contracts_v2/definitions/terrain-manifest.schema.json` for each terrain's `manifest.json`.
- Adding a file: a new terrain is a folder named by its id with a `manifest.json`, and a registry
  entry whose `manifestPath` names that manifest. `cargo xtask schema validate` checks the
  registry against its schema, and `cargo xtask schema terrain-manifest --terrain <id>` checks the
  manifest against its schema and the terrain contract compiled into the gate
  (`tools_v2/developer-tools/src/map_verification/terrain_manifest.rs`), which knows `everon` and
  `arland` only and exits 2 for any other id.

## Producers and consumers

- Producers: the registry is written by hand, and a registry bump follows a phase that
  `world verify-phase` passes. The terrain folders are written by the developer tools in
  `tools_v2/developer-tools/`, which resolve these paths through
  `tools_v2/developer-tools/src/repository_layout.rs`: the world export pipeline, the map raster
  pipeline and the blueprint compiler, from the raw exports of the `tbd-export` addon's
  [Workbench](/documentation_v2/glossary/n_to_z.md#workbench) plugins.
- Consumers:
  - the API's `/map-assets` mount, and its test
    `apps/website/api_v2/tests/map_assets_rate_limit_exemption.rs`, which fetches
    `terrain-registry.json` to prove the mount is outside the rate limiter;
  - the map engine in the browser, per terrain, over `/map-assets/<terrain>/…`;
  - the world export steps above (`tools_v2/developer-tools/src/world_export_pipeline/`) and
    `cargo xtask schema validate`, which read the registry;
  - `cargo xtask deploy website`, whose rsync excludes this folder, so each host keeps its own
    copy, and whose asset preflight treats a host's `assets_v2/terrains/terrain-registry.json` as
    the sign that its copy is in place
    (`tools_v2/xtask/src/commands/deploy/website/asset_preflight.rs`);
  - the staging compose file `apps/website/docker-compose.staging.yml`, which mounts this folder
    read-only into the API container as `MAP_ASSETS_DIR`, and the API's systemd unit
    `tools_v2/xtask/deploy/systemd/tbd-website-api.service`, which points `MAP_ASSETS_DIR` here.

## Boundaries

- Depends on: the schemas in `contracts_v2/definitions/` and the classification rules in
  `contracts_v2/rules/prefab-classify.json`.
- Used by: the API, the map engine, the developer tools, the xtask schema, verify, `ci` and deploy
  commands, the staging compose file and the CI workflows in `.github/workflows/`, as listed here
  and in each terrain's README.
- Rules:
  - a terrain is reached only through its manifest, and its folder name, the manifest's
    `terrainId` and the registry's `terrainId` are the same string
    (`cargo xtask schema terrain-manifest --terrain <id>` checks the manifest's);
  - the registry follows its schema (`cargo xtask schema validate`) and stays at this path, since
    the deploy preflight and the API test look for it by name
    (`the_asset_under_test_is_under_the_exempt_mount`);
  - an LFS pattern in `.gitattributes` exists before the first file of its kind is committed.

## Related documentation

- [Map assets](/assets_v2/README.md) — how this folder sits beside the glyph set.
- [Uploaded terrain volume specification](/assets_v2/storage_spec/README.md) — a production volume
  for uploaded terrains with the same manifest contract, which no code reads.
- [Local development](/documentation_v2/runbooks/local_development.md) — pulling the LFS map
  assets and serving them to the app.
- [Website deployment](/documentation_v2/runbooks/website_deployment.md) — how a deployed host
  gets its terrain tree.
