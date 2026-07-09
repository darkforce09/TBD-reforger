# T-151.4.1 verify log — fix T-151.4 building wipe + road joins

**Baseline:** tag **T-151.4** (`723490a0`).  
**Slice:** corrective — buildings missing at town clusters; road tears at curves; forest note (soft).

---

## Root cause — P0 buildings missing

**Not** missing export / satellite confusion. W3 residency + prefab path was intact.

### Bug A — empty `pushToEngine` wiped the GPU lane

`runViewport` always called `pushToEngine()` when `set_viewport` returned no missing ids
(same pin key, or all ids already `inflight`). On first pin, Rust `rebuild_buffers()` runs
**before** chunks arrive → `fill_buf` empty. A second debounced settle then did:

```text
upload_world_buildings([]) → remove_lane(WorldBuildings)
```

W4 camera traffic (forest + roads + basemap) made this race much easier to hit than T-151.3.

### Bug B — aborted fetch left ids stuck in `inflight`

`fetchAndQueue` aborted the previous AbortController but never cleared residency `inflight`.
Those ids were never re-requested (`!inflight` filter) → permanent empty fill for the pin set.

### Fix

| Layer | Change |
|-------|--------|
| `residency.rs` | `clear_inflight`, `mark_inflight`, `pin_settled`, `inflight_count`; same-key re-request when unsettled + empty inflight; stats JSON additive fields |
| `wgpuWorldLoader.ts` | abort → clear_inflight → mark_inflight(active ids); **skip empty upload** while inflight/pending/!settled; `window.__wgpuWorldStats` debug |
| `engine.rs` | empty fill + `visible=true` is **sticky** (does not remove lane); clear only via `visible=false` / `clear_world_buildings` |

Native test: `clear_inflight_allows_same_key_rerequest`.

---

## P1 roads — joins/caps

`expand_polyline_strip` now builds **miter joins** (bevel at miter limit 4× half-width) and
**round end caps** (8-seg semicircle), matching Deck `capRounded` / `jointRounded`.

L9 midpoint width test still green; new tests for corner coverage + end cap.

---

## P2 forest

Unchanged algorithm (Deck parity): TBDD iso=1 + Path B mega hulls. Over-dense fill is a shared
T-090.8.1/N11 policy — **not** a W4-only bug. No export rewrite; no α change this slice.

---

## Automated gates — ALL PASS

| Gate | Result |
|------|--------|
| `cargo fmt --check` | **PASS** |
| `cargo clippy --all-targets -- -D warnings` | **PASS** |
| `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings` | **PASS** |
| `cargo test -p map-engine-core --all-features` | **PASS** (incl. new residency + polyline tests) |
| `cargo test -p map-engine-render` | **PASS** — 10 |
| `cargo build --workspace` | **PASS** |
| `make wasm` | **PASS** — **4,009,368 B** (T-151.4 was 4,005,415; **+3,953**) |
| `npm test` | **PASS** — **371** (world.parity / residency.parity / pick green) |
| `npm run build` + `lint` | **PASS** |
| entry isolation | **PASS** |

---

## Manual (operator)

| ID | Check | How |
|----|-------|-----|
| **S1** | Town ~8–9 buildings **present** on `?engine=wgpu` | After settle: `window.__wgpuWorldStats` → `building_instances` / `world_building_instances` **> 0**, `pin_settled: true` |
| **S2** | Curved road / junction — **no tears** | Zoom into a bend; casing+centerline continuous |
| **S3** | Deck unchanged; forest still dense (honest) | `?engine=` off; forest note above |
| **S4** | Stats non-zero at S1 camera | Paste `__wgpuWorldStats` + `engine.stats()` JSON here |

**Operator paste (S1/S4):** _pending browser settle_

Expected shape:

```json
{
  "chunks_pinned": N,
  "chunks_resident": N,
  "building_instances": >=8,
  "inflight_count": 0,
  "pin_settled": true,
  "world_building_instances": >=8,
  "pending": 0
}
```

---

## Before / after (instance counts)

| State | Town cluster buildings drawn | `world_building_instances` (expected) |
|-------|------------------------------|----------------------------------------|
| T-151.3 | ~8–9 visible | non-zero after settle |
| T-151.4 broken | **0** (lane wiped) | 0 after race |
| **T-151.4.1** | ~8–9 restored | non-zero; sticky empty mid-flight |

---

**Ready for Cursor doc sync.**
