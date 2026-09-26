# Backend text

Text helpers shared by every script that talks to the platform backend or logs about it.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/API/Http/
└── TBD_BackendText.c  JSON string escaping, RFC 3339 UTC time, two-digit padding, backend description
```

## How it works

`JsonEscape` copies its input through `string.Format` (`string.Replace` mutates in place),
escapes backslashes and then quotes, and folds newline, carriage return and tab to spaces so a
payload stays one log line. `UtcNowIso8601` reads the engine's UTC date and clock and writes
`2026-07-25T16:31:28Z`, the shape the backend's `DateTime<Utc>` parses; `Pad2` zero-pads to
two digits. `DescribeBackend(noneText)` prints the configured backend URL for a log line, never a
secret: `noneText` when no URL is set, `<url> (NO TOKEN)` when the server token is empty.
`TBD_GameRuntimeHttp.DescribeBackend` keeps the machine-credential form of the same line.

## Authority

- Server: nothing of its own; its callers are the server's backend clients.
- Client: nothing.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_BackendConfig` in `apps/mod/tbd-framework/Scripts/Game/TBD/API/`; the engine's `System` UTC clock.
- Used by: none at present; it replaces `JsonEscape`, `DescribeBackend`, `UtcNowIso8601` and
  `Pad2` in `TBD_ResultsReporter`, `TBD_IdentityLink`, `TBD_RuntimeStatusReadings` and
  `TBD_GameRuntimeHttp`.
- Rules: no secret is ever printed; lines added stay ASCII; `cargo xtask mod compile` checks that
  the scripts compile.

## Related documentation

- [Platform bridge](/apps/mod/tbd-framework/Scripts/Game/TBD/API/README.md) — the backend clients that use these helpers
