//! Fixed gear ResourceNames of a slot loadout (T-068.11 compiled block).
//! Empty string = slot not set (the compiler omits empty fields; JsonLoadContext
//! leaves absent keys at the initializer).
//! @contract mission.schema.json#/$defs/slot (loadout.gear)
class TBD_SlotGearStruct
{
	string primary;  //!< Primary weapon ResourceName - engine weapon slot 0 (slotType "primary").
	string optic;    //!< Optic ResourceName. T-181.10 - mounted into the PRIMARY weapon's storage only.
	string magazine; //!< Magazine ResourceName. T-181.10 - loaded into the PRIMARY weapon's storage only.
	// T-182 - the other three authored weapon slots. The editor has always written all four
	// (arsenal_rules.rs WEAPON_SLOTS); the compiler selected only slot 0 and dropped these three,
	// so a player authored with an RPG spawned without it. Names are the EDITOR's own vocabulary
	// so the compiled document reads the same words the Arsenal UI shows. None of the three carry
	// optic/magazine sub-slots today - those ride the primary alone.
	string launcher;  //!< Launcher ResourceName - engine weapon slot 1 (the second untyped long slot).
	string handgun;   //!< Sidearm ResourceName - engine weapon slot 2 (slotType "secondary").
	string throwable; //!< Throwable ResourceName - engine weapon slot 3 (slotType "grenade").
	string uniform;  //!< Jacket/uniform ResourceName.
	string vest;     //!< Vest ResourceName (armoredVest wins in the compiler).
	string helmet;   //!< Head cover ResourceName.
	string pants;    //!< Pants ResourceName (A3 - wear map arrives complete).
	string boots;    //!< Boots ResourceName (A3).
	string handwear; //!< Gloves ResourceName (A3).
	string backpack; //!< Worn backpack ResourceName (A3).
}

//! One container cargo row (loadout-export v2 {container,item,qty}).
//! @contract mission.schema.json#/$defs/slot (loadout.cargo[])
class TBD_SlotCargoStruct
{
	string container; //!< Wear container key: vest / pants / jacket / backpack.
	string item;      //!< Item ResourceName.
	int qty = 1;      //!< Units to insert (>= 1).
}

//! Optional per-slot Arsenal loadout (T-068.11): the kit prefab stays the base
//! character; T-068.12 layers this on the spawned HUMAN player.
//! T-181.10 - applied by TBD_LoadoutApplication on EVERY spawn of the slot body (mission
//! load and every rematerialization), so a life never inherits the previous one's state.
//! @contract mission.schema.json#/$defs/slot (loadout)
class TBD_SlotLoadoutStruct
{
	ref TBD_SlotGearStruct gear;               //!< Fixed gear block (null = none).
	ref array<ref TBD_SlotCargoStruct> cargo;  //!< Container cargo rows (null = none).
}

//! One flattened ORBAT slot instance with exact spawn position (mission slots[]).
//! Field names must equal the JSON keys (JsonLoadContext maps by name).
//! @contract mission.schema.json#/$defs/slot
class TBD_MissionSlotStruct
{
	//! Sentinel for "y absent from JSON". JsonLoadContext leaves a missing key at the
	//! field initializer, and no real ASL height approaches -1e6 m, so the initializer
	//! doubles as the presence flag (standard JSON cannot carry NaN/Infinity).
	//! Golden: `golden-missions/slot-y-absent-and-present.json` (T-249) - one slot omits y,
	//! one authors it; schema gate refuses if that fixture is missing or one-sided.
	static const float Y_ABSENT = -1000000;

	string id;            //!< Human-readable label: {faction}:{groupCallsign}:{role}:{index} - DERIVED each compile.
	string uid;           //!< Stable slot identity (B1): the editor doc slot id, survives recompiles. Empty on pre-B1 documents.
	string faction;       //!< Faction key (matches mission factions[].key).
	string groupCallsign; //!< Owning squad callsign.
	string role;          //!< Role label within the squad.
	string kit;           //!< Loadout alias (kit:<id>).
	float x;              //!< Spawn world X, metres.
	float z;              //!< Spawn world Z, metres.
	float y = -1000000;   //!< Optional spawn height, metres ASL (schema 1.2). Y_ABSENT when not in JSON.
	float headingDeg;     //!< Spawn heading, degrees.
	ref TBD_SlotLoadoutStruct loadout; //!< Optional Arsenal loadout (T-068.11; null when absent).

	//------------------------------------------------------------------------------------------------
	//! True when the mission JSON carried an explicit y for this slot.
	bool HasJsonY()
	{
		return y != Y_ABSENT;
	}

	//------------------------------------------------------------------------------------------------
	//! B1 - the durable key for spawn points / rosters / logs: uid when present
	//! (survives recompiles), else the derived display id (pre-B1 documents).
	string Key()
	{
		if (!uid.IsEmpty())
			return uid;
		return id;
	}
}
