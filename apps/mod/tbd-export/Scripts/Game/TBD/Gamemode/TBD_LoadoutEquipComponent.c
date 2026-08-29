/**
 * TBD_LoadoutEquipComponent.c - T-068.5 / T-068.5.1 Virtual Arsenal loadout equip test.
 *
 * Reads $profile:TBD_LoadoutTest.json (the web Arsenal "loadout-export.json" download,
 * packages/tbd-schema/schema/loadout-export.schema.json) and equips its four gear slots
 * (primary / uniform / vest / helmet) onto a freshly spawned, otherwise-empty US character.
 *
 * T-068.5.1 - VISUAL FIX: the previous pass used SCR_InventoryStorageManagerComponent.TryInsertItem,
 * which returns true while the item sits in storage (not worn) -> character spawned naked despite
 * "equip OK" logs. The wear path uses the real equip APIs and a deferred worn-verify gate.
 *
 * T-068.12 - the equip/verify/cargo machinery moved to the shared
 * TBD_LoadoutApplication (TBD_LoadoutEquipHelper.c) so this dev harness and the
 * SpawnManager PLAYER path run identical code; this component keeps only the
 * $profile file read, the contract guards, and the test-NPC spawn. Its log
 * lines are tagged [TBD][Loadout][TestNPC] (the production slot-body path players receive
 * logs [TBD][Loadout][Slot] - the tag TBD_SpawnManager.SpawnSlotBody hands to
 * TBD_LoadoutApplication) so E2E evidence is unambiguous. T-612: this comment used to name
 * [TBD][Loadout][Player], which no Print has ever emitted - greps built from it match
 * nothing on a working pass.
 *
 * T-199 - THIS READER NOW ACCEPTS THE FILE THE WEB ARSENAL ACTUALLY WRITES.
 * loadout-export.schema.json is a oneOf over loadoutVersion "1" and "2", and the Arsenal
 * download is a v2 document (the editor holds a wear map, four slot-indexed weapons and
 * cargo - none of which the v1 branch can express). This component accepted "1" alone and
 * hard-failed everything else, so the ONLY consumer of the download refused the download.
 * Both branches are read now, and v2 is read from its OWN fields rather than from the
 * derived legacy gear block, so the launcher / sidearm / throwable / pants / boots / gloves
 * / backpack / cargo that T-182 taught the equip path to carry actually reach it.
 *
 * Server-only, dev-gated. Wired onto Prefabs/Systems/TBD_GameMode.et so a Workbench wb_play of
 * Missions/TBD_Dev_POC.conf runs it. Spawn @ 6400/6400 = the TBD_Dev_POC game-mode coords (the
 * player lands there), so the dressed pawn is visible without flying the camera.
 */

[ComponentEditorProps(category: "TBD/Framework", description: "Dev test: equip $profile:TBD_LoadoutTest.json gear onto a spawned empty US character.")]
class TBD_LoadoutEquipComponentClass : SCR_BaseGameModeComponentClass {}

//------------------------------------------------------------------------------------------------
//! DTO mirrors loadout-export.schema.json "gear" object (each value a ResourceName or null/"").
//! @contract loadout-export.schema.json#/$defs/gear
class TBD_LoadoutGearStruct
{
	string primary; //!< Primary weapon ResourceName (empty = none).
	string uniform; //!< Uniform ResourceName (empty = none).
	string vest;    //!< Vest ResourceName (empty = none).
	string helmet;  //!< Helmet ResourceName (empty = none).
	// T-199 - the schema has always allowed these two optional gear keys and this reader has
	// always ignored them, so a v1 file that asked for a scope got a bare rifle. The equip path
	// mounts both (TBD_LoadoutApplication.BeginWeaponPhase), so there is nothing to defer.
	string optic;    //!< Optic ResourceName, mounted into the primary (empty = none).
	string magazine; //!< Magazine ResourceName, loaded into the primary (empty = none).
}

