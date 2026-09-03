//! Enfusion `.et` prefab text — a tolerant parser for the entity-template grammar plus the
//! inheritance resolver that turns a prefab path into the facts the BLAS/instance pipeline
//! needs (T-090.11.2): the mesh (`MeshObject.Object`), door parameters, the attach socket
//! (`Hierarchy.PivotID`), slot-bone mappings and the child entity list with their local
//! transforms.
//!
//! Grammar (as observed in the shipped files, no spec exists):
//!
//! ```text
//! Class [: "{GUID}parent.et"] {            // root: class + optional base prefab
//!  ID "F0DB…"                              // prop: key + scalar values
//!  components {                            // block
//!   MeshObject "{guid}" { Object "{guid}Assets/x.xob" }
//!   DoorComponent "{guid}" : "{guid}base.ct" { AngleRange -120 }
//!   m_vCenter PointInfo "{guid}" { Offset 1 2 3 }   // typed block: key Type "guid" {…}
//!   "Additional hit zones" { SCR_WindowHitZone Default { … } }
//!   LODFactors { 20 5 1 1 1 }               // value list block
//!  }
//!  SlotBoneMappings { SlotBoneMappingObject "{guid}" { BonePrefix "socket_x" Prefab "{guid}y.et" } }
//!  {                                       // anonymous block: the children
//!   Building : "{guid}win.et" { ID "…" components { Hierarchy "{guid}" { PivotID "Socket_Win_01" } } coords 0 0 0 }
//!   $grp GenericEntity : "{guid}bed.et" { { ID "…" coords 1 2 3 angles 0 -90 0 scale 1.1 } { … } }
//!  }
//! }
//! ```
//!
//! Everything is a `Block` (name, optional type words, optional guid, optional base, props,
//! child blocks, anonymous blocks, bare values); the resolver then reads the few paths it
//! cares about. Unknown constructs are kept, never rejected — the grammar is bigger than the
//! slice needs.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use anyhow::{Context, Result, bail};

use super::pak::AssetSource;

/// One parsed `{ … }` node.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Block {
    /// The first head word: class name (`GenericEntity`), component name (`MeshObject`) or
    /// the property key of a typed block (`m_vCenter`).
    pub name: String,
    /// Further bare head words (`PointInfo` in `m_vCenter PointInfo "{…}" {`, `Default` in
    /// `SCR_WindowHitZone Default {`).
    pub types: Vec<String>,
    /// `"{GUID}"` instance id when present.
    pub guid: Option<String>,
    /// `: "{GUID}path"` base prefab / component template, GUID stripped.
    pub base: Option<String>,
    /// `$grp` marker: the anonymous children are instances of THIS head.
    pub grp: bool,
    /// `key value…` properties in file order (a key may repeat).
    pub props: Vec<(String, Vec<String>)>,
    /// Named child blocks in file order.
    pub blocks: Vec<Block>,
    /// Anonymous `{ … }` children in file order.
    pub anon: Vec<Block>,
    /// Bare scalars of a value-list block (`LODFactors { 20 5 1 1 1 }`).
    pub values: Vec<String>,
    /// Parse-time marker: this statement had no body (it is folded into the parent's
    /// `props`, so a `Block` a caller can reach never has it set).
    pub is_prop: bool,
}

impl Block {
    /// First value of the first `key` prop.
    pub fn prop(&self, key: &str) -> Option<&str> {
        self.props
            .iter()
            .find(|(k, _)| k == key)
            .and_then(|(_, v)| v.first().map(String::as_str))
    }

    /// All values of the first `key` prop.
    pub fn prop_values(&self, key: &str) -> Option<&[String]> {
        self.props
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_slice())
    }

    pub fn prop_f64(&self, key: &str) -> Option<f64> {
        self.prop(key).and_then(|s| s.parse().ok())
    }

    /// `key a b c` as three floats.
    pub fn prop_vec3(&self, key: &str) -> Option<[f64; 3]> {
        let v = self.prop_values(key)?;
        if v.len() < 3 {
            return None;
        }
        Some([v[0].parse().ok()?, v[1].parse().ok()?, v[2].parse().ok()?])
    }

    /// First named child block called `name`.
    pub fn block(&self, name: &str) -> Option<&Block> {
        self.blocks.iter().find(|b| b.name == name)
    }

    pub fn blocks_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Block> + 'a {
        self.blocks.iter().filter(move |b| b.name == name)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Num(String),
    LBrace,
    RBrace,
    Colon,
    Grp,
}

