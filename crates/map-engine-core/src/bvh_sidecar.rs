//! The binary `.bvh` sidecar codec — the on-disk half of [`super`] (`bvh.rs`), split out
//! per the SIZE gate. Format, versioning and the determinism contract are documented in the
//! parent module; every public item here is re-exported from `map_engine_core::bvh`, so
//! callers never name this module.

use super::{Bvh, BvhNode, MAX_PARSE_DEPTH, SurfaceKind, kind_of};

pub const SIDECAR_MAGIC: [u8; 4] = *b"TBVH";
/// The version the emitter writes. [`BvhSidecar::parse`] also accepts 1.
pub const SIDECAR_VERSION: u32 = 2;
/// Oldest version the parser accepts (no flags word, no kinds section).
pub const SIDECAR_VERSION_MIN: u32 = 1;
/// Header flags (version ≥ 2, header word 5): bit 0 = a kinds section follows `tri_order`.
pub const FLAG_KINDS: u32 = 1;
const HEADER_LEN: usize = 32;

/// Byte length of the kinds section for `ntris` triangles: one code each, padded to 4.
fn kinds_section_len(ntris: u64) -> u64 {
    ntris.div_ceil(4) * 4
}

/// The ONE authority for the determinism cast: quantize f64→f32 per component, then
/// build the BVH over [`lift_verts`] of the result — never over the raw f64s.
pub fn quantize_verts(verts: &[[f64; 3]]) -> Vec<[f32; 3]> {
    verts
        .iter()
        .map(|v| [v[0] as f32, v[1] as f32, v[2] as f32])
        .collect()
}

/// Exact widening — `lift_verts(quantize_verts(v))` round-trips every representable f32.
pub fn lift_verts(verts: &[[f32; 3]]) -> Vec<[f64; 3]> {
    verts
        .iter()
        .map(|v| [f64::from(v[0]), f64::from(v[1]), f64::from(v[2])])
        .collect()
}

/// Every way [`BvhSidecar::parse`] rejects bytes. The battery in `bvh_tests.rs` exercises
/// one case per variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BvhParseError {
    TooShort {
        len: usize,
    },
    BadMagic([u8; 4]),
    UnsupportedVersion(u32),
    NonZeroReserved,
    EmptyMesh,
    LengthMismatch {
        expected: u64,
        actual: u64,
    },
    NonFiniteVert {
        vert: u32,
    },
    TriIndexOutOfBounds {
        tri: u32,
    },
    NonFiniteNodeBound {
        node: u32,
    },
    LeafRangeOutOfBounds {
        node: u32,
    },
    ChildOutOfBounds {
        node: u32,
    },
    NodeRevisited {
        node: u32,
    },
    OrphanNodes {
        visited: u32,
        nnodes: u32,
    },
    TreeTooDeep,
    LeafCoverageMismatch {
        covered: u64,
        ntris: u32,
    },
    TriOrderOutOfBounds {
        slot: u32,
    },
    TriOrderNotPermutation {
        tri: u32,
    },
    /// A kinds byte this build cannot decode (see [`SurfaceKind::from_u8`]).
    UnknownKind {
        tri: u32,
        code: u8,
    },
    /// A non-zero byte in the kinds section's alignment padding.
    KindsPadding,
}

