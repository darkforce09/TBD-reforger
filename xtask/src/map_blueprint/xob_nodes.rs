//! XOB HEAD node table — the scene hierarchy of a Reforger `.xob` (T-090.11.2): the
//! `Scene_Root`, every `socket_*` empty a child prefab attaches to, and the string table
//! the COLL chunk's material subranges index. Reverse-engineered in this repo (2026-09-03)
//! from `FarmHouse_E_1L01.xob`; no public parser existed.
//!
//! Layout inside the HEAD payload:
//! - `+48`: a NUL-terminated string table — visual material names + `{GUID}*.emat` paths,
//!   then `Scene_Root`, the socket names, mesh-group names, the collider mesh names
//!   (`UTM_BD_*`, `UTM_FV_*`), `{GUID}*.gamemat` paths, layer-preset names (`FireView`, …),
//!   portal names. Node / material indices count in a space that starts a few entries
//!   after the raw table start (the material block); the base is recovered as
//!   `raw_index(Scene_Root) − root_record.name_idx`.
//! - Right before the first `LZO4` descriptor: the node records, 36 bytes each, root first:
//!   `u32 name_idx · f32 pos×3 · f32 quat×4 (x, y, z, w) · u16 next_sibling · u16 first_child`
//!   (`0xFFFF` = none). Positions and rotations are parent-relative; sockets sit directly
//!   under the root in every file seen so far, and the walk below composes the chain anyway.
//!
//! The COLL chunk (see `xob.rs`) refers to the same index space: each collider record's
//! leading `u16` is its layer-preset name (`Building`, `FireView`, `Glass`, `Foliage`, …) and
//! each trimesh subrange's `u16` is the `.gamemat` for that run of triangles.

use anyhow::{Context, Result, bail};
use map_engine_core::geometry::rigid::Rigid;

const RECORD_LEN: usize = 36;
const NONE: u16 = 0xFFFF;

#[derive(Debug, Clone, PartialEq)]
pub struct XobNode {
    pub name_idx: u32,
    pub pos: [f32; 3],
    /// Rotation quaternion `[x, y, z, w]`.
    pub quat: [f32; 4],
    pub next_sibling: u16,
    pub first_child: u16,
}

/// The decoded hierarchy + string table.
#[derive(Debug, Clone, PartialEq)]
pub struct XobNodes {
    /// The HEAD string table from its first real name on — index `i` here IS node /
    /// material name index `i`.
    pub strings: Vec<String>,
    /// Byte offset of the string table inside the HEAD payload (diagnostics).
    pub name_base: usize,
    /// Records in file order (root first).
    pub nodes: Vec<XobNode>,
    /// Parent record index per node (`None` for the root).
    pub parent: Vec<Option<usize>>,
    /// Byte offset of the record table in the file (diagnostics).
    pub table_offset: usize,
}

/// A socket resolved to the model's root frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Socket {
    pub name: String,
    pub node: usize,
    /// Model-root → socket rigid transform.
    pub local: Rigid,
}

fn u16le(p: &[u8]) -> u16 {
    u16::from_le_bytes([p[0], p[1]])
}
fn u32le(p: &[u8]) -> u32 {
    u32::from_le_bytes([p[0], p[1], p[2], p[3]])
}
fn u32be(p: &[u8]) -> u32 {
    u32::from_be_bytes([p[0], p[1], p[2], p[3]])
}
fn f32le(p: &[u8]) -> f32 {
    f32::from_le_bytes([p[0], p[1], p[2], p[3]])
}

/// Byte-scan for an IFF chunk id from offset 12 (mirrors `xob.rs::find_chunk`, returning
/// the payload's FILE offset as well so record offsets can be reported absolutely).
fn find_chunk_at(data: &[u8], id: &[u8; 4]) -> Option<(usize, usize)> {
    let mut pos = 12usize;
    while pos + 8 <= data.len() {
        if &data[pos..pos + 4] == id {
            let size = u32be(&data[pos + 4..pos + 8]) as usize;
            if size > 0 && size < 100_000_000 && pos + 8 + size <= data.len() {
                return Some((pos + 8, size));
            }
        }
        pos += 1;
    }
    None
}

