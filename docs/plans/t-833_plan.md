# T-833 — Plan

## Context

Wave-205 MINOR: the rotation ring sets facing to the release bearing about the centre instead of applying the drag's angular delta, and there is no live preview (T-648 superseded). Operator option c: default orbit about the selection centre, Ctrl = each unit turns in place. Snap ladder at `mission_editor.rs:422`; drag machinery in `select_tool.rs`.

## Approach

1. Verify on main: press the ring 170° off facing, release unmoved → facing jumps (red).
2. `select_tool.rs`: record grab bearing + heading at press; apply delta per move; zero rotation until the cursor moves; per-frame preview via the T-796 `bind_vehicle_preview_lane` pattern and the T-788 batch API.
3. `mission_editor.rs`: Ctrl modifier → in-place; one undo step; fix the `eden_help` comment.

## Risks

- Multi-select orbit changes positions (z-family) — one transaction, one undo.

## Verification

- `cargo xtask mk leptos-gates` · `cargo xtask platform wave gate --slice T-833`
