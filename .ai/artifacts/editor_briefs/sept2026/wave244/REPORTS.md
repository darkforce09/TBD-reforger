# Wave 244 slice reports (command-centre digest)

Landed on main as merges `84db01f94` (T-935.13) and `da076d47e` (T-673).
Base `5b80d0e4f`. `platform wave land --bookkeeping` refused T-673 because the slice
gate was FAIL on expected `UNREAD_WIRE_FIELDS`; hand-merged, then retired/re-pinned
the rows here.

## T-673 — accepted with expected schema red
- pwd: worktree + `slice/T-673`. Three commits. Eight owned `.c` files (twins).
- Perturbation: export-only HEX_DIGITS edit → lockstep FAIL; restored → `OK: compiled clean`.
- `mod compile` clean (11333 classes, +2). Rpc max 8 params: style packed as 6-int `xs` trailer.
- Slice gate FAIL only on UNREAD_WIRE_FIELDS (size 3, rotationDeg 3, brush 2, color 2, alpha 3, shape 34).
- Editor UI not in owns (dock_right.rs / entity.rs) — reported, not done.
- Area fill not drawn (`SCR_MapMarkerBase` has no brush API).

## T-935.13 — accepted, SLICE GATE PASS @ `234e9aff6`
- dem.raw unfilled. descriptors kept (1623). T-985 hot-set fetch after archive boot. T-993 Range dispatch.
- flate2 kept. gz-JSON kept as emitter input. water / unified v2 emitters skipped (no staging/).
- Outside owns (called out, assertion flips only): catalog_emit.rs, manifest.rs, chunk_bin.rs, prefab_load.rs.

## Command-centre UNREAD retirement
- DELETE: rotationDeg, brush, color, alpha (baseline 0).
- RE-PIN: shape 32 → 34 (keep zone-geometry tripwire).
- RE-PIN: size 0 → 3, keep T-681 as owner (entity OBJ-SIZE still unread).
- mission.schema.json `$defs/marker` descriptions: READ SINCE T-673.