fn tokenize(src: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
        } else if c == b'{' {
            out.push(Tok::LBrace);
            i += 1;
        } else if c == b'}' {
            out.push(Tok::RBrace);
            i += 1;
        } else if c == b':' {
            out.push(Tok::Colon);
            i += 1;
        } else if c == b'"' {
            let start = i + 1;
            let mut j = start;
            while j < b.len() && b[j] != b'"' {
                j += 1;
            }
            out.push(Tok::Str(src[start..j].to_string()));
            i = j + 1;
        } else {
            let start = i;
            while i < b.len()
                && !b[i].is_ascii_whitespace()
                && b[i] != b'{'
                && b[i] != b'}'
                && b[i] != b'"'
            {
                i += 1;
            }
            let word = &src[start..i];
            if word == "$grp" {
                out.push(Tok::Grp);
            } else if word.starts_with(|ch: char| ch.is_ascii_digit())
                || (word.starts_with('-') && word.len() > 1)
                || word.starts_with('.')
            {
                out.push(Tok::Num(word.to_string()));
            } else {
                out.push(Tok::Ident(word.to_string()));
            }
        }
    }
    out
}

/// `{GUID}path` → `path`; anything else unchanged.
pub fn strip_guid(s: &str) -> &str {
    if s.starts_with('{') {
        if let Some(end) = s.find('}') {
            return &s[end + 1..];
        }
    }
    s
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self, n: usize) -> Option<&Tok> {
        self.toks.get(self.pos + n)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }

    /// Parse statements until `}` (consumed) or end of input, into `into`.
    fn body(&mut self, into: &mut Block, depth: usize) -> Result<()> {
        if depth > 64 {
            bail!("prefab nesting deeper than 64");
        }
        let mut grp_pending = false;
        loop {
            let Some(t) = self.peek(0).cloned() else {
                if depth > 0 {
                    bail!("unbalanced braces: end of prefab text inside a block");
                }
                return Ok(());
            };
            match t {
                Tok::RBrace => {
                    self.pos += 1;
                    return Ok(());
                }
                Tok::LBrace => {
                    self.pos += 1;
                    let mut child = Block::default();
                    self.body(&mut child, depth + 1)?;
                    into.anon.push(child);
                }
                Tok::Grp => {
                    self.pos += 1;
                    grp_pending = true;
                }
                Tok::Num(n) => {
                    self.pos += 1;
                    into.values.push(n);
                }
                Tok::Str(s) => {
                    // A string at statement position is a key only when something that
                    // can only follow a key comes next.
                    match self.peek(1) {
                        Some(Tok::LBrace | Tok::Colon | Tok::Num(_)) => {
                            let mut blk = self.statement(s, depth)?;
                            blk.grp = std::mem::take(&mut grp_pending);
                            self.push_statement(into, blk);
                        }
                        Some(Tok::Ident(_)) if matches!(self.peek(2), Some(Tok::LBrace)) => {
                            let mut blk = self.statement(s, depth)?;
                            blk.grp = std::mem::take(&mut grp_pending);
                            self.push_statement(into, blk);
                        }
                        _ => {
                            self.pos += 1;
                            into.values.push(s);
                        }
                    }
                }
                Tok::Ident(id) => {
                    let mut blk = self.statement(id, depth)?;
                    blk.grp = std::mem::take(&mut grp_pending);
                    self.push_statement(into, blk);
                }
                Tok::Colon => bail!("stray ':' in prefab text"),
            }
        }
    }

    /// A parsed statement is either a block (has a body) or a prop (no body); props are
    /// carried as a Block with `values` and folded into `props` here.
    fn push_statement(&mut self, into: &mut Block, blk: Block) {
        if blk.is_prop {
            into.props.push((blk.name, blk.values));
        } else {
            into.blocks.push(blk);
        }
    }

    /// `key …` — decide between a block head and a prop, consume it.
    fn statement(&mut self, key: String, depth: usize) -> Result<Block> {
        self.pos += 1; // the key
        let mut blk = Block {
            name: key,
            ..Block::default()
        };
        // Head words: bare identifiers (types / names) until something decisive.
        loop {
            match self.peek(0).cloned() {
                Some(Tok::LBrace) => {
                    self.pos += 1;
                    self.body(&mut blk, depth + 1)?;
                    return Ok(blk);
                }
                Some(Tok::Colon) => {
                    self.pos += 1;
                    match self.next() {
                        Some(Tok::Str(base)) => blk.base = Some(strip_guid(&base).to_string()),
                        other => bail!("expected a quoted base after ':' (got {other:?})"),
                    }
                    match self.next() {
                        Some(Tok::LBrace) => {}
                        other => bail!("expected '{{' after the base prefab (got {other:?})"),
                    }
                    self.body(&mut blk, depth + 1)?;
                    return Ok(blk);
                }
                Some(Tok::Str(s)) => {
                    // `Key "{guid}" {` / `Key "{guid}" : "base" {` → block; otherwise a prop
                    // whose values start with this string. A guid head always starts with
                    // `{` — `ID "F0DB…"` followed by the anonymous children block is a prop.
                    match self.peek(1) {
                        Some(Tok::LBrace | Tok::Colon)
                            if blk.values.is_empty() && s.starts_with('{') =>
                        {
                            self.pos += 1;
                            blk.guid = Some(s);
                            continue;
                        }
                        _ => {
                            self.pos += 1;
                            blk.values.push(s);
                            return Ok(self.finish_prop(blk));
                        }
                    }
                }
                Some(Tok::Num(n)) => {
                    self.pos += 1;
                    blk.values.push(n);
                    return Ok(self.finish_prop(blk));
                }
                Some(Tok::Ident(w)) => {
                    // A bare word after the key: part of a block head when a block follows
                    // it (`Key Type {`, `Key Type "{guid}" {`, `Key Type : "base" {`), else
                    // the prop's single enum-like value (`Event SOUND_OPEN_FINISH`).
                    let heads_block = matches!(
                        (self.peek(1), self.peek(2)),
                        (Some(Tok::LBrace | Tok::Colon), _)
                            | (Some(Tok::Str(_)), Some(Tok::LBrace | Tok::Colon))
                    );
                    self.pos += 1;
                    if heads_block {
                        blk.types.push(w);
                        continue;
                    }
                    blk.values.push(w);
                    return Ok(self.finish_prop(blk));
                }
                Some(Tok::Grp) | Some(Tok::RBrace) | None => {
                    // Key with no value (rare) — keep as an empty prop.
                    return Ok(self.finish_prop(blk));
                }
            }
        }
    }

    /// Consume trailing numeric values of a prop (`coords 0 0 0`, `Flags 0x403 0`).
    fn finish_prop(&mut self, mut blk: Block) -> Block {
        while let Some(Tok::Num(n)) = self.peek(0).cloned() {
            self.pos += 1;
            blk.values.push(n);
        }
        blk.is_prop = true;
        blk
    }
}

