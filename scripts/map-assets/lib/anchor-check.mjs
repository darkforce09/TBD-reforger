// T-090.3.1 — P1-4 anchor check (shared, dep-free).
//
// One function, two data sources (non-circularity rule): verify-phase.mjs feeds it the K=32
// deterministic sample from the STAGED raw JSONL + the COMMITTED chunk artifacts;
// verify-map-object-golden.mjs feeds it the synthetic golden fixture
// (packages/tbd-schema/golden/map-objects/phased/P1-anchor-fixture.json). The remap + partition
// formulas here are intentionally re-implemented — they must AGREE with build-world-objects.mjs
// without importing from it, so a chunking/serialization bug in the builder cannot self-certify.
//
// Conventions (plan decisions 2 + 4):
//   raw (engine):  x = east, y = altitude, z = north, headingDeg = GetAngles()[1]
//   map:           x = engine.x, y = engine.z, z = engine.y, rotationDeg = headingDeg
//   partition:     cell = clamp(floor(coord / chunkSizeM), 0, cells-1)
//                  (half-open interior, closed last row/col — coord == worldSizeM lands in the
//                  last cell, never dropped, never doubled)
//   chunk rows:    all-number 5-tuple [prefabId, x, y, z, rotationDeg]

/** Engine-space raw row -> map-space point. */
export function remapRawToMap(raw) {
  return { x: raw.x, y: raw.z, z: raw.y, rotationDeg: raw.headingDeg ?? 0 };
}

/** Grid cell index for one map coordinate (clamped floor — see header). */
export function cellOf(coord, chunkSizeM, worldSizeM) {
  const cells = Math.max(1, Math.round(worldSizeM / chunkSizeM));
  return Math.min(Math.max(Math.floor(coord / chunkSizeM), 0), cells - 1);
}

export function chunkKey(cx, cy) {
  return `${cx}_${cy}`;
}

/** Read prefabId / map x / map y off a chunk row (5-tuple or expanded object). */
function rowPrefabId(row) {
  return Array.isArray(row) ? row[0] : row.prefabId;
}
function rowX(row) {
  return Array.isArray(row) ? row[1] : row.x;
}
function rowY(row) {
  return Array.isArray(row) ? row[2] : row.y;
}

/**
 * P1-4: every raw building anchor must find, in its OWN home chunk, a kind=building instance
 * within toleranceM whose prefab resolves to the anchor's resourceName. A nearer building match
 * in one of the 8 neighbor chunks while the home chunk misses = partition drift -> error.
 *
 * @param {object} opts
 * @param {Array}  opts.anchors    raw rows ({resourceName, x, y, z, headingDeg})
 * @param {Array}  opts.prefabs    prefab rows ({prefabId, resourceName, kind})
 * @param {Function} opts.getChunk (cx, cy) -> { instances: [...] } | null
 * @param {number} opts.chunkSizeM
 * @param {number} opts.worldSizeM
 * @param {number} [opts.toleranceM=2]
 * @returns {string[]} errors (empty = PASS)
 */
export function checkAnchors({ anchors, prefabs, getChunk, chunkSizeM, worldSizeM, toleranceM = 2 }) {
  const errors = [];
  const prefabById = new Map(prefabs.map((p) => [p.prefabId, p]));
  const buildingIds = new Set(prefabs.filter((p) => p.kind === "building").map((p) => p.prefabId));

  // Returns the nearest building instance AND (T-090.3.3) the nearest one whose prefab matches
  // `matchRn` — co-located instances (stacked containers share an x/y) tie at 0 m, so "nearest"
  // alone can land on the twin and false-fail the resourceName identity.
  const nearestBuilding = (chunk, mx, my, matchRn) => {
    let best = null;
    let bestMatch = null;
    for (const row of chunk?.instances ?? []) {
      const pid = rowPrefabId(row);
      if (!buildingIds.has(pid)) continue;
      const dx = rowX(row) - mx;
      const dy = rowY(row) - my;
      const dist = Math.hypot(dx, dy);
      if (!best || dist < best.dist) best = { dist, prefabId: pid };
      if (matchRn && prefabById.get(pid)?.resourceName === matchRn && (!bestMatch || dist < bestMatch.dist)) {
        bestMatch = { dist, prefabId: pid };
      }
    }
    return { best, bestMatch };
  };

  for (const anchor of anchors) {
    const m = remapRawToMap(anchor);
    const cx = cellOf(m.x, chunkSizeM, worldSizeM);
    const cy = cellOf(m.y, chunkSizeM, worldSizeM);
    const label = `anchor ${anchor.resourceName} @ map(${m.x},${m.y}) -> chunk ${chunkKey(cx, cy)}`;

    const { best: home, bestMatch } = nearestBuilding(getChunk(cx, cy), m.x, m.y, anchor.resourceName);
    const homeOk = bestMatch && bestMatch.dist <= toleranceM;

    let neighborBest = null;
    for (let dx = -1; dx <= 1; dx++) {
      for (let dy = -1; dy <= 1; dy++) {
        if (dx === 0 && dy === 0) continue;
        const ncx = cx + dx;
        const ncy = cy + dy;
        if (ncx < 0 || ncy < 0) continue;
        const hit = nearestBuilding(getChunk(ncx, ncy), m.x, m.y).best;
        if (hit && (!neighborBest || hit.dist < neighborBest.dist)) neighborBest = hit;
      }
    }

    if (!homeOk) {
      const homeDesc = home
        ? `nearest home building ${home.dist.toFixed(3)} m (prefab ${home.prefabId})`
        : "no building instance in home chunk";
      errors.push(`${label}: FAIL — ${homeDesc}, tolerance ${toleranceM} m`);
      continue;
    }
    if (neighborBest && neighborBest.dist < home.dist) {
      errors.push(
        `${label}: partition drift — neighbor chunk holds a nearer building (${neighborBest.dist.toFixed(3)} m < home ${home.dist.toFixed(3)} m)`,
      );
    }
  }
  return errors;
}
