# In-game administration

The powers listed admins hold over a running round, and the two surfaces that reach them: the
`#tbd` chat commands and the admin screen. Every power passes one server-side permission gate and
lands in one audit trail, because under one life a respawn or a forced stage decides the
[event](/documentation_v2/glossary/a_to_f.md#event).

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Session/Admin/
├── TBD_AdminAudit.c                           the bounded audit trail of every admin attempt
├── TBD_AdminClient.c                          client cache of the last snapshot and action result
├── TBD_AdminCommands.c                        modded `SCR_ChatComponent`: the `#tbd` chat commands
├── TBD_AdminData.c                            the snapshot models: player rows, audit rows, payload
├── TBD_AdminService.c                         the admin authority: permission gate and the powers
├── TBD_AdminServiceDeploymentAuthorization.c  respawn and deploy replies while the platform decides
├── TBD_AdminSnapshotService.c                 builds the snapshot on the server; wire format
└── UI/                                        the admin screen over the shared shell
```

## How it works

```text
chat "#tbd ..."  -> SCR_ChatComponent.OnNewMessage (server) -> TBD_AdminService.IsAdmin
                    -> TBD_AdminCommands.Dispatch -> TBD_AdminService power -> TBD_AdminAudit
F8 / "#tbd menu" -> TBD_AdminClient.Open -> TBD_AdminScreen
                    -> TBD_RequestAdminSnapshot / TBD_RequestAdminAction  --RPC-->  server
                    -> TBD_AdminService.Execute + TBD_AdminSnapshotService.BuildForAdmin
                    <--RPC-- TBD_RpcDo_AdminSnapshot / TBD_RpcDo_AdminActionResult
                    -> TBD_AdminClient.Accept -> the screen redraws
```

`TBD_AdminService.IsAdmin` reads the vanilla admin list (`SCR_PlayerListedAdminManagerComponent`)
on the authority, and the caller is always the player id of the controller the request arrived on,
never an argument. `Execute` runs the menu's actions (`TBD_EAdminAction`: `RESPAWN` a player whose
life is spent, `DEPLOY` a live player with no body, `STAGE_ADVANCE`); `ForceStage`, `Safestart` and
`Identity` serve the chat forms that take arguments. `TBD_AdminServiceDeploymentAuthorization`
rewords a respawn or deploy the platform is still deciding (`AUTHORIZING`) or cannot authorize
now (`UNAUTHORIZED`). Refused attempts by non-admins are counted per player and surface
(`NoteDeniedAccess`).

The chat commands, for listed admins only; everyone else gets "TBD: admin only.":

| Command | Effect |
|---|---|
| `#tbd missions` (also `#tbd`, `#tbd list`) | lists the missions the platform lets this server deploy, numbered from 1 |
| `#tbd mission <n>` | asks the platform to deploy mission n; the server restarts only when the platform runs it |
| `#tbd refresh` | reloads the mission list |
| `#tbd backend <url> [token]` | repoints the backend URL and service token, saves them, reloads the list |
| `#tbd validate` | replays the mission validation findings |
| `#tbd dead` | lists who has spent their life |
| `#tbd respawn <playerId>` | the one-life escape hatch: a fresh body for a spent life |
| `#tbd deploy <playerId>` | puts a live player with no body into the world |
| `#tbd stage [next\|<NAME>]` | forces the stage machine |
| `#tbd safestart [status\|go\|<seconds>]` | reads or ends the warm-up, or sets its length |
| `#tbd identity [status\|override <phrase>\|enforce]` | whether this host can enforce one life; the signed waiver |
| `#tbd audit` | replays the last 12 audit entries |
| `#tbd menu` | opens the admin screen on the caller's client |

The mission commands call `TBD_DeployableMissionList` and `TBD_MissionDeploymentRelay` in
`apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`. Replies go to the admin by
private chat and to the server console.

`TBD_AdminAudit` keeps at most `MAX_ENTRIES` (60) entries across worlds in one process, of which
unauthorized-access refusals may hold at most 12, so an attacker cannot flush real actions out.
Each entry is logged on the `Admin` channel, refusals at WARNING. `TBD_AdminSnapshotService`
builds the payload one admin may see (mission, stage, validation findings, players, the newest 20
audit lines) and serialises it as tab-separated lines of at most 400, every field prefixed with `.`
so no field is ever empty on the wire. A non-admin's payload holds the refusal and nothing else.

## Authority

- Server: `TBD_AdminService`, `TBD_AdminAudit`, `TBD_AdminSnapshotService.BuildForAdmin` and the
  chat commands (`@authority server`); every mutating function refuses off the authority, and the
  chat hook returns on a client before dispatching.
- Client: `TBD_AdminClient` and the screen; they hold the server's last answer, never a permission.
- Owner: the snapshot and action-result replies, received on the requesting admin's client.
- RPCs: none declared here. The admin transport lives on the modded `SCR_PlayerController` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/TBD_MissionBrowser.c`:
  `TBD_RpcAsk_AdminSnapshot` and `TBD_RpcAsk_AdminAction` (Reliable, Server) and
  `TBD_RpcDo_AdminSnapshot`, `TBD_RpcDo_AdminActionResult` and `TBD_RpcDo_OpenAdminMenu`
  (Reliable, Owner).
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_SpawnManager` (lives, bodies, respawn and deploy, the identity waiver) in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_FrameworkManager` and
  `TBD_SafestartManager` in `apps/mod/tbd-framework/Scripts/Game/TBD/Gamemode/`;
  `TBD_MissionLoader` and `TBD_MissionValidator` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; `TBD_IdentityLink` and
  `TBD_BackendConfig` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the MissionSelector
  scripts above; the engine's `SCR_PlayerListedAdminManagerComponent` and `SCR_ChatComponent`.
- Used by: `TBD_MissionBrowser` (the admin RPCs, the F8 key), `TBD_MissionDeploymentRelay`,
  `TBD_FleetPlayerActions`, `TBD_DeploymentAuthorization` and `TBD_SpawnManager`, which write to
  `TBD_AdminAudit`; the `TBD_UIAdmin` preset in
  `apps/mod/tbd-framework/Configs/System/chimeraMenus.conf`; the key bindings in
  `apps/mod/tbd-framework/Configs/System/Actions/TBD_AdminMenu.conf`.
- Rules: one permission gate (`TBD_AdminService.IsAdmin`) and one audit trail for both surfaces;
  the caller id never comes from the wire; every attempt, refused ones included, is audited; the
  snapshot for a non-admin carries nothing but the refusal; lines added stay ASCII and
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [In-game menu specification](/documentation_v2/mod/tbd-framework/UI/in_game_menu/in_game_menu_specification.md)
  — the pause menu and admin screen as built, and the admin suite design target
- [Admin help ticket specification](/documentation_v2/mod/tbd-framework/UI/admin_help_ticket/admin_help_ticket_specification.md)
  — the admin help ticket and tickets module, designed and not built