/// Parse a whole `.et` file into its root blocks (one per file in practice).
pub fn parse_et(src: &str) -> Result<Vec<Block>> {
    let mut p = Parser {
        toks: tokenize(src),
        pos: 0,
    };
    let mut root = Block::default();
    p.body(&mut root, 0)?;
    if p.pos < p.toks.len() {
        bail!("unbalanced braces in prefab text");
    }
    let mut roots = root.blocks;
    // Props at file level are meaningless; a value list too. Anonymous top-level blocks
    // are not a thing either — keep what parsed and let the resolver complain.
    roots.extend(root.anon);
    Ok(roots)
}

/// Rotating-door parameters (`DoorComponent`).
#[derive(Debug, Clone, PartialEq)]
pub struct DoorParams {
    pub angle_range_deg: f64,
    pub closed_angle_deg: f64,
    pub initial_angle_deg: f64,
    /// `AngleRange` was read from a prefab in the chain (else the 90° default).
    pub angle_range_explicit: bool,
}

/// Sliding-door parameters (`SlidingDoorComponent`).
#[derive(Debug, Clone, PartialEq)]
pub struct SlidingParams {
    pub opened_distance: f64,
    pub initial_distance: f64,
}

/// One child entity placement inside a prefab.
#[derive(Debug, Clone, PartialEq)]
pub struct ChildRef {
    pub class: String,
    /// Child prefab path (GUID stripped).
    pub prefab: String,
    /// The child's `ID "…"` when present (stable within the prefab; the instance id seed).
    pub id: Option<String>,
    /// `Hierarchy.PivotID` — the parent socket the child is attached to.
    pub pivot_id: Option<String>,
    pub coords: [f64; 3],
    /// `angles pitch yaw roll` (degrees, Enfusion order).
    pub angles_deg: [f64; 3],
    pub scale: f64,
}

