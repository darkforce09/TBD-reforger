// T-090.1 — The ONE place tile Y-axis inversion happens. Do not flip y anywhere else.
//
// Our exported pyramids are stored XYZ on disk: tile y=0 is the NORTHERNMOST row.
// The editor renders in COORDINATE_SYSTEM.CARTESIAN with flipY:false (origin bottom-left,
// +Y north), so when we walk tiles for the viewport we index them south-first (y=0 = the
// SOUTHERN world edge, matching +Y north). Those two conventions are vertically opposite,
// so the south-first index must be converted to the on-disk XYZ row before building the URL:
//
//     xyzRow = 2**z - 1 - y
//
// A past refactor dropped this and shipped an upside-down basemap — keep the flip here, in one
// helper, and never interpolate a raw `y` into a tile URL template elsewhere.

/** Number of tiles along one axis at pyramid level `z`. */
export function tilesPerAxis(z: number): number {
  return 2 ** z
}

/**
 * Build a tile URL from a `{z}/{x}/{y}` template, converting the caller's south-first
 * `y` index to the on-disk XYZ (north-first) row via `xyzRow = 2**z - 1 - y`.
 */
export function tileUrl(template: string, z: number, x: number, y: number): string {
  const xyzRow = 2 ** z - 1 - y
  return template.replace('{z}', String(z)).replace('{x}', String(x)).replace('{y}', String(xyzRow))
}
