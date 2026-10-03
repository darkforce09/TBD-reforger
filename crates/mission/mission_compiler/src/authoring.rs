//! **Role:** The editor-input structs the game-document compiler parses a stored payload into,
//! private to the crate.
//! **Position:** `mission_compiler::authoring` in the `mission_compiler` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

/// Authored editor payload accepted by the compiler; unknown fields do not become game fields.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct EditorPayload {
    /// Zones.
    pub(crate) zones: Vec<ZoneIn>,

    /// Entities.
    pub(crate) entities: Vec<EntityIn>,

    /// Vehicles.
    pub(crate) vehicles: Vec<VehicleIn>,

    /// Settings.
    pub(crate) settings: Option<SettingsIn>,
    /// Editor.
    pub(crate) editor: EditorGraph,

    /// Environment.
    pub(crate) environment: serde_json::Value,

    /// A bare [`serde_json::Value`] for exactly the reason [`Self::environment`] is one: stored payloads are immutable, and a typed field here would let one wrong-typed key in an existing payload become a permanent `CompileError::Parse` → HTTP 500. The typing happens in `mission/win_conditions.rs`, which REPORTS a refusal and falls back to the derivation instead of failing the compile.
    #[serde(rename = "winConditions")]
    pub(crate) win_conditions: Option<serde_json::Value>,

    /// Tasks.
    pub(crate) tasks: Option<serde_json::Value>,

    /// Radio plan.
    #[serde(rename = "radioPlan")]
    pub(crate) radio_plan: Option<serde_json::Value>,

    /// Weather timeline.
    #[serde(rename = "weatherTimeline")]
    pub(crate) weather_timeline: Option<serde_json::Value>,

    /// Audio.
    pub(crate) audio: Option<serde_json::Value>,

    /// Spawn modules.
    #[serde(rename = "spawnModules")]
    pub(crate) spawn_modules: Option<serde_json::Value>,

    /// Tactical graphics.
    #[serde(rename = "tacticalGraphics")]
    pub(crate) tactical_graphics: Option<serde_json::Value>,
}

impl EditorPayload {
    /// The authored blocks as a payload root `mission/extensions.rs` can read.
    pub(crate) fn authored_block_value(&self, key: &str) -> Option<&serde_json::Value> {
        match key {
            "winConditions" => self.win_conditions.as_ref(),
            "tasks" => self.tasks.as_ref(),
            "radioPlan" => self.radio_plan.as_ref(),
            "weatherTimeline" => self.weather_timeline.as_ref(),
            "audio" => self.audio.as_ref(),
            "spawnModules" => self.spawn_modules.as_ref(),
            "tacticalGraphics" => self.tactical_graphics.as_ref(),
            _ => None,
        }
    }

    /// Authored blocks root using the supplied domain data.
    pub(crate) fn authored_blocks_root(&self) -> serde_json::Value {
        let mut root = serde_json::Map::new();
        for block in mission_model::authored_blocks::AUTHORED_BLOCKS {
            if let Some(v) = self.authored_block_value(block.key) {
                root.insert(block.key.to_string(), v.clone());
            }
        }
        serde_json::Value::Object(root)
    }
}

/// Authored `settings` row — the three schema properties, all optional.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct SettingsIn {
    /// Respawn.
    pub(crate) respawn: Option<String>,
    /// Spectator policy.
    pub(crate) spectator_policy: Option<String>,
    /// Night vision.
    pub(crate) night_vision: Option<bool>,
}

/// Domain representation of editor graph.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct EditorGraph {
    /// Factions.
    pub(crate) factions: Vec<FactionIn>,
    /// Squads.
    pub(crate) squads: Vec<SquadIn>,
    /// Slots.
    pub(crate) slots: Vec<SlotIn>,
}

/// Domain representation of faction in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct FactionIn {
    /// Id.
    pub(crate) id: String,
    /// Key.
    pub(crate) key: String,
    /// Name.
    pub(crate) name: String,
    /// Squad ids.
    pub(crate) squad_ids: Vec<String>,

    /// The row is the better home for a reason that outlives the convenience: the compiled `briefings` map is `additionalProperties`-open, so an entry naming a faction the author later DELETED still validates, and the compile would ship orders for a side that no longer exists. On the row that state is unrepresentable — delete the faction and its briefing goes with it.
    pub(crate) briefing: Option<BriefingIn>,
}