/// What a prefab resolves to after walking its inheritance chain.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResolvedPrefab {
    pub path: String,
    pub class: String,
    /// Base chain, nearest first.
    pub chain: Vec<String>,
    /// `MeshObject.Object`, GUID stripped (`Common/Models/Default.xob` counts as none).
    pub mesh: Option<String>,
    pub door: Option<DoorParams>,
    pub sliding: Option<SlidingParams>,
    /// This prefab's own `Hierarchy.PivotID` (the socket it expects to sit on).
    pub hierarchy_pivot: Option<String>,
    /// `SlotBoneMappings`: (bone prefix, prefab path).
    pub slot_bones: Vec<(String, String)>,
    pub children: Vec<ChildRef>,
}

const DEFAULT_ANGLE_RANGE_DEG: f64 = 90.0;

fn is_placeholder_mesh(p: &str) -> bool {
    p.ends_with("Common/Models/Default.xob")
}

fn read_child(head: &Block, inst: &Block) -> ChildRef {
    let pivot = inst
        .block("components")
        .and_then(|c| c.block("Hierarchy"))
        .and_then(|h| h.prop("PivotID"))
        .map(ToString::to_string);
    ChildRef {
        class: head.name.clone(),
        prefab: head.base.clone().unwrap_or_default(),
        id: inst.prop("ID").map(ToString::to_string),
        pivot_id: pivot,
        coords: inst.prop_vec3("coords").unwrap_or([0.0; 3]),
        angles_deg: inst.prop_vec3("angles").unwrap_or([0.0; 3]),
        scale: inst.prop_f64("scale").unwrap_or(1.0),
    }
}

/// The facts of ONE file (no inheritance applied).
#[derive(Debug, Clone, Default)]
#[allow(clippy::type_complexity)]
struct OwnFacts {
    class: String,
    base: Option<String>,
    mesh: Option<String>,
    door: Option<(Option<f64>, Option<f64>, Option<f64>, bool)>, // range, closed, initial, enabled
    sliding: Option<(Option<f64>, Option<f64>, bool)>,
    hierarchy_pivot: Option<String>,
    slot_bones: Vec<(String, String)>,
    children: Vec<ChildRef>,
}

fn enabled(b: &Block) -> bool {
    b.prop("Enabled") != Some("0")
}

fn own_facts(root: &Block) -> OwnFacts {
    let mut f = OwnFacts {
        class: root.name.clone(),
        base: root.base.clone(),
        ..OwnFacts::default()
    };
    if let Some(components) = root.block("components") {
        if let Some(m) = components.block("MeshObject") {
            f.mesh = m.prop("Object").map(|s| strip_guid(s).to_string());
        }
        if let Some(d) = components.block("DoorComponent") {
            f.door = Some((
                d.prop_f64("AngleRange"),
                d.prop_f64("ClosedAngle"),
                d.prop_f64("InitialAngle"),
                enabled(d),
            ));
        }
        if let Some(s) = components.block("SlidingDoorComponent") {
            f.sliding = Some((
                s.prop_f64("OpenedDistance"),
                s.prop_f64("InitialDistance"),
                enabled(s),
            ));
        }
        if let Some(h) = components.block("Hierarchy") {
            f.hierarchy_pivot = h.prop("PivotID").map(ToString::to_string);
        }
    }
    if let Some(sb) = root.block("SlotBoneMappings") {
        for m in sb.blocks_named("SlotBoneMappingObject") {
            if let (Some(prefix), Some(prefab)) = (m.prop("BonePrefix"), m.prop("Prefab")) {
                f.slot_bones
                    .push((prefix.to_string(), strip_guid(prefab).to_string()));
            }
        }
    }
    for list in &root.anon {
        for head in &list.blocks {
            if head.base.is_none() {
                continue;
            }
            if head.grp || (!head.anon.is_empty() && head.prop("ID").is_none()) {
                for inst in &head.anon {
                    f.children.push(read_child(head, inst));
                }
            } else {
                f.children.push(read_child(head, head));
            }
        }
    }
    f
}