impl core::fmt::Display for BvhParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooShort { len } => write!(f, "sidecar too short: {len} bytes < 32-byte header"),
            Self::BadMagic(m) => write!(f, "bad magic {m:?} (want TBVH)"),
            Self::UnsupportedVersion(v) => write!(
                f,
                "unsupported sidecar version {v} (want {SIDECAR_VERSION_MIN}..={SIDECAR_VERSION})"
            ),
            Self::NonZeroReserved => write!(f, "reserved header words / flag bits are not zero"),
            Self::EmptyMesh => write!(f, "empty mesh (zero verts, tris, or nodes)"),
            Self::LengthMismatch { expected, actual } => {
                write!(f, "header implies {expected} bytes, file has {actual}")
            }
            Self::NonFiniteVert { vert } => write!(f, "vertex {vert} has a non-finite component"),
            Self::TriIndexOutOfBounds { tri } => {
                write!(f, "triangle {tri} indexes past the vertex table")
            }
            Self::NonFiniteNodeBound { node } => write!(f, "node {node} has a non-finite bound"),
            Self::LeafRangeOutOfBounds { node } => {
                write!(f, "leaf node {node} range exceeds tri_order")
            }
            Self::ChildOutOfBounds { node } => {
                write!(
                    f,
                    "internal node {node} children out of bounds or not forward-only"
                )
            }
            Self::NodeRevisited { node } => write!(f, "node {node} is reachable twice (diamond)"),
            Self::OrphanNodes { visited, nnodes } => {
                write!(
                    f,
                    "only {visited} of {nnodes} nodes reachable from the root"
                )
            }
            Self::TreeTooDeep => write!(f, "tree depth exceeds {MAX_PARSE_DEPTH}"),
            Self::LeafCoverageMismatch { covered, ntris } => {
                write!(
                    f,
                    "leaves do not tile tri_order exactly ({covered} of {ntris} slots)"
                )
            }
            Self::TriOrderOutOfBounds { slot } => {
                write!(f, "tri_order slot {slot} indexes past the triangle table")
            }
            Self::TriOrderNotPermutation { tri } => {
                write!(f, "tri_order repeats triangle {tri}")
            }
            Self::UnknownKind { tri, code } => {
                write!(
                    f,
                    "triangle {tri} has surface-kind code {code} (max {})",
                    SurfaceKind::MAX_CODE
                )
            }
            Self::KindsPadding => write!(f, "kinds section padding is not zero"),
        }
    }
}

impl std::error::Error for BvhParseError {}

/// A parsed sidecar, ready to raycast: verts are pre-lifted to f64 so
/// `sc.bvh.any_hit(&sc.verts, &sc.tris, ..)` is the whole call; `kinds` is parallel to
/// `tris` (all `Opaque` for a version-1 file).
#[derive(Debug)]
pub struct BvhSidecar {
    pub verts: Vec<[f64; 3]>,
    pub tris: Vec<[u32; 3]>,
    pub bvh: Bvh,
    pub kinds: Vec<SurfaceKind>,
}

impl BvhSidecar {
    /// An all-`Opaque` sidecar from parts already built over `verts` — the test scenes' and
    /// the pre-v2 callers' constructor. Panics when `bvh` was built over a different mesh.
    #[must_use]
    pub fn opaque(verts: Vec<[f64; 3]>, tris: Vec<[u32; 3]>, bvh: Bvh) -> Self {
        assert_eq!(
            bvh.tri_order.len(),
            tris.len(),
            "bvh was built over a different mesh"
        );
        let kinds = vec![SurfaceKind::Opaque; tris.len()];
        Self {
            verts,
            tris,
            bvh,
            kinds,
        }
    }

    /// Kind of triangle `tri` (`Opaque` past the table — never for a parsed file).
    #[must_use]
    pub fn kind(&self, tri: u32) -> SurfaceKind {
        kind_of(&self.kinds, tri)
    }

    /// `(opaque, glass, foliage)` triangle counts.
    #[must_use]
    pub fn kind_counts(&self) -> (usize, usize, usize) {
        let mut n = (0usize, 0usize, 0usize);
        for k in &self.kinds {
            match k {
                SurfaceKind::Opaque => n.0 += 1,
                SurfaceKind::Glass => n.1 += 1,
                SurfaceKind::Foliage => n.2 += 1,
            }
        }
        n
    }
}

