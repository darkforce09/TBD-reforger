# T-152.12 — Text lane resurrection (16 B uniform) + upright orientation

**Ticket:** T-152 · **Slice:** T-152.12 (remediation ladder #1)
**Status:** `ready`
**Executor:** **claude-code** (Claude Code)
**Authority:** T-152 program hub · audit [`t152_11_fidelity_audit_report.md`](../../../.ai/artifacts/t152_11_fidelity_audit_report.md) §4 S3, §7
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152` · branch `ticket/T-152` · tag **`T-152.12`**
**Depends on:** T-152.11 audit (shipped @ `a8a7a22c`)
**Blocks:** T-152.13, T-152.16, T-152.17 (every visible-label slice)

## In one sentence

Make the wgpu text lane actually draw (land the `TextUniforms` 16 B pad fix that is sitting uncommitted in the worktree) and draw **upright** (add the missing V-flip in `vs_text`), with GPU gates on both backends so a dead or inverted text pipeline can never ship silently again.

---

## Problem

Two stacked defects (audit §7):

1. **Dead lane at every committed tag.** Committed `shader.wgsl` declares `TextUniforms { px_to_m: f32, _pad: vec3<f32> }` — WGSL align-16 makes the struct **32 B**, but the bind-group layout pins `min_binding_size = TEXT_UNIFORM_BYTES = 16` (`crates/map-engine-render/src/engine.rs:186-187`, `:1250`). wgpu validation rejects the text pipeline → town/road/height labels (T-152.7/.8/.9) never drew. The fix (3×f32 pads) exists **only as an uncommitted working-tree diff**, documented in no verify log.
2. **Upside-down text.** `vs_text` assigns `out.uv = mix(vec2(u0,v0), vec2(u1,v1), in.unit)` (`shader.wgsl:224`) — world-top (`unit.y=1`) samples the **bottom** of the y-down atlas cell. The correct convention exists in `vs_textured`: `out.uv = vec2(in.unit.x, 1.0 - in.unit.y)` (`shader.wgsl:56-57`). All three label lanes (`WorldLabels`, `WorldTownLabels`, `WorldRoadLabels`) share the one text pipeline (`engine.rs:905-912`), so every label is vertically mirrored — operator S3 "upside-down and back-to-front".

No automated gate in .7–.10 ever exercised the GPU text path — all label gates were CPU pack/declutter math. That gate hole ships here too.

---

## Goal

1. **Land the uniform fix:** commit the in-tree hotfix (`_pad: vec3<f32>` → `_pad0/_pad1/_pad2: f32` + warning comments) exactly as it stands.
2. **Orientation fix:** flip V in `vs_text` UV interpolation — U unchanged, yaw path unchanged.
3. **Guard gates:** a WGSL-source guard (no `vec3` inside `TextUniforms`), a UV-orientation unit proof, and a **GPU readback** gate that renders an asymmetric glyph through the real text pipeline on **WebGPU and WebGL2** headless backends and asserts upright orientation + successful pipeline creation.
4. Verify log `.ai/artifacts/t152_12_verify_log.md`.

---

## Out of scope

- Font/atlas fidelity (8×8 uppercase blob font) — **T-152.13**.
- Label zoom bands, data quality, kind routing — **T-152.16/.17**.
- Any TS change (lanes are wired and default-on — audit §4 S8).
- Registry/doc sync (Cursor/setup pass owns).

---

## Locked decisions

| # | Decision | Rationale |
|---|----------|-----------|
| L1 | `TextUniforms` stays **16 B** (4×f32); do NOT bump the Rust buffer to 32 B instead | Smallest change; comment in hotfix already warns against vec3 pad |
| L2 | Orientation fix is **V-flip in `vs_text` only**: `mix(..., vec2(in.unit.x, 1.0 - in.unit.y))` | Mirrors `vs_textured:56-57` convention; leaves yaw/world math untouched |
| L3 | GPU gate uses the existing headless harness pattern (cached Chromium: SwiftShader WebGL2 + lavapipe WebGPU via CDP, T-151 self_check infra) | Byte-level proof without operator |
| L4 | Upright assertion = pixel-mass asymmetry of glyph **"7"** (top bar heavy): lit-pixel count in top half > bottom half of glyph bbox, both backends | Robust to AA/filtering; no golden image needed |
| L5 | WGSL source guard test lives in `map-engine-render` unit tests (parse `shader.wgsl` string, assert `TextUniforms` block has no `vec3`) | Locks the exact bug class that killed .7–.10 |
| L6 | No TS logic changes; the **only** permitted `apps/**` diff is the one-line `text:` self-check registration in the DEV spike page (`WgpuCanvas.tsx` `__selfChecks` map — thin wasm call, same pattern as the 8 existing checks); vitest + FE build must still pass | LANGUAGE GATE |
| L7 | Commit `T-152.12:` · tag `T-152.12` · verify log | House convention |

---

## Pinned numbers

| Quantity | Value | Source |
|----------|-------|--------|
| `TEXT_UNIFORM_BYTES` | **16** | `engine.rs:186-187` |
| Text atlas grid | 16×6 cells, 8×8 px, 128×48 | `text_layout.rs:164-165` |
| Label lanes on text pipeline | 3 (`WorldLabels`, `WorldTownLabels`, `WorldRoadLabels`) | `engine.rs:905-912` |
| Correct V convention | `v = 1.0 − unit.y` | `shader.wgsl:56-57` |

---

## Tasks

1. Commit-stage the in-tree hotfix (engine.rs comment + shader.wgsl 3×f32 pads) — do not rewrite it.
2. `vs_text` V-flip (L2). Audit `text_layout.rs` for tests/comments encoding the old mapping; update as needed.
3. Rust unit tests: WGSL guard (L5) + UV-orientation proof (mirror the mix formula on the four quad corners, assert `unit.y=1 → v0` after fix).
4. GPU gate: text self-check draw path (asymmetric "7" at pinned position) + readback assertion (L4) on both backends; wire into the existing self_check/harness entry point.
5. Full verify suite (below) + verify log + commit + tag.

---

## Mathematical acceptance matrix

| Gate | Predicate | Class |
|------|-----------|-------|
| **G1** | `shader.wgsl` `TextUniforms` block contains **no `vec3`** token; struct field count = 4×f32 (unit test) | Source guard |
| **G2** | UV corner proof: post-fix formula maps `unit=(0,1)→(u0,v0)`, `(0,0)→(u0,v1)`, `(1,1)→(u1,v0)` (unit test) | Class R |
| **G3** | Text pipeline **creates without validation error** on WebGPU **and** WebGL2 headless (harness log clean) | GPU smoke |
| **G4** | Glyph "7" readback: `lit(top half) > lit(bottom half)` of glyph bbox on **both** backends | GPU-R upright |
| **G5** | All three label lanes accept an upload + draw ≥1 instance in harness without error | Lane smoke |
| **G6** | `cargo fmt --check`, `clippy -D warnings` (native + wasm target), `cargo test -p map-engine-core -p map-engine-render`, `make wasm` exit 0 | Regression |
| **G7** | FE diff = exactly the one-line spike-page `text:` check registration (no other `apps/**` change); `npm test` + `npm run build` + `npm run lint` exit 0 | LANGUAGE GATE |

---

## Verify

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core
cargo test -p map-engine-render          # G1, G2 live here
make wasm
# GPU harness (G3–G5) — reuse the T-151 headless entry; adjust to actual script name
scripts/website/wgpu-gpu-verify.sh || make wgpu-verify
cd apps/website/frontend && npm test && npm run build && npm run lint
```

---

## Manual acceptance

- **M1:** `?` Mission Creator @ Everon, Map view — town/road/height labels **visible** and **upright** at z ∈ [−2, +1].
- **M2:** Zoom/pan across Morton + Gorey — no mirrored/flipped glyphs anywhere.

---

## Documentation sync (Cursor, after merge)

Registry `T-152.12 → shipped`; hub ladder row; CLAUDE.md §Status via `./scripts/ticket sync`.

---

## Claude Code prompt — T-152.12 (copy-paste)

Authority: this spec. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the T-152 worktree:
  /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152

Implement **T-152.12** — text lane resurrection (16 B uniform) + upright orientation.

═══ PREFLIGHT ═══
  cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
  git status --porcelain   # expect the TextUniforms hotfix already in tree (engine.rs + shader.wgsl) — KEEP it
  make wasm

═══ READ (in order — spec wins) ═══
  1. docs/specs/Mission_Creator_Architecture/t152_12_text_lane_orientation.md
  2. .ai/artifacts/t152_11_fidelity_audit_report.md §7 (text lane deep-dive)
  3. crates/map-engine-render/src/shader.wgsl (vs_textured :56-57, TextUniforms :196-201, vs_text :207-231)
  4. crates/map-engine-render/src/engine.rs (:186-187 TEXT_UNIFORM_BYTES, :905-912 lanes, :1250 min_binding_size)
  5. crates/map-engine-render/src/text_layout.rs (atlas bake + pack)

═══ PROBLEM ═══
  Committed TextUniforms has vec3 pad → 32 B struct vs 16 B binding → text pipeline dead at tags .7–.10.
  vs_text lacks the V-flip vs_textured has → all labels upside-down once the lane is alive.

═══ SHIPPED (do not reopen) ═══
  T-152.0–.11 per verify logs + audit. Do not refactor lanes/atlas beyond this slice.

═══ LANGUAGE GATE ═══
  Rust/WGSL ONLY. Zero TS changes (G7 asserts apps/** untouched).
  STOP IF: about to edit apps/website/frontend/** — nothing there is broken for this slice.

═══ LOCKED ═══
  - TextUniforms stays 16 B (4×f32) — land the in-tree hotfix as-is
  - vs_text fix = V-flip only: mix(..., vec2(in.unit.x, 1.0 - in.unit.y))
  - GPU gate: "7" top/bottom pixel-mass on WebGPU + WebGL2 headless (T-151 harness pattern)
  - WGSL source-guard unit test (no vec3 in TextUniforms)

═══ DO ═══
  1. Keep + stage the in-tree 16 B hotfix
  2. vs_text V-flip
  3. Unit tests G1 (WGSL guard) + G2 (UV corner proof)
  4. GPU self-check: text pipeline smoke + upright "7" readback, both backends (G3–G5)
  5. Full verify suite (spec §Verify)
  6. .ai/artifacts/t152_12_verify_log.md; commit "T-152.12: ..." ; tag T-152.12

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/**, TICKET_*.md, CLAUDE.md markers
  - Touch text_layout glyph art (T-152.13), label data, zoom bands (T-152.16/.17)
  - Bump the uniform to 32 B instead of fixing the WGSL struct

═══ VERIFY (all exit 0) ═══
  (bash block from spec §Verify)

═══ MANUAL ═══
  M1–M2 per spec (operator browser, PENDING allowed at ship)

═══ RETURN ═══
  - Commit SHA + tag T-152.12
  - Verify log path with G1–G7 table
  - GPU readback numbers (top/bottom lit counts per backend)
```
