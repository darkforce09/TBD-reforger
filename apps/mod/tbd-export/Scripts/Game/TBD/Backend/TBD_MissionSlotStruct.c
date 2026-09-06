//! Fixed gear ResourceNames of a slot loadout (T-068.11 compiled block).
//! Empty string = slot not set (the compiler omits empty fields; JsonLoadContext
//! leaves absent keys at the initializer).
//! @contract mission.schema.json#/$defs/slot (loadout.gear)
class TBD_SlotGearStruct
{
	string primary;  //!< Primary weapon ResourceName - engine weapon slot 0 (slotType "primary").
	string optic;    //!< Optic ResourceName. T-181.10 - mounted into the PRIMARY weapon's storage only.
	string magazine; //!< Magazine ResourceName. T-181.10 - loaded into the PRIMARY weapon's storage only.
	ref array<string> attachments; //!< T-310. Arsenal attachment ResourceNames. Empty/absent = none; Count() is presence.
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

	// T-674.2 -- the per-seat identity block (schemaVersion 1.3), the five T-180.1 values the
	// compile used to drop in silence (the T-216 ledger gap). Every one is OPTIONAL: the emitter
	// skips the key entirely when there is nothing wire-safe to say, so JsonLoadContext leaves the
	// field at "" and a pre-1.3 mission reads exactly as it did before.
	//
	// PRESENCE IS AN EMPTY-STRING TEST, never a null test. These are scalars, so the initializer
	// IS the sentinel -- the `ref <class>` allocation landmine documented on `loadout` above does
	// not apply, and `minLength: 1` on the schema side means an EMPTY value can never be authored:
	// flatten drops a blank whole rather than emitting it (`emit_wire_safe_identity`). So
	// non-empty == authored, with no third state to defend against.
	string callsign;  //!< This SEAT's own callsign. NOT `groupCallsign` above -- that is the SQUAD's, and conflating the two is the mistake the T-216 ledger was opened over.
	string rank;      //!< Rank ladder token, lowercase: private|corporal|sergeant|lieutenant|colonel and the two between. Enum-gated by the emitter, so an off-ladder value never arrives.
	string stance;    //!< Initial spawn pose: stand|crouch|prone. Enum-gated by the emitter.
	string unitName;  //!< Authored unit name for this seat. camelCase on the wire; JsonLoadContext binds by field NAME, so the spelling here IS the contract.
	string tag;       //!< TBD-only slot tag, mirroring the ORBAT-slot tag the events path carries (T-010).

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

	//------------------------------------------------------------------------------------------------
	//! T-674.2 -- did this slot author ANY of the five identity keys?
	//!
	//! One `||` per line rather than a chain, for the reason `TBD_SpawnManager.HasAuthoredLoadout`
	//! spells out: a long boolean chain is the shape of the measured "Formula too complex"
	//! landmine, whose second diagnostic is a misleading "Incompatible parameter".
	bool HasIdentity()
	{
		if (!callsign.IsEmpty()) return true;
		if (!rank.IsEmpty())     return true;
		if (!stance.IsEmpty())   return true;
		if (!unitName.IsEmpty()) return true;
		if (!tag.IsEmpty())      return true;
		return false;
	}

	//------------------------------------------------------------------------------------------------
	//! T-674.2 -- the best human label this seat can offer, most specific first:
	//! authored unit name, else its own callsign, else the derived display `id` (which always
	//! exists and already reads `faction:squad:role:n`).
	//!
	//! NOT a fallback to `groupCallsign`: that is the SQUAD's name, so using it here would print
	//! every seat of a squad under one identical label and read as though the identity had been
	//! applied when nothing was authored at all.
	string IdentityLabel()
	{
		if (!unitName.IsEmpty())
			return unitName;
		if (!callsign.IsEmpty())
			return callsign;
		return id;
	}
}
