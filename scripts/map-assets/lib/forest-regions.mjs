#!/usr/bin/env node
// T-090.3.2 — Path B derived-hull forest regions (t090_8_forest_vegetation_regions.md §Export).
// Pure + deterministic (F6: same instances -> same rings). Shared by build-world-objects.mjs
// (derive + write), verify-phase.mjs (re-derive from committed chunks + compare) and the
// schema-package golden gate S14.
//
// Pipeline (all constants locked here, echoed into the regions artifact + ops log):
//   1. bin kind=tree instances into the worldSize/32 cell grid (same clamp(floor(coord/32))
//      partition rule as chunks — lib/anchor-check.mjs cellOf)
//   2. dense = cell count >= DENSITY_THRESHOLD; 8-connected components; components with fewer
//      than MIN_COMPONENT_CELLS cells are discarded (their trees stay unassigned)
//   3. boundary-trace each component's cell union into rectilinear closed rings (grid-edge walk,
//      interior-on-left => outer ring CCW, holes CW). The trace IS the deterministic concave
//      hull — no alpha-shape dependency.
//   4. aggregate per region: treeCount (exact bin sum), dominantSpeciesClass (majority of member
//      trees' class; top share < DOMINANT_SHARE -> "mixed"), areaHa, densityPerHa.
// Ship identity (F2, holds by construction): sum(regions.treeCount) + unassignedTrees
//   == total tree instances fed in.

import { cellOf } from "./anchor-check.mjs";

export const REGION_CELL_M = 32;
export const DENSITY_THRESHOLD = 2; // trees per 32 m cell (~19.5/ha) — forest floor
export const MIN_COMPONENT_CELLS = 8; // ~0.82 ha — smaller stands stay unassigned
export const DOMINANT_SHARE = 0.66; // top species share below this -> "mixed"

const round4 = (v) => Math.round(v * 10000) / 10000;

/**
 * Derive forest regions from tree instances.
 * @param {Array<{x: number, y: number, class: string}>} trees rounded, in-bounds tree rows
 * @param {{ worldSizeM: number, terrainId: string, cellM?: number, densityThreshold?: number,
 *           minComponentCells?: number, dominantShare?: number }} opts
 * @returns {{ regions: object[], unassignedTrees: number, binnedTreeCount: number,
 *            denseCellCount: number, componentCount: number, keptComponentCount: number,
 *            params: object }}
 */
