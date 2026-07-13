# T-152.12 — Claude Code handoff (text lane resurrection + orientation)

**Active slice:** T-152.12 (remediation ladder #1 — filed from the T-152.11 audit)
**Implementing agent:** **Claude Code** (spec prompt block is `./scripts/ticket prompt`-extractable)
**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`
**Branch:** `ticket/T-152`
**Hub:** [`t152_map_cartographic_fidelity_program.md`](../../docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md) §Remediation ladder
**Spec:** [`t152_12_text_lane_orientation.md`](../../docs/specs/Mission_Creator_Architecture/t152_12_text_lane_orientation.md)
**Audit basis:** [`t152_11_fidelity_audit_report.md`](t152_11_fidelity_audit_report.md) §7

## Preflight

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git status --porcelain   # TextUniforms hotfix expected in tree (engine.rs + shader.wgsl) — KEEP
make wasm
```

## What this slice is

The entire T-152 label surface (towns/roads/heights) never drew at any committed tag: the WGSL `TextUniforms` struct is 32 B (vec3 pad) against a 16 B binding. The fix exists uncommitted in the worktree. Once alive, all text renders upside-down: `vs_text` lacks the `1.0 − unit.y` V-flip every other textured lane applies. This slice lands both fixes with GPU gates on both backends so neither bug class can ship silently again.

## Primary files

| Path | Role |
|------|------|
| `crates/map-engine-render/src/shader.wgsl` | `TextUniforms` (:196-201, hotfix in tree) · `vs_text` UV (:224 — add V-flip) · reference `vs_textured` (:56-57) |
| `crates/map-engine-render/src/engine.rs` | `TEXT_UNIFORM_BYTES=16` (:186-187) · `min_binding_size` (:1250) · label lanes (:905-912) |
| `crates/map-engine-render/src/text_layout.rs` | Atlas bake (y-down) + pack — check tests for old UV assumptions |

## Gates

G1 WGSL guard (no `vec3` in TextUniforms) · G2 UV corner proof (unit.y=1 → v0) · G3 pipeline creation smoke WebGPU+WebGL2 · G4 upright "7" readback both backends · G5 three-lane upload smoke · G6 cargo/clippy/wasm · G7 zero `apps/**` diffs + FE suites green.

## Out of scope

Font fidelity (.13) · label data/bands (.16/.17) · any TS change.

## After ship

Verify log `.ai/artifacts/t152_12_verify_log.md` → registry `T-152.12 → shipped` + `advance-slice` → sync → **T-152.13**.