/// Resolves prefabs through an [`AssetSource`], memoized by path.
pub struct PrefabResolver<'a> {
    source: &'a dyn AssetSource,
    cache: HashMap<String, Rc<ResolvedPrefab>>,
}

impl<'a> PrefabResolver<'a> {
    pub fn new(source: &'a dyn AssetSource) -> Self {
        Self {
            source,
            cache: HashMap::new(),
        }
    }

    pub fn resolve(&mut self, path: &str) -> Result<Rc<ResolvedPrefab>> {
        let mut visiting = HashSet::new();
        self.resolve_inner(path, &mut visiting)
    }

    fn resolve_inner(
        &mut self,
        path: &str,
        visiting: &mut HashSet<String>,
    ) -> Result<Rc<ResolvedPrefab>> {
        let key = super::pak::normalize_path(path);
        if let Some(r) = self.cache.get(&key) {
            return Ok(r.clone());
        }
        if !visiting.insert(key.clone()) {
            bail!("prefab inheritance cycle through {path}");
        }
        let text = self
            .source
            .read_text(path)
            .with_context(|| format!("read prefab {path}"))?;
        let roots = parse_et(&text).with_context(|| format!("parse prefab {path}"))?;
        let root = roots
            .first()
            .with_context(|| format!("{path}: no root block"))?;
        let own = own_facts(root);
        let base = match &own.base {
            Some(b) if b.ends_with(".et") => Some(self.resolve_inner(b, visiting)?),
            _ => None,
        };
        let mut out = match &base {
            Some(b) => ResolvedPrefab {
                path: path.to_string(),
                class: own.class.clone(),
                chain: std::iter::once(b.path.clone())
                    .chain(b.chain.iter().cloned())
                    .collect(),
                mesh: b.mesh.clone(),
                door: b.door.clone(),
                sliding: b.sliding.clone(),
                hierarchy_pivot: b.hierarchy_pivot.clone(),
                slot_bones: b.slot_bones.clone(),
                children: b.children.clone(),
            },
            None => ResolvedPrefab {
                path: path.to_string(),
                class: own.class.clone(),
                ..ResolvedPrefab::default()
            },
        };
        if let Some(m) = own.mesh {
            out.mesh = (!is_placeholder_mesh(&m)).then_some(m);
        }
        if let Some((range, closed, initial, on)) = own.door {
            if on {
                let prev = out.door.take();
                out.door = Some(DoorParams {
                    angle_range_deg: range
                        .or(prev.as_ref().map(|p| p.angle_range_deg))
                        .unwrap_or(DEFAULT_ANGLE_RANGE_DEG),
                    closed_angle_deg: closed
                        .or(prev.as_ref().map(|p| p.closed_angle_deg))
                        .unwrap_or(0.0),
                    initial_angle_deg: initial
                        .or(prev.as_ref().map(|p| p.initial_angle_deg))
                        .unwrap_or(0.0),
                    angle_range_explicit: range.is_some()
                        || prev.as_ref().is_some_and(|p| p.angle_range_explicit),
                });
            } else {
                out.door = None;
            }
        }
        if let Some((dist, initial, on)) = own.sliding {
            if on {
                let prev = out.sliding.take();
                out.sliding = Some(SlidingParams {
                    opened_distance: dist
                        .or(prev.as_ref().map(|p| p.opened_distance))
                        .unwrap_or(0.0),
                    initial_distance: initial
                        .or(prev.as_ref().map(|p| p.initial_distance))
                        .unwrap_or(0.0),
                });
            } else {
                out.sliding = None;
            }
        }
        if own.hierarchy_pivot.is_some() {
            out.hierarchy_pivot = own.hierarchy_pivot;
        }
        for (prefix, prefab) in own.slot_bones {
            match out.slot_bones.iter_mut().find(|(p, _)| *p == prefix) {
                Some(slot) => slot.1 = prefab,
                None => out.slot_bones.push((prefix, prefab)),
            }
        }
        out.children.extend(own.children);
        let rc = Rc::new(out);
        self.cache.insert(key.clone(), rc.clone());
        visiting.remove(&key);
        Ok(rc)
    }
}

#[cfg(test)]
mod tests {
    use super::super::pak::DirSource;
    use super::*;
    use crate::map_blueprint::tests::fixture;

