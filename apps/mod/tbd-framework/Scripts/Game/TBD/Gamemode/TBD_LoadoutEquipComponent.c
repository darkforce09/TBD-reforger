/**
 * TBD_LoadoutEquipComponent.c - T-068.5 / T-068.5.1 Virtual Arsenal loadout equip test.
 *
 * Reads $profile:TBD_LoadoutTest.json (the web Arsenal "loadout-export.json" download,
 * packages/tbd-schema/schema/loadout-export.schema.json) and equips its four gear slots
 * (primary / uniform / vest / helmet) onto a freshly spawned, otherwise-empty US character.
 *
 * T-068.5.1 — VISUAL FIX: the previous pass used SCR_InventoryStorageManagerComponent.TryInsertItem,
 * which returns true while the item sits in storage (not worn) → character spawned naked despite
 * "equip OK" logs. The wear path uses the real equip APIs and a deferred worn-verify gate.
 *
 * T-068.12 — the equip/verify/cargo machinery moved to the shared
 * TBD_LoadoutApplication (TBD_LoadoutEquipHelper.c) so this dev harness and the
 * SpawnManager PLAYER path run identical code; this component keeps only the
 * $profile file read, the v1 contract guards, and the test-NPC spawn. Its log
 * lines are tagged [TBD][Loadout][TestNPC] (the player path logs
 * [TBD][Loadout][Player]) so E2E evidence is unambiguous.
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
}

//! DTO mirrors loadout-export.schema.json root.
//! @contract loadout-export.schema.json#/
class TBD_LoadoutExportStruct
{
	string loadoutVersion;          //!< Export format version (const "1").
	string modpackId;               //!< Source modpack id.
	ref TBD_LoadoutGearStruct gear; //!< The four gear slots.
}

//------------------------------------------------------------------------------------------------
class TBD_LoadoutEquipComponent : SCR_BaseGameModeComponent
{
	protected static const string LOADOUT_PATH = "$profile:TBD_LoadoutTest.json";
	//! Canonical modpack id the web exporter / registry emit (T-122 T14/M10).
	protected static const string EXPECTED_MODPACK_ID = "00000000-0000-4000-a000-000000000001";

	[Attribute("0", desc: "Run the loadout equip test on play (dev only — default OFF; do not ship enabled on TBD_GameMode).")]
	bool m_bRunLoadoutTest;

	[Attribute("{520EC961A090BBD5}Prefabs/Characters/Factions/BLUFOR/US_Army/Character_US_Base.et", desc: "Empty/minimal US body to equip onto (no baked kit).")]
	ResourceName m_sTestCharacter;

	[Attribute("6400 0 6400", desc: "World origin for the test spawn (TBD_Dev_POC game mode coords).")]
	vector m_vSpawnOrigin;

	protected IEntity m_Character;
	protected ref TBD_LoadoutApplication m_App; // strong ref until its settle tick completes

	//------------------------------------------------------------------------------------------------
	//! @authority server — the dev equip test spawns and dresses the test NPC server-side only.
	override void OnPostInit(IEntity owner)
	{
		super.OnPostInit(owner);

		// Authority only — entity spawn + equip must run on the server.
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
		if (!ctx.ReadValue("", doc) || !doc.gear)
		{
			Print("[TBD][Loadout] FAILED: parse error in TBD_LoadoutTest.json", LogLevel.ERROR);
			return;
		}

		Print(string.Format("[TBD][Loadout] Loaded TBD_LoadoutTest.json (version %1, modpack %2)", doc.loadoutVersion, doc.modpackId));

		// --- A1.1: contract guards (T-122 M9/M10) ---------------------------------------------
		// loadoutVersion is pinned to "1" by loadout-export.schema.json; reject a future shape
		// rather than equipping it as if it were v1.
		if (doc.loadoutVersion != "1")
		{
			Print("[TBD][Loadout] FAILED: unsupported loadoutVersion '" + doc.loadoutVersion + "' (expected '1')", LogLevel.ERROR);
			return;
		}
		// A loadout built for a different modpack likely references prefab GUIDs this mod can't
		// resolve — warn (don't hard-fail, so a known-good cross-pack test can still proceed).
		if (doc.modpackId != EXPECTED_MODPACK_ID)
			Print("[TBD][Loadout] WARNING: modpackId '" + doc.modpackId + "' != expected '" + EXPECTED_MODPACK_ID + "' — prefabs may not resolve", LogLevel.WARNING);

		// --- spawn the empty test character ---------------------------------------------------
		m_Character = SpawnTestCharacter();
		if (!m_Character)
		{
			Print("[TBD][Loadout] FAILED: could not spawn test character " + m_sTestCharacter, LogLevel.ERROR);
			return;
		}

		// --- A2-A5: run the shared application (equip → settle tick → worn-verify) -----------
		// v1 gear maps 1:1 onto the T-068.11 gear block; no cargo in the v1 file contract.
		TBD_SlotGearStruct gear = new TBD_SlotGearStruct();
		gear.primary = doc.gear.primary;
		gear.uniform = doc.gear.uniform;
		gear.vest = doc.gear.vest;
		gear.helmet = doc.gear.helmet;
		TBD_SlotLoadoutStruct loadout = new TBD_SlotLoadoutStruct();
		loadout.gear = gear;

		m_App = new TBD_LoadoutApplication(m_Character, loadout, "[TBD][Loadout][TestNPC]", "loadout-test");
		m_App.Run();
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
