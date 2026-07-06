//! The slot Structure-of-Arrays materialized from the `yrs` document (plan §9.1 criterion 1).
//!
//! One row per slot; **row order is arbitrary** (`yrs` map-iteration order, which need not match the
//! JS `Y.Doc` insertion order), so every parity check keys by `ids[row]` — the contract is
//! set-equality, not row alignment. Numeric columns are `f32` (the JS `Float32Array` store boundary):
//! a JS `x` compares to a column value as `Math.fround(x) === col[row]` (Class R). String-valued
//! fields (`role`/`tag`/`squadId`/layer) are interned into first-seen dictionaries + an integer index
//! column — the exact shape the cutover feeds to deck.gl and the spatial indices.

use std::collections::HashMap;

/// Stance codes (dense, deck-attribute-friendly). Mirrors `Slot['stance']`.
pub const STANCE_STAND: u8 = 0;
pub const STANCE_CROUCH: u8 = 1;
pub const STANCE_PRONE: u8 = 2;

/// Sentinel index for an absent optional — a slot with no `tag`, or one filed in no Outliner folder.
pub const NONE_IDX: u32 = u32::MAX;

/// Columnar view of every slot in the document. `ids[row]` is the slot's string id (the join key);
/// the `*_idx` columns index into the parallel `roles`/`tags`/`squads`/`layers` dictionaries.
#[derive(Default, Clone)]
pub struct SlotSoa {
    pub ids: Vec<String>,
    pub xs: Vec<f32>,
    pub ys: Vec<f32>,
    pub zs: Vec<f32>,
    pub rotations: Vec<f32>,
    pub stance: Vec<u8>,
    pub role_idx: Vec<u32>,
    pub tag_idx: Vec<u32>,
    pub squad_idx: Vec<u32>,
    pub layer_idx: Vec<u32>,
    /// Interned dictionaries (first-seen order); a `*_idx` value indexes into the matching one.
    pub roles: Vec<String>,
    pub tags: Vec<String>,
    pub squads: Vec<String>,
    pub layers: Vec<String>,
}

impl SlotSoa {
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

/// First-seen string interner: `intern(s)` returns a stable index and appends to `words` the first
/// time it sees `s`. Deterministic given a fixed materialization order over one document.
pub(crate) struct Interner {
    map: HashMap<String, u32>,
    pub words: Vec<String>,
}

impl Interner {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
            words: Vec::new(),
        }
    }

    pub(crate) fn intern(&mut self, s: &str) -> u32 {
        if let Some(&i) = self.map.get(s) {
            return i;
        }
        let i = self.words.len() as u32;
        self.words.push(s.to_string());
        self.map.insert(s.to_string(), i);
        i
    }
}
