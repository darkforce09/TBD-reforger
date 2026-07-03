// T-090.1.2.5.2 — Enfusion `.topo` (World Editor "Export Map Data" / in-game 2D map
// geometry) decoder. Fully offline (pak VFS) — the exact-hydrology source that lets the
// water composite run as a one-button pipeline on any terrain whose paks we can read.
//
// Format (cracked this slice on worlds/Eden/Eden.topo, 10.3 MB):
//   header (0x18 bytes, u32 LE):
//     @0x00  0
//     @0x04  count of the largest type class (Eden: 394 = type-5 roads) — informational
//     @0x08  0
//     @0x0c  0
//     @0x10  section count (Eden: 6 — the same record set at 6 LOD/simplification levels)
//     @0x14  records per section (Eden: 888)
//   section 1 records start at 0x18; sections 2..N are each prefixed by u32 recordCount.
//   record:
//     u8   type
//     u32  vertexCount
//     vertexCount × (f32 x, f32 y)   — LITTLE-endian; x = world metres east;
//                                      y = NORTH-UP IMAGE metres (worldZ = worldSize − y)
//     u32  K
//     K × u32 attrs                  — sparse per-record attributes (rare; meaning unknown)
//
// Type semantics (Eden, validated by overlay statistics — see the .2.5.2 spike JSON):
//   0 = airfield/runway line work (the 5 records sit exactly on the NW-airfield
//       engine-flattened runways — this is also the proof that y is north-up image space)
//   1 = RIVER network — 12 segments whose bboxes chain end-to-end through the central
//       lake and terminate at the DEM coastline
//   2 = minor watercourses / streams (110 small segments, incl. the SE-massif channels)
//   3 = road class A (367), 5 = road class B (394) — grey SAP corridor overlay ≈ 1.0
//
// Only section 1 (full detail) is used for mask building; the LOD sections exist for the
// in-game map renderer.
//
// CLI: node scripts/map-assets/decode-topo.mjs [--terrain everon] [--stats]
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));

// Per-terrain config — add a row per terrain; zero Eden hardcodes below this table.
export const TOPO_TERRAINS = {
  everon: { topoPath: "worlds/Eden/Eden.topo", worldSizeM: 12800 },
  arland: { topoPath: "worlds/Arland/Arland.topo", worldSizeM: 4096 },
};

export const TOPO_TYPES = {
  AIRFIELD: 0,
  RIVER: 1,
  STREAM: 2,
  ROAD_A: 3,
  ROAD_B: 5,
};

const HEADER_LEN = 0x18;

async function getVfs() {
  const { getVfs } = await import(join(HERE, "decode-edds.mjs"));
  return getVfs();
}

/** Parse one record at `pos`; returns { rec, next } or null if the bytes don't fit. */
function parseRecord(buf, pos, worldSizeM) {
  if (pos + 5 > buf.length) return null;
  const type = buf[pos];
  const count = buf.readUInt32LE(pos + 1);
  const vStart = pos + 5;
  const vEnd = vStart + count * 8;
  if (count === 0 || count > 2_000_000 || vEnd + 4 > buf.length) return null;
  const inRange = (v) => v > -2000 && v < worldSizeM + 2000;
  if (!inRange(buf.readFloatLE(vStart)) || !inRange(buf.readFloatLE(vEnd - 8))) return null;
  const K = buf.readUInt32LE(vEnd);
  if (K > 64 || vEnd + 4 + K * 4 > buf.length) return null;
  const verts = new Float32Array(count * 2);
  for (let i = 0; i < count * 2; i++) verts[i] = buf.readFloatLE(vStart + i * 4);
  const attrs = [];
  for (let i = 0; i < K; i++) attrs.push(buf.readUInt32LE(vEnd + 4 + i * 4));
  return { rec: { type, verts, attrs }, next: vEnd + 4 + K * 4 };
}

/**
 * Decode a terrain's .topo. Returns { sections, records } where `records` is section 1
 * (full detail): [{ type, verts: Float32Array [x0,y0,x1,y1,…], attrs }] with y in
 * NORTH-UP IMAGE metres.
 */
export async function decodeTopo(terrain = "everon") {
  const cfg = TOPO_TERRAINS[terrain];
  if (!cfg) throw new Error(`no topo config for terrain "${terrain}" (add it to TOPO_TERRAINS)`);
  const vfs = await getVfs();
  const buf = vfs.readFile(cfg.topoPath);
  const sectionCount = buf.readUInt32LE(0x10);
  const perSection = buf.readUInt32LE(0x14);

  const sections = [];
  let pos = HEADER_LEN;
  for (let s = 0; s < sectionCount; s++) {
    let expect = perSection;
    if (s > 0) {
      expect = buf.readUInt32LE(pos);
      pos += 4;
    }
    const records = [];
    for (let r = 0; r < expect; r++) {
      const hit = parseRecord(buf, pos, cfg.worldSizeM);
      if (!hit) throw new Error(`topo parse broke: section ${s} record ${r} @0x${pos.toString(16)}`);
      records.push(hit.rec);
      pos = hit.next;
    }
    sections.push(records);
  }
  return { terrain, worldSizeM: cfg.worldSizeM, sectionCount, perSection, sections, records: sections[0], bytes: buf.length, consumed: pos };
}

// ── CLI ──────────────────────────────────────────────────────────────────────────────────
if (import.meta.url === `file://${process.argv[1]}`) {
  const terrain = process.argv.includes("--terrain")
    ? process.argv[process.argv.indexOf("--terrain") + 1]
    : "everon";
  const t = await decodeTopo(terrain);
  console.log(
    `[topo] ${terrain}: ${t.sectionCount} sections × ${t.perSection} records, consumed ${t.consumed}/${t.bytes} bytes`,
  );
  const hist = {};
  for (const r of t.records) {
    hist[r.type] = hist[r.type] || { n: 0, verts: 0 };
    hist[r.type].n++;
    hist[r.type].verts += r.verts.length / 2;
  }
  for (const [type, h] of Object.entries(hist)) {
    console.log(`[topo]   type ${type}: ${h.n} records, ${h.verts} vertices`);
  }
}
