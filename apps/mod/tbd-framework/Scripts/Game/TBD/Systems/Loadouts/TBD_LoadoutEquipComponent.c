/**
 * @file TBD_LoadoutEquipComponent.c
 * @brief Dev harness: dress a spawned test character with the Arsenal loadout export.
 *
 * Role: reads `$profile:TBD_LoadoutTest.json` (the Arsenal `loadout-export.json` download,
 * `loadout-export.schema.json`, both `loadoutVersion` branches), maps it onto a
 * `TBD_SlotLoadoutStruct` and runs `TBD_LoadoutApplication` over an empty test character, so the
 * harness exercises the same equip path as slot bodies. Its lines carry the tag
 * `[TBD][Loadout][TestNPC]`; slot bodies log `[TBD][Loadout][Slot]`.  Position: a game-mode
 * component on `Prefabs/Systems/TBD_GameMode.et`, run by a Workbench play of
 * `Missions/TBD_Dev_POC.conf`; spawns at 6400/6400, where the player lands.
 * State: the test character and its application, on the server.  Invariants: off unless
 * `m_bRunLoadoutTest` is set; never runs on a client; a v2 document is read from its own `wear`,
 * `weapons` and `cargo` fields, never from its derived `gear` block.
 */

//! Editor class of `TBD_LoadoutEquipComponent`.
[ComponentEditorProps(category: "TBD/Framework", description: "Dev test: equip $profile:TBD_LoadoutTest.json gear onto a spawned empty US character.")]
class TBD_LoadoutEquipComponentClass : SCR_BaseGameModeComponentClass {}

//! Dev harness that dresses a test character from the Arsenal loadout export.
class TBD_LoadoutEquipComponent : SCR_BaseGameModeComponent
{
	protected static const string LOADOUT_PATH = "$profile:TBD_LoadoutTest.json"; //!< the Arsenal export the harness reads
	protected static const string EXPECTED_MODPACK_ID = "00000000-0000-4000-a000-000000000001"; //!< modpack id the web exporter and registry emit

	[Attribute("0", desc: "Run the loadout equip test on play (dev only -- default OFF; do not ship enabled on TBD_GameMode).")]
	bool m_bRunLoadoutTest; //!< default false; true runs the harness on play

	[Attribute("{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et", desc: "Empty/minimal US body to equip onto (no baked kit).")]
	ResourceName m_sTestCharacter; //!< default the bare US character prefab

	[Attribute("6400 0 6400", desc: "World origin for the test spawn (TBD_Dev_POC game mode coords).")]
	vector m_vSpawnOrigin; //!< world metres; the Y component is replaced by the surface height

	protected IEntity m_Character; //!< the spawned test character
	protected ref TBD_LoadoutApplication m_App; //!< held until the application is done

	//! Schedule the harness 3 s after init when it is enabled, so the world surface and replication
	//! are ready.
	//! @authority server
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Authority only -- entity spawn + equip must run on the server.
		if (TBD_Authority.IsClient())
			return;

		if (!m_bRunLoadoutTest)
			return;

		// Defer so the world surface + replication are ready (mirrors TBD_RegistryPocComponent).
		GetGame().GetCallqueue().CallLater(RunLoadoutTest, 3000, false);
	}

	//! Read and check the export file, build the slot loadout, spawn the test character and run the
	//! application. Every failure is one `[TBD][Loadout]` ERROR line and ends the run.
	//! @authority server
	protected void RunLoadoutTest()
	{
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

		// loadout-export.schema.json is a oneOf over loadoutVersion "1" and "2" -- both are real,
		// shipping shapes, and the web Arsenal writes "2". Reject anything else rather than
		// equipping a future shape as if we understood it.
		if (doc.loadoutVersion != "1" && doc.loadoutVersion != "2")
		{
			Print("[TBD][Loadout] FAILED: unsupported loadoutVersion '" + doc.loadoutVersion + "' (expected '1' or '2')", LogLevel.ERROR);
			return;
		}
		// A loadout built for a different modpack likely references prefab GUIDs this mod can't
		// resolve -- warn (don't hard-fail, so a known-good cross-pack test can still proceed).
		if (doc.modpackId != EXPECTED_MODPACK_ID)
			Print("[TBD][Loadout] WARNING: modpackId '" + doc.modpackId + "' != expected '" + EXPECTED_MODPACK_ID + "' -- prefabs may not resolve", LogLevel.WARNING);

		TBD_SlotLoadoutStruct loadout = BuildSlotLoadout(doc);
		if (!loadout)
			return; // BuildSlotLoadout already named the fault

		m_Character = SpawnTestCharacter();
		if (!m_Character)
		{
			Print("[TBD][Loadout] FAILED: could not spawn test character " + m_sTestCharacter, LogLevel.ERROR);
			return;
		}

		m_App = new TBD_LoadoutApplication(m_Character, loadout, "[TBD][Loadout][TestNPC]", "loadout-test");
		m_App.Run();
	}

	//! Map the export document onto the slot-loadout shape `TBD_LoadoutApplication` runs. v1 copies
	//! its `gear` block. v2 reads its own `wear`, `weapons` and `cargo`, which carry the launcher,
	//! sidearm, throwable, pants, boots, gloves, backpack and cargo that the derived `gear` block
	//! cannot. Weapons match on the (slotIndex, slotType) pair of the Mission Creator's Arsenal
	//! weapon slots and the mission compiler: slots 0 and 1 are both `primary`, so the index
	//! separates rifle from launcher and the type rejects a mis-authored row.
	//! @return the loadout, or null when the document describes none (already logged)
	protected TBD_SlotLoadoutStruct BuildSlotLoadout(TBD_LoadoutExportStruct doc)
	{
		TBD_SlotGearStruct gear = new TBD_SlotGearStruct();
		TBD_SlotLoadoutStruct loadout = new TBD_SlotLoadoutStruct();
		loadout.gear = gear;

		if (doc.loadoutVersion == "1")
		{
			// v1 has no wear map, second weapon slot or cargo, so the gear block is everything.
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
					// optic/magazine exist on the primary rifle alone -- the other three slots
					// have no sub-slots in the editor, so nothing is dropped by not reading them.
					gear.optic = w.optic;
					gear.magazine = w.magazine;
					if (w.attachments && w.attachments.Count() > 0)
						gear.attachments = w.attachments;
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
					Print(string.Format("[TBD][Loadout] WARNING: %1 names weapon slot (%2, %3), which is not one of the four the equip path knows -- NOT equipped", w.weapon, w.slotIndex, w.slotType), LogLevel.WARNING);
					continue;
				}

				// Only the primary's attachments reach `gear.attachments`; others are named and dropped.
				if (!(w.slotIndex == 0 && w.slotType == "primary") && w.attachments && !w.attachments.IsEmpty())
					Print(string.Format("[TBD][Loadout] WARNING: %1 attachment(s) authored on %2 are NOT mounted -- this path mounts only the primary's optic and magazine", w.attachments.Count(), w.weapon), LogLevel.WARNING);
			}
		}

		// Cargo rows share the compiled `{container, item, qty}` shape and the same four containers.
		loadout.cargo = doc.cargo;
		return loadout;
	}

	//! Spawn the test character prefab on the ground at `m_vSpawnOrigin`.
	//! @return the character, or null when the prefab does not load
	//! @authority server
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
