# RCON client

A BattlEye RCon client for the Arma Reforger dedicated server's own
[RCON](/documentation_v2/glossary/n_to_z.md#rcon) port (the `rcon` block of the server config, UDP 19999
by default), written from the protocol specification. The
[fleet host agent](/documentation_v2/glossary/a_to_f.md#fleet-host-agent) reads the player list through it
and logs out through it on shutdown.

## Contents

```text
apps/fleet_host_agent/src/rcon/
├── command_sequence.rs       one-byte command sequence numbers that wrap after 255
├── mod.rs                    the module tree; re-exports `RconClient` and its settings, timings and errors
├── packet_codec.rs           the datagram layout, its CRC32 and the three packet types
├── rcon_client.rs            `RconClient`, the handle commands go through, with `RconTimings` and `RconError`
├── rcon_session.rs           the task that owns the socket: login, retransmission, keep-alive, logout
├── reforger_commands.rs      `#players` and `@logout`, and the reading of the player listing
├── response_assembly.rs      reassembly of a response split into parts
├── server_message_window.rs  duplicate detection for server messages
└── tests/                    unit tests for the codec, numbering, reassembly, duplicates and listing
```

## How it works

`RconClient::start` binds an ephemeral UDP port of the server's address family, connects it to
the server so datagrams from any other address never arrive, and spawns one `RconSession` task.
Commands queue on a channel (16 deep) and run one at a time; nothing is sent until the first
command, which logs in.

The protocol, as `packet_codec` encodes it:

- Every datagram reads `'B' 'E' | CRC32 (little-endian) | 0xFF | type | payload`, the CRC32
  (IEEE) covering every byte from `0xFF` onward. A datagram with a wrong checksum or header is
  dropped.
- Login (`0x00`) carries the password; the server answers accepted or refused. A refused password
  fails the command at once (`RconError::LoginRejected`); an unanswered login is retransmitted,
  then reported as `LoginUnanswered`.
- Command (`0x01`) carries a sequence number that starts at 0 and wraps after 255, then the
  command text. An unanswered command is retransmitted under the same number. A response split
  into parts (each starting `0x00 | parts | index`) is joined in index order as bytes, whatever
  order and however often the parts arrive, so a character split across parts survives.
- Server message (`0x02`) is acknowledged with its sequence number, every copy of it; a
  64-number window delivers a repeated message to the log once, and a new login clears it.

With the default `RconTimings`, a packet waits 1 s for its answer and is sent at most 4 times; an
empty command is sent whenever 30 s pass without one, below the server's 45 s timeout. When a
command or keep-alive goes unanswered, the session counts as lost and an unanswered command is
sent once more after a new login, so one command, login included, takes at most 16 s, inside the
ledger's 30 s execution window for RCON actions. Delivery is therefore at least once, which is
safe because the agent sends only reads: `#players` lists the session's players (a command RCON
monitor clients may run too), each row `<playerId> ; <identity UID> ; <name>` under a
`Players on server:` header, with any other line kept in `raw_lines`. `RconClient::log_out` sends
`@logout`, which frees the client's slot on the server at once, and stops the task. A command must
be 1 to 1024 bytes of text without control characters. Reforger's RCON has no broadcast command.

## Boundaries

- Depends on: `crate::secret_text` (the password, exposed only in the login packet); the
  `crc32fast`, `tokio` (net, sync, time), `serde_json`, `thiserror` and `tracing` crates; the
  commands of the Bohemia Interactive wiki page "Arma Reforger:Server Management".
- Used by: `crate::agent_configuration`, which builds `RconSettings`;
  `crate::command_execution`, which runs `list_players` through `RconClient::execute`;
  `apps/fleet_host_agent/src/main.rs`, which starts the client and calls `log_out` on shutdown;
  and `apps/fleet_host_agent/tests/rcon_transport.rs`, which runs it against a lossy in-process
  BattlEye RCon server.
- Rules: only commands that are safe to repeat go through the client; a corrupted or foreign
  datagram is dropped, never trusted (`tests/packet_codec.rs`); a retransmitted command keeps its
  sequence number (`rcon_transport_lost_request_is_retransmitted_with_the_same_sequence_number`).
