# Backend connection

The server's connection to the platform backend: the settings it reads from the profile, the shared
transport of every machine-credential route, which hands each answer from the engine's REST callback
thread to the main thread, the classification of its answers, and the text helpers every backend
payload and backend log line uses.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/
├── TBD_BackendConfig.c            reads the backend URL and the machine credential from the profile
├── TBD_BackendText.c              JSON string escaping, UUID check, RFC 3339 UTC time, two-digit padding, backend description
├── TBD_GameRuntimeAnswer.c        classifies an answer: success, 409 refusal, transient, permanent
├── TBD_GameRuntimeHttp.c          the machine-credential transport: one answer per call on the main thread, backoff
└── TBD_GameRuntimeRestCallback.c  one request's engine callback, recording its answer on the engine's callback thread
```

## How it works

### One authentication tier

`TBD_BackendConfig` reads `$profile:TBD_BackendConfig.json` (copied from
`apps/mod/tbd-framework/Data/backend.example.json`), whose keys are the fields of
`TBD_BackendConfigFile`:

| Key | Default | Sent as | Used for |
|---|---|---|---|
| `backendUrl` | none; no platform connection without it | the base of every URL | every call |
| `machineCredential` | none | `Authorization: Bearer tbdm_...` | every `/api/v1/game-runtime/`, `/api/v1/fleet-executor/` and `/api/v1/ingest/` route |

The `machineCredential` is this server's `mod_runtime`
[machine credential](/documentation/glossary/g_to_m.md#machine-credential), issued by an administrator.
A value that does not start with `tbdm_`, such as the example's placeholder, counts as unset: no
deployment is read, no runtime session starts, no roster loads, no fleet command is claimed, no
link is confirmed and the telemetry queue holds its entries.
`Reload` re-reads the file while the server runs and keeps the settings in force when the file does
not read or parse; the loops that wait on the platform (`TBD_DeployedMission` and
`TBD_RosterLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`) call it,
so a credential pasted in later is picked up without a restart. `SetBackend`, behind the admin
chat command `#tbd backend <url>`, repoints the URL, keeps the credential and saves the file. The
credential is never logged. The mission and its [event](/documentation/glossary/a_to_f.md#event) are not configured here:
they come with the deployment the platform holds for this server.

### The machine-credential transport

`TBD_GameRuntimeHttp.Post` and `Get` take the full path (`ROUTE_PREFIX` names the game-runtime
prefix; the fleet executor, the link confirmation and the telemetry delivery pass their own
`/api/v1/fleet-executor/` and `/api/v1/ingest/` paths), open a `RestContext` with the bearer credential, the JSON
content type and a 15 s timeout (`REQUEST_TIMEOUT_S`), and deliver exactly one
`TBD_GameRuntimeAnswer` to the sender's `TBD_GameRuntimeCall` subclass; a request the engine never
reports is answered transient by a 25 s watchdog (`WATCHDOG_MS`). `TBD_GameRuntimeAnswer` classifies
by HTTP status and by the `details.code` of a 409, never by message text: `SUCCESS` (200, 201 or
202), `NO_CONTENT` (204, which the engine reports through the success handler as the raw status
`HttpCode` names no member for; only the fleet command claim answers it), `REFUSED` (a 409 fence refusal, with its parsed details), `TRANSIENT` (no answer, a timeout, 408 or
a server error) or `PERMANENT` (any other client error), and keeps the `details.code` of any error,
such as `NO_DEPLOYMENT`, for the caller. `BackoffMs` gives the exponential retry delay every loop
uses.

### Threads

The engine runs a `RestCallback`'s success and error handlers on its REST callback thread, not on
the main thread. An entity spawned from there loses its signals ("Trying to register a signal ...
outside of the main thread. Request ignored.") and, while the main thread is still creating the
world's entities, deadlocks the server until the engine's watchdog force-crashes it
("Application hangs (force crash) 300 s"). So the transport keeps every answer handler off that
thread:

```text
REST callback thread                          main thread
TBD_GameRuntimeRestCallback.CaptureSuccess/   TBD_GameRuntimeHttp.DeliverReportedAnswers
  CaptureError: status, transport result,       (repeats every frame while a call is in flight)
  body into the callback's own fields,           -> oldest reported call out of flight
  m_bReported last                               -> TBD_GameRuntimeAnswer.Reported
                                                 -> call.OnAnswered(answer)
                                              TBD_GameRuntimeHttp.OnCallWatchdog (25 s)
                                                 -> the reported answer, else TRANSIENT
```

- The two capture handlers are the only code that runs on the REST callback thread. They write
  the fields of the callback they are handed and nothing else: no container, no call queue, no
  log, no entity, no player.
- Every `OnAnswered` runs on the main thread, from the delivery pass or the watchdog, and never
  inside `Post` or `Get`, so every sender may spawn, kick, broadcast, restart the scenario, chat or
  arm the call queue from its answer handler.
- `Post`, `Get` and every static of the transport run on the main thread; `Track` arms the
  delivery pass there, and the pass leaves the call queue once no call is in flight.
- A call the engine never reports is answered `TRANSIENT` by its watchdog, whose detail names the
  engine's own transport result (`no answer within 25000 ms (engine rest=...)`), and its callback
  is retired: kept (at most 16) until the engine reports it, because the script owns a callback
  while its request is in flight. The delivery pass and the watchdogs live on the game's call
  queue, which outlives a world, so a call sent before a scenario restart is still answered once.

### Backend text

`JsonEscape` copies its input through `string.Format` (`string.Replace` mutates in place),
escapes backslashes and then quotes, and folds newline, carriage return and tab to spaces so a
payload stays one log line. `UtcNowIso8601` reads the engine's UTC date and clock and writes
`2026-07-25T16:31:28Z`, the shape the backend's `DateTime<Utc>` parses; `Pad2` zero-pads to
two digits. `IsUuid` recognises a canonical UUID, the check optional `format: uuid` wire fields
pass before they are sent. `DescribeBackend(noneText)` prints the configured backend for a log line,
never the credential: `noneText` when no URL is set, else `TBD_GameRuntimeHttp.DescribeBackend`,
the URL with the reason a credential is unusable.

## Authority

- Server: everything; the machine credential lives only in the authority's profile.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's `RestApi`, `RestContext`, `RestCallback`, `ScriptCallQueue` (the
  delivery pass and the watchdogs), `JsonLoadContext`, `JsonSaveContext`, `FileIO` and `System` UTC
  clock.
- Used by: every sender of the game-runtime routes: `TBD_DeployedMission`,
  `TBD_MissionArtifactVerification` and `TBD_RosterLoader` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; `TBD_DeploymentAuthorization`,
  `TBD_DeploymentRequest`, `TBD_DeploymentRequestQueue` and `TBD_DeploymentEndQueue` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_DeployableMissionList` and
  `TBD_MissionDeploymentRelay` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`;
  the runtime session, fleet commands, identity link confirmation and match telemetry delivery in
  `apps/mod/tbd-framework/Scripts/Game/TBD/API/`. `TBD_BackendText` by the identity link, the
  results report, the match telemetry reports and the runtime session there.
  `TBD_BackendConfig.SetBackend` by `TBD_AdminCommands`.
- Rules: no secret is ever printed; answers are read by status and `details.code`, never by message
  text; nothing but the capture handlers runs on the engine's REST callback thread, and every answer
  handler runs on the main thread; lines added stay ASCII; `cargo xtask mod compile` checks that
  the scripts compile.

## Related documentation

- [Platform bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — the backend clients that use this connection
- [Server infrastructure domain](/apps/api/src/server_infrastructure/README.md) — machine
  credentials and the routes this transport reaches