/// Domain representation of squad in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct SquadIn {
    /// Id.
    pub(crate) id: String,

    /// Editor faction row id this squad belongs to (`faction-{SIDE}` or a minted `f…` id).
    pub(crate) faction_id: String,
    /// Callsign.
    pub(crate) callsign: String,
    /// Name.
    pub(crate) name: String,
    /// Slot ids.
    pub(crate) slot_ids: Vec<String>,

    /// Leader slot id.
    pub(crate) leader_slot_id: serde_json::Value,
}

/// Domain representation of slot in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct SlotIn {
    /// Id.
    pub(crate) id: String,
    /// Index.
    pub(crate) index: i64,
    /// Role.
    pub(crate) role: String,
    /// Asset id.
    pub(crate) asset_id: String,
    /// Position.
    pub(crate) position: PositionIn,

    /// Loadout.
    pub(crate) loadout: Option<serde_json::Value>,

    /// Tag.
    pub(crate) tag: serde_json::Value,

    /// Callsign.
    pub(crate) callsign: serde_json::Value,
    /// Rank.
    pub(crate) rank: serde_json::Value,
    /// Stance.
    pub(crate) stance: serde_json::Value,

    /// Unit name.
    pub(crate) unit_name: serde_json::Value,
}

/// Domain representation of position in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(crate) struct PositionIn {
    /// X.
    pub(crate) x: f64,
    /// Y.
    pub(crate) y: f64,
    /// Z.
    pub(crate) z: f64,
    /// Rotation.
    pub(crate) rotation: f64,
}

/// Domain representation of entity in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct EntityIn {
    /// Id.
    pub(crate) id: String,
    /// Alias.
    pub(crate) alias: String,
    /// Resource name.
    #[serde(rename = "resourceName")]
    pub(crate) resource_name: String,
    /// Position.
    pub(crate) position: Option<PositionIn>,
    /// Faction.
    pub(crate) faction: String,
}

/// Domain representation of vehicle in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct VehicleIn {
    /// Id.
    pub(crate) id: String,
    /// Resource name.
    #[serde(rename = "resourceName")]
    pub(crate) resource_name: String,
    /// Position.
    pub(crate) position: Option<PositionIn>,

    /// Map-placed side marker (`faction-BLUFOR`). Optional — squad-attached vehicles may omit it.
    #[serde(rename = "factionId")]
    pub(crate) faction_id: String,

    /// Squad id.
    #[serde(rename = "squadId")]
    pub(crate) squad_id: String,

    /// `$defs/entityInventory` rows verbatim — become `entity.inventory` with no transform.
    pub(crate) cargo: Vec<EntityInventoryIn>,

    /// Crew.
    pub(crate) crew: serde_json::Value,
}

/// Domain representation of entity inventory in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(crate) struct EntityInventoryIn {
    /// Item.
    pub(crate) item: String,
    /// Qty.
    pub(crate) qty: i64,
}

/// The three prose fields are `Option<String>` so an ABSENT key and an authored `""` stay distinguishable all the way through to the emitted bytes. Both are legal and the mod renders both as nothing, but they are different authorial acts and the compiled document should not claim the author blanked a field they never opened.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(crate) struct BriefingIn {
    /// Situation.
    pub(crate) situation: Option<String>,
    /// Mission.
    pub(crate) mission: Option<String>,
    /// Execution.
    pub(crate) execution: Option<String>,
    /// Markers.
    pub(crate) markers: Vec<MarkerIn>,
}

/// Domain representation of marker in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(crate) struct MarkerIn {
    /// X.
    pub(crate) x: f64,
    /// Z.
    pub(crate) z: f64,
    /// Icon.
    pub(crate) icon: String,
    /// Label.
    pub(crate) label: String,
}

/// One authored payload `zones[]` row — mirrors `mission.schema.json#/$defs/zone`.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ZoneIn {
    /// Id.
    pub(crate) id: String,
    /// Kind.
    #[serde(rename = "type")]
    pub(crate) kind: String,
    /// Label.
    pub(crate) label: String,
    /// Faction.
    pub(crate) faction: String,
    /// Shape.
    pub(crate) shape: Option<ShapeIn>,
    /// Rules.
    pub(crate) rules: Option<serde_json::Value>,
}

/// Domain representation of shape in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ShapeIn {
    /// Circle.
    pub(crate) circle: Option<CircleIn>,
    /// Polygon.
    pub(crate) polygon: Option<Vec<Vec<f64>>>,
}

/// Domain representation of circle in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(crate) struct CircleIn {
    /// X.
    pub(crate) x: f64,
    /// Z.
    pub(crate) z: f64,
    /// R.
    pub(crate) r: f64,
}
