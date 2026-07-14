# T-152.1 — Grok Code handoff (wgpu text lane)

**Active slice:** T-152.1  
**Implementing agent:** **Grok 4.5 in Cursor** (not Claude Code; do **not** `./scripts/ticket run`)  
**CWD:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`  
**Branch:** `ticket/T-152`  
**Hub:** [`t152_map_cartographic_fidelity_program.md`](../../docs/specs/Mission_Creator_Architecture/t152_map_cartographic_fidelity_program.md)  
**Spec:** [`t152_1_wgpu_text_lane.md`](../../docs/specs/Mission_Creator_Architecture/t152_1_wgpu_text_lane.md)

---

## Preflight

```bash
cd /home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152
git rev-parse --abbrev-ref HEAD   # must be ticket/T-152
test "$(basename "$(git rev-parse --show-toplevel)")" = "TBD-T-152"
```

---

## What this slice is

Rust-owned **map text lane** (SDF or canvas→texture) + **importance-distance declutter** stub (T-144 G8). Thin TS bridge only (≤80 LOC). Unblocks height markers (.7), town labels (.8), road names (.9).

---

## Dependencies shipped

- T-152.0 docs pass (hub + all specs + registry)
- T-151 wgpu engine (text lane builds on `map-engine-render`)

---

## Primary files

| Path | Role |
|------|------|
| `crates/map-engine-core/src/label/` | `LabelSpec`, declutter |
| `crates/map-engine-render/src/text/` | Font/atlas + TextLane + shader |
| `crates/map-engine-wasm/` | wasm exports |
| `apps/website/frontend/src/features/tactical-map/wgpu/wgpuTextLane.ts` | Thin bridge ≤80 LOC |
| `.ai/artifacts/t152_1_verify_log.md` | G1–G8 PASS table |

---

## Gates

Every Gn in the spec Mathematical acceptance matrix must be **PASS**. No PARTIAL / DEFERRED.

---

## Out of scope

Town/road data · height peak detect · landmark icons · HTML overlays · docs/registry edits

---

## After ship

Tag **T-152.1** · verify log · Cursor advances `active_slice` → **T-152.2** only after all Gn PASS.

**Ready for Cursor doc sync.**
