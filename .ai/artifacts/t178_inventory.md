# T-178 inventory — density FS canopy + guides

**Locked A1:** island density texture + `fs_forest_density` (not sticky mesh).

## Equations

```text
N = 1601 = 12800/8 + 1 = 25*(65-1) + 1
CHUNK = 64
gx(cx,i) = cx*CHUNK + i
gy(cy,j) = cy*CHUNK + j
tex_row = (N-1) - gy          # north = texture row 0 (vs_textured)
tex_col = gx
pack_u16(c): R=c&0xFF, G=c>>8, B=0, A=255
FS count = round(R*255) + round(G*255)*256
inside = count >= CANOPY_MASS_ISO (2.0)
outline rim = fwidth(binary inside) when forestOutline LOD
```

## Stats

| Key | Ready value |
|-----|-------------|
| `forest_mode` | `"density"` |
| `forest_density_w/h` | `1601` |
| `forest_polygons` | `625` when fill LOD on |
| `forest_outline_segments` | `0` @ z=-2; `1` @ z=-1 probe |

## Retire

- Progressive `push_composite` mesh fill
- `upload_polygon_mesh` role 5 for forest
- CAMERA_GESTURE forest compose skip (params-only)

## Chrome

- A2: no Outliner label
- A3: continuous YouTube stems
- A4: `data-guide-toggle` click
