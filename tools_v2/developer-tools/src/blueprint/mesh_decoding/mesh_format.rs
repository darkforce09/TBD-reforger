//! Minimal Enfusion XOB9 model reader — just enough to pull triangle geometry out of a
//! Reforger `.xob` for the mesh→voxel-dump generator. Positions, indices, packed vertex
//! normals, and per-submesh material indices; no UVs/tangents/skinning.
//!
//! Format knowledge: the community-reverse-engineered XOB9 layout as implemented by
//! Cyrex0/Enfusion-Unpacker (`src/formats/xob_parser.cpp`, itself following
//! `xob_to_obj.py` from enfusion_toolkit). This is an independent Rust implementation of
//! the documented byte layout; no code was copied.
//!
//! Layout (IFF/FORM container, chunk sizes big-endian):
//! - `FORM <be32 size> XOB9HEAD` file header.
//! - `HEAD` chunk: material resource strings (`{16-hex GUID}path`), then one 116-byte
//!   `LZO4` descriptor per (LOD tier × submesh): quality_tier @+0x04, compressed size
//!   @+0x14, decompressed size @+0x1C, format_flags @+0x20 (upper-byte bit 4 → 16-byte
//!   position stride, else 12), bbox min/max (f32×3) @+0x24/+0x30, triangle_count u16
//!   @+0x4C, unique_verts u16 @+0x4E, submesh_idx u16 @+0x52.
//! - `LODS` chunk: LZ4 block stream — `<le32 header>` per block (size = header & 0x7FFFFFFF,
//!   0 terminates, ≤ 0x20000), raw-LZ4 payload; match offsets reach back across block
//!   boundaries (≤ 64 KiB window), so decoding into one contiguous buffer is exact.
//! - Decompressed stream holds one region per descriptor in REVERSE order (descriptor 0's
//!   region is at the END). Region layout: index array (tri×3 u16) → a second, equal-sized
//!   index array (skipped) → positions (unique_verts × stride, xyz f32 LE) → packed normals
//!   (4 bytes/vertex, i8 xyz ÷ 127).

use anyhow::{Context, Result, bail};

pub struct XobMesh {
    pub verts: Vec<[f64; 3]>,
    /// Per-vertex unit-ish normals from the packed i8 stream (len == verts).
    pub vert_normals: Vec<[f64; 3]>,
    pub tris: Vec<[u32; 3]>,
    /// Per-triangle submesh index (parallel to `tris`) — indexes `materials` when in range.
    pub tri_submesh: Vec<u16>,
    /// Material resource paths in HEAD order.
    pub materials: Vec<String>,
    /// Every descriptor found, for diagnostics (`--stats`).
    pub descriptors: Vec<LodDescriptor>,
    /// The quality tier actually loaded.
    pub tier: u32,
    /// Per-triangle material index in the HEAD name space (COLL trimesh subranges — resolve
    /// through `xob_nodes::XobNodes::name`); `u32::MAX` when the triangle's record carries no
    /// subrange table (box colliders) or the mesh is a visual LOD.
    pub tri_material: Vec<u32>,
    /// COLL collider records in file order (empty for a visual-LOD mesh).
    pub records: Vec<CollRecord>,
}

/// One COLL collider record's header facts (T-090.11.2). `layer_idx` names the layer
/// preset (`Building`, `FireView`, `Glass`, `Foliage`, …) and `mesh_idx` the collider mesh
/// (`UTM_BD_*`), both in the HEAD name space.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollRecord {
    pub shape: u8,
    pub layer_idx: u16,
    pub mesh_idx: u16,
    pub first_mat_idx: u16,
    /// Range of this record's triangles in `tris` / `tri_submesh` / `tri_material`.
    pub tri_start: usize,
    pub tri_count: usize,
}

#[derive(Debug, Clone)]
pub struct LodDescriptor {
    pub quality_tier: u32,
    pub decomp_size: u32,
    pub format_flags: u32,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    pub triangle_count: u16,
    pub unique_verts: u16,
    pub submesh_idx: u16,
    pub position_stride: usize,
}

struct Submesh {
    verts: Vec<[f64; 3]>,
    normals: Vec<[f64; 3]>,
    tris: Vec<[u32; 3]>,
}

/* ───────────────────────────── COLL: the collision chunk ─────────────────────────────
 *
 * Reverse-engineered in this repo (2026-08-29) from CardboardBox_01.xob (one 76-byte box
 * record, half-extents match the visual AABB) and FarmHouse_E_1L01.xob (two trimesh
 * records, 62,332 bytes consumed byte-exact; vertex AABB reproduces the engine dump's
 * bounds to the centimeter, chimney included). No public parser existed for this chunk.
 *
 * COLL payload = sequence of collider records, each:
 *   u8 shape_type · u8 0xFF · u16 layer_idx (layer-preset NAME in the HEAD name space:
 *   `Building`, `FireView`, `Glass`, `Foliage`, … — T-090.11.2) ·
 *   rotation 3×3 f32 (row-major) · center 3×f32 · f32 0 · u16 pair (mesh name idx,
 *   first material idx) · u32 0 ·
 *   shape payload:
 *     type 3 (box):     half-extents 3×f32
 *     type 4 (convex):  u16 nverts · u16 nfaces · u16 nedges · u16 nidx · verts nverts×3×f32
 *                       · face/edge tables of 2·nidx·2 + 4·nedges + 4·nfaces bytes (undecoded;
 *                       the hull is rebuilt from the vertices, see `hull.rs`) — a conifer's
 *                       `UCX_C` trunk (10 verts, 208 table bytes) and `UCX_Fol` canopy
 *                       (19 verts, 748 bytes) pin the stride
 *     type 5 (trimesh): u16 nverts · u16 ntris · verts nverts×3×f32 · indices ntris×3×u16
 *                       (no subrange table — the header's first material covers every tri;
 *                       a conifer's `UTM_F` fire geometry, 315 verts / 619 tris)
 *     type 6 (trimesh): u16 nverts · u16 ntris · u32 nsub · nsub×(u16 material, u16 last_tri)
 *                       · verts nverts×3×f32 · indices ntris×3×u16
 * The subrange table is the per-triangle game material: each entry names a
 * `Common/Materials/Game/<stem>.gamemat` (same name space as the node records, see
 * `xob_nodes.rs`) for the run of triangles ending at `last_tri` (inclusive; runs are back
 * to back from triangle 0) — the farmhouse's record 0 carries nine (tiles_ceramic … brick)
 * ending at 8, 18, 78, …, 1128 for its 1129 triangles. `VOLM` stays unparsed: the layer
 * preset lives in the record header.
 */

#[cfg(test)]
#[path = "../tests/xob_tests.rs"]
pub(crate) mod tests;

#[path = "mesh_format/u16le.rs"]
mod u16le;
pub use u16le::aabb;
use u16le::f32le;
use u16le::find_chunk;
pub use u16le::has_coll;
use u16le::mat_apply;
pub use u16le::parse_xob;
use u16le::u16le;
use u16le::u32le;
use u16le::vec3le;

#[path = "mesh_format/parse_coll.rs"]
mod parse_coll;
pub use parse_coll::parse_coll;

#[cfg(test)]
pub(crate) use u16le::lz4_decompress_chained;
