**Status:** live

# Glossary terms G to M

The glossary's entries from G to M, in alphabetical order, each in the format of the
[glossary entry template](/documentation_v2/standards/templates/glossary_entry.md). The
[glossary index](/documentation_v2/glossary/README.md) lists every term and says how a document
links one.

### game runtime

The TBD framework mod running inside a dedicated server, seen from the API: with a `mod_runtime`
[machine credential](#machine-credential) it opens and ends runtime sessions, sends heartbeats, reads
rosters and artifacts, authorizes player lives into slots, and runs broadcasts, kicks and loads.

In code: the `/api/v1/game-runtime/` routes; `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.

See: [fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command), [deployment](/documentation_v2/glossary/a_to_f.md#deployment).

### gate

A check command that passes or fails a change. Most exit 0 when every check held, 1 on a violation
and 2 when a check could not run, so a missing input never reads as a pass: the repository
verifications, the headless browser gates of the `gate` binary, the mod compile gate, and the
[factory](/documentation_v2/glossary/a_to_f.md#factory)'s cheap slice gate and full wave gate.

In code: `Verdict` in `tools_v2/verification-core/src/verdict.rs`; `cargo xtask verify` over `tools_v2/xtask/src/verifications/`; `tools_v2/developer-tools/src/bin/gate.rs`; `cargo xtask platform wave gate`; `cargo xtask mod compile`.

See: [slice](/documentation_v2/glossary/n_to_z.md#slice), [Testing and CI](/documentation_v2/runbooks/testing_and_ci.md), [Editor gates](/documentation_v2/runbooks/editor_gates.md).

### identity and access

The API domain of sign-in and the caller's own account: Discord OAuth2 login, token refresh, logout,
the [dev login](/documentation_v2/glossary/a_to_f.md#dev-login), `/api/v1/me`, and the Discord-to-Arma identity link handshake.

In code: `apps/website/api_v2/src/identity_and_access/`.

See: [Identity and access domain](/apps/website/api_v2/src/identity_and_access/README.md).

### lane

A draw-order layer of the map: the map engine names 48 lanes (`LaneRole`), basemap first; the
graphics engine sorts draws by an opaque `LaneId`. Other lanes are named in full (wave lanes).

In code: `LaneRole` in `apps/website/map-engine/src/overlay/lanes.rs`; `LaneId` in `apps/website/graphics-engine/src/frame/ids.rs`.

See: [frame packet](/documentation_v2/glossary/a_to_f.md#frame-packet), [Map overlay](/apps/website/map-engine/src/overlay/README.md).

### machine credential

A per-server secret that authenticates one program on a game host: `host_agent` for the
[fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) or `mod_runtime` for the [game runtime](#game-runtime). An
administrator issues one (its secret shows once), lists them without secrets and revokes each alone.

In code: `MachineCredential` and `ExecutorKind` in `apps/website/api_v2/src/server_infrastructure/models/machine_credential.rs`.

See: [Machine credentials evidence](/documentation_v2/website/api_v2/verification_evidence/machine_credentials.md).

### match telemetry

The API domain that takes in what game servers report: runtime-session heartbeats with the live
server status, and finished match results behind the service token.

In code: `apps/website/api_v2/src/match_telemetry/`.

See: [Match telemetry domain](/apps/website/api_v2/src/match_telemetry/README.md).

### mission

The platform document a mission maker authors in the [Mission Creator](#mission-creator), with its
versions, reviews, [artifacts](/documentation_v2/glossary/a_to_f.md#artifact) and deployments; its status is `draft`, `pending_approval`,
`live`, `rejected` or `archived`. It is not the [mission header](#mission-header) a server boots.

In code: `Mission`, `MissionVersion` and `MissionStatus` in `apps/website/api_v2/src/missions/models/mission.rs`; `contracts_v2/definitions/mission.schema.json`; some code spells it [scenario](/documentation_v2/glossary/n_to_z.md#scenario).

See: [missions](#missions), [event](/documentation_v2/glossary/a_to_f.md#event).

### Mission Creator

The CAD editor in which mission makers build a [mission](#mission) on a top-down 2D map, at
`/missions/:id/edit`, for the `mission_maker` [role](/documentation_v2/glossary/n_to_z.md#role) and above; the Arsenal's paper doll is
its only 3D view. Prose never calls it the Scenario Creator; code identifiers say editor.

In code: `apps/website/frontend/src/v2/apps/editor/`; `MissionEditorPage` in its `mission_editor.rs`.

See: [Mission Creator documentation](/documentation_v2/website/frontend/apps/editor/README.md).

### mission deployment

A request that runs an approved [artifact](/documentation_v2/glossary/a_to_f.md#artifact) on one game server: the API issues a
[fleet command](/documentation_v2/glossary/a_to_f.md#fleet-command) (`load_mission` on the same terrain, `restart_with_mission` for
another), follows it to confirmation, and cancels it while no executor has claimed it.

In code: `apps/website/api_v2/src/missions/handlers/mission_deployments.rs`; `contracts_v2/definitions/mission-deployment.schema.json`.

See: [deployment](/documentation_v2/glossary/a_to_f.md#deployment), [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario).

### mission header

Enfusion's world plus game-mode configuration that a dedicated server boots: an
`SCR_MissionHeader` config naming the world, the game mode, and the name and description players
see. It is not a [mission](#mission), which the mod's mission loader loads into the running game.

In code: `apps/mod/tbd-framework/Missions/` (`TBD_Dev_POC.conf`); `game.scenarioId` in `tools_v2/xtask/dedicated_server_profiles/tbd-dev-server.config.json`; the code says scenario.

See: [fleet scenario](/documentation_v2/glossary/a_to_f.md#fleet-scenario), [Game server staging](/documentation_v2/runbooks/game_server_staging/README.md).

### missions

The API domain that owns [missions](#mission): the library, the versions the Mission Creator saves,
submission and [approvals](/documentation_v2/glossary/a_to_f.md#approvals), artifacts, mission deployments, the [armory](/documentation_v2/glossary/a_to_f.md#armory), the
faction library, the item [registry](/documentation_v2/glossary/n_to_z.md#registry) and the game-runtime routes that serve artifacts.

In code: `apps/website/api_v2/src/missions/`.

See: [Missions domain](/apps/website/api_v2/src/missions/README.md).

### mod

The Arma Reforger modification the repository ships, in three addons (Enfusion packages, each with
an `addon.gproj`): `tbd-framework`, the game mod that runs TBD sessions; `tbd-export`, the Workbench
export tooling; `tbd-emcp`, the Workbench bridge handlers the Enfusion MCP tools call.

In code: `apps/mod/tbd-framework/`, `apps/mod/tbd-export/`, `apps/mod/tbd-emcp/`.

See: [EnfScript](/documentation_v2/glossary/a_to_f.md#enfscript), [Mod suite](/apps/mod/README.md).

### modpack

A named, versioned set of Arma Reforger mods that players download to join, with its total size,
Workshop link and mod rows (Workshop ID, mod GUID, optional version pin, key dependency); one
modpack is current. The item [registry](/documentation_v2/glossary/n_to_z.md#registry) is kept per
modpack.

In code: `Modpack` and `ModpackMod` in `apps/website/api_v2/src/community_content/models/modpack.rs`; the `/api/v1/modpacks` routes in `apps/website/api_v2/src/community_content/routes.rs`; `ModpacksPage` at `/modpacks` in `apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/`.

See: [community content](/documentation_v2/glossary/a_to_f.md#community-content), [Modpacks page](/apps/website/frontend/src/v2/pages/doctrine_and_info/modpacks/README.md).
