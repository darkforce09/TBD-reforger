# Phase 0.1 — REST Spike (main team)

> **Status:** GREEN (HTTP contract + Enfusion loader verified on dedicated server, 2026-06-13).

## Goal

Prove the closed loop the whole architecture depends on: a game server can
**fetch a mission as JSON over HTTP** with a bearer token and **POST results +
telemetry** back. File-based loading is the designed fallback if this path proves
unreliable.

## What was built

In [`Tbdevent_Website`](../../Tbdevent_Website):

- Server-token middleware — [`internal/middleware/servertoken.go`](../../Tbdevent_Website/internal/middleware/servertoken.go)
- Game-server handler — [`internal/handlers/gameserver.go`](../../Tbdevent_Website/internal/handlers/gameserver.go)
  - `GET /api/missions/{id}/compiled` (serves compiled mission JSON from `MISSIONS_DIR`)
  - `POST /api/results` (log-only)
  - `POST /api/telemetry` (log-only)
- Wired into the production router — [`internal/server/server.go`](../../Tbdevent_Website/internal/server/server.go)
- Spike harness (no DB) — [`cmd/restspike`](../../Tbdevent_Website/cmd/restspike)
- Spike client emulating the dedicated server — [`scripts/rest-spike.sh`](../../Tbdevent_Website/scripts/rest-spike.sh)
- Handler tests — [`internal/handlers/gameserver_test.go`](../../Tbdevent_Website/internal/handlers/gameserver_test.go)

## How to run

```bash
cd Tbdevent_Website
export GOROOT="$PWD/.tools/go" PATH="$PWD/.tools/go/bin:$PATH"

# automated tests (no network, no DB)
go test ./internal/handlers/

# live loop: start the harness, then run the client
GAME_SERVER_TOKENS=spike-token MISSIONS_DIR="$PWD/missions" PORT=8099 go run ./cmd/restspike &
BASE_URL=http://127.0.0.1:8099 GAME_SERVER_TOKEN=spike-token MISSION_ID=msn_8f3a2c bash scripts/rest-spike.sh
```

## Result (2026-06-13)

```
==> GET mission msn_8f3a2c
    HTTP 200
    mission "name": "Bridgehead at Levie"
==> POST results
    HTTP 202
==> POST telemetry
    HTTP 202
==> unauthenticated request should be rejected
    HTTP 401
PASS: REST loop (GET mission, POST results, POST telemetry, auth gate) verified.
```

Server access log confirmed a 4361-byte mission body served in well under 1 ms and
the auth gate returning 401 without a valid token.

## Go / no-go

- **HTTP contract: GO.** The backend serves missions and accepts results/telemetry
  under bearer auth, reusing the exact production handlers and middleware.
- **Enfusion loader: GO.** `TBD_MissionLoader.c` performs the same `GET` from a live
  dedicated server (`bash scripts/run-dev-server.sh`), caches to `$profile:missions/`,
  with file fallback if the API is down. Verified 2026-06-13.
