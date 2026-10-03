//! The names a caller of the mission model imports with `use mission_model::prelude::*;`: the
//! error, every id, the compiled rows and the block registry.

pub use crate::authored_blocks::{
    AUTHORED_BLOCKS, AuthoredBlock, AuthoredBlocks, DOCUMENT_OWNED_BLOCKS, ExtensionBlocks,
};
pub use crate::compiled::entities::{
    ModEntity, ModEntityInventory, ModNet, ModOrbatFaction, ModOrbatGroup, ModOrbatRole, ModSlot,
    ModSlotCargo, ModSlotGear, ModSlotLoadout, ModVehicle, ModVehicleSeat,
};
pub use crate::compiled::mission::{
    ModBriefing, ModCircle, ModEnvironment, ModFaction, ModFlow, ModMarker, ModMeta, ModRadioPlan,
    ModSettings, ModWinConditions, ModZone, ModZoneShape,
};
pub use crate::ids::{
    AudioEmitterId, FactionPresetId, MarkerId, MissionId, MissionTemplateId, MusicCueId, NetId,
    SlotId, SlotUid, SpawnModuleId, TacticalGraphicId, TaskId, TriggerId, ZoneId,
};
pub use crate::{Error, Result};
