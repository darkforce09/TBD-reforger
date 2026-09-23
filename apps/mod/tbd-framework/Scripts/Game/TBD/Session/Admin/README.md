# Session/Admin

Referee administration panel, match overrides, player moderation, and audit trails.

### Roles & Responsibilities
- `TBD_AdminData.c`: DTO structs for referee actions, match metrics, and player management entries.
- `TBD_AdminService.c`: Authority service verifying player admin credentials before executing commands.
- `TBD_AdminServiceDeploymentAuthorization.c`: `modded class TBD_AdminService` - what respawn and deploy tell the admin while the TBD platform decides the target's seat (`AUTHORIZING`) or cannot authorize it now (`UNAUTHORIZED`).
- `TBD_AdminCommands.c`: The `#tbd` chat commands. `#tbd missions` lists the missions the platform lets this server deploy (numbered from 1, platform order); `#tbd mission <n>` asks the platform to deploy mission n (`Session/MissionSelector/TBD_MissionDeploymentRelay.c`) - nothing restarts until the platform's deployment command runs; `#tbd refresh` reloads the list; `#tbd backend <url> [token]` repoints the backend. Also stage override, player respawn and deploy, audit, menu.
- `TBD_AdminAudit.c`: Persistent audit logging recording timestamped referee actions and operator IDs.
- `TBD_AdminSnapshotService.c`: Gathers match diagnostics and validation metrics for the admin interface.
- `TBD_AdminClient.c`: Client RPC caller sending command requests to the server.
- `UI/TBD_AdminScreen.c`: Dashboard UI displaying server metrics, stage controls, and player moderation tools.

### Call Flow & Contracts
Admin triggers action in `UI/TBD_AdminScreen` -> `TBD_AdminClient` sends targeted RPC -> `TBD_AdminService` authenticates player -> executes command via `TBD_AdminCommands` -> logs entry to `TBD_AdminAudit`.
