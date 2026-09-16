//! Role: input.
//! Position: `mission/ast` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Authored editor payload accepted by the compiler; unknown fields do not become game fields.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct EditorPayload {
    /// Zones.
    pub(in crate::data::scenario) zones: Vec<ZoneIn>,

    /// Entities.
    pub(in crate::data::scenario) entities: Vec<EntityIn>,

    /// Vehicles.
    pub(in crate::data::scenario) vehicles: Vec<VehicleIn>,

    /// Settings.
    pub(in crate::data::scenario) settings: Option<SettingsIn>,
    /// Editor.
    pub(in crate::data::scenario) editor: EditorGraph,

    /// Environment.
    pub(in crate::data::scenario) environment: serde_json::Value,

    /// A bare [`serde_json::Value`] for exactly the reason [`Self::environment`] is one: stored payloads are immutable, and a typed field here would let one wrong-typed key in an existing payload become a permanent `CompileError::Parse` → HTTP 500. The typing happens in `mission/win_conditions.rs`, which REPORTS a refusal and falls back to the derivation instead of failing the compile.
    #[serde(rename = "winConditions")]
    pub(in crate::data::scenario) win_conditions: Option<serde_json::Value>,

    /// Tasks.
    pub(in crate::data::scenario) tasks: Option<serde_json::Value>,

    /// Radio plan.
    #[serde(rename = "radioPlan")]
    pub(in crate::data::scenario) radio_plan: Option<serde_json::Value>,

    /// Weather timeline.
    #[serde(rename = "weatherTimeline")]
    pub(in crate::data::scenario) weather_timeline: Option<serde_json::Value>,

    /// Audio.
    pub(in crate::data::scenario) audio: Option<serde_json::Value>,

    /// Spawn modules.
    #[serde(rename = "spawnModules")]
    pub(in crate::data::scenario) spawn_modules: Option<serde_json::Value>,

    /// Tactical graphics.
    #[serde(rename = "tacticalGraphics")]
    pub(in crate::data::scenario) tactical_graphics: Option<serde_json::Value>,
}

impl EditorPayload {
    /// The authored blocks as a payload root `mission/extensions.rs` can read.
    pub(in crate::data::scenario) fn authored_block_value(
        &self,
        key: &str,
    ) -> Option<&serde_json::Value> {
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
    pub(in crate::data::scenario) fn authored_blocks_root(&self) -> serde_json::Value {
        let mut root = serde_json::Map::new();
        for block in crate::data::scenario::extensions::AUTHORED_BLOCKS {
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
pub(in crate::data::scenario) struct SettingsIn {
    /// Respawn.
    pub(in crate::data::scenario) respawn: Option<String>,
    /// Spectator policy.
    pub(in crate::data::scenario) spectator_policy: Option<String>,
    /// Night vision.
    pub(in crate::data::scenario) night_vision: Option<bool>,
}

/// Domain representation of editor graph.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct EditorGraph {
    /// Factions.
    pub(in crate::data::scenario) factions: Vec<FactionIn>,
    /// Squads.
    pub(in crate::data::scenario) squads: Vec<SquadIn>,
    /// Slots.
    pub(in crate::data::scenario) slots: Vec<SlotIn>,
}

/// Domain representation of faction in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(in crate::data::scenario) struct FactionIn {
    /// Id.
    pub(in crate::data::scenario) id: String,
    /// Key.
    pub(in crate::data::scenario) key: String,
    /// Name.
    pub(in crate::data::scenario) name: String,
    /// Squad ids.
    pub(in crate::data::scenario) squad_ids: Vec<String>,

    /// The row is the better home for a reason that outlives the convenience: the compiled `briefings` map is `additionalProperties`-open, so an entry naming a faction the author later DELETED still validates, and the compile would ship orders for a side that no longer exists. On the row that state is unrepresentable — delete the faction and its briefing goes with it.
    pub(in crate::data::scenario) briefing: Option<BriefingIn>,
}

/// Domain representation of squad in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(in crate::data::scenario) struct SquadIn {
    /// Id.
    pub(in crate::data::scenario) id: String,

    /// Editor faction row id this squad belongs to (`faction-{SIDE}` or a minted `f…` id).
    pub(in crate::data::scenario) faction_id: String,
    /// Callsign.
    pub(in crate::data::scenario) callsign: String,
    /// Name.
    pub(in crate::data::scenario) name: String,
    /// Slot ids.
    pub(in crate::data::scenario) slot_ids: Vec<String>,

    /// Leader slot id.
    pub(in crate::data::scenario) leader_slot_id: serde_json::Value,
}

/// Domain representation of slot in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(in crate::data::scenario) struct SlotIn {
    /// Id.
    pub(in crate::data::scenario) id: String,
    /// Index.
    pub(in crate::data::scenario) index: i64,
    /// Role.
    pub(in crate::data::scenario) role: String,
    /// Asset id.
    pub(in crate::data::scenario) asset_id: String,
    /// Position.
    pub(in crate::data::scenario) position: PositionIn,

