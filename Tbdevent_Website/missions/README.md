# missions/

Compiled mission JSON served by `GET /api/missions/{id}/compiled` to dedicated
game servers (server-token authed). The file name is `{missionId}.json`.

## Resolution order (handler)

1. Read from this directory (`MISSIONS_DIR`) if present
2. Fall back to published mission in Postgres (when uploaded via admin API)
3. 404 if neither exists

For local dev, these are static copies of the golden missions in
[`../../tbd-schema/golden-missions/`](../../tbd-schema/golden-missions/).

After a successful REST fetch, the Enfusion loader also caches to
`$profile:missions/{missionId}.json` on the dedicated server.

| File | Source golden mission |
|------|-----------------------|
| `msn_8f3a2c.json` | `bridgehead-at-levie.json` — **18 slots**, schema **1.1** |
| `msn_2d91be.json` | `last-stand-at-montfort.json` |

## Dev token

Must match `GAME_SERVER_TOKENS` in `.env` and `TBD_BackendConfig.json` on the
game server profile. Default dev value: `dev-server-token-change-in-prod`.
