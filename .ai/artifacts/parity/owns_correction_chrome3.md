# Correction — T-636 / T-637 / T-638 owns were too narrow, and wave 5 collides

**2026-08-01, main thread.** `owns_parity.md` §"What was not verified" flagged that
`owns_and_waves.md` §5.3 assigns each chrome ticket exactly one post-split module while its own §2
names symbols spanning several, for **T-636, T-637 and T-638**. Operator asked for this to be
settled before filing. It is settled, and one wave breaks.

## The root cause — the layout constants are not local to a module

```
$ for c in DOCK_LEFT_PX DOCK_RIGHT_PX TOOLBELT_BAND_PX STRIP_TOP_PX; do
    printf "%-18s " "$c"; grep -rln "$c" --include="*.rs" .; done

DOCK_LEFT_PX       select_tool.rs  eden_chrome.rs  mission_editor.rs
DOCK_RIGHT_PX      mission_editor.rs  eden_chrome.rs  select_tool.rs
TOOLBELT_BAND_PX   select_tool.rs  eden_chrome.rs  mission_editor.rs
STRIP_TOP_PX       select_tool.rs  eden_chrome.rs  mission_editor.rs
```

**All four are read by three files.** They are not merely style constants — `select_tool.rs` uses
them to convert pointer coordinates into world coordinates, and `mission_editor.rs` uses them to
size the wgpu canvas. So **any ticket that changes a dock width, the toolbelt band or the strip
height touches three files**, and the post-split `eden_layout.rs` is only one of them.

§5.3 assigned these tickets a single module each. That is the error: moving the constants into
`eden_layout.rs` does not move their *readers*.

## Corrected `owns`

| Ticket | §5.3 said | Actual |
|---|---|---|
| **T-636** full-width status bar | `eden_toolbelt.rs` | `eden_toolbelt.rs` · `eden_layout.rs` · `mission_editor.rs` · `select_tool.rs` |
| **T-637** dock density + 240px equalisation | `eden_tree.rs`, `eden_dock_left.rs` | `eden_dock_left.rs` · `eden_dock_right.rs` · `eden_tree.rs` · `eden_layout.rs` · `mission_editor.rs` · `select_tool.rs` |
| **T-638** dock collapse | `eden_layout.rs` | `eden_dock_left.rs` · `eden_dock_right.rs` · `eden_layout.rs` · `mission_editor.rs` · `select_tool.rs` |

Mount points confirmed at `mission_editor.rs:2052` (`DockLeft`), `:2059` (`DockRight`), `:2070`
(`BottomToolbelt`) — so all three tickets touch the host as well as their module.

**§2 of `owns_and_waves.md` was right and §5.3 was wrong** for T-636 and T-638: §2 already listed
`eden_chrome+mission_editor` for T-636 and `eden_chrome+mission_editor+select_tool` for T-638,
correctly calling T-638 *"a chokepoint: three hot files at once"*. Only the post-split narrowing
lost it.

**T-637 is different — it was under-scoped in both**, and partly by me. §2 gives it `eden_chrome`
alone, which was right when the ticket read *"dock density — 85% empty"*. The chrome direction
committed at `004a9c6d` rescoped it to *"Eden's density **and** the 240px equalisation"*, and
equalising the dock widths is precisely what pulls in the constants and their three readers. My
change widened the footprint; the packing predates it.

## The consequence — wave 5 collides

From the combined packing:

| Wave | Tickets | Status |
|---|---|---|
| 2 | T-636 · T-640 · T-656 | **safe** — T-640 is dem/contours, T-656 is a new `validate` file |
| 3 | T-638 · T-657 | **safe** — T-657 is `validate` |
| 5 | T-637 · T-648 · T-660 | **COLLIDES** |

**T-648** (transform: Shift-rotate, snap grid, widget) owns
`mission_editor.rs` + `select_tool.rs` + `editor_ops.rs`.
**T-637** now owns `mission_editor.rs` + `select_tool.rs` + four modules.

Two claimants on two hot files in one wave. Wave 5 cannot run as packed.

## The fix — move T-637, do not split the wave

Splitting wave 5 gives **19 waves**. Moving T-637 keeps **18**, because there are later waves whose
rows touch neither `mission_editor.rs` nor `select_tool.rs`.

Recommended: **T-637 moves to wave 12** (currently `T-650` alone — `eden_dock_right.rs` +
`editor_ops.rs` + `doc/store.rs`). That collides on nothing, and T-650 is the compositions ticket
whose own storage location is already marked `low` confidence, so the wave was soft anyway.

Wave 5 then runs T-648 · T-660 (2 rows), wave 12 runs T-650 · T-637 (2 rows). **Total stays 18,
mean 2.39 unchanged.**

## What this does not change

The three chrome tickets stay in three different waves regardless, so the *ordering* logic survives
— only wave 5's membership was wrong. No dependency edge is affected: T-637 gates nothing and is
gated only by wave 0.

## What it says about the split decision

`mission_editor.rs` goes from **17 claimants to 18** once T-637 is counted correctly (T-636 and
T-638 were already in the 17). That does not change the operator's decision — splitting
`mission_editor.rs` alone still buys ~1 wave, and `editor_ops.rs` sits one behind at 16 — but it is
one more piece of evidence that **every geometry change in this editor is an input-handling change**,
because the constants that define the chrome are the same constants that unproject the pointer.

Worth stating in the split ticket if it is ever revisited: extracting `eden_layout.rs` does not
decouple layout from input. A real decoupling would need the pointer transform to read the geometry
at runtime rather than importing the constants.