export function deriveForestRegions(trees, opts) {
  const {
    worldSizeM,
    terrainId,
    cellM = REGION_CELL_M,
    densityThreshold = DENSITY_THRESHOLD,
    minComponentCells = MIN_COMPONENT_CELLS,
    dominantShare = DOMINANT_SHARE,
  } = opts;
  const cells = Math.round(worldSizeM / cellM);

  // 1. bin
  const counts = new Uint32Array(cells * cells);
  for (const t of trees) {
    counts[cellOf(t.y, cellM, worldSizeM) * cells + cellOf(t.x, cellM, worldSizeM)]++;
  }
  let denseCellCount = 0;
  const dense = new Uint8Array(cells * cells);
  for (let k = 0; k < counts.length; k++) {
    if (counts[k] >= densityThreshold) {
      dense[k] = 1;
      denseCellCount++;
    }
  }

  // 2. 8-connected components over dense cells (deterministic row-major seed order)
  const comp = new Int32Array(cells * cells).fill(-1);
  const components = []; // { cells: number[], minCx, minCy, firstIdx }
  const stack = [];
  for (let seed = 0; seed < dense.length; seed++) {
    if (!dense[seed] || comp[seed] !== -1) continue;
    const id = components.length;
    const member = [];
    let minCx = Infinity;
    let minCy = Infinity;
    stack.push(seed);
    comp[seed] = id;
    while (stack.length) {
      const k = stack.pop();
      member.push(k);
      const cy = Math.floor(k / cells);
      const cx = k - cy * cells;
      if (cx < minCx) minCx = cx;
      if (cy < minCy) minCy = cy;
      for (let dy = -1; dy <= 1; dy++) {
        for (let dx = -1; dx <= 1; dx++) {
          if (dx === 0 && dy === 0) continue;
          const nx = cx + dx;
          const ny = cy + dy;
          if (nx < 0 || nx >= cells || ny < 0 || ny >= cells) continue;
          const nk = ny * cells + nx;
          if (dense[nk] && comp[nk] === -1) {
            comp[nk] = id;
            stack.push(nk);
          }
        }
      }
    }
    member.sort((a, b) => a - b);
    components.push({ cells: member, minCx, minCy, firstIdx: member[0] });
  }

  // 3. keep + order components; remap kept ids onto the comp grid (discarded -> -1)
  const kept = components
    .map((c, i) => ({ ...c, origId: i }))
    .filter((c) => c.cells.length >= minComponentCells)
    .sort((a, b) => a.minCy - b.minCy || a.minCx - b.minCx || a.firstIdx - b.firstIdx);
  const regionIdByOrig = new Map(kept.map((c, i) => [c.origId, i]));
  const regionOfCell = new Int32Array(cells * cells).fill(-1);
  for (const [k, id] of comp.entries()) {
    if (id !== -1 && regionIdByOrig.has(id)) regionOfCell[k] = regionIdByOrig.get(id);
  }

  // 4. species tally + exact tree counts per region (second pass over trees)
  const speciesTally = kept.map(() => new Map());
  const treeCounts = new Array(kept.length).fill(0);
  let unassignedTrees = 0;
  for (const t of trees) {
    const k = cellOf(t.y, cellM, worldSizeM) * cells + cellOf(t.x, cellM, worldSizeM);
    const r = regionOfCell[k];
    if (r === -1) {
      unassignedTrees++;
      continue;
    }
    treeCounts[r]++;
    const m = speciesTally[r];
    m.set(t.class, (m.get(t.class) ?? 0) + 1);
  }

  // 5. rings + aggregates
  const regions = kept.map((c, i) => {
    const memberSet = new Set(c.cells);
    const rings = traceRings(memberSet, cells).map((ring) =>
      ring.map(([gx, gy]) => [gx * cellM, gy * cellM]),
    );
    const tally = [...speciesTally[i].entries()].sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1));
    const total = treeCounts[i];
    let dominant = "unknown";
    if (tally.length && total > 0) {
      dominant = tally[0][1] / total >= dominantShare ? tally[0][0] : "mixed";
    }
    const areaHa = round4((c.cells.length * cellM * cellM) / 10000);
    return {
      id: `forest-${terrainId}-${String(i + 1).padStart(3, "0")}`,
      kind: "forest",
      polygon: rings,
      treeCount: total,
      dominantSpeciesClass: dominant,
      densityPerHa: areaHa > 0 ? round4(total / areaHa) : 0,
      areaHa,
      coverType: "soft",
      source: "derived-hull",
    };
  });

  return {
    regions,
    unassignedTrees,
    binnedTreeCount: trees.length,
    denseCellCount,
    componentCount: components.length,
    keptComponentCount: kept.length,
    params: { cellM, densityThreshold, minComponentCells, dominantShare },
  };
}

