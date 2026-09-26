# Backend connection

The server's connection to the platform backend: the settings it reads from the profile, the shared
transport of every machine-credential route and the classification of its answers, and the text
helpers every backend payload and backend log line uses.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/
├── TBD_BackendConfig.c      reads the backend URL and both secrets from the profile
├── TBD_BackendText.c        JSON string escaping, RFC 3339 UTC time, two-digit padding, backend description
├── TBD_GameRuntimeAnswer.c  classifies an answer: success, 409 refusal, transient, permanent
└── TBD_GameRuntimeHttp.c    the machine-credential transport: one answer per call, backoff
```

## How it works

### Two authentication tiers

`TBD_BackendConfig` reads `$profile:TBD_BackendConfig.json` (copied from
`apps/mod/tbd-framework/Data/backend.example.json`), whose keys are the fields of
`TBD_BackendConfigFile`:

| Key | Default | Sent as | Used for |
|---|---|---|---|
| `backendUrl` | none; no platform connection without it | the base of every URL | every call |
| `serverToken` | none | `X-Service-Token` | `POST /api/v1/ingest/link-confirm` and `POST /api/v1/ingest/match-results` |
| `machineCredential` | none | `Authorization: Bearer tbdm_...` | every `/api/v1/game-runtime/` and `/api/v1/fleet-executor/` route |

The `machineCredential` is this server's `mod_runtime`
[machine credential](/documentation_v2/glossary/g_to_m.md#machine-credential), issued by an administrator.
A value that does not start with `tbdm_`, such as the example's placeholder, counts as unset: no
deployment is read, no runtime session starts, no roster loads and no fleet command is claimed.
`Reload` re-reads the file while the server runs and keeps the settings in force when the file does
not read or parse; the loops that wait on the platform (`TBD_DeployedMission` and
`TBD_RosterLoader` in `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`) call it,
so a credential pasted in later is picked up without a restart. `SetBackend`, behind the admin
chat command `#tbd backend`, repoints the URL and token and saves the file. Neither secret is
logged. The mission and its [event](/documentation_v2/glossary/a_to_f.md#event) are not configured here:
they come with the deployment the platform holds for this server.

### The machine-credential transport

`TBD_GameRuntimeHttp.Post` and `Get` open a `RestContext` with the bearer credential, the JSON
content type and a 15 s timeout (`REQUEST_TIMEOUT_S`), and deliver exactly one
`TBD_GameRuntimeAnswer` to the sender's `TBD_GameRuntimeCall` subclass; a request the engine never
reports is answered transient by a 25 s watchdog (`WATCHDOG_MS`). `TBD_GameRuntimeAnswer` classifies
by HTTP status and by the `details.code` of a 409, never by message text: `SUCCESS` (2xx),
`REFUSED` (a 409 fence refusal, with its parsed details), `TRANSIENT` (no answer, a timeout, 408 or
a server error) or `PERMANENT` (any other client error), and keeps the `details.code` of any error,
such as `NO_DEPLOYMENT`, for the caller. `BackoffMs` gives the exponential retry delay every loop
uses.

### Backend text

`JsonEscape` copies its input through `string.Format` (`string.Replace` mutates in place),
escapes backslashes and then quotes, and folds newline, carriage return and tab to spaces so a
payload stays one log line. `UtcNowIso8601` reads the engine's UTC date and clock and writes
`2026-07-25T16:31:28Z`, the shape the backend's `DateTime<Utc>` parses; `Pad2` zero-pads to
two digits. `DescribeBackend(noneText)` prints the configured backend URL for a log line, never a
secret: `noneText` when no URL is set, `<url> (NO TOKEN)` when the server token is empty.
`TBD_GameRuntimeHttp.DescribeBackend` keeps the machine-credential form of the same line.

## Authority

- Server: everything; the machine credential and the service token live only in the authority's
  profile.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: the engine's `RestApi`, `RestContext`, `RestCallback`, `JsonLoadContext`,
  `JsonSaveContext`, `FileIO` and `System` UTC clock.
- Used by: every sender of the game-runtime routes: `TBD_DeployedMission`,
  `TBD_MissionArtifactVerification` and `TBD_RosterLoader` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`; `TBD_DeploymentAuthorization`,
  `TBD_DeploymentRequest`, `TBD_DeploymentRequestQueue` and `TBD_DeploymentEndQueue` in
  `apps/mod/tbd-framework/Scripts/Game/TBD/Systems/Spawning/`; `TBD_DeployableMissionList` and
  `TBD_MissionDeploymentRelay` in `apps/mod/tbd-framework/Scripts/Game/TBD/Session/MissionSelector/`;
  the runtime session and fleet commands in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`.
  `TBD_BackendText` by the identity link, the results report and the runtime session there.
  `TBD_BackendConfig.SetBackend` by `TBD_AdminCommands`.
- Rules: no secret is ever printed; answers are read by status and `details.code`, never by message
  text; lines added stay ASCII; `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Platform bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — the backend clients that use this connection
- [Server infrastructure domain](/apps/website/api_v2/src/server_infrastructure/README.md) — machine
  credentials and the routes this transport reaches