//! DTO mirrors the loadout-export v2 "wear" map - the canonical engine LoadoutSlotInfo keys
//! the schema documents. That map is pattern-open so mod-added LoadoutAreaType subclasses stay
//! representable; this reader declares only the areas TBD_LoadoutApplication can equip, and
//! JsonLoadContext ignores the rest rather than failing the read.
//! @contract loadout-export.schema.json#/oneOf/1/properties/wear
class TBD_LoadoutWearStruct
{
	string headCover;   //!< -> gear.helmet
	string jacket;      //!< -> gear.uniform
	string pants;
	string boots;
	string vest;        //!< -> gear.vest, unless armoredVest is worn
	string armoredVest; //!< -> gear.vest (wins; the locked single-vest rule)
	string backpack;
	string handwear;
}

//! DTO mirrors loadout-export.schema.json #/$defs/weapon - one slot-indexed weapon.
//! @contract loadout-export.schema.json#/$defs/weapon
class TBD_LoadoutWeaponStruct
{
	int slotIndex = -1;            //!< Engine weapon slot. -1 = key absent (schema minimum is 0).
	string slotType;               //!< "primary" / "secondary" / "grenade".
	string weapon;                 //!< Weapon ResourceName.
	string optic;                  //!< Primary (slot 0) only - no other slot has sub-slots.
	string magazine;               //!< Primary (slot 0) only.
	ref array<string> attachments; //!< T-197 attachment set - see the WARNING in BuildSlotLoadout.
}

//! DTO mirrors loadout-export.schema.json root - BOTH oneOf branches in one struct.
//!
//! T-199 - `loadoutVersion` is the discriminator, so a reader that must accept both branches
//! declares every branch's keys and lets the guard decide which set is authoritative.
//! JsonLoadContext maps by name and leaves absent keys at their initializer, so the v2 fields
//! stay empty on a v1 document and vice versa. Presence of a ref field is NEVER the test -
//! JsonLoadContext over-allocates them (the T-181.41 finding).
//! @contract loadout-export.schema.json#/
class TBD_LoadoutExportStruct
{
	string loadoutVersion;          //!< Export format version ("1" or "2").
	string modpackId;               //!< Source modpack id.
	ref TBD_LoadoutGearStruct gear; //!< v1: the authored gear slots. v2: DERIVED, unread here.
	// --- v2 (T-068.10.4) - the authoritative fields on a v2 document ---
	ref TBD_LoadoutWearStruct wear;                 //!< Worn areas by engine slot name.
	ref array<ref TBD_LoadoutWeaponStruct> weapons; //!< Slot-indexed weapons.
	ref array<ref TBD_SlotCargoStruct> cargo;       //!< Container cargo rows {container,item,qty}.
}

//------------------------------------------------------------------------------------------------
class TBD_LoadoutEquipComponent : SCR_BaseGameModeComponent
{
	protected static const string LOADOUT_PATH = "$profile:TBD_LoadoutTest.json";
	//! Canonical modpack id the web exporter / registry emit (T-122 T14/M10).
	protected static const string EXPECTED_MODPACK_ID = "00000000-0000-4000-a000-000000000001";

	[Attribute("0", desc: "Run the loadout equip test on play (dev only - default OFF; do not ship enabled on TBD_GameMode).")]
	bool m_bRunLoadoutTest;

	[Attribute("{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et", desc: "Empty/minimal US body to equip onto (no baked kit).")]
	ResourceName m_sTestCharacter;

	[Attribute("6400 0 6400", desc: "World origin for the test spawn (TBD_Dev_POC game mode coords).")]
	vector m_vSpawnOrigin;

	protected IEntity m_Character;
	protected ref TBD_LoadoutApplication m_App; // strong ref until its settle tick completes

	//------------------------------------------------------------------------------------------------
	//! @authority server - the dev equip test spawns and dresses the test NPC server-side only.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Authority only - entity spawn + equip must run on the server.
		if (RplSession.Mode() == RplMode.Client)
			return;

		if (!m_bRunLoadoutTest)
			return;

