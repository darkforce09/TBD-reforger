# T-174 — MC sat fidelity + heatmap removal + dock guide-line fix

**Status:** SHIPPED @ tag **T-174** / `bbb99526` · **Branch:** `main`  
**Depends on:** T-173 (shipped)  
**Verify:** [`.ai/artifacts/t174_verify_log.md`](../../.ai/artifacts/t174_verify_log.md) · inventory [`.ai/artifacts/t174_inventory.md`](../../.ai/artifacts/t174_inventory.md)  
**Evidence:** [`.ai/artifacts/t174_operator_screens/`](../../.ai/artifacts/t174_operator_screens/)  
**Scope shipped:** `apps/website/frontend/**`, `crates/map-engine-*`. **Not** `apps/mod/`.

**No silent deferrals.** Soft “later / optional / fold forward” forbidden unless the operator explicitly says `defer X` / `skip X`.

## Shipped outcome

| ID | Result |
|----|--------|
| S1 | Localhost sat = preview→full progressive (dropped `sat_dev_preview_default`). Default `make leptos` gets the full TBDS mip chain. `?sat=preview` = Range-only (CI/gates + fast local). `?sat=full` is a no-op. |
| S2 | Density-heatmap **glow removed end-to-end** (no Mission Settings toggle). LOD rung `heatmap_trees` kept (glyph-suppression + forest-mass). Island zoom = forest mass, no green wash. |
| S3 | Guide stems clip to tree rows (`relative` on escaping hosts); no full-dock bright rails. |

Operator override during plan review: *“Remove the heatmap, it's not something I want.”* → S2 is removal, not “default off + toggle”.

**Manual visual:** headless gates green; pixel-aesthetic confirm (sharp sat / no glow / no dock rails) is an operator browser pass on `make leptos` — see verify log §Manual.

## Why (pre-ship)

Post T-173 operator eye-pass 2026-07-18:

1. **Satellite is way too low-res** (blurry at island + local zoom).  
2. **Tree heat map / green glow** — do not want it (density heatmap overlay).  
3. **Full-height bright vertical lines** on left Outliner + right Asset Browser docks.  

Otherwise T-173 is generally good.

## Confirmed leads (recon — historical)

| ID | Bug | Lead |
|----|-----|------|
| S1 | Low-res sat | `satellite.rs` — `sat_dev_preview_default` stuck localhost on preview mip unless `?sat=full`. |
| S2 | Green glow | T-173 H1 density heatmap upload — operator wanted **removal**, not a toggle. |
| S3 | Full-height dock lines | T-173 P7 `guide_spans` escaped hosts → dock-tall rails. |

## Acceptance (met)

| ID | Done when |
|----|-----------|
| S1 | `make leptos` without `?sat=preview` → sharp sat (progressive full). Gates keep `?sat=preview`. |
| S2 | No density-heatmap glow at any zoom; path excised. |
| S3 | No full-panel-height bright rails; hierarchy stems only. |