fn read_record(b: &[u8]) -> XobNode {
    XobNode {
        name_idx: u32le(&b[0..4]),
        pos: [f32le(&b[4..8]), f32le(&b[8..12]), f32le(&b[12..16])],
        quat: [
            f32le(&b[16..20]),
            f32le(&b[20..24]),
            f32le(&b[24..28]),
            f32le(&b[28..32]),
        ],
        next_sibling: u16le(&b[32..34]),
        first_child: u16le(&b[34..36]),
    }
}

fn plausible(n: &XobNode) -> bool {
    let q = n.quat;
    let norm = (f64::from(q[0]).powi(2)
        + f64::from(q[1]).powi(2)
        + f64::from(q[2]).powi(2)
        + f64::from(q[3]).powi(2))
    .sqrt();
    n.name_idx < 100_000
        && n.pos.iter().all(|c| c.is_finite() && c.abs() < 1.0e5)
        && q.iter().all(|c| c.is_finite())
        && (0.9..=1.1).contains(&norm)
        && (n.next_sibling == NONE || n.next_sibling < 65_000)
        && (n.first_child == NONE || n.first_child < 65_000)
}

/// Split the HEAD string table. The payload starts with a handful of header words (a few
/// `u32`s and the bounds — three words in the door leaf, none in the farmhouse), so the
/// table begins at the first run of ≥ 2 printable bytes that ends in a NUL; stray single
/// printable bytes inside the header words are skipped. Index 0 of the returned table IS
/// index 0 of the node / material name space (the first visual material name). The
/// returned offset is the table's byte position in the payload (diagnostics).
fn string_table(head: &[u8]) -> (Vec<String>, usize) {
    let printable = |c: u8| c.is_ascii_graphic() || c == b' ';
    let mut start = 32usize.min(head.len());
    loop {
        while start < head.len() && !printable(head[start]) {
            start += 1;
        }
        if start >= head.len() {
            return (Vec::new(), start);
        }
        let mut end = start;
        while end < head.len() && printable(head[end]) {
            end += 1;
        }
        if end - start >= 2 && end < head.len() && head[end] == 0 {
            break;
        }
        start = end;
    }
    let mut out = Vec::new();
    let mut cur = Vec::new();
    let mut i = start;
    while i < head.len() {
        let c = head[i];
        if c == 0 {
            out.push(String::from_utf8_lossy(&cur).into_owned());
            cur.clear();
        } else if printable(c) {
            cur.push(c);
        } else {
            break;
        }
        i += 1;
    }
    if !cur.is_empty() {
        out.push(String::from_utf8_lossy(&cur).into_owned());
    }
    (out, start)
}

/// Decode the string table and (when the model has a hierarchy) the node table of an XOB9
/// file. Models without a `Scene_Root` (door leaves, glass panes, trees, props) return an
/// empty `nodes` — their COLL records still name layers and materials through `strings`.
pub fn parse_head_nodes(data: &[u8]) -> Result<XobNodes> {
    if data.len() < 12 || &data[0..4] != b"FORM" || &data[8..11] != b"XOB" {
        bail!("not a FORM/XOB9 file (magic mismatch)");
    }
    let (head_off, head_len) = find_chunk_at(data, b"HEAD").context("XOB: no HEAD chunk")?;
    let head = &data[head_off..head_off + head_len];
    let (strings, table_start) = string_table(head);
    if strings.is_empty() {
        bail!("XOB: HEAD carries no string table");
    }
    let scene_root = strings.iter().position(|s| s == "Scene_Root");
    // Records end where the first LZO4 descriptor begins.
    let lzo = head
        .windows(4)
        .position(|w| w == b"LZO4")
        .context("XOB: no LZO4 descriptor in HEAD")?;
    // Walk backwards in 36-byte steps while the record still looks like one; the walk is
    // only trusted when it lands on the `Scene_Root` record.
    let mut count = 0usize;
    while lzo >= (count + 1) * RECORD_LEN {
        let at = lzo - (count + 1) * RECORD_LEN;
        let rec = read_record(&head[at..at + RECORD_LEN]);
        if !plausible(&rec) {
            break;
        }
        count += 1;
    }
    let table = lzo - count * RECORD_LEN;
    let mut nodes: Vec<XobNode> = (0..count)
        .map(|i| read_record(&head[table + i * RECORD_LEN..table + (i + 1) * RECORD_LEN]))
        .collect();
    let rooted = match (scene_root, nodes.first()) {
        (Some(sr), Some(root)) => root.name_idx as usize == sr,
        _ => false,
    };
    if !rooted {
        nodes.clear();
    }
    let count = nodes.len();
    let name_base = table_start;
    // Parent links from the sibling/child encoding.
    let mut parent = vec![None; count];
    let mut stack: Vec<usize> = if count > 0 { vec![0] } else { Vec::new() };
    let mut seen = vec![false; count];
    if let Some(root_seen) = seen.first_mut() {
        *root_seen = true;
    }
    while let Some(p) = stack.pop() {
        let mut c = nodes[p].first_child;
        let mut guard = 0;
        while c != NONE && guard < count {
            let ci = c as usize;
            if ci >= count || seen[ci] {
                break;
            }
            seen[ci] = true;
            parent[ci] = Some(p);
            stack.push(ci);
            c = nodes[ci].next_sibling;
            guard += 1;
        }
    }
    let out = XobNodes {
        strings,
        name_base,
        nodes,
        parent,
        table_offset: if count > 0 { head_off + table } else { 0 },
    };
    for (i, n) in out.nodes.iter().enumerate() {
        match out.name(n.name_idx) {
            Some(s) if !s.is_empty() && !s.starts_with('{') => {}
            other => bail!("XOB: node record {i} has an implausible name {other:?}"),
        }
    }
    Ok(out)
}