		// Defer so the world surface + replication are ready (mirrors TBD_RegistryPocComponent).
		GetGame().GetCallqueue().CallLater(RunLoadoutTest, 3000, false);
	}

	//------------------------------------------------------------------------------------------------
	protected void RunLoadoutTest()
	{
		// --- A1: read + parse $profile:TBD_LoadoutTest.json -----------------------------------
		if (!FileIO.FileExists(LOADOUT_PATH))
		{
			Print("[TBD][Loadout] FAILED: no file at " + LOADOUT_PATH, LogLevel.ERROR);
			return;
		}

		JsonLoadContext ctx = new JsonLoadContext();
		if (!ctx.LoadFromFile(LOADOUT_PATH))
		{
			Print("[TBD][Loadout] FAILED: could not read " + LOADOUT_PATH, LogLevel.ERROR);
			return;
		}

		TBD_LoadoutExportStruct doc = new TBD_LoadoutExportStruct();
		if (!ctx.ReadValue("", doc))
		{
			Print("[TBD][Loadout] FAILED: parse error in TBD_LoadoutTest.json", LogLevel.ERROR);
			return;
		}

		Print(string.Format("[TBD][Loadout] Loaded TBD_LoadoutTest.json (version %1, modpack %2)", doc.loadoutVersion, doc.modpackId));

		// --- A1.1: contract guards (T-122 M9/M10; T-199) --------------------------------------
		// loadout-export.schema.json is a oneOf over loadoutVersion "1" and "2" - both are real,
		// shipping shapes, and the web Arsenal writes "2". Reject anything else rather than
		// equipping a future shape as if we understood it.
		if (doc.loadoutVersion != "1" && doc.loadoutVersion != "2")
		{
			Print("[TBD][Loadout] FAILED: unsupported loadoutVersion '" + doc.loadoutVersion + "' (expected '1' or '2')", LogLevel.ERROR);
			return;
		}
		// A loadout built for a different modpack likely references prefab GUIDs this mod can't
		// resolve - warn (don't hard-fail, so a known-good cross-pack test can still proceed).
		if (doc.modpackId != EXPECTED_MODPACK_ID)
			Print("[TBD][Loadout] WARNING: modpackId '" + doc.modpackId + "' != expected '" + EXPECTED_MODPACK_ID + "' - prefabs may not resolve", LogLevel.WARNING);

		TBD_SlotLoadoutStruct loadout = BuildSlotLoadout(doc);
		if (!loadout)
			return; // BuildSlotLoadout already named the fault

		// --- spawn the empty test character ---------------------------------------------------
		m_Character = SpawnTestCharacter();
		if (!m_Character)
		{
			Print("[TBD][Loadout] FAILED: could not spawn test character " + m_sTestCharacter, LogLevel.ERROR);
			return;
		}

		// --- A2-A5: run the shared application (equip -> settle tick -> worn-verify) -----------
		m_App = new TBD_LoadoutApplication(m_Character, loadout, "[TBD][Loadout][TestNPC]", "loadout-test");
		m_App.Run();
	}

	//------------------------------------------------------------------------------------------------
	//! Map the export document onto the compiled slot-loadout shape TBD_LoadoutApplication runs.
	//! Returns null only when the document cannot describe a loadout at all (already logged).
	//!
	//! T-199 - WHY v2 IS NOT READ THROUGH ITS `gear` BLOCK.
	//! A v2 document carries a DERIVED legacy `gear` block precisely so a v1-shaped reader keeps
	//! working, and reading that would have been four lines. It would also have thrown away exactly
	//! what T-182 widened this equip path to carry: the launcher, the sidearm, the throwable, pants,
	//! boots, gloves, the backpack and every cargo row - none of which fit `gear`'s six
	//! schema-pinned keys. So v2 is read from its own fields and the derived block is left to
	//! readers that only know v1.
	//!
	//! The (slotIndex, slotType) PAIRS are the editor's own table (arsenal_rules.rs WEAPON_SLOTS)
	//! and the same pairs the compiler selects on (mission/flatten.rs mod_slot_loadout) - keep all
	//! three byte-identical. Matching the pair rather than the index alone matters because slots 0
	//! and 1 are both slotType "primary" (two untyped long slots), so the index is what separates
	//! rifle from launcher while slotType is what stops a mis-authored row landing in the wrong key.
	protected TBD_SlotLoadoutStruct BuildSlotLoadout(TBD_LoadoutExportStruct doc)
	{
		TBD_SlotGearStruct gear = new TBD_SlotGearStruct();
		TBD_SlotLoadoutStruct loadout = new TBD_SlotLoadoutStruct();
		loadout.gear = gear;

		if (doc.loadoutVersion == "1")
		{
			// v1 gear maps 1:1 onto the T-068.11 gear block; the v1 branch has no wear map, no
			// second weapon slot and no cargo, so there is nothing else in the file to carry.
			if (!doc.gear)
			{
				Print("[TBD][Loadout] FAILED: v1 document carries no gear block", LogLevel.ERROR);
				return null;
			}
			gear.primary = doc.gear.primary;
			gear.uniform = doc.gear.uniform;
			gear.vest = doc.gear.vest;
			gear.helmet = doc.gear.helmet;
			gear.optic = doc.gear.optic;
			gear.magazine = doc.gear.magazine;
			return loadout;
		}

		// --- v2: wear map + slot-indexed weapons + cargo rows ---------------------------------
		if (doc.wear)
		{
			gear.uniform = doc.wear.jacket;
			// The locked single-vest rule: a character wears one vest, and the armored one wins.
			gear.vest = doc.wear.armoredVest;
			if (gear.vest.IsEmpty())
				gear.vest = doc.wear.vest;
			gear.helmet = doc.wear.headCover;
			gear.pants = doc.wear.pants;
			gear.boots = doc.wear.boots;
			gear.handwear = doc.wear.handwear;
			gear.backpack = doc.wear.backpack;
		}

		if (doc.weapons)
		{
			foreach (TBD_LoadoutWeaponStruct w : doc.weapons)
			{
				if (!w || w.weapon.IsEmpty())
					continue;

				if (w.slotIndex == 0 && w.slotType == "primary")
				{
					gear.primary = w.weapon;
					// optic/magazine exist on the primary rifle alone - the other three slots
					// have no sub-slots in the editor, so nothing is dropped by not reading them.
					gear.optic = w.optic;
					gear.magazine = w.magazine;
				}
				else if (w.slotIndex == 1 && w.slotType == "primary")
					gear.launcher = w.weapon;
				else if (w.slotIndex == 2 && w.slotType == "secondary")
					gear.handgun = w.weapon;
				else if (w.slotIndex == 3 && w.slotType == "grenade")
					gear.throwable = w.weapon;
				else
				{
					// A slot pair this equip path has no equip call for. Landing it in one of the
					// four we DO know would put the item somewhere nobody asked for, so it is
					// named and skipped instead.
					Print(string.Format("[TBD][Loadout] WARNING: %1 names weapon slot (%2, %3), which is not one of the four the equip path knows - NOT equipped", w.weapon, w.slotIndex, w.slotType), LogLevel.WARNING);
					continue;
				}

				// T-197 - attachments are authored per weapon, but this equip path mounts only the
				// primary's optic and magazine (TBD_SlotGearStruct carries no attachment field, so
				// the compiled mission cannot express them either). Say so by name; a file whose
				// suppressor silently vanished is exactly the kind of quiet loss T-181.10 banned.
				if (w.attachments && !w.attachments.IsEmpty())
					Print(string.Format("[TBD][Loadout] WARNING: %1 attachment(s) authored on %2 are NOT mounted - this path mounts only the primary's optic and magazine", w.attachments.Count(), w.weapon), LogLevel.WARNING);
			}
		}

		// {container,item,qty} is byte-identical to the compiled cargo row, and the container
		// vocabulary is the same closed four (TBD_LoadoutApplication.GarmentForContainer).
		loadout.cargo = doc.cargo;
		return loadout;
	}

	//------------------------------------------------------------------------------------------------
	protected IEntity SpawnTestCharacter()
	{
		Resource resource = Resource.Load(m_sTestCharacter);
		if (!resource || !resource.IsValid())
		{
			Print("[TBD][Loadout] Resource.Load failed for character " + m_sTestCharacter, LogLevel.ERROR);
			return null;
		}

		float x = m_vSpawnOrigin[0];
		float z = m_vSpawnOrigin[2];
		float y = GetGame().GetWorld().GetSurfaceY(x, z);
		vector pos = Vector(x, y, z);

		EntitySpawnParams params = new EntitySpawnParams();
		params.TransformMode = ETransformMode.WORLD;
		Math3D.MatrixIdentity4(params.Transform);
		params.Transform[3] = pos;

		IEntity ent = GetGame().SpawnEntityPrefab(resource, GetGame().GetWorld(), params);
		if (ent)
			Print(string.Format("[TBD][Loadout] test spawn %1 (%2) @ %3", ent.GetID().ToString(), m_sTestCharacter, pos.ToString()));

		return ent;
	}

}
