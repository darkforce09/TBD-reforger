//! Role: authored block.
//! Position: `mission/extensions/authored` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::{Map, Serialize, SerializeMap, Serializer, Value};

/// One authored top-level block: the JSON key, and the check that says whether the author's value is one this compile can carry.
pub struct AuthoredBlock {
    /// The top-level key, spelled exactly as `mission.schema.json` declares it.
    pub key: &'static str,

    /// The typed validator for this block's value.
    pub validate: fn(&Value) -> Result<(), String>,
}

/// Every block the editor authors and the compile carries verbatim.
pub const AUTHORED_BLOCKS: &[AuthoredBlock] = &[
    AuthoredBlock {
        key: "radioPlan",
        validate: crate::data::scenario::radio_plan::validate,
    },
    AuthoredBlock {
        key: "winConditions",
        validate: crate::data::scenario::win_conditions::validate,
    },
    AuthoredBlock {
        key: "tasks",
        validate: crate::data::scenario::tasks::validate,
    },
    AuthoredBlock {
        key: "weatherTimeline",
        validate: crate::data::scenario::weather::validate,
    },
    AuthoredBlock {
        key: "audio",
        validate: crate::data::scenario::audio::validate,
    },
    AuthoredBlock {
        key: "spawnModules",
        validate: crate::data::scenario::spawn_modules::validate,
    },
    AuthoredBlock {
        key: "tacticalGraphics",
        validate: crate::data::scenario::tactical_graphics::validate,
    },
];

/// The authored blocks the compiled document MODELS with a typed field of its own, and which [`ExtensionBlocks`] must therefore not carry — see the module header's "two destinations".
pub const DOCUMENT_OWNED_BLOCKS: &[&str] = &["radioPlan", "winConditions"];

/// Is `key` an authored block?.
#[must_use]
pub fn is_authored_block(key: &str) -> bool {
    AUTHORED_BLOCKS.iter().any(|b| b.key == key)
}

/// Copy every authored block present in `env` onto the compiled payload root, verbatim.
pub fn copy_authored_blocks(env: &Value, dst: &mut Map<String, Value>) -> Vec<&'static str> {
    let mut copied = Vec::new();
    for block in AUTHORED_BLOCKS {
        let Some(value) = env.get(block.key) else {
            continue;
        };

        if value.is_null() {
            continue;
        }
        dst.insert(block.key.to_string(), value.clone());
        copied.push(block.key);
    }
    copied
}

/// Every field is an `Option`; `None` means "the author wrote no such block", which is what the emitter tests to decide between the authored value and the derivation it has always run.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AuthoredBlocks {
    /// Radio plan.
    pub radio_plan: Option<crate::data::scenario::radio_plan::AuthoredRadioPlan>,

    /// Win conditions.
    pub win_conditions: Option<crate::data::scenario::win_conditions::AuthoredWinConditions>,
}

impl AuthoredBlocks {
    /// Parse every document-modelled block out of a payload root.
    #[must_use]
    pub fn parse(payload: &Value) -> (Self, Vec<(&'static str, String)>) {
        let mut out = Self::default();
        let mut refusals = Vec::new();

        for block in AUTHORED_BLOCKS {
            if !DOCUMENT_OWNED_BLOCKS.contains(&block.key) {
                continue;
            }
            let Some(value) = payload.get(block.key) else {
                continue;
            };
            if value.is_null() {
                continue;
            }
            if let Err(clause) = (block.validate)(value) {
                refusals.push((block.key, clause));
                continue;
            }

            match block.key {
                "winConditions" => {
                    out.win_conditions = crate::data::scenario::win_conditions::parse(value).ok();
                }
                "radioPlan" => {
                    out.radio_plan = crate::data::scenario::radio_plan::parse(value).ok();
                }
                _ => {}
            }
        }

        (out, refusals)
    }
}

/// The optional authored blocks as they reach the compiled document — `#[serde(flatten)]`ed onto `ModMissionDocument` so each emits at the document ROOT, in [`AUTHORED_BLOCKS`] order.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ExtensionBlocks {
    /// `(key, value)` in [`AUTHORED_BLOCKS`] order. A `Vec` rather than a map because the order is the schema's and must be reproducible — `/compiled` is re-fetched by the game server and must not change under it.
    pub(super) blocks: Vec<(&'static str, Value)>,
}

impl ExtensionBlocks {
    /// Read the carrier's blocks out of a payload root — `flatten.rs`'s ONE call site.
    #[must_use]
    pub fn from_payload(payload: &Value) -> (Self, Vec<(&'static str, String)>) {
        let mut out = Self::default();
        let mut refusals = Vec::new();

        for block in AUTHORED_BLOCKS {
            if DOCUMENT_OWNED_BLOCKS.contains(&block.key) {
                continue;
            }
            let Some(value) = payload.get(block.key) else {
                continue;
            };
            if value.is_null() {
                continue;
            }
            match (block.validate)(value) {
                Ok(()) => out.set(block.key, value.clone()),
                Err(clause) => refusals.push((block.key, clause)),
            }
        }

        (out, refusals)
    }

    /// Place one block. Replaces an existing value for the same key rather than appending a second one, so the carrier cannot emit a duplicate key however it is driven.
    pub fn set(&mut self, key: &'static str, value: Value) {
        if let Some(slot) = self.blocks.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            self.blocks.push((key, value));
        }
    }

    /// The value carried for `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.blocks.iter().find(|(k, _)| *k == key).map(|(_, v)| v)
    }

    /// Is empty using the supplied domain data.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// How many blocks are carried.
    #[must_use]
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
}

impl Serialize for ExtensionBlocks {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.blocks.len()))?;
        for (key, value) in &self.blocks {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}