fn u32le(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}
fn f32le(b: &[u8]) -> f32 {
    f32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

impl BvhSidecar {
    /// Full structural validation — after `parse` succeeds, [`Bvh::any_hit`] can neither
    /// panic nor hang no matter what the bytes were. See the module doc for the format.
    pub fn parse(bytes: &[u8]) -> Result<BvhSidecar, BvhParseError> {
        use BvhParseError as E;
        if bytes.len() < HEADER_LEN {
            return Err(E::TooShort { len: bytes.len() });
        }
        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if magic != SIDECAR_MAGIC {
            return Err(E::BadMagic(magic));
        }
        let version = u32le(&bytes[4..8]);
        if !(SIDECAR_VERSION_MIN..=SIDECAR_VERSION).contains(&version) {
            return Err(E::UnsupportedVersion(version));
        }
        let nverts = u32le(&bytes[8..12]);
        let ntris = u32le(&bytes[12..16]);
        let nnodes = u32le(&bytes[16..20]);
        // Version 1 had three reserved words; version 2 turned the first into flags.
        let flags = u32le(&bytes[20..24]);
        let known_flags = if version >= 2 { FLAG_KINDS } else { 0 };
        if flags & !known_flags != 0 || bytes[24..32].iter().any(|&b| b != 0) {
            return Err(E::NonZeroReserved);
        }
        let has_kinds = flags & FLAG_KINDS != 0;
        if nverts == 0 || ntris == 0 || nnodes == 0 {
            return Err(E::EmptyMesh);
        }
        // Entirely in u64 (max term < 2^37): only after this equality do usize section
        // offsets exist, which keeps 32-bit wasm free of overflow by construction. Exact
        // equality also rejects trailing garbage.
        let kinds_len = if has_kinds {
            kinds_section_len(u64::from(ntris))
        } else {
            0
        };
        let expected = HEADER_LEN as u64
            + 12 * u64::from(nverts)
            + 12 * u64::from(ntris)
            + 32 * u64::from(nnodes)
            + 4 * u64::from(ntris)
            + kinds_len;
        if bytes.len() as u64 != expected {
            return Err(E::LengthMismatch {
                expected,
                actual: bytes.len() as u64,
            });
        }

        let mut off = HEADER_LEN;
        let mut verts_f32: Vec<[f32; 3]> = Vec::with_capacity(nverts as usize);
        for (i, rec) in bytes[off..off + nverts as usize * 12]
            .chunks_exact(12)
            .enumerate()
        {
            let v = [f32le(&rec[0..4]), f32le(&rec[4..8]), f32le(&rec[8..12])];
            if v.iter().any(|c| !c.is_finite()) {
                return Err(E::NonFiniteVert { vert: i as u32 });
            }
            verts_f32.push(v);
        }
        off += nverts as usize * 12;

        let mut tris: Vec<[u32; 3]> = Vec::with_capacity(ntris as usize);
        for (i, rec) in bytes[off..off + ntris as usize * 12]
            .chunks_exact(12)
            .enumerate()
        {
            let t = [u32le(&rec[0..4]), u32le(&rec[4..8]), u32le(&rec[8..12])];
            if t.iter().any(|&idx| idx >= nverts) {
                return Err(E::TriIndexOutOfBounds { tri: i as u32 });
            }
            tris.push(t);
        }
        off += ntris as usize * 12;

        let mut nodes: Vec<BvhNode> = Vec::with_capacity(nnodes as usize);
        for (i, rec) in bytes[off..off + nnodes as usize * 32]
            .chunks_exact(32)
            .enumerate()
        {
            let min = [f32le(&rec[0..4]), f32le(&rec[4..8]), f32le(&rec[8..12])];
            let max = [
                f32le(&rec[12..16]),
                f32le(&rec[16..20]),
                f32le(&rec[20..24]),
            ];
            if min.iter().chain(max.iter()).any(|c| !c.is_finite()) {
                return Err(E::NonFiniteNodeBound { node: i as u32 });
            }
            let left_first = u32le(&rec[24..28]);
            let count = u32le(&rec[28..32]);
            if count > 0 {
                if u64::from(left_first) + u64::from(count) > u64::from(ntris) {
                    return Err(E::LeafRangeOutOfBounds { node: i as u32 });
                }
            } else {
                // Forward-only children: guarantees the reachability walk (and any_hit)
                // terminates; the adjacency pair must both exist.
                if u64::from(left_first) + 1 >= u64::from(nnodes) || left_first as usize <= i {
                    return Err(E::ChildOutOfBounds { node: i as u32 });
                }
            }
            nodes.push(BvhNode {
                min,
                max,
                left_first,
                count,
            });
        }
        off += nnodes as usize * 32;

        let mut tri_order: Vec<u32> = Vec::with_capacity(ntris as usize);
        let mut seen_tri = vec![false; ntris as usize];
        for (slot, rec) in bytes[off..off + ntris as usize * 4]
            .chunks_exact(4)
            .enumerate()
        {
            let t = u32le(rec);
            if t >= ntris {
                return Err(E::TriOrderOutOfBounds { slot: slot as u32 });
            }
            if seen_tri[t as usize] {
                return Err(E::TriOrderNotPermutation { tri: t });
            }
            seen_tri[t as usize] = true;
            tri_order.push(t);
        }
        off += ntris as usize * 4;

        let kinds: Vec<SurfaceKind> = if has_kinds {
            let mut kinds = Vec::with_capacity(ntris as usize);
            for (i, &code) in bytes[off..off + ntris as usize].iter().enumerate() {
                kinds.push(SurfaceKind::from_u8(code).ok_or(E::UnknownKind {
                    tri: i as u32,
                    code,
                })?);
            }
            if bytes[off + ntris as usize..].iter().any(|&b| b != 0) {
                return Err(E::KindsPadding);
            }
            kinds
        } else {
            vec![SurfaceKind::Opaque; ntris as usize]
        };

        // Structural walk: every node reachable exactly once, depth bounded, and the
        // leaves must tile tri_order exactly (no gaps, no overlaps).
        let mut visited = vec![false; nnodes as usize];
        let mut slot_covered = vec![false; ntris as usize];
        let mut covered = 0u64;
        let mut visit_count = 0u32;
        let mut stack: Vec<(u32, u32)> = vec![(0, 0)];
        while let Some((n, depth)) = stack.pop() {
            if depth > MAX_PARSE_DEPTH {
                return Err(E::TreeTooDeep);
            }
            let ni = n as usize;
            if visited[ni] {
                return Err(E::NodeRevisited { node: n });
            }
            visited[ni] = true;
            visit_count += 1;
            let node = &nodes[ni];
            if node.count > 0 {
                for k in node.left_first..node.left_first + node.count {
                    if slot_covered[k as usize] {
                        return Err(E::LeafCoverageMismatch { covered, ntris });
                    }
                    slot_covered[k as usize] = true;
                    covered += 1;
                }
            } else {
                stack.push((node.left_first, depth + 1));
                stack.push((node.left_first + 1, depth + 1));
            }
        }
        if visit_count != nnodes {
            return Err(E::OrphanNodes {
                visited: visit_count,
                nnodes,
            });
        }
        if covered != u64::from(ntris) {
            return Err(E::LeafCoverageMismatch { covered, ntris });
        }

        Ok(BvhSidecar {
            verts: lift_verts(&verts_f32),
            tris,
            bvh: Bvh { nodes, tri_order },
            kinds,
        })
    }
}

/// Serialize a version-2 sidecar. Infallible: malformed *bytes* only enter through
/// [`BvhSidecar::parse`]; a mismatched input here is programmer error. Contract: `bvh` was
/// built over `lift_verts(verts_f32)` and exactly these `tris`; `kinds` is parallel to `tris`.
pub fn emit_bytes(
    verts_f32: &[[f32; 3]],
    tris: &[[u32; 3]],
    kinds: &[SurfaceKind],
    bvh: &Bvh,
) -> Vec<u8> {
    assert!(!tris.is_empty(), "emit_bytes on empty mesh");
    assert_eq!(
        bvh.tri_order.len(),
        tris.len(),
        "bvh was built over a different mesh"
    );
    assert_eq!(
        kinds.len(),
        tris.len(),
        "kinds table is not parallel to the triangle table"
    );
    assert!(u32::try_from(verts_f32.len()).is_ok() && u32::try_from(bvh.nodes.len()).is_ok());
    let kinds_len = kinds_section_len(tris.len() as u64) as usize;
    let mut out = Vec::with_capacity(
        HEADER_LEN
            + verts_f32.len() * 12
            + tris.len() * 12
            + bvh.nodes.len() * 32
            + tris.len() * 4
            + kinds_len,
    );
    out.extend_from_slice(&SIDECAR_MAGIC);
    out.extend_from_slice(&SIDECAR_VERSION.to_le_bytes());
    out.extend_from_slice(&(verts_f32.len() as u32).to_le_bytes());
    out.extend_from_slice(&(tris.len() as u32).to_le_bytes());
    out.extend_from_slice(&(bvh.nodes.len() as u32).to_le_bytes());
    out.extend_from_slice(&FLAG_KINDS.to_le_bytes());
    out.extend_from_slice(&[0u8; 8]);
    for v in verts_f32 {
        for c in v {
            out.extend_from_slice(&c.to_le_bytes());
        }
    }
    for t in tris {
        for i in t {
            out.extend_from_slice(&i.to_le_bytes());
        }
    }
    for n in &bvh.nodes {
        for c in n.min {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for c in n.max {
            out.extend_from_slice(&c.to_le_bytes());
        }
        out.extend_from_slice(&n.left_first.to_le_bytes());
        out.extend_from_slice(&n.count.to_le_bytes());
    }
    for t in &bvh.tri_order {
        out.extend_from_slice(&t.to_le_bytes());
    }
    for k in kinds {
        out.push(k.code());
    }
    out.resize(out.len() + (kinds_len - kinds.len()), 0);
    out
}
