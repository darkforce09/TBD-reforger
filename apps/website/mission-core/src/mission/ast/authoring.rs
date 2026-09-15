//! Role: input.
//! Position: `mission/ast` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

/// Authored editor payload accepted by the compiler; unknown fields do not become game fields.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct EditorPayload {
    /// Zones.
    pub(in crate::mission) zones: Vec<ZoneIn>,

    /// Entities.
    pub(in crate::mission) entities: Vec<EntityIn>,

    /// Vehicles.
    pub(in crate::mission) vehicles: Vec<VehicleIn>,

    /// Settings.
    pub(in crate::mission) settings: Option<SettingsIn>,
    /// Editor.
    pub(in crate::mission) editor: EditorGraph,

    /// Environment.
    pub(in crate::mission) environment: serde_json::Value,

    /// A bare [`serde_json::Value`] for exactly the reason [`Self::environment`] is one: stored payloads are immutable, and a typed field here would let one wrong-typed key in an existing payload become a permanent `CompileError::Parse` → HTTP 500. The typing happens in `mission/win_conditions.rs`, which REPORTS a refusal and falls back to the derivation instead of failing the compile.
    #[serde(rename = "winConditions")]
    pub(in crate::mission) win_conditions: Option<serde_json::Value>,

    /// Tasks.
    pub(in crate::mission) tasks: Option<serde_json::Value>,

    /// Radio plan.
    #[serde(rename = "radioPlan")]
    pub(in crate::mission) radio_plan: Option<serde_json::Value>,

    /// Weather timeline.
    #[serde(rename = "weatherTimeline")]
    pub(in crate::mission) weather_timeline: Option<serde_json::Value>,

    /// Audio.
    pub(in crate::mission) audio: Option<serde_json::Value>,

    /// Spawn modules.
    #[serde(rename = "spawnModules")]
    pub(in crate::mission) spawn_modules: Option<serde_json::Value>,

    /// Tactical graphics.
    #[serde(rename = "tacticalGraphics")]
    pub(in crate::mission) tactical_graphics: Option<serde_json::Value>,
}

impl EditorPayload {
    /// The authored blocks as a payload root `mission/extensions.rs` can read.
    pub(in crate::mission) fn authored_block_value(&self, key: &str) -> Option<&serde_json::Value> {
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
    pub(in crate::mission) fn authored_blocks_root(&self) -> serde_json::Value {
        let mut root = serde_json::Map::new();
        for block in crate::mission::extensions::AUTHORED_BLOCKS {
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
pub(in crate::mission) struct SettingsIn {
    /// Respawn.
    pub(in crate::mission) respawn: Option<String>,
    /// Spectator policy.
    pub(in crate::mission) spectator_policy: Option<String>,
    /// Night vision.
    pub(in crate::mission) night_vision: Option<bool>,
}

/// Domain representation of editor graph.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct EditorGraph {
    /// Factions.
    pub(in crate::mission) factions: Vec<FactionIn>,
    /// Squads.
    pub(in crate::mission) squads: Vec<SquadIn>,
    /// Slots.
    pub(in crate::mission) slots: Vec<SlotIn>,
}

/// Domain representation of faction in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(in crate::mission) struct FactionIn {
    /// Id.
    pub(in crate::mission) id: String,
    /// Key.
    pub(in crate::mission) key: String,
    /// Name.
    pub(in crate::mission) name: String,
    /// Squad ids.
    pub(in crate::mission) squad_ids: Vec<String>,

    /// The row is the better home for a reason that outlives the convenience: the compiled `briefings` map is `additionalProperties`-open, so an entry naming a faction the author later DELETED still validates, and the compile would ship orders for a side that no longer exists. On the row that state is unrepresentable — delete the faction and its briefing goes with it.
    pub(in crate::mission) briefing: Option<BriefingIn>,
}

/// Domain representation of squad in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(in crate::mission) struct SquadIn {
    /// Id.
    pub(in crate::mission) id: String,

    /// Editor faction row id this squad belongs to (`faction-{SIDE}` or a minted `f…` id).
    pub(in crate::mission) faction_id: String,
    /// Callsign.
    pub(in crate::mission) callsign: String,
    /// Name.
    pub(in crate::mission) name: String,
    /// Slot ids.
    pub(in crate::mission) slot_ids: Vec<String>,

    /// Leader slot id.
    pub(in crate::mission) leader_slot_id: serde_json::Value,
}

/// Domain representation of slot in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub(in crate::mission) struct SlotIn {
    /// Id.
    pub(in crate::mission) id: String,
    /// Index.
    pub(in crate::mission) index: i64,
    /// Role.
    pub(in crate::mission) role: String,
    /// Asset id.
    pub(in crate::mission) asset_id: String,
    /// Position.
    pub(in crate::mission) position: PositionIn,

