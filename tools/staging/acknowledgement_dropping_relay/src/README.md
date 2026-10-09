# Acknowledgement-dropping relay source

The engine behind the lost-acknowledgement cases of the `staging_fleet` receipt: a loopback relay
between the host agent of one fleet instance and the API that passes every exchange through
unchanged and, once armed, withholds one executor answer past the agent's request timeout and then
closes its connection, so the API has recorded a claim or a result that the agent never hears
about.

## Contents

```text
tools/staging/acknowledgement_dropping_relay/src/
├── cli.rs               the `serve` and `control` commands and their exit codes
├── connection_abort.rs  the listener whose connections the relay can end without writing a byte
├── control_socket.rs    the mode-600 control socket, its line protocol, and the `control` client
├── drop_policy.rs       the arming, the claim and result routes it matches, the status document
├── error.rs             `Error`, `Result` and `error_chain`, the one-line rendering of an error and its causes
├── http_client.rs       `http_client_builder`: every HTTP client, built after the rustls ring provider is installed
├── lib.rs               the crate root: module header, `mod` lines and the public surface
├── prelude.rs           the command line, the control client, the settings and the status for glob import
├── relay.rs             `start` and `serve`: forwarding, the withheld answer, the event log
└── relay_settings.rs    the loopback rule on the listen address and the upstream, the hold time
```

## How it works

```text
host agent ──▶ 127.0.0.1:P (relay) ──▶ upstream http://127.0.0.1:8080 (API)
                     │                          │
                     │  read the request whole  │  forward: method, path, query,
                     │                          │  end-to-end headers, body
                     ▼                          ▼
               drop policy ◀── the upstream's answer, read whole
                     │
       disarmed, or not the armed answer ──▶ passed back unchanged
       armed, and a 200 to the armed route ──▶ record it, disarm, hold 30 s,
                                               abort the connection (no byte written)
```

1. `serve` checks both addresses before anything binds: the listen address is an IP literal in
   `127.0.0.0/8` or `::1`, and the upstream is `http://` on a loopback IP literal or on
   `localhost`, which the relay pins to `127.0.0.1` instead of resolving.
2. It creates the control socket, mode 600, and refuses to start when a relay already answers on
   it or when the path holds something that is not a socket; a socket nobody answers on, left by
   a relay that stopped, is replaced. The socket file is removed when the relay stops.
3. Every exchange is forwarded without its hop-by-hop headers (and those its `Connection` header
   names), with the `Host` of the upstream origin, and with no header of the relay's own; the
   answer comes back with the upstream's status, end-to-end headers and body. When the upstream
   cannot be reached, the relay answers `502` itself.
4. `control arm drop-next-claim-response` arms the next `200` answer to
   `POST /api/v1/fleet-executor/commands/claim`; `drop-next-result-response` the next `200` answer
   to `POST /api/v1/fleet-executor/commands/{commandId}/result`. A `204` claim answer (nothing
   claimable), any other status and any other route pass through and keep the arming; the first
   matching answer spends it, under the same lock, so one arming withholds exactly one answer.
5. The withheld answer is held for 30 s, ten seconds past the host agent's 20 s request timeout,
   and then its connection is aborted: every later write on it fails, so hyper closes the
   connection without a byte of the answer. By then the agent has given up and retries, and the
   relay, disarmed again, passes the retry through.

`control status` (and every `control` request) prints one JSON line:

```json
{"arming":"disarmed","listen":"127.0.0.1:18085","upstream":"http://127.0.0.1:8080",
 "withhold_milliseconds":30000,"forwarded_count":42,"drop_count":1,
 "last_drop":{"response":"claim","command_id":"…","fencing_token":1,"upstream_status":200,
              "withheld_at_unix_ms":1790000000000}}
```

`arming` is `disarmed`, `drop-next-claim-response` or `drop-next-result-response`. A claim drop
takes the command id and the fencing token from the claimed command it withheld; a result drop
takes the command id from the route and the fencing token from the report the agent sent.

## Public surface

- `entrypoint`: the executable's command line (see the
  [executables README](/tools/developer_tools/src/bin/README.md#acknowledgement-dropping-relay)).
- `start(RelaySettings, RelayLog) -> RunningRelay` on the caller's runtime, and `serve`, which
  runs the relay on a runtime of its own until SIGTERM or SIGINT.
- `RelaySettings` with `from_flags`, `UpstreamOrigin`, `loopback_listen_address`,
  `AGENT_REQUEST_TIMEOUT` and `DEFAULT_WITHHOLD`.
- `send_control_command` and `ControlCommand`, and the status types `RelayStatus`, `Arming`,
  `DropRecord` (its command named by a `FleetCommandId`), `DropTarget` and `ExecutorResponse`,
  which deserialize the printed status.
- `Error`, `Result` and `error_chain`; `prelude`.

## Boundaries

- Depends on: `axum` (the listener, through a listener of its own whose connections can be
  aborted), `reqwest` (the upstream client: no proxy, no redirects), `tokio`, `clap`, `serde`,
  `serde_json`, `newtype_ids`, `time_source` (the drop record's wall-clock stamp) and `thiserror`.
- Used by: `developer_tools`' `acknowledgement-dropping-relay` executable, which the unit
  `acknowledgement-dropping-relay@N` runs on the staging host (installed by
  `cargo xtask deploy staging`), and the staging fleet procedure, which runs `control` there.
- Rules:
  - The relay never stores, logs or reports the `Authorization` header or any body; a log line
    names at most the method, the path, the command id and its fencing token.
  - The claim and result documents follow `fleet-command.schema.json` (the `@contract` tags in
    `drop_policy.rs`, checked by the contract-citation gate).

## Related documentation

- [Staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md) — the
  fleet procedure's waves W13 and W14 and the lost-acknowledgement cases they judge.
- [Acknowledgement-dropping relay](/tools/staging/acknowledgement_dropping_relay/README.md) — the
  crate this folder is the source of.
- [Staging verification engines, end to end](/documentation/tools/staging/staging_verification_engines.md)
  — the load engine and the relay, their flows and decisions.
