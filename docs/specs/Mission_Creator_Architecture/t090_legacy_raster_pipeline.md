# T-090 — Legacy raster pipeline (frozen / cancelled)

**Status:** **retired** — do not schedule new work here  
**Superseded by:** [`t090_10_map_engine_v2.md`](t090_10_map_engine_v2.md)  
**Authority:** T-144.1 @ `b1949182`

---

## What this was

Before T-144, map readability was pursued by **offline raster compositing**:

- Dual tile pyramids: `tiles/satellite/` + `tiles/map/`
- ImageMagick / Node compose scripts (`build-map-cartographic.mjs`, water masks, land-cover tints)
- Baking `.topo` roads into satellite or map pixels (T-090.1.2.9)
- Tile prefetch for pyramid pan (T-090.1.2.3)

A3 does **none** of this for readability — it draws vectors from world data every frame.

---

## Shipped legacy (keep running, do not extend)

| Slice | Shipped | Keep as |
|-------|---------|---------|
| T-090.1.2.8 | `db9057ef` | **Photo field** — `everon-sat.tbd-sat` (= A3 `DrawField`) |
| T-090.1.2.x SAP/water | various | Source for unified sat only |
| T-090.1.1 | `6e06e679` | Legacy `tiles/map/` — **frozen** |
| T-090.1.1.1 | `018ea70d` | Legacy land-cover bake — **frozen** |
| T-090.1.2.6 | `b958e3b4` | Hillshade slider — **keep** (layer 3 in v2) |

**Rule:** No new `make map-cartographic-everon` features. No new compose passes. Sat water composite stays until v2 sea-band layer replaces it visually.

---

## Cancelled slices (registry `cancelled`)

| Slice | Was | Why cancelled |
|-------|-----|---------------|
| **T-090.1.2.9** | Bake road strokes into satellite raster | Roads = T-090.5 `PathLayer` from export |
| **T-090.1.2.3** | Prefetch legacy tile pyramid | Superseded by tbd-sat; A3 has no pyramid |

---

## Scripts / paths — disposition

| Path / script | v2 disposition |
|---------------|----------------|
| `scripts/map-assets/build-map-cartographic.mjs` | **freeze** — no new features |
| `make map-cartographic-everon` | **freeze** — dev-only legacy rebuild |
| `packages/map-assets/*/tiles/map/` | **legacy** — optional fallback until v2 layers cover Map mode |
| `scripts/map-assets/build-landcover-mask.mjs` | **replace** by export density grid (T-090.8) |
| `docs/specs/.../t090_basemap_dual_view.md` | **supersede** — crossfade model in T-090.10 |
| `t090_1_2_satellite_backlog.md` | **historical** — no new `.2.x` slices |

---

## Migration note for Mission Creator

Until T-090.5 ships v2 layers:

- **Satellite** tab: unified `tbd-sat` (unchanged)
- **Map** tab: legacy `tiles/map/` pyramid OR interim crossfade — plan in T-090.10.1

End state: **one photo texture + vector/cartographic layers**; Map/Satellite = opacity + toggle, not two pipelines.
