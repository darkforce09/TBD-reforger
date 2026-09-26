# Tab lock browser transport

The browser half of the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s cross-tab
writer role: which tab writes the local draft of a [mission](/documentation_v2/glossary/g_to_m.md#mission)
that several tabs have open. The parent module,
`apps/website/frontend/src/v2/apps/editor/shell/tab_lock.rs`, holds the pure half (the roles, the
message and stamp types, `decide_save`, the fallback `elect`, the banner and its copy) and declares
this module for the `wasm32` build, with an inline stub of the same functions for native builds.

## Contents

```text
apps/website/frontend/src/v2/apps/editor/shell/tab_lock/
└── live.rs  the writer Web Lock, the presence channel, the write stamps and `__missionTabs`
```

## How it works

`join`, which the draft writer's tab sync calls, opens the `BroadcastChannel`
`tbd-mission-tabs:<mission id>`, announces the tab (`hello`, answered with `here`) and requests the
Web Lock `tbd-mission-writer:<mission id>`. The tab is read-only from the request until the browser
grants the lock, holds the writer role for as long as it holds the lock, and a closing writer hands
the role to the next queued tab with no message at all. Where `navigator.locks` is missing or
refuses the request, the parent's `elect` picks the oldest announced tab. `leave` posts `bye` on
page hide so the other tabs re-elect at once, `announce_saved` posts `saved` after a write lands so
they pull the record, and `join` for another mission releases the old lock and channel first, since
the editor is a client-side route. `write_stamp` and `read_stamp` keep the last writer's tab id and
time in `localStorage` under `tbd-mission-draft:<record key>`, the input `decide_save` uses to
choose a write-through or a merge. `register_bridge` publishes `window.__missionTabs` (`role`,
`peers`, `tab_id`).

## Boundaries

- Depends on: the parent's types, constants and role state (`Msg`, `Presence`, `Stamp`, `TabRole`,
  `CHANNEL_PREFIX`, `WRITER_LOCK_PREFIX`, `PEERS`, `LOCK_DECIDED`, `ON_PEER_SAVED`); the browser's
  `navigator.locks`, `BroadcastChannel` and `localStorage` through `web_sys` and `js_sys`;
  `serde_json` for the messages and stamps.
- Used by: through the parent's re-exports, the draft writer in
  `apps/website/frontend/src/v2/apps/editor/shell/persist.rs` and
  `apps/website/frontend/src/v2/apps/editor/shell/persist/save_scheduler.rs` (`join`,
  `register_bridge`, `leave`, `announce_saved`, `read_stamp`, `write_stamp`).
- Rules: only the lock holder writes, and a stamp that is missing or names another tab means a
  merge, never a blind overwrite (`t190_a_second_tab_cannot_silently_overwrite_the_first` and
  `t190_the_oldest_tab_writes_and_the_election_is_total` in
  `apps/website/frontend/src/v2/apps/editor/shell/tests/tab_lock/writer_election_and_conflict.rs`);
  a message is one flat struct, so a message from a build with other fields is ignored rather than
  half-read.

## Related documentation

- [Mission Creator feature inventory: data persistence and compile](/documentation_v2/website/frontend/apps/editor/feature_inventory/data_persistence_and_compile.md) — one writer tab per mission.
