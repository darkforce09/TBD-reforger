//! Role: soa.
//! Position: `doc/crdt` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use std::collections::HashMap;

/// Stance codes (dense, deck-attribute-friendly). Mirrors `Slot['stance']`.
pub const STANCE_STAND: u8 = 0;

/// Canonical stance crouch value.
pub const STANCE_CROUCH: u8 = 1;

/// Canonical stance prone value.
pub const STANCE_PRONE: u8 = 2;

/// Sentinel index for an absent optional — a slot with no `tag`, or one filed in no Outliner folder.
pub const NONE_IDX: u32 = u32::MAX;

/// Row-aligned slot columns and interned strings materialized from the mission document.
#[derive(Default, Clone)]
pub struct SlotSoa {
    /// Ids.
    pub ids: Vec<String>,

    /// Xs.
    pub xs: Vec<f32>,

    /// Ys.
    pub ys: Vec<f32>,

    /// Interleaved `[x0,y0,x1,y1,…]` (length `2·len`) — the deck.gl `getPosition` binary attribute, read zero-copy through `slot_xy_ptr` (criterion 6). Row-aligned with `xs`/`ys`.
    pub xy: Vec<f32>,

    /// Zs.
    pub zs: Vec<f32>,

    /// Rotations.
    pub rotations: Vec<f32>,

    /// Stance.
    pub stance: Vec<u8>,

    /// Role idx.
    pub role_idx: Vec<u32>,

    /// Tag idx.
    pub tag_idx: Vec<u32>,

    /// Squad idx.
    pub squad_idx: Vec<u32>,

    /// Layer idx.
    pub layer_idx: Vec<u32>,

    /// Side keys.
    pub side_keys: Vec<String>,

    /// Interned dictionaries (first-seen order); a `*_idx` value indexes into the matching one.
    pub roles: Vec<String>,

    /// Tags.
    pub tags: Vec<String>,

    /// Squads.
    pub squads: Vec<String>,

    /// Layers.
    pub layers: Vec<String>,
}

impl SlotSoa {
    /// Len using the supplied domain data.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Is empty using the supplied domain data.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}

/// First-seen string interner: `intern(s)` returns a stable index and appends to `words` the first time it sees `s`. Deterministic given a fixed materialization order over one document.
pub(crate) struct Interner {
    map: HashMap<String, u32>,

    /// Words.
    pub words: Vec<String>,
}

impl Interner {
    /// New using the supplied domain data.
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
            words: Vec::new(),
        }
    }

    /// Intern using the supplied domain data.
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
