# missions/

Compiled mission JSON served by `GET /api/missions/{id}/compiled` to dedicated
game servers (server-token authed). The file name is `{missionId}.json`.

For Phase 0.1 these are static copies of the golden missions in
[`../../tbd-schema/golden-missions/`](../../tbd-schema/golden-missions/). Later
phases serve published, content-hashed missions from object storage / the
database instead of this directory.

| File | Source golden mission |
|------|-----------------------|
| `msn_8f3a2c.json` | `bridgehead-at-levie.json` |
| `msn_2d91be.json` | `last-stand-at-montfort.json` |
