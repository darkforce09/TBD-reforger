# Undo groups and clocks

How the [mission](/documentation/glossary/g_to_m.md#mission) document's undo history groups edits:
the capture window that merges one gesture's transactions into one undo step, the clock that
freezes while an explicit group is open, and the cap that forgets the oldest steps.

## Contents

```text
crates/mission/mission_crdt/src/undo_groups/
├── clocks.rs  the window and cap constants, the grouping clock, the platform clock, the cap math
├── mod.rs     the module tree; re-exports the public items of `clocks.rs`
└── tests/     unit tests of the grouping clock's floor and anchor and of the cap math
```

## How it works

`MissionDocCore` builds its `yrs` undo manager from `undo_options`: a capture window of
`GESTURE_WINDOW_MS` (300 ms), only the document's local origin tracked, and a `GroupingClock` as
the timestamp source. `yrs` merges tracked transactions whose timestamps fall inside one window
into one stack item, so the transactions of a single gesture undo together.

```text
begin_group ─▶ depth 0→1: anchor = inner clock now ─▶ every now() returns the anchor
     │           (nested begin_group calls only count)
     ▼
end_group ──▶ depth 1→0: anchor cleared, returns true ─▶ MissionDocCore resets the undo manager,
                                                          so the next local edit is a new step
```

An explicit group therefore wins over the window however long it stays open. The inner clock is
any `time_source::Clock`, the host clock seam: `MissionDocCore::with_undo_clock` takes one, and a
document built without one reads `platform_clock()`, the wall clock of the compilation target
(`Date.now()` on `wasm32`, the system clock natively), so no host installs a clock and no
browser binding enters this crate. The grouping clock adapts that reading to the `yrs` undo
timestamp and raises it to at least 1, because an anchor of 0 means no open group. Tests drive it
with `time_source::ManualClock`.

`MAX_UNDO_GROUPS` (200) caps the history. `hidden_prefix_after` returns how many of the oldest
stack items to hide once more than 200 are visible; the count only grows, the items stay on the
`yrs` stack, and `undo_depth` and `undo` ignore them. Each stack item is one group, so a group is
dropped whole, never split.

## Boundaries

- Depends on: `yrs` (`Origin`, `sync::Clock`, `sync::Timestamp`, `undo::Options`),
  `time_source` (`Clock`, `PlatformClock`) and the standard library's atomics, `Arc` and
  `HashSet`.
- Used by: `MissionDocCore`, which builds its undo manager here, takes its host clock through
  `with_undo_clock` and applies the cap in `undo_depth`.
- Rules:
  - every reading is at least 1 ms, and an open group reads its anchor until the outermost end
    (`grouping_clock_never_reads_zero`,
    `open_group_freezes_the_reading_until_the_outermost_end` in `tests/grouping_clock.rs`);
  - the cap drops the oldest whole group (`hidden_prefix_math_drops_oldest_whole_group`, same
    file; `depth_cap_drops_oldest_of_201_groups` in the mission document's tests);
  - edits closer together than the window are one step, edits further apart are two, and an open
    explicit group holds its edits together past the window (the undo group tests of the mission
    document).
