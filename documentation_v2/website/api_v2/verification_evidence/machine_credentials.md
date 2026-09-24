**Status:** live

# Machine credentials and runtime sessions

This note records the implemented design of server-scoped machine authentication and the runtime
sessions that fence game-runtime heartbeats. It describes the code; acceptance evidence is the
command output recorded in progress_checkpoint.md.

## Credentials

- `server_machine_credentials` (migration 0046) holds one row per issued credential: its server,
  its executor kind (`host_agent` or `mod_runtime`), a label, the administrator who issued it,
  use, and revocation (who, when, why). Only the SHA-256 of the secret is stored.
- The secret reads `tbdm_<credential id>_<64 hex digits>`. The id selects the row and the digest of
  the whole secret is compared in constant time; malformed, unknown and mismatched secrets answer
  the same 401. A revoked credential answers 401, a deactivated server 403.
- Administrators issue (`POST /servers/{id}/credentials`, the secret is shown once), list
  (`GET /servers/{id}/credentials`) and revoke (`DELETE /servers/{id}/credentials/{credentialId}
  ?reason=`) credentials. Each write locks the server row, reauthorizes the administrator on that
  transaction and audits in it. Revocation is independent per credential, idempotent, and ends
  every runtime session the credential authenticated.
- `MachineCaller` (`server_infrastructure/services/machine_authentication.rs`) is the extractor
  of every machine route. Handlers require their executor kind and check every resource against
  the caller's server: a roster, session or deployment of another server answers 403.

## Runtime sessions

- `server_runtime_sessions` (migration 0047) records each boot of a server's game runtime with a
  per-server generation; at most one session per server is open.
  `POST /game-runtime/sessions` locks the server, ends the open session as `superseded` and takes
  the next generation.
- A heartbeat (`POST /game-runtime/sessions/{sessionId}/heartbeats`) carries the generation and a
  sequence that must exceed every sequence the session admitted; gaps are allowed. The fence and
  the status write commit together, so a stale generation (`STALE_GENERATION`), a duplicate or
  reordered message (`STALE_SEQUENCE`) or a message of an ended session (`RUNTIME_SESSION_ENDED`)
  is refused with 409 and writes nothing. The body cannot name a server.
- A session without an admitted heartbeat for 60 seconds expires; the expiry worker marks the
  server offline and republishes its status. A runtime ends its own session with
  `POST /game-runtime/sessions/{sessionId}/end`. Ending a session for any reason ends the player
  lives still open in it.
- Game-runtime routes live under `/api/v1/game-runtime/` on the global rate-limit tier: a mission
  start spawns every player of a server within seconds, which the 1/s strict tier would serialize.

## Consumers

- Server Control issues, lists and revokes credentials
  (`pages/administration/server_control/machine_credentials/`); the secret is shown once.
- The game runtime reads `machineCredential` from `$profile:TBD_BackendConfig.json`
  (`TBD_BackendConfig.c`); `TBD_RuntimeSession.c` opens, heartbeats and ends the runtime
  session. Without a credential it holds no session and runs the last verified cached artifact,
  if any.
- `cargo xtask deploy staging` writes `TBD_MOD_RUNTIME_CREDENTIAL` into the profile on every
  deploy and `TBD_HOST_AGENT_CREDENTIAL` into the host agent's owner-only credential file;
  `cargo xtask setup server-profile` writes `TBD_MACHINE_CREDENTIAL`; `cargo xtask mod playtest
  --mission` issues a `mod_runtime` credential per run and revokes it when the server stops.

## Tests

`tests/server_machine_credentials.rs` (`server_credentials_*`), `tests/runtime_session_fencing.rs`
(`heartbeat_fencing_*`), the adapted heartbeat and roster suites, and the unit tests in
`services/tests/machine_credentials.rs`.
