**Status:** live

# Glossary terms G to M

The glossary's entries from G to M, in alphabetical order, each in the format of the
[glossary entry template](/documentation/standards/templates/glossary_entry.md). The
[glossary index](/documentation/glossary/README.md) lists every term and says how a document
links one.

### game runtime

The TBD framework mod running inside a dedicated server, seen from the API: with a `mod_runtime`
[machine credential](#machine-credential) it opens and ends runtime sessions, sends heartbeats, reads
rosters and artifacts, authorizes player lives into slots, and runs broadcasts, kicks and loads.

In code: the `/api/v1/game-runtime/` routes; `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.

See: [fleet command](/documentation/glossary/a_to_f.md#fleet-command), [deployment](/documentation/glossary/a_to_f.md#deployment).

### gate

A check command that passes or fails a change. Most exit 0 when every check held, 1 on a violation
and 2 when a check could not run, so a missing input never reads as a pass: the repository
verifications, the headless browser gates of the `gate` binary, the mod compile gate, and the
[factory](/documentation/glossary/a_to_f.md#factory)'s cheap slice gate and full wave gate.

In code: `Verdict` in `tools/foundation/verification_core/src/verdict.rs`; `cargo xtask verify` over the check crates in `tools/checks/`; `tools/developer_tools/src/bin/gate.rs`; `cargo xtask platform wave gate`; `cargo xtask mod compile`.

See: [slice](/documentation/glossary/n_to_z.md#slice), [Testing and CI](/documentation/runbooks/testing_and_ci.md), [Editor gates](/documentation/runbooks/editor_gates.md).

### identity and access

The API domain of sign-in and the caller's own account: Discord OAuth2 login, token refresh, logout,
the [dev login](/documentation/glossary/a_to_f.md#dev-login), `/api/v1/me`, and the Discord-to-Arma identity link handshake.

In code: `crates/api/api_identity_and_access/src/`.

See: [Identity and access domain](/crates/api/api_identity_and_access/src/README.md).

### lane

A draw-order layer of the map: `map_draw_lanes` names 48 lanes (`LaneRole`), basemap first; the
graphics crates sort draws by an opaque `LaneId`. Other lanes are named in full (wave lanes).

In code: `LaneRole` in `crates/map_overlay/map_draw_lanes/src/lane_roles.rs`; `LaneId` in `crates/graphics/render_primitives/src/frame/ids.rs`.

See: [frame packet](/documentation/glossary/a_to_f.md#frame-packet), [Map draw lanes](/crates/map_overlay/map_draw_lanes/README.md).

### load workload

The committed definition of the staging load run, `tools/xtask/staging/load_workload.json`: its
seed, a 60-second ramp and 1,800 measured seconds, 100 virtual clients over five source addresses at
27 requests per second, the per-address ceilings, the account hold time, and the weighted request
mix with each request's class, method, path and body templates and expected statuses. The run's
`workload_sha256` covers it and `load_population.json`, each length-framed.

In code: `workload_plan.rs` in `tools/staging/staging_load_plan/src/`, which refuses unknown fields; the data folder `tools/xtask/staging/`.

See: [synthetic load account](/documentation/glossary/n_to_z.md#synthetic-load-account), [staging harness](/documentation/glossary/n_to_z.md#staging-harness), [Staging load data](/tools/xtask/staging/README.md).

### machine credential

A per-server secret that authenticates one program on a game host: `host_agent` for the
[fleet host agent](/documentation/glossary/a_to_f.md#fleet-host-agent) or `mod_runtime` for the [game runtime](#game-runtime). An
administrator issues one (its secret shows once), lists them without secrets and revokes each alone.

In code: `MachineCredential` in `crates/api/api_server_infrastructure/src/models/machine_credential.rs`; `ExecutorKind` in `crates/contracts/fleet_wire_contract/src/executor_kind.rs`.

See: [Machine credentials evidence](/documentation/apps/api/verification_evidence/machine_credentials.md).

### match telemetry

The API domain that takes in what game servers report, each call authenticated by the server's
`mod_runtime` [machine credential](#machine-credential): runtime-session heartbeats with the live
server status, match registrations, numbered results revisions and batches of detailed combat,
medical and vehicle events. It also serves the read of a match's detailed events.

In code: `crates/api/api_match_telemetry/src/`.

See: [Match telemetry domain](/crates/api/api_match_telemetry/src/README.md).

### mission

The platform document a mission maker authors in the [Mission Creator](#mission-creator), with its
versions, reviews, [artifacts](/documentation/glossary/a_to_f.md#artifact) and deployments; its status is `draft`, `pending_approval`,
`live`, `rejected` or `archived`. It is not the [mission header](#mission-header) a server boots.

In code: `Mission`, `MissionVersion` and `MissionStatus` in `crates/api/api_missions/src/models/mission.rs`; `contracts/definitions/mission.schema.json`; some code spells it [scenario](/documentation/glossary/n_to_z.md#scenario).

See: [missions](#missions), [event](/documentation/glossary/a_to_f.md#event).

### Mission Creator

The CAD editor in which mission makers build a [mission](#mission) on a top-down 2D map, at
`/missions/:id/edit`, for the `mission_maker` [role](/documentation/glossary/n_to_z.md#role) and above; the Arsenal's paper doll is
its only 3D view. Prose never calls it the Scenario Creator; code identifiers say editor.

In code: `apps/frontend/src/workspaces/editor/`; `MissionEditorPage` in its `mission_editor.rs`.

See: [Mission Creator documentation](/documentation/apps/frontend/workspaces/editor/README.md).

### mission deployment

A request that runs an approved [artifact](/documentation/glossary/a_to_f.md#artifact) on one game server: the API issues a
[fleet command](/documentation/glossary/a_to_f.md#fleet-command) (`load_mission` on the same terrain, `restart_with_mission` for
another), follows it to confirmation, and cancels it while no executor has claimed it.

In code: `crates/api/api_missions/src/handlers/mission_deployments.rs`; `contracts/definitions/mission-deployment.schema.json`.

See: [deployment](/documentation/glossary/a_to_f.md#deployment), [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario).

### mission header

Enfusion's world plus game-mode configuration that a dedicated server boots: an
`SCR_MissionHeader` config naming the world, the game mode, and the name and description players
see. It is not a [mission](#mission), which the mod's mission loader loads into the running game.

In code: `apps/mod/tbd-framework/Missions/` (`TBD_Dev_POC.conf`); `game.scenarioId` in `tools/xtask/dedicated_server_profiles/tbd-dev-server.config.json`; the code says scenario.

See: [fleet scenario](/documentation/glossary/a_to_f.md#fleet-scenario), [Game server staging](/documentation/runbooks/game_server_staging/README.md).

### missions

The API domain that owns [missions](#mission): the library, the versions the Mission Creator saves,
submission and [approvals](/documentation/glossary/a_to_f.md#approvals), artifacts, mission deployments, the [armory](/documentation/glossary/a_to_f.md#armory), the
faction library, the item [registry](/documentation/glossary/n_to_z.md#registry) and the game-runtime routes that serve artifacts.

In code: `crates/api/api_missions/src/`.

See: [Missions domain](/crates/api/api_missions/src/README.md).

### mod

The Arma Reforger modification the repository ships, in three addons (Enfusion packages, each with
an `addon.gproj`): `tbd-framework`, the game mod that runs TBD sessions; `tbd-export`, the Workbench
export tooling; `tbd-emcp`, the Workbench bridge handlers the Enfusion MCP tools call.

In code: `apps/mod/tbd-framework/`, `apps/mod/tbd-export/`, `apps/mod/tbd-emcp/`.

See: [EnfScript](/documentation/glossary/a_to_f.md#enfscript), [Mod suite](/apps/mod/README.md).

### modpack

A named, versioned set of Arma Reforger mods that players download to join, with its total size,
Workshop link and mod rows (Workshop ID, mod GUID, optional version pin, key dependency); one
modpack is current. The item [registry](/documentation/glossary/n_to_z.md#registry) is kept per
modpack.

In code: `Modpack` and `ModpackMod` in `crates/api/api_community_content/src/models/modpack.rs`; the `/api/v1/modpacks` routes in `crates/api/api_community_content/src/routes.rs`; `ModpacksPage` at `/modpacks` in `apps/frontend/src/pages/doctrine_and_info/modpacks/`.

See: [community content](/documentation/glossary/a_to_f.md#community-content), [Modpacks page](/apps/frontend/src/pages/doctrine_and_info/modpacks/README.md).
