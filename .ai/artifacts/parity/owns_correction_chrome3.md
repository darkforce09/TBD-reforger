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

## The consequence — a collision, but not where this document first said

**Self-correction, same session.** The first version of this file reported the collision at
**wave 5**, against the table in `owns_and_waves.md`. That table is the **30-ticket packing, which
`owns_parity.md` superseded** with the combined 43-ticket one. In the combined packing wave 5 is
`T-636 · T-646` and T-637 already sits at **wave 16**. I analysed a stale table — the same
inherited-source error this program keeps recording, committed while fixing an instance of it.

**The finding survives the relocation.** Checking the widened `owns` against the *combined* packing:

| Wave | Tickets | Status |
|---|---|---|
| 5 | T-636 · T-646 | **safe** — T-636's widened set adds `mission_editor` + `select_tool`; T-646 is `asset_catalog` + `eden_dock_right` + `editor_ops`. Disjoint. |
| 7 | T-638 · T-659 · T-657 | **safe** — T-638 already carried `mission_editor` + `select_tool` in this packing; T-659 is `eden_top_strip` + `editor_ops`, T-657 is `validate`. |
| **16** | **T-069⊕T-213 · T-637** | **COLLIDES** |

**T-069⊕T-213** (markers) owns `eden_dock_right` + `editor_ops` + `mission_editor` + `store` +
`draw_order` + `engine`.
**T-637** now owns `eden_dock_left` + **`eden_dock_right`** + `eden_tree` + `eden_layout` +
**`mission_editor`** + `select_tool`.

They share **two** files — `eden_dock_right.rs` and `mission_editor.rs`. Wave 16 cannot run as
packed.

## The fix — move T-637 to wave 12

Wave 12 is `T-649 · T-632`. T-649 owns `mission_editor` + `select_tool` + `attributes` +
`editor_ops` — **that also collides** with the widened T-637 on two files. So not there either.

Checking the remaining waves for one whose rows touch none of T-637's six files:

- **Wave 11** — `T-655` (`validation_panel` + `mission_editor`) ✗ collides on `mission_editor`
- **Wave 13** — `P-10` (`mission_editor`) ✗
- **Wave 17** — `T-079d` (`mission_editor` …) ✗

**Every candidate wave has a `mission_editor.rs` claimant**, which is the whole point of that file
being the binding constraint at 18 claimants. There is no free slot.

**So T-637 gets its own wave, and the program becomes 19.** That is the honest cost of the chrome
direction: equalising the dock widths converts a single-module cosmetic ticket into a six-file
geometry-and-input ticket, and the binding file has no spare capacity left.

Wave 16 runs `T-069⊕T-213` alone; a new **wave 18** runs `T-637` alone. **19 waves, 43 tickets,
mean 2.26.**

Two single-agent waves reappear at the tail — exactly what the `eden_chrome.rs` split was meant to
eliminate. That is not an argument against the chrome direction; it is the clearest evidence yet
for the **second split**, which the operator has declined for now on risk grounds. Recorded so the
trade is visible if it is revisited.

## What this does not change

No dependency edge is affected — T-637 gates nothing and is gated only by wave 0, so it can sit
anywhere after wave 0. The three chrome tickets remain in three different waves either way, so the
ordering logic survives; only wave 16's membership and the total count change.

## What it says about the split decision

`mission_editor.rs` goes from **17 claimants to 18** once T-637 is counted correctly (T-636 and
T-638 were already in the 17). That does not change the operator's decision — splitting
`mission_editor.rs` alone still buys ~1 wave, and `editor_ops.rs` sits one behind at 16 — but it is
one more piece of evidence that **every geometry change in this editor is an input-handling change**,
because the constants that define the chrome are the same constants that unproject the pointer.

Worth stating in the split ticket if it is ever revisited: extracting `eden_layout.rs` does not
decouple layout from input. A real decoupling would need the pointer transform to read the geometry
at runtime rather than importing the constants.