impl XobNodes {
    /// Resolve an index in the node / material name space (the string table itself).
    #[must_use]
    pub fn name(&self, idx: u32) -> Option<&str> {
        self.strings.get(idx as usize).map(String::as_str)
    }

    /// Does the model carry a hierarchy (a `Scene_Root` record)?
    #[must_use]
    pub fn has_hierarchy(&self) -> bool {
        !self.nodes.is_empty()
    }

    #[must_use]
    pub fn node_name(&self, node: usize) -> Option<&str> {
        self.nodes.get(node).and_then(|n| self.name(n.name_idx))
    }

    /// Parent-relative transform of one record.
    #[must_use]
    pub fn local_of(&self, node: usize) -> Rigid {
        let n = &self.nodes[node];
        Rigid::from_quat_pos(
            [
                f64::from(n.quat[0]),
                f64::from(n.quat[1]),
                f64::from(n.quat[2]),
                f64::from(n.quat[3]),
            ],
            [
                f64::from(n.pos[0]),
                f64::from(n.pos[1]),
                f64::from(n.pos[2]),
            ],
        )
    }

    /// Model-root → node transform (the parent chain composed).
    #[must_use]
    pub fn world_of(&self, node: usize) -> Rigid {
        let mut chain = vec![node];
        let mut cur = node;
        while let Some(p) = self.parent[cur] {
            chain.push(p);
            cur = p;
        }
        let mut acc = Rigid::identity();
        for &n in chain.iter().rev() {
            acc = acc.compose(&self.local_of(n));
        }
        acc
    }

    /// Every non-root node whose name starts with `socket` (any case), resolved to the
    /// model root, in record order.
    #[must_use]
    pub fn sockets(&self) -> Vec<Socket> {
        (1..self.nodes.len())
            .filter_map(|i| {
                let name = self.node_name(i)?;
                name.to_ascii_lowercase()
                    .starts_with("socket")
                    .then(|| Socket {
                        name: name.to_string(),
                        node: i,
                        local: self.world_of(i),
                    })
            })
            .collect()
    }

