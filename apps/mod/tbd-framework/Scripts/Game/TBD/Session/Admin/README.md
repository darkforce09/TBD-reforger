# Session/Admin

Referee administration panel, match overrides, player moderation, and audit trails.

### Roles & Responsibilities
- `TBD_AdminData.c`: DTO structs for referee actions, match metrics, and player management entries.
- `TBD_AdminService.c`: Authority service verifying player admin credentials before executing commands.
- `TBD_AdminCommands.c`: Implementation of administrative actions (stage override, player respawn, kick, pause).
- `TBD_AdminAudit.c`: Persistent audit logging recording timestamped referee actions and operator IDs.
- `TBD_AdminSnapshotService.c`: Gathers match diagnostics and validation metrics for the admin interface.
- `TBD_AdminClient.c`: Client RPC caller sending command requests to the server.
- `UI/TBD_AdminScreen.c`: Dashboard UI displaying server metrics, stage controls, and player moderation tools.

### Call Flow & Contracts
Admin triggers action in `UI/TBD_AdminScreen` -> `TBD_AdminClient` sends targeted RPC -> `TBD_AdminService` authenticates player -> executes command via `TBD_AdminCommands` -> logs entry to `TBD_AdminAudit`.