    /// Loadout.
    pub(in crate::mission) loadout: Option<serde_json::Value>,

    /// Tag.
    pub(in crate::mission) tag: serde_json::Value,

    /// Callsign.
    pub(in crate::mission) callsign: serde_json::Value,
    /// Rank.
    pub(in crate::mission) rank: serde_json::Value,
    /// Stance.
    pub(in crate::mission) stance: serde_json::Value,

    /// Unit name.
    pub(in crate::mission) unit_name: serde_json::Value,
}

/// Domain representation of position in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct PositionIn {
    /// X.
    pub(in crate::mission) x: f64,
    /// Y.
    pub(in crate::mission) y: f64,
    /// Z.
    pub(in crate::mission) z: f64,
    /// Rotation.
    pub(in crate::mission) rotation: f64,
}

/// Domain representation of entity in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct EntityIn {
    /// Id.
    pub(in crate::mission) id: String,
    /// Alias.
    pub(in crate::mission) alias: String,
    /// Resource name.
    #[serde(rename = "resourceName")]
    pub(in crate::mission) resource_name: String,
    /// Position.
    pub(in crate::mission) position: Option<PositionIn>,
    /// Faction.
    pub(in crate::mission) faction: String,
}

/// Domain representation of vehicle in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct VehicleIn {
    /// Id.
    pub(in crate::mission) id: String,
    /// Resource name.
    #[serde(rename = "resourceName")]
    pub(in crate::mission) resource_name: String,
    /// Position.
    pub(in crate::mission) position: Option<PositionIn>,

    /// Map-placed side marker (`faction-BLUFOR`). Optional — squad-attached vehicles may omit it.
    #[serde(rename = "factionId")]
    pub(in crate::mission) faction_id: String,

    /// Squad id.
    #[serde(rename = "squadId")]
    pub(in crate::mission) squad_id: String,

    /// `$defs/entityInventory` rows verbatim — become `entity.inventory` with no transform.
    pub(in crate::mission) cargo: Vec<EntityInventoryIn>,

    /// Crew.
    pub(in crate::mission) crew: serde_json::Value,
}

/// Domain representation of entity inventory in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct EntityInventoryIn {
    /// Item.
    pub(in crate::mission) item: String,
    /// Qty.
    pub(in crate::mission) qty: i64,
}

/// The three prose fields are `Option<String>` so an ABSENT key and an authored `""` stay distinguishable all the way through to the emitted bytes. Both are legal and the mod renders both as nothing, but they are different authorial acts and the compiled document should not claim the author blanked a field they never opened.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct BriefingIn {
    /// Situation.
    pub(in crate::mission) situation: Option<String>,
    /// Mission.
    pub(in crate::mission) mission: Option<String>,
    /// Execution.
    pub(in crate::mission) execution: Option<String>,
    /// Markers.
    pub(in crate::mission) markers: Vec<MarkerIn>,
}

/// Domain representation of marker in.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct MarkerIn {
    /// X.
    pub(in crate::mission) x: f64,
    /// Z.
    pub(in crate::mission) z: f64,
    /// Icon.
    pub(in crate::mission) icon: String,
    /// Label.
    pub(in crate::mission) label: String,
}

/// One authored payload `zones[]` row — mirrors `mission.schema.json#/$defs/zone`.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct ZoneIn {
    /// Id.
    pub(in crate::mission) id: String,
    /// Kind.
    #[serde(rename = "type")]
    pub(in crate::mission) kind: String,
    /// Label.
    pub(in crate::mission) label: String,
    /// Faction.
    pub(in crate::mission) faction: String,
    /// Shape.
    pub(in crate::mission) shape: Option<ShapeIn>,
    /// Rules.
    pub(in crate::mission) rules: Option<serde_json::Value>,
}

/// Domain representation of shape in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct ShapeIn {
    /// Circle.
    pub(in crate::mission) circle: Option<CircleIn>,
    /// Polygon.
    pub(in crate::mission) polygon: Option<Vec<Vec<f64>>>,
}

/// Domain representation of circle in.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
pub(in crate::mission) struct CircleIn {
    /// X.
    pub(in crate::mission) x: f64,
    /// Z.
    pub(in crate::mission) z: f64,
    /// R.
    pub(in crate::mission) r: f64,
}