// ---- rectilinear boundary trace ------------------------------------------------------------------
// Directed boundary edges with the region interior on the LEFT (y-up world):
//   bottom side -> (x,y)->(x+1,y)     right side -> (x+1,y)->(x+1,y+1)
//   top side    -> (x+1,y+1)->(x,y+1) left side  -> (x,y+1)->(x,y)
// Stitch into loops; at a checkerboard corner (two outgoing candidates) take the sharpest LEFT
// turn relative to the incoming direction — keeps rings simple + deterministic. Collinear runs
// collapse to turn vertices; each ring is rotated to start at its lexicographically smallest
// (gy, gx) vertex and closed (first vertex repeated last). Outer ring (largest |shoelace|) first,
// holes after, sorted by (minGy, minGx).
function traceRings(memberSet, cells) {
  const edges = new Map(); // startKey -> edge[] (sorted later via deterministic insert order)
  const addEdge = (x0, y0, x1, y1) => {
    const key = `${x0}_${y0}`;
    let list = edges.get(key);
    if (!list) edges.set(key, (list = []));
    list.push({ x0, y0, x1, y1, used: false });
  };
  const sorted = [...memberSet].sort((a, b) => a - b);
  for (const k of sorted) {
    const y = Math.floor(k / cells);
    const x = k - y * cells;
    if (y === 0 || !memberSet.has(k - cells)) addEdge(x, y, x + 1, y); // bottom
    if (x === cells - 1 || !memberSet.has(k + 1)) addEdge(x + 1, y, x + 1, y + 1); // right
    if (y === cells - 1 || !memberSet.has(k + cells)) addEdge(x + 1, y + 1, x, y + 1); // top
    if (x === 0 || !memberSet.has(k - 1)) addEdge(x, y + 1, x, y); // left
  }

  const allStarts = [...edges.keys()].sort((a, b) => {
    const [ax, ay] = a.split("_").map(Number);
    const [bx, by] = b.split("_").map(Number);
    return ay - by || ax - bx;
  });

  const rings = [];
  for (const startKey of allStarts) {
    for (const first of edges.get(startKey)) {
      if (first.used) continue;
      const verts = [[first.x0, first.y0]];
      let cur = first;
      cur.used = true;
      let guard = 0;
      const maxSteps = cells * cells * 4;
      while (true) {
        if (++guard > maxSteps) throw new Error("traceRings: non-terminating walk (bug)");
        const dx = cur.x1 - cur.x0;
        const dy = cur.y1 - cur.y0;
        // collapse collinear: only push a vertex when direction changes
        const candidates = (edges.get(`${cur.x1}_${cur.y1}`) ?? []).filter((e) => !e.used);
        if (cur.x1 === first.x0 && cur.y1 === first.y0) {
          break; // loop closed
        }
        if (candidates.length === 0) throw new Error("traceRings: dead-end walk (bug)");
        // turn priority: left, straight, right (relative to incoming (dx,dy); left = (-dy,dx))
        const dirRank = (e) => {
          const ex = e.x1 - e.x0;
          const ey = e.y1 - e.y0;
          if (ex === -dy && ey === dx) return 0; // left
          if (ex === dx && ey === dy) return 1; // straight
          if (ex === dy && ey === -dx) return 2; // right
          return 3; // back (impossible by construction)
        };
        candidates.sort((a, b) => dirRank(a) - dirRank(b));
        const next = candidates[0];
        const ndx = next.x1 - next.x0;
        const ndy = next.y1 - next.y0;
        if (ndx !== dx || ndy !== dy) verts.push([next.x0, next.y0]);
        next.used = true;
        cur = next;
      }
      // seam collinearity: if the closing edge direction equals the first edge direction, the
      // start vertex is not a turn — drop it.
      if (verts.length >= 3) {
        const [x0, y0] = verts[0];
        const [x1, y1] = verts[1];
        const [xl, yl] = verts[verts.length - 1];
        const firstDir = [Math.sign(x1 - x0), Math.sign(y1 - y0)];
        const closeDir = [Math.sign(x0 - xl), Math.sign(y0 - yl)];
        if (firstDir[0] === closeDir[0] && firstDir[1] === closeDir[1]) verts.shift();
      }
      rings.push(canonicalizeRing(verts));
    }
  }

  rings.sort((a, b) => Math.abs(shoelace(b)) - Math.abs(shoelace(a)) || minKey(a) - minKey(b));
  return rings;
}

function canonicalizeRing(verts) {
  let best = 0;
  for (let i = 1; i < verts.length; i++) {
    const [bx, by] = verts[best];
    const [x, y] = verts[i];
    if (y < by || (y === by && x < bx)) best = i;
  }
  const rotated = [...verts.slice(best), ...verts.slice(0, best)];
  rotated.push([...rotated[0]]); // close
  return rotated;
}

function shoelace(ring) {
  let s = 0;
  for (let i = 0; i < ring.length - 1; i++) {
    s += ring[i][0] * ring[i + 1][1] - ring[i + 1][0] * ring[i][1];
  }
  return s / 2;
}

function minKey(ring) {
  let min = Infinity;
  for (const [x, y] of ring) min = Math.min(min, y * 1e6 + x);
  return min;
}