    /// Loadout.
    pub(in crate::data::scenario) loadout: Option<serde_json::Value>,

    /// Tag.
    pub(in crate::data::scenario) tag: serde_json::Value,

    /// Callsign.
    pub(in crate::data::scenario) callsign: serde_json::Value,
    /// Rank.
    pub(in crate::data::scenario) rank: serde_json::Value,
    /// Stance.
    pub(in crate::data::scenario) stance: serde_json::Value,

    /// Unit name.
    pub(in crate::data::scenario) unit_name: serde_json::Value,
}

/// Domain representation of position in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct PositionIn {
    /// X.
    pub(in crate::data::scenario) x: f64,
    /// Y.
    pub(in crate::data::scenario) y: f64,
    /// Z.
    pub(in crate::data::scenario) z: f64,
    /// Rotation.
    pub(in crate::data::scenario) rotation: f64,
}

/// Domain representation of entity in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct EntityIn {
    /// Id.
    pub(in crate::data::scenario) id: String,
    /// Alias.
    pub(in crate::data::scenario) alias: String,
    /// Resource name.
    #[serde(rename = "resourceName")]
    pub(in crate::data::scenario) resource_name: String,
    /// Position.
    pub(in crate::data::scenario) position: Option<PositionIn>,
    /// Faction.
    pub(in crate::data::scenario) faction: String,
}

/// Domain representation of vehicle in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct VehicleIn {
    /// Id.
    pub(in crate::data::scenario) id: String,
    /// Resource name.
    #[serde(rename = "resourceName")]
    pub(in crate::data::scenario) resource_name: String,
    /// Position.
    pub(in crate::data::scenario) position: Option<PositionIn>,

    /// Map-placed side marker (`faction-BLUFOR`). Optional — squad-attached vehicles may omit it.
    #[serde(rename = "factionId")]
    pub(in crate::data::scenario) faction_id: String,

    /// Squad id.
    #[serde(rename = "squadId")]
    pub(in crate::data::scenario) squad_id: String,

    /// `$defs/entityInventory` rows verbatim — become `entity.inventory` with no transform.
    pub(in crate::data::scenario) cargo: Vec<EntityInventoryIn>,

    /// Crew.
    pub(in crate::data::scenario) crew: serde_json::Value,
}

/// Domain representation of entity inventory in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct EntityInventoryIn {
    /// Item.
    pub(in crate::data::scenario) item: String,
    /// Qty.
    pub(in crate::data::scenario) qty: i64,
}

/// The three prose fields are `Option<String>` so an ABSENT key and an authored `""` stay distinguishable all the way through to the emitted bytes. Both are legal and the mod renders both as nothing, but they are different authorial acts and the compiled document should not claim the author blanked a field they never opened.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct BriefingIn {
    /// Situation.
    pub(in crate::data::scenario) situation: Option<String>,
    /// Mission.
    pub(in crate::data::scenario) mission: Option<String>,
    /// Execution.
    pub(in crate::data::scenario) execution: Option<String>,
    /// Markers.
    pub(in crate::data::scenario) markers: Vec<MarkerIn>,
}

/// Domain representation of marker in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct MarkerIn {
    /// X.
    pub(in crate::data::scenario) x: f64,
    /// Z.
    pub(in crate::data::scenario) z: f64,
    /// Icon.
    pub(in crate::data::scenario) icon: String,
    /// Label.
    pub(in crate::data::scenario) label: String,
}

/// One authored payload `zones[]` row — mirrors `mission.schema.json#/$defs/zone`.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct ZoneIn {
    /// Id.
    pub(in crate::data::scenario) id: String,
    /// Kind.
    #[serde(rename = "type")]
    pub(in crate::data::scenario) kind: String,
    /// Label.
    pub(in crate::data::scenario) label: String,
    /// Faction.
    pub(in crate::data::scenario) faction: String,
    /// Shape.
    pub(in crate::data::scenario) shape: Option<ShapeIn>,
    /// Rules.
    pub(in crate::data::scenario) rules: Option<serde_json::Value>,
}

/// Domain representation of shape in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct ShapeIn {
    /// Circle.
    pub(in crate::data::scenario) circle: Option<CircleIn>,
    /// Polygon.
    pub(in crate::data::scenario) polygon: Option<Vec<Vec<f64>>>,
}

/// Domain representation of circle in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::data::scenario) struct CircleIn {
    /// X.
    pub(in crate::data::scenario) x: f64,
    /// Z.
    pub(in crate::data::scenario) z: f64,
    /// R.
    pub(in crate::data::scenario) r: f64,
}
