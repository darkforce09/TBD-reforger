# Acknowledgement-dropping relay

The `acknowledgement_dropping_relay` crate: the engine behind the lost-acknowledgement cases of the
`staging_fleet` receipt, and the command line of the `acknowledgement-dropping-relay` executable. A
loopback relay between the host agent of one fleet instance and the API passes every exchange
through unchanged and, once armed over its control socket, withholds one executor answer past the
agent's request timeout and then closes its connection, so the API has recorded a claim or a result
that the agent never hears about.

## Contents

```text
tools/staging/acknowledgement_dropping_relay/
├── Cargo.toml  the `acknowledgement_dropping_relay` library package: axum, reqwest, rustls (ring), tokio, clap, `thiserror`, layout tier 1
└── src/        the relay, the drop policy, the control socket, the settings and the command line
```

## How it works

```text
host agent ──▶ relay (127.0.0.1:18085) ──▶ API (loopback origin)
                    ▲
fleet procedure ── ssh ── acknowledgement-dropping-relay control … arm | disarm | status
```

`serve` checks that the listen address and the upstream are loopback, creates the mode-600 control
socket and relays every exchange; `control` arms the next `200` answer to a claim or to a result
report, which the relay records, holds for 30 s (ten seconds past the agent's timeout) and then
drops by aborting the connection. `src/README.md` describes the forwarding, the arming and the
status document.

## Getting started

The executable is built from `developer_tools` (`cargo build -p developer_tools --bin
acknowledgement-dropping-relay`); the staging deploy builds it on the staging host.

## Configuration

No feature and no environment variable: the `serve` flags `--listen`, `--upstream` and
`--control-socket` and the `control` flag `--control-socket` carry everything, and the unit
`acknowledgement-dropping-relay@N` fills them in.

## Public surface

- At the crate root: `entrypoint`, `start`, `serve`, `RunningRelay`, `RelayLog`, `RelaySettings`,
  `UpstreamOrigin`, `loopback_listen_address`, `AGENT_REQUEST_TIMEOUT`, `DEFAULT_WITHHOLD`,
  `send_control_command`, `ControlCommand`, `MAXIMUM_COMMAND_BYTES`, the status types
  (`RelayStatus`, `Arming`, `DropRecord`, `DropTarget`, `ExecutorResponse`, `FleetCommandId`),
  `Error`, `Result` and `error_chain`.
- `prelude`: the command line, the control client, the settings, the status and `Error`.

## Boundaries

- Depends on: `axum`, `reqwest`, `rustls` (the ring provider), `tokio`, `clap`, `serde`, `serde_json`, `newtype_ids`,
  `time_source` (the drop record's stamp) and `thiserror`.
- Used by: `developer_tools`' `acknowledgement-dropping-relay` binary, which the unit
  `acknowledgement-dropping-relay@N` runs on the staging host and the staging fleet procedure runs
  `control` with.
- Rules: tier 1 of `tools/staging` (`cargo xtask verify crate-tiers`); never a dependency of
  xtask; the relay binds and forwards on loopback only, and never stores, logs or reports the
  `Authorization` header.

## Related documentation

- [Staging tool crates](/tools/staging/README.md) — the three staging crates.
- [Staging verification engines](/documentation/tools/staging/staging_verification_engines.md) —
  the member load and the relay end to end.
- [Staging design note](/documentation/crates/api/api_server/verification_evidence/staging.md) — the fleet
  procedure's waves W13 and W14 and the cases they judge.
