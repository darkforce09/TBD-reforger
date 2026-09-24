# Server Intel

## Status

`doc-complete` — live cached/status telemetry, with truthful intelligence availability (T-939.4 recovery).

## Summary

- **Route:** `/server-intel`; the page uses `AuthGate`.
- **Live source:** `apps/website/frontend/src/pages/public/server_intel.rs`.
- **Purpose:** show server health, environment, terrain, address and required modpack without inventing tactical reports.

## Behavior

The page selects the first active row from `GET /servers`, or the first row when none is active. An empty list shows “No servers configured.” Loading and fetch failures have separate states. There is no static guest telemetry path or environment-variable server-name override.

Cached status supplies the initial population, FPS, uptime, game time and weather. Typed `ServerStatusDto` SSE frames replace that cached status; the stream is aborted on route departure. Terrain comes from the optional joined server field, and the modpack card uses the server's required-modpack data. Copy copies the real IP and port; Launch retains its honest client-required behavior.

Recent Intelligence displays “Recent intelligence is unavailable.” No operational event feed supplies this section, so it emits no invented event timestamps, hostile-movement reports or maintenance claims. Real server/status/environment/modpack values remain visible.

## API Dependencies

- `GET /servers` → `{data:[...]}` with cached status, terrain and required modpack.
- `GET /servers/:id/status/stream` → authenticated typed status SSE updates through the shared stream client.

## Verification

With the seeded server fixture, verify 47/64 players, 58.7 FPS, 05:30:42 uptime, 06:42/overcast, terrain and modpack. The initial visual capture proves cached rendering; live SSE requires a separate controlled stream check. An empty server list must still show its empty state. Confirm both old fabricated intelligence messages and their fixed timestamps are absent.

The default-server selection remains unchanged; this recovery does not add a server picker or a new intelligence feed. The existing T-088 multi-server-picker deferral remains in place.