    /// Socket by name, case-insensitive (prefab `PivotID`s are not always cased like the
    /// model).
    #[must_use]
    pub fn socket(&self, name: &str) -> Option<Socket> {
        self.sockets()
            .into_iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A HEAD payload: 48 header bytes (with a stray printable byte, like the shipped
    /// files), a string table, filler, node records, one LZO4 tag.
    pub(crate) fn synth_head(strings: &[&str], records: &[XobNode]) -> Vec<u8> {
        let mut head = vec![0u8; 48];
        head[40] = b'k';
        for s in strings {
            head.extend_from_slice(s.as_bytes());
            head.push(0);
        }
        head.extend_from_slice(&[0x11, 0x22, 0x33, 0x44]); // binary filler the scan must stop at
        for r in records {
            head.extend_from_slice(&r.name_idx.to_le_bytes());
            for c in r.pos {
                head.extend_from_slice(&c.to_le_bytes());
            }
            for c in r.quat {
                head.extend_from_slice(&c.to_le_bytes());
            }
            head.extend_from_slice(&r.next_sibling.to_le_bytes());
            head.extend_from_slice(&r.first_child.to_le_bytes());
        }
        head.extend_from_slice(b"LZO4");
        head.extend_from_slice(&[0u8; 112]);
        head
    }

    pub(crate) fn wrap_xob(head: &[u8]) -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(b"FORM");
        file.extend_from_slice(&0u32.to_be_bytes());
        file.extend_from_slice(b"XOB9HEAD");
        file.extend_from_slice(&(head.len() as u32).to_be_bytes());
        file.extend_from_slice(head);
        let total = (file.len() - 8) as u32;
        file[4..8].copy_from_slice(&total.to_be_bytes());
        file
    }

    fn rec(name_idx: u32, pos: [f32; 3], quat: [f32; 4], next: u16, child: u16) -> XobNode {
        XobNode {
            name_idx,
            pos,
            quat,
            next_sibling: next,
            first_child: child,
        }
    }

    #[test]
    fn node_table_decodes_sockets_and_the_name_space_starts_at_the_first_material() {
        // Two leading material strings + one empty entry precede Scene_Root (index 3).
        let strings = [
            "Mat_A",
            "{AAAA}Assets/a.emat",
            "",
            "Scene_Root",
            "socket_door_01",
            "Socket_Win_01",
            "Mesh_Group",
            "{BBBB}Common/Materials/Game/wood.gamemat",
        ];
        let s = (0.5f32).sqrt();
        let records = [
            rec(3, [0.0; 3], [0.0, 0.0, 0.0, 1.0], NONE, 1),
            rec(4, [2.0, 0.0, -1.0], [0.0, s, 0.0, s], 2, NONE), // yaw +90° about Y
            rec(5, [1.0, 1.5, 3.0], [0.0, 0.0, 0.0, 1.0], NONE, 3), // child below
            rec(6, [0.0, 0.5, 0.0], [0.0, 0.0, 0.0, 1.0], NONE, NONE), // Mesh_Group under the window
        ];
        let file = wrap_xob(&synth_head(&strings, &records));
        let nodes = parse_head_nodes(&file).expect("node table");
        assert_eq!(nodes.name_base, 48, "table starts after the header words");
        assert!(nodes.has_hierarchy());
        assert_eq!(nodes.nodes.len(), 4);
        assert_eq!(nodes.node_name(0), Some("Scene_Root"));
        assert_eq!(nodes.name(0), Some("Mat_A"));
        assert_eq!(
            nodes.name(7),
            Some("{BBBB}Common/Materials/Game/wood.gamemat")
        );
        assert_eq!(nodes.parent, vec![None, Some(0), Some(0), Some(2)]);
        let sockets = nodes.sockets();
        assert_eq!(sockets.len(), 2);
        assert_eq!(sockets[0].name, "socket_door_01");
        let door = &sockets[0].local;
        assert!((door.t[0] - 2.0).abs() < 1e-6 && (door.t[2] + 1.0).abs() < 1e-6);
        // Yaw +90° about Y maps local +x onto -z (Enfusion is left-handed: standard R_y).
        let px = door.dir([1.0, 0.0, 0.0]);
        assert!((px[0]).abs() < 1e-6 && (px[2] + 1.0).abs() < 1e-6, "{px:?}");
        // Case-insensitive lookup; the nested mesh group composes through its parent.
        assert_eq!(nodes.socket("SOCKET_WIN_01").unwrap().node, 2);
        let grp = nodes.world_of(3);
        assert!((grp.t[1] - 2.0).abs() < 1e-6 && (grp.t[2] - 3.0).abs() < 1e-6);
    }

    /// The T-090.11.2 acceptance on the real farmhouse: 26 records (root + 25 sockets), COLL
    /// record 0's nine subranges resolve to the nine game materials in file order and cover
    /// all 1129 triangles, record 1 (`FireView`) resolves too. Needs the operator's extract.
    #[test]
    #[ignore = "needs ~/ReforgerExtract/unpacked/…/FarmHouse_E_1L01.xob"]
    fn real_farmhouse_nodes_sockets_and_materials() {
        let home = std::env::var("HOME").unwrap();
        let path = std::path::PathBuf::from(home).join(
            "ReforgerExtract/unpacked/Assets/Structures/Houses/Farm/FarmHouse_E_1L01/FarmHouse_E_1L01.xob",
        );
        let Ok(data) = std::fs::read(&path) else {
            return;
        };
        let nodes = parse_head_nodes(&data).expect("farmhouse node table");
        assert_eq!(nodes.nodes.len(), 26, "root + 25 sockets");
        assert_eq!(nodes.node_name(0), Some("Scene_Root"));
        let sockets = nodes.sockets();
        assert_eq!(sockets.len(), 25);
        let names: Vec<&str> = sockets.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"socket_door_ext_left_01"));
        assert!(names.contains(&"Socket_Win_50x75_01"));
        assert!(
            sockets
                .iter()
                .all(|s| s.local.t.iter().all(|c| c.abs() < 20.0))
        );
        let mesh = super::super::xob::parse_coll(&data).expect("coll");
        assert_eq!(mesh.records.len(), 2);
        assert_eq!(
            nodes.name(u32::from(mesh.records[0].layer_idx)),
            Some("Building")
        );
        assert_eq!(
            nodes.name(u32::from(mesh.records[1].layer_idx)),
            Some("FireView")
        );
        assert_eq!(
            nodes.name(u32::from(mesh.records[0].mesh_idx)),
            Some("UTM_BD_FarmHouse_E_1L01")
        );
        let r0 = &mesh.records[0];
        assert_eq!(r0.tri_count, 1129);
        let mut order: Vec<String> = Vec::new();
        for t in r0.tri_start..r0.tri_start + r0.tri_count {
            let m = mesh.tri_material[t];
            assert_ne!(m, u32::MAX, "triangle {t} has no material");
            let name = nodes.name(m).expect("material name").to_string();
            assert!(name.ends_with(".gamemat"), "{name}");
            if order.last() != Some(&name) {
                order.push(name);
            }
        }
        let stems: Vec<String> = order
            .iter()
            .map(|p| super::super::surface_kind::gamemat_stem(p))
            .collect();
        assert_eq!(
            stems,
            [
                "tiles_ceramic",
                "wood",
                "concrete",
                "carpet",
                "tiles",
                "metal",
                "stone",
                "wood_floor",
                "brick"
            ]
        );
        let r1 = &mesh.records[1];
        assert_eq!(r1.tri_count, 2883);
        assert!((r1.tri_start..r1.tri_start + r1.tri_count).all(|t| {
            nodes
                .name(mesh.tri_material[t])
                .is_some_and(|n| n.ends_with(".gamemat"))
        }));
    }

    /// A model without a hierarchy (door leaf, pane, tree) still yields its name space.
    #[test]
    fn hierarchy_less_models_keep_their_string_table() {
        let file = wrap_xob(&synth_head(
            &[
                "Doors_Village_E",
                "{X}a.emat",
                "Leaf",
                "DoorFireView",
                "UBX_Leaf",
                "{Y}wood.gamemat",
            ],
            &[],
        ));
        let nodes = parse_head_nodes(&file).expect("strings only");
        assert!(!nodes.has_hierarchy());
        assert!(nodes.sockets().is_empty());
        assert_eq!(nodes.name(3), Some("DoorFireView"));
        assert_eq!(nodes.name(5), Some("{Y}wood.gamemat"));
        // Records that do not start at Scene_Root are ignored, not trusted.
        let file = wrap_xob(&synth_head(
            &["Mat", "Other"],
            &[XobNode {
                name_idx: 1,
                pos: [0.0; 3],
                quat: [0.0, 0.0, 0.0, 1.0],
                next_sibling: NONE,
                first_child: NONE,
            }],
        ));
        assert!(!parse_head_nodes(&file).unwrap().has_hierarchy());
        assert!(parse_head_nodes(b"NOPE").is_err());
    }
}
