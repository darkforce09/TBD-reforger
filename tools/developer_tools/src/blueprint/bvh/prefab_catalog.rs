//! Enfusion `.et` prefab text — a tolerant parser for the entity-template grammar plus the
//! inheritance resolver that turns a prefab path into the facts the BLAS/instance pipeline
//! needs: the mesh (`MeshObject.Object`), door parameters, the attach socket
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

use crate::enfusion_pak::AssetSource;

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
        let key = crate::enfusion_pak::normalize_path(path);
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
#[path = "../tests/prefab/tests.rs"]
mod tests;

#[path = "prefab_catalog/tokenize.rs"]
mod tokenize;
use tokenize::is_placeholder_mesh;
use tokenize::own_facts;
pub use tokenize::parse_et;
pub use tokenize::strip_guid;
