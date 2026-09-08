# Server Control (Admin)

## Status

`doc-complete` — live inventory and supported RCON requests (T-270/T-598); terrain consumes the existing joined API field (T-939.4 recovery).

## Summary

- **Route:** `/admin/server`; admin access through `AdminGate`.
- **Live source:** `apps/website/frontend/src/pages/admin/server_control.rs`.
- **Purpose:** select a configured server, inspect its status and modpack, and submit supported host-control requests.

## Behavior

The page loads `GET /servers` as `DataEnvelope<ServerRowDto>` and selects the active server, falling back to the first row. The detail shows the real endpoint, player counts, FPS, uptime, current-match identifier and required modpack. Terrain uses the joined `ServerRowDto.terrain`: Everon/Arland receive display labels, other nonblank terrain names remain intact, and missing or blank values show an em dash.

Restart, mapped quick actions and the console use `POST /admin/servers/{id}/rcon`. Request and response errors remain visible. Success wording reads the returned `accepted`, `delivered`, `state` and `detail`; it does not infer delivery from an HTTP acknowledgement alone. The transport supports restart; unsupported verbs can return 503.

Stop, Launch, Swap Modpack and Global Broadcast retain their explicit disabled/unavailable behavior. The console records actual requests and outcomes; it does not display fabricated historical traffic. No-server, loading and request-error states remain distinct.

## API Dependencies

- `GET /servers` → `{data:[ServerRowDto...]}` including cached status, required modpack and optional terrain.
- `POST /admin/servers/{id}/rcon` → validated action body; successful response includes `action`, `accepted`, `delivered`, `state`, `detail` and `audited`.

## Verification

Check an active server with terrain `everon`, then null/blank terrain: the display shows Everon or an em dash respectively. Switching servers must retain their own status, address and modpack. Non-admin users cannot access the page. RCON outcome tests cover the actual response fields; the initial visual fixture does not prove a live server restart.

The recovery verification uses deterministic server fixtures and checks the supported T-270/T-598 behavior together with the terrain readout. Operator server actions still require the configured host control agent and real server environment.
