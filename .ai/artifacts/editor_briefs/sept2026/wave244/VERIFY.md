You are the adversarial verifier for TBD-Reforger factory **wave 244**.

Do NOT fix. Do NOT commit. Do NOT file tickets. Leave main exactly as you found it.
End with an explicit list of what you attacked and FAILED to break.

## Wave
- Base (pre-merge): `5b80d0e4f`
- T-935.13 merge: `84db01f94`
- T-673 merge: `da076d47e`
- UNREAD retirement (command centre): `d383334c7`
- HEAD at dispatch: whatever `git rev-parse HEAD` says; do not assume.

Repo: `/run/media/system/Disk_2/Projects/TBD-Reforger` on **main**. Host cargo.
`export CARGO_TARGET_DIR=/home/Samuel/.cache/tbd-target`

## What landed
1. **T-935.13** — everon binary cutover. dem.raw UNFILLED (operator). T-981 REFUTED: `prefabs/descriptors/` stays. T-985: occluder archive boot must still fetch blas-manifest hot list. T-993: satellite Range dispatch accepts v2 header; encoding may still be tbd-sat-v1. flate2 KEPT. gz-JSON kept as emitter input. water/unified-v2 emitters skipped (no staging/).
2. **T-673** — six marker style fields on `TBD_MissionMarkerStruct`. Rpc() max 8 params so style is a 6-int trailer on `xs`. Color `#rrggbb` → nearest `SetColorEntry` enum. Area fill NOT drawn. Slice gate FAIL on UNREAD was expected.

## Attack these (highest risk)
- A gate that reports success on code it never examined (always BLOCKER).
- T-985: `init_from_archive` still returning before the hot-list fetch (dead mechanism).
- T-993: satellite still refuses v2; or schema `containerVersion` bumped when it should not have.
- T-981: archive swallowed blocking prefabs / descriptors deleted.
- dem.raw filled despite operator, or elevation.dem invented.
- flate2 dropped while sniff/emitters still need it.
- T-673 twins not lockstep (framework vs export), or a new `.c` without an export twin (T-946.26).
- UNREAD: rotationDeg/brush/color/alpha still present; size deleted instead of re-pinned; shape deleted instead of 34; T-681 lost its tripwire.
- LFS pointers on main (`vers` magic) for the new `.bin` / `.rkyv` (merge used LFS filters off; pull was required).
- world-los / catalog gates silently vacuous (`if catalog.exists()`).
- Marker Rpc trailer: icon-only missions no longer bit-identical; dedicated client drops style.
- `schema_gates.rs` T-935.13 edits colliding with UNREAD retirement.

## Severity
- BLOCKER: main broken, data at risk, or a gate that passed code it never examined.
- MAJOR: a shipped ticket does not do what it claims, or can destroy operator-authored work.
- MINOR/NIT: everything else.

Restore anything you mutate (`touch` after git checkout restore).

## Report
findings [{severity, path:line, claim, command, evidence}]
attacked_and_failed [list]
main_left_clean {yes/no, porcelain}
