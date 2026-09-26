# Once-per-key logging

Gates over `TBD_Log` that write a line once per key, so a recurring condition is reported
without flooding the log.

## Contents

```text
apps/mod/tbd-framework/Scripts/Game/TBD/Core/Logging/
├── TBD_AnnounceOnce.c  an informational line once per key until the key is rearmed
└── TBD_WarnOnce.c      a WARNING line once per channel and key, optionally bounded per channel
```

## How it works

`TBD_WarnOnce.Warn(channel, key, message, maxKeysPerChannel)` keeps one set of warned keys per
channel and writes `[TBD][<channel>] <message>` at WARNING the first time a key is seen. With
`maxKeysPerChannel` at or above 0 the channel's set is cleared once it holds more keys than that,
so a key may warn again; the default -1 keeps every key.

`TBD_AnnounceOnce` keeps one set of claimed keys. `Claim(key)` is true the first time since the
last `Rearm(key)`; `Event` and `Kv` write `TBD_Log.Event` or `TBD_Log.Kv` behind a claim.
A runtime that re-arms per world or per mission calls `Rearm` when it resets.

## Authority

- Server: nothing of its own; both run wherever they are called.
- Client: nothing of its own.
- Owner: nothing.
- RPCs: none.
- Replicated properties: none.

## Boundaries

- Depends on: `TBD_Log` in `apps/mod/tbd-framework/Scripts/Game/TBD/Core/`.
- Used by: none at present; they replace the `WarnOnce` copies in `TBD_UIIcons`,
  `TBD_BriefingService` and `TBD_LoadoutPreviewDresser`, and the `AnnounceOnce` and
  `AnnounceEmptyOnce` flags of the mission runtimes and the objective and play-area components.
- Rules: the line shape is exactly `[TBD][<channel>] <message>`; lines added stay ASCII;
  `cargo xtask mod compile` checks that the scripts compile.

## Related documentation

- [Framework core utilities](/apps/mod/tbd-framework/Scripts/Game/TBD/Core/README.md) — `TBD_Log` and the rest of the core
