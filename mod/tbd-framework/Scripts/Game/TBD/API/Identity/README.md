# Player identity and linking

The one accessor of the `arma_id` every backend payload carries, and the `#tbd link <code>` chat
command that links that game identity to a TBD website account.

## Contents

```text
mod/tbd-framework/Scripts/Game/TBD/API/Identity/
├── TBD_IdentityLink.c         the `#tbd link` chat surface: usage, status, local validation
├── TBD_IdentityLinkConfirm.c  the serial confirm queue: one POST in flight, player replies
├── TBD_IdentityLinkPending.c  one queued confirm request
└── TBD_PlayerIdentity.c       the one accessor of the `arma_id` every payload carries
```

## How it works

`TBD_PlayerIdentity.GetArmaId` returns the engine's player identity exactly as it goes on the wire,
or empty when the host issues none; `IsDurable` is false for the `00bbbddd-` name hash a listen
host synthesizes. Link confirmation and match results both use it, so the `arma_id` a link writes
is byte for byte the one match results carry.

```text
website: POST /api/v1/me/link -> 6-digit code, live for 10 minutes
player types `#tbd link <code>`
  -> TBD_AdminCommands chat hook -> TBD_IdentityLink.TryConsumeBeforeBroadcast (never broadcast)
  -> TBD_IdentityLink.Submit: identity resolves, is durable, backend configured, queue not full
  -> TBD_IdentityLinkConfirm: POST /api/v1/ingest/link-confirm {code, arma_id, arma_character}
     through TBD_GameRuntimeHttp, with the machine credential as Authorization: Bearer
  -> private chat reply per HTTP status (404, 409, 400, 401/403, no status, other)
```

`TBD_IdentityLink` handles `#tbd link <code>` and `#tbd link status`. The line is consumed before
the chat broadcast, so the code never reaches public chat; the authority sends one confirmation at
a time (at most 16 waiting), each call carrying its request, so an answer is matched to its player;
the transport's watchdog answers a call the engine never reports. A confirmation is interactive: it
is held in memory only and never put on the durable telemetry queue. Replies go to the player
privately through `TBD_PlayerChat`; an asynchronous reply is sent only while the player id still
resolves to the identity stamped at enqueue, since a dedicated server recycles player ids. A player
without a durable identity is refused and told why: `users.arma_id` is UNIQUE, so a seat number or
a name hash there binds the account to whoever holds that seat or name next.

## Authority

- Server: the link flow, the identity lookup and every reply; `TBD_IdentityLink.Arm` returns on a
  client.
- Client: `TBD_IdentityLink.TryConsumeBeforeBroadcast` runs on every peer from the chat hook in
  `TBD_AdminCommands` and swallows a `#tbd link` line so it is never broadcast; only the authority
  sends it on.
- Owner: nothing.
- RPCs: none; replies go through private chat (`SCR_ChatComponent.SendPrivateMessage`).
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_GameRuntimeHttp`, `TBD_GameRuntimeAnswer`, `TBD_BackendConfig` and
  `TBD_BackendText` in `mod/tbd-framework/Scripts/Game/TBD/API/Http/`; `TBD_PlayerChat`,
  `TBD_Authority` and `TBD_Log` in `mod/tbd-framework/Scripts/Game/TBD/Core/`; the engine's
  `SCR_PlayerIdentityUtils`. Over HTTP, the link confirmation of
  `crates/api/api_identity_and_access/src/`.
- Used by: `TBD_AdminCommands` in `mod/tbd-framework/Scripts/Game/TBD/Session/Admin/` (the chat
  hook) and `TBD_MissionLoader` in `mod/tbd-framework/Scripts/Game/TBD/Systems/Mission/Loaders/`
  (`Arm`); `TBD_PlayerIdentity` by the results report, the fleet commands, `TBD_RosterLoader`,
  `TBD_DeploymentAuthorization` and `TBD_MissionDeploymentRelay`.
- Rules: every `arma_id` on the wire comes from `TBD_PlayerIdentity.GetArmaId`; the link code is
  never echoed in public chat or logs; lines added stay ASCII and `cargo xtask mod compile`
  checks that the scripts compile.

## Related documentation

- [Platform bridge](/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — the machine-credential tier
- [Identity and access domain](/crates/api/api_identity_and_access/src/README.md) — the link
  code handshake the `#tbd link` command completes
- [Discord identity link specification](/documentation/mod/tbd-framework/UI/discord_identity_link/discord_identity_link_specification.md)
  — the in-game linking flow
