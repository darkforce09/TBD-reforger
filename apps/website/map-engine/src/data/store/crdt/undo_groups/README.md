# Undo groups and clocks

How the [mission](/documentation_v2/glossary/g_to_m.md#mission) document's undo history groups edits:
the capture window that merges one gesture's transactions into one undo step, the clock that
freezes while an explicit group is open, and the cap that forgets the oldest steps.

## Contents

```text
apps/website/map-engine/src/data/store/crdt/undo_groups/
├── clocks.rs  the window and cap constants, the grouping clock, the host clock hook, the cap math
├── mod.rs     the module tree; re-exports the public items of `clocks.rs`
└── tests/     unit tests for the window, explicit groups and the depth cap
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
`default_inner_clock`: in test builds a clock that advances past the window on every read, so each
transaction is its own step unless grouped; in other builds the wall clock, which on
`wasm32-unknown-unknown` is the function the host passes to `install_wasm_now` once at start (it
reads 1 until then), so the crate needs no `wasm-bindgen`. The grouping clock raises every reading
to at least 1, because an anchor of 0 means no open group. `ManualClock` is a settable clock for
tests, handed to `MissionDocCore::with_undo_clock`.

`MAX_UNDO_GROUPS` (200) caps the history. `hidden_prefix_after` returns how many of the oldest
stack items to hide once more than 200 are visible; the count only grows, the items stay on the
`yrs` stack, and `undo_depth` and `undo` ignore them. Each stack item is one group, so a group is
dropped whole, never split.

## Boundaries

- Depends on: `yrs` (`Origin`, `sync::Clock`, `sync::Timestamp`, `undo::Options`) and the standard
  library's atomics, `Arc`, `HashSet` and `OnceLock`.
- Used by:
  - `crate::data::store::rows`, whose `MissionDocCore` builds its undo manager here and applies
    the cap in `undo_depth`;
  - through the re-exports of `crate::data::store`, `install_wasm_now`, called at wasm start by
    `apps/website/frontend/src/v2/apps/editor/bridge/host_state/undo_grouped_gestures.rs`; the
    store also re-exports `GESTURE_WINDOW_MS`, `MAX_UNDO_GROUPS` and `ManualClock`, which only
    this folder's tests read.
- Rules:
  - edits closer together than the window are one step and edits further apart are two
    (`three_position_ops_within_50ms_are_one_undo_group`,
    `edits_separated_by_more_than_the_window_are_two_groups` in `tests/cases_1.rs`);
  - an open explicit group holds its edits together past the window, and closing it splits it
    from the next gesture (`explicit_group_wins_over_the_window`,
    `end_group_splits_from_the_next_gesture`);
  - the cap drops the oldest whole group (`depth_cap_drops_oldest_of_201_groups`,
    `hidden_prefix_math_drops_oldest_whole_group`).