    #[test]
    fn tokenizer_and_block_shapes() {
        let src = r#"GenericEntity : "{0011}Prefabs/Base.et" {
 ID "AB12"
 components {
  MeshObject "{22}" { Object "{33}Assets/x.xob" LODFactors { 20 5 1 } }
  DoorComponent "{44}" : "{55}Prefabs/door.ct" { Enabled 0 AngleRange -120 DoorAnimationType WholeEntity }
  m_vCenter PointInfo "{66}" { Offset 1.5 2 -3 }
  "Additional hit zones" { SCR_WindowHitZone Default { "Kinetic multiplier" 4 } }
  Tags { "OpenGate" "Other" }
  Flags 0x403 0
 }
 {
  $grp Building : "{77}Prefabs/win.et" { { ID "1" coords 1 2 3 } { ID "2" angles 0 -90 0 scale 1.5 } }
  GenericEntity : "{88}Prefabs/table.et" { ID "3" components { Hierarchy "{99}" { PivotID "socket_a" AutoTransform 1 } } coords 0 0 -0.1 }
 }
}"#;
        let roots = parse_et(src).expect("parse");
        assert_eq!(roots.len(), 1);
        let r = &roots[0];
        assert_eq!(r.name, "GenericEntity");
        assert_eq!(r.base.as_deref(), Some("Prefabs/Base.et"));
        assert_eq!(r.prop("ID"), Some("AB12"));
        let c = r.block("components").unwrap();
        let mesh = c.block("MeshObject").unwrap();
        assert_eq!(mesh.guid.as_deref(), Some("{22}"));
        assert_eq!(mesh.prop("Object"), Some("{33}Assets/x.xob"));
        assert_eq!(
            mesh.block("LODFactors").unwrap().values,
            vec!["20", "5", "1"]
        );
        let door = c.block("DoorComponent").unwrap();
        assert_eq!(door.base.as_deref(), Some("Prefabs/door.ct"));
        assert_eq!(door.prop_f64("AngleRange"), Some(-120.0));
        assert_eq!(door.prop("DoorAnimationType"), Some("WholeEntity"));
        assert_eq!(door.prop("Enabled"), Some("0"));
        let center = c.block("m_vCenter").unwrap();
        assert_eq!(center.types, vec!["PointInfo"]);
        assert_eq!(center.prop_vec3("Offset"), Some([1.5, 2.0, -3.0]));
        let hz = c.block("Additional hit zones").unwrap();
        let wz = hz.block("SCR_WindowHitZone").unwrap();
        assert_eq!(wz.types, vec!["Default"]);
        assert_eq!(wz.prop_f64("Kinetic multiplier"), Some(4.0));
        assert_eq!(c.block("Tags").unwrap().values, vec!["OpenGate", "Other"]);
        assert_eq!(c.prop_values("Flags").unwrap(), ["0x403", "0"]);
        assert_eq!(r.anon.len(), 1);
        let kids = &r.anon[0].blocks;
        assert!(kids[0].grp && kids[0].anon.len() == 2);
        assert_eq!(kids[0].anon[1].prop_vec3("angles"), Some([0.0, -90.0, 0.0]));
        assert_eq!(kids[1].prop_vec3("coords"), Some([0.0, 0.0, -0.1]));
        assert_eq!(strip_guid("{ABC}Prefabs/x.et"), "Prefabs/x.et");
        assert_eq!(strip_guid("plain"), "plain");
        assert!(parse_et("A { B { }").is_err(), "unbalanced");
    }

    fn fixture_source() -> DirSource {
        DirSource {
            root: fixture("prefab"),
        }
    }

    #[test]
    fn resolver_walks_inheritance_sockets_and_children() {
        let src = fixture_source();
        let mut r = PrefabResolver::new(&src);
        let house = r.resolve("Prefabs/Houses/House_Wood.et").expect("house");
        assert_eq!(house.class, "SCR_DestructibleBuildingEntity");
        assert_eq!(
            house.chain,
            vec![
                "Prefabs/Houses/House_Base.et".to_string(),
                "Prefabs/Core/Building_Base.et".to_string()
            ]
        );
        // Mesh comes from House_Base; Building_Base's placeholder never wins.
        assert_eq!(house.mesh.as_deref(), Some("Assets/Houses/House.xob"));
        assert_eq!(
            house.slot_bones,
            vec![
                (
                    "socket_door_left".to_string(),
                    "Prefabs/Doors/DoorSet.et".to_string()
                ),
                (
                    "socket_win".to_string(),
                    "Prefabs/Windows/Window.et".to_string()
                ),
            ]
        );
        // Base children (door + two windows) come first, then the furniture composition.
        let kinds: Vec<(&str, Option<&str>)> = house
            .children
            .iter()
            .map(|c| (c.prefab.as_str(), c.pivot_id.as_deref()))
            .collect();
        assert_eq!(
            kinds,
            vec![
                ("Prefabs/Doors/DoorSet.et", Some("socket_door_left_01")),
                ("Prefabs/Windows/Window.et", Some("socket_win_01")),
                ("Prefabs/Windows/Window.et", Some("socket_win_02")),
                ("Prefabs/Core/Probe.et", None),
                ("Prefabs/Furniture/Furniture_01.et", None),
            ]
        );
        assert_eq!(house.children[0].coords, [0.0, 0.0, -0.1]);
        assert_eq!(house.children[3].coords, [2.655, 2.289, 4.876]);
        assert_eq!(house.children[4].id.as_deref(), Some("F1"));
        assert!(house.door.is_none() && house.sliding.is_none());

        let furniture = r
            .resolve("Prefabs/Furniture/Furniture_01.et")
            .expect("furniture");
        assert_eq!(furniture.children.len(), 3);
        assert_eq!(furniture.children[0].prefab, "Prefabs/Props/Table.et");
        assert_eq!(furniture.children[0].coords, [1.035, 0.28, -7.666]);
        assert_eq!(furniture.children[0].angles_deg, [0.0, 91.667, 0.0]);
        assert_eq!(furniture.children[2].scale, 1.152);
        assert_eq!(furniture.children[2].angles_deg, [88.816, -180.0, 96.7]);

        let set = r.resolve("Prefabs/Doors/DoorSet.et").expect("door set");
        assert_eq!(set.mesh.as_deref(), Some("Assets/Doors/DoorFrame.xob"));
        assert_eq!(set.children.len(), 1);
        assert_eq!(
            set.children[0].pivot_id.as_deref(),
            Some("socket_door_LEFT")
        );
        assert_eq!(set.children[0].prefab, "Prefabs/Doors/Door_Leaf.et");

        let leaf = r.resolve("Prefabs/Doors/Door_Leaf.et").expect("leaf");
        assert_eq!(leaf.mesh.as_deref(), Some("Assets/Doors/Door_Leaf.xob"));
        let d = leaf.door.as_ref().expect("rotating door");
        assert_eq!(d.angle_range_deg, -120.0);
        assert!(d.angle_range_explicit);
        assert_eq!(d.closed_angle_deg, 0.0);
        assert!(leaf.sliding.is_none());

        let plain = r
            .resolve("Prefabs/Doors/Door_Plain.et")
            .expect("plain door");
        let d = plain.door.as_ref().expect("door from the base chain");
        assert_eq!(d.angle_range_deg, DEFAULT_ANGLE_RANGE_DEG);
        assert!(!d.angle_range_explicit);

        let barn = r.resolve("Prefabs/Doors/Door_Sliding.et").expect("sliding");
        assert!(
            barn.door.is_none(),
            "DoorComponent Enabled 0 drops the rotating door"
        );
        assert_eq!(barn.sliding.as_ref().unwrap().opened_distance, 2.05);

        let window = r.resolve("Prefabs/Windows/Window.et").expect("window");
        assert_eq!(window.mesh.as_deref(), Some("Assets/Windows/Win.xob"));
        assert_eq!(
            window.slot_bones,
            vec![(
                "socket_glass".to_string(),
                "Prefabs/Windows/Glass.et".to_string()
            )]
        );
        assert_eq!(window.children.len(), 2);
        assert_eq!(
            window.children[1].pivot_id.as_deref(),
            Some("socket_glass_002")
        );
        let glass = r.resolve("Prefabs/Windows/Glass.et").expect("glass");
        assert_eq!(glass.mesh.as_deref(), Some("Assets/Windows/Glass_01.xob"));
        assert!(glass.hierarchy_pivot.is_none());
        // Memoized: a second resolve is the same Rc.
        let again = r.resolve("prefabs/windows/glass.et").unwrap();
        assert!(Rc::ptr_eq(&glass, &again));
        assert!(r.resolve("Prefabs/Missing.et").is_err());
    }
}
