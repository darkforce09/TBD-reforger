/**
 * @file TBD_LoadoutExportStruct.c
 * @brief The Arsenal loadout-export document, both schema branches, as read by the dev harness.
 *
 * Role: JSON DTOs for `loadout-export.schema.json`: the root with its `loadoutVersion`
 * discriminator, the v1 `gear` block, the v2 `wear` map and slot-indexed `weapons`.
 * Position: filled by `TBD_LoadoutEquipComponent` from `$profile:TBD_LoadoutTest.json`; cargo rows
 * reuse `TBD_SlotCargoStruct`.
 * State: none.  Invariants: field names are the JSON keys; `JsonLoadContext` leaves absent keys at
 * their initializer and allocates absent `ref` fields, so presence is an emptiness test, never a
 * null test on a nested object.
 */

//! The v1 `gear` object: each value a ResourceName, empty or null for none.
//! @contract loadout-export.schema.json#/$defs/gear
class TBD_LoadoutGearStruct
{
	string primary; //!< Primary weapon ResourceName (empty = none).
	string uniform; //!< Uniform ResourceName (empty = none).
	string vest;    //!< Vest ResourceName (empty = none).
	string helmet;  //!< Helmet ResourceName (empty = none).
	string optic;    //!< Optic ResourceName, mounted into the primary (empty = none).
	string magazine; //!< Magazine ResourceName, loaded into the primary (empty = none).
}

//! The v2 `wear` map, keyed by engine `LoadoutSlotInfo` names. The schema keeps the map open for
//! mod-added areas; only the areas `TBD_LoadoutApplication` equips are declared, and
//! `JsonLoadContext` skips the rest.
//! @contract loadout-export.schema.json#/oneOf/1/properties/wear
class TBD_LoadoutWearStruct
{
	string headCover;   //!< -> gear.helmet
	string jacket;      //!< -> gear.uniform
	string pants; //!< -> gear.pants
	string boots; //!< -> gear.boots
	string vest;        //!< -> gear.vest, unless armoredVest is worn
	string armoredVest; //!< -> gear.vest (wins; the locked single-vest rule)
	string backpack; //!< -> gear.backpack
	string handwear; //!< -> gear.handwear
}

//! One slot-indexed weapon of the v2 `weapons` array.
//! @contract loadout-export.schema.json#/$defs/weapon
class TBD_LoadoutWeaponStruct
{
	int slotIndex = -1;            //!< Engine weapon slot. -1 = key absent (schema minimum is 0).
	string slotType;               //!< "primary" / "secondary" / "grenade".
	string weapon;                 //!< Weapon ResourceName.
	string optic;                  //!< Primary (slot 0) only -- no other slot has sub-slots.
	string magazine;               //!< Primary (slot 0) only.
	ref array<string> attachments; //!< attachment ResourceNames; mounted only for the primary (slot 0)
}

//! The export root, both `oneOf` branches in one struct: `loadoutVersion` selects which fields
//! are authoritative, and the other branch's fields stay empty.
//! @contract loadout-export.schema.json#/
class TBD_LoadoutExportStruct
{
	string loadoutVersion;          //!< Export format version ("1" or "2").
	string modpackId;               //!< Source modpack id.
	ref TBD_LoadoutGearStruct gear; //!< v1: the authored gear slots. v2: DERIVED, unread here.
	ref TBD_LoadoutWearStruct wear;                 //!< Worn areas by engine slot name.
	ref array<ref TBD_LoadoutWeaponStruct> weapons; //!< Slot-indexed weapons.
	ref array<ref TBD_SlotCargoStruct> cargo;       //!< Container cargo rows {container,item,qty}.
}
